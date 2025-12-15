const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs').promises;
const Store = require('electron-store');

const store = new Store();

function createWindow() {
  const win = new BrowserWindow({
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
    win.loadFile('project-selector.html');
  } else {
    // Show editor
    win.loadFile('index.html');
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
