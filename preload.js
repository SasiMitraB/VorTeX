const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('api', {
  openFiles: () => ipcRenderer.invoke('dialog:openFiles'),
  writeFile: (filePath, content) => ipcRenderer.invoke('file:write', filePath, content),
  saveAs: (defaultPath, content) => ipcRenderer.invoke('dialog:saveFile', defaultPath, content)
});
