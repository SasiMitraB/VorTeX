const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('api', {
  openFiles: () => ipcRenderer.invoke('dialog:openFiles'),
  writeFile: (filePath, content) => ipcRenderer.invoke('file:write', filePath, content),
  saveAs: (defaultPath, content) => ipcRenderer.invoke('dialog:saveFile', defaultPath, content),
  openFolder: () => ipcRenderer.invoke('dialog:openFolder'),
  readTree: (rootPath) => ipcRenderer.invoke('fs:tree', rootPath),
  readFileAt: (filePath) => ipcRenderer.invoke('fs:readFile', filePath),
  configGet: (key) => ipcRenderer.invoke('config:get', key),
  configSet: (key, value) => ipcRenderer.invoke('config:set', key, value),
  scanProjects: (projectsFolder) => ipcRenderer.invoke('projects:scan', projectsFolder)
});
