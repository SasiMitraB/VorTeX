const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs').promises;
const Store = require('electron-store');

// LaTeX Services
const FileWatcher = require('./src/services/FileWatcher');
const semanticIndex = require('./src/services/SemanticIndex');
const { searchLabels, searchBibEntries } = require('./src/services/FuzzyMatcher');

const store = new Store();
let fileWatcher = null;
let mainWindow = null;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1000,
    height: 700,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      webSecurity: false // Allow file:// protocol for PDF previews
    }
  });

  // Check if we should show project selector or editor
  const projectsFolder = store.get('projectsFolder');
  const currentProject = store.get('currentProject');
  
  if (!projectsFolder || !currentProject) {
    // Show project selector
    mainWindow.loadFile('project-selector.html');
  } else {
    // Show editor and initialize file watching
    mainWindow.loadFile('index.html');
    initializeLatexServices(currentProject);
  }
}

app.whenReady().then(() => {
  createWindow();

  app.on('activate', function () {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', function () {
  if (process.platform !== 'darwin') app.quit();
});

ipcMain.handle('dialog:openFiles', async () => {
  const result = await dialog.showOpenDialog({
    properties: ['openFile', 'multiSelections']
  });
  if (result.canceled) return [];
  const files = await Promise.all(
    result.filePaths.map(async (p) => ({ path: p, content: await fs.readFile(p, 'utf-8') }))
  );
  return files;
});

ipcMain.handle('dialog:openFolder', async () => {
  const result = await dialog.showOpenDialog({
    properties: ['openDirectory']
  });
  if (result.canceled || !result.filePaths.length) return null;
  return result.filePaths[0];
});

async function buildTree(dirPath) {
  const entries = await fs.readdir(dirPath, { withFileTypes: true });
  const children = await Promise.all(entries.map(async (entry) => {
    const full = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      return {
        name: entry.name,
        path: full,
        type: 'folder',
        children: await buildTree(full)
      };
    }
    return {
      name: entry.name,
      path: full,
      type: 'file'
    };
  }));
  return children.sort((a, b) => {
    if (a.type !== b.type) return a.type === 'folder' ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}

ipcMain.handle('fs:tree', async (event, rootPath) => {
  if (!rootPath) return [];
  return await buildTree(rootPath);
});

ipcMain.handle('fs:readFile', async (event, filePath) => {
  const content = await fs.readFile(filePath, 'utf-8');
  return { path: filePath, content };
});

ipcMain.handle('file:write', async (event, filePath, content) => {
  await fs.writeFile(filePath, content, 'utf-8');
  return { success: true };
});

ipcMain.handle('dialog:saveFile', async (event, defaultPath, content) => {
  const res = await dialog.showSaveDialog({ defaultPath });
  if (res.canceled || !res.filePath) return { canceled: true };
  await fs.writeFile(res.filePath, content, 'utf-8');
  return { canceled: false, filePath: res.filePath };
});

ipcMain.handle('config:get', async (event, key) => {
  return store.get(key);
});

ipcMain.handle('config:set', async (event, key, value) => {
  store.set(key, value);
  return { success: true };
});

ipcMain.handle('projects:scan', async (event, projectsFolder) => {
  if (!projectsFolder) return [];
  try {
    const entries = await fs.readdir(projectsFolder, { withFileTypes: true });
    const projects = [];
    
    for (const entry of entries) {
      if (!entry.isDirectory()) continue;
      const projectPath = path.join(projectsFolder, entry.name);
      
      // Find preview PDF (prefer main.pdf, fallback to any PDF)
      let previewPdf = null;
      try {
        const files = await fs.readdir(projectPath);
        const mainPdf = files.find(f => f.toLowerCase() === 'main.pdf');
        if (mainPdf) {
          previewPdf = path.join(projectPath, mainPdf);
        } else {
          const anyPdf = files.find(f => f.toLowerCase().endsWith('.pdf'));
          if (anyPdf) previewPdf = path.join(projectPath, anyPdf);
        }
      } catch (err) {
        // Ignore read errors for individual projects
      }
      
      projects.push({
        name: entry.name,
        path: projectPath,
        previewPdf
      });
    }
    
    return projects.sort((a, b) => a.name.localeCompare(b.name));
  } catch (err) {
    console.error('Error scanning projects:', err);
    return [];
  }
});

// ============================================
// LaTeX Services Initialization & IPC Handlers
// ============================================

/**
 * Initialize LaTeX services for a project
 */
async function initializeLatexServices(projectPath) {
  try {
    console.log('Initializing LaTeX services for:', projectPath);
    
    // Initialize semantic index
    await semanticIndex.initProject(projectPath);
    
    // Create file watcher
    fileWatcher = new FileWatcher();
    
    // Listen for file changes
    fileWatcher.on('file-change', async (event) => {
      console.log('File change:', event.type, event.path);
      
      if (event.type === 'deleted') {
        semanticIndex.removeFile(event.path);
      } else {
        // Read file and update index
        try {
          const content = await fs.readFile(event.path, 'utf-8');
          await semanticIndex.updateFile(event.path, content);
        } catch (err) {
          console.error('Error reading file for indexing:', err);
        }
      }
      
      // Notify renderer
      if (mainWindow && !mainWindow.isDestroyed()) {
        mainWindow.webContents.send('file-change', event);
      }
    });
    
    fileWatcher.on('ready', async (files) => {
      console.log(`File watcher ready with ${files.length} files`);
      
      // Initial indexing of all files
      await indexAllFiles(files);
      
      // Notify renderer that index is ready
      if (mainWindow && !mainWindow.isDestroyed()) {
        mainWindow.webContents.send('index-ready', semanticIndex.getStats());
      }
    });
    
    // Start watching
    await fileWatcher.watchProject(projectPath);
  } catch (error) {
    console.error('Error initializing LaTeX services:', error);
  }
}

/**
 * Index all files in parallel
 */
async function indexAllFiles(files) {
  const BATCH_SIZE = 10;
  
  for (let i = 0; i < files.length; i += BATCH_SIZE) {
    const batch = files.slice(i, i + BATCH_SIZE);
    await Promise.all(batch.map(async (filePath) => {
      try {
        const content = await fs.readFile(filePath, 'utf-8');
        await semanticIndex.updateFile(filePath, content);
      } catch (err) {
        console.error(`Error indexing ${filePath}:`, err);
      }
    }));
  }
  
  console.log('Initial indexing complete:', semanticIndex.getStats());
}

/**
 * Stop LaTeX services
 */
async function stopLatexServices() {
  if (fileWatcher) {
    await fileWatcher.stopWatching();
    fileWatcher = null;
  }
  semanticIndex.clear();
}

// IPC: Get completion data (labels or citations)
ipcMain.handle('latex:getCompletionData', async (event, type, currentFile) => {
  try {
    if (type === 'labels') {
      return semanticIndex.getAllLabels();
    } else if (type === 'citations') {
      return semanticIndex.getAllCitations();
    } else if (type === 'sections') {
      return semanticIndex.getSectionsForFile(currentFile);
    }
    return [];
  } catch (error) {
    console.error('Error getting completion data:', error);
    return [];
  }
});

// IPC: Fuzzy search
ipcMain.handle('latex:fuzzySearch', async (event, query, type, currentFile) => {
  try {
    if (type === 'labels') {
      const labels = semanticIndex.getAllLabels();
      return searchLabels(query, labels, currentFile);
    } else if (type === 'citations') {
      const citations = semanticIndex.getAllCitations();
      return searchBibEntries(query, citations);
    }
    return [];
  } catch (error) {
    console.error('Error in fuzzy search:', error);
    return [];
  }
});

// IPC: Reindex a specific file
ipcMain.handle('latex:reindexFile', async (event, filePath) => {
  try {
    const content = await fs.readFile(filePath, 'utf-8');
    await semanticIndex.updateFile(filePath, content);
    return { success: true, stats: semanticIndex.getStats() };
  } catch (error) {
    console.error('Error reindexing file:', error);
    return { success: false, error: error.message };
  }
});

// IPC: Get index stats
ipcMain.handle('latex:getStats', async () => {
  return semanticIndex.getStats();
});

// IPC: Initialize services for a project (called when switching projects)
ipcMain.handle('latex:initProject', async (event, projectPath) => {
  await stopLatexServices();
  await initializeLatexServices(projectPath);
  return { success: true };
});

// Clean up on app quit
app.on('before-quit', async () => {
  await stopLatexServices();
});
