const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs').promises;

function createWindow() {
  const win = new BrowserWindow({
    width: 1000,
    height: 700,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  });

  win.loadFile('index.html');
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
