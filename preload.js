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
  scanProjects: (projectsFolder) => ipcRenderer.invoke('projects:scan', projectsFolder),
  openCSV: () => ipcRenderer.invoke('dialog:openCSV')
});

// LaTeX services API
contextBridge.exposeInMainWorld('latexServices', {
  // Subscribe to file changes
  onFileChange: (callback) => {
    ipcRenderer.on('file-change', (event, data) => callback(data));
  },

  // Subscribe to index ready event
  onIndexReady: (callback) => {
    ipcRenderer.on('index-ready', (event, stats) => callback(stats));
  },

  // Get completion data (labels or citations)
  getCompletionData: (type, currentFile) =>
    ipcRenderer.invoke('latex:getCompletionData', type, currentFile),

  // Get sections for a file
  getSections: (filePath) =>
    ipcRenderer.invoke('latex:getCompletionData', 'sections', filePath),

  // Fuzzy search
  fuzzySearch: (query, type, currentFile) =>
    ipcRenderer.invoke('latex:fuzzySearch', query, type, currentFile),

  // Force re-index a file
  reindexFile: (filePath) => ipcRenderer.invoke('latex:reindexFile', filePath),

  // Get index statistics
  getStats: () => ipcRenderer.invoke('latex:getStats'),

  // Initialize services for a project
  initProject: (projectPath) => ipcRenderer.invoke('latex:initProject', projectPath),

  // Remove listeners
  removeAllListeners: () => {
    ipcRenderer.removeAllListeners('file-change');
    ipcRenderer.removeAllListeners('index-ready');
  },
});

contextBridge.exposeInMainWorld('menuAPI', {
  onInsertTable: (callback) => ipcRenderer.on('menu:insert-table', callback)
});
