const { app, BrowserWindow, ipcMain, dialog, protocol } = require('electron');
const path = require('path');
const Store = require('electron-store');

// Modular imports
const { createApplicationMenu } = require('./src/main/menu');
const { registerIpcHandlers } = require('./src/main/ipc');
const latexManager = require('./src/main/latexManager');

const store = new Store();
let mainWindow = null;

// Register scheme as privileged
// protocol.registerSchemesAsPrivileged([
//   { scheme: 'vortex', privileges: { secure: true, standard: true, supportFetchAPI: true, bypassCSP: true } }
// ]);

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

  // Share window instance with LatexManager for events
  latexManager.setMainWindow(mainWindow);

  // Check if we should show project selector or editor
  const projectsFolder = store.get('projectsFolder');
  const currentProject = store.get('currentProject');
  const skipSelector = process.env.SKIP_SELECTOR === 'true';

  if (!skipSelector && (!projectsFolder || !currentProject)) {
    // Show project selector
    mainWindow.loadFile('project-selector.html');
  } else {
    // Show editor and initialize file watching
    mainWindow.loadFile('index.html');
    if (currentProject) {
      latexManager.initialize(currentProject);
    }
  }
}

app.whenReady().then(() => {
  createApplicationMenu(app, () => mainWindow);
  registerIpcHandlers(ipcMain, dialog, store);
  createWindow();

  app.on('activate', function () {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', function () {
  if (process.platform !== 'darwin') app.quit();
});

// Clean up on app quit
app.on('before-quit', async () => {
  await latexManager.stop();
});
