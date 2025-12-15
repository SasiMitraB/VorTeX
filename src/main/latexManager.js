const fs = require('fs').promises;
const FileWatcher = require('../services/FileWatcher');
const semanticIndex = require('../services/SemanticIndex');

class LatexManager {
    constructor() {
        this.fileWatcher = null;
        this.mainWindow = null;
    }

    setMainWindow(window) {
        this.mainWindow = window;
    }

    async initialize(projectPath) {
        try {
            console.log('Initializing LaTeX services for:', projectPath);

            // Initialize semantic index
            await semanticIndex.initProject(projectPath);

            // Create file watcher
            this.fileWatcher = new FileWatcher();

            // Listen for file changes
            this.fileWatcher.on('file-change', async (event) => {
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
                if (this.mainWindow && !this.mainWindow.isDestroyed()) {
                    this.mainWindow.webContents.send('file-change', event);
                }
            });

            this.fileWatcher.on('ready', async (files) => {
                console.log(`File watcher ready with ${files.length} files`);

                // Initial indexing of all files
                await this.indexAllFiles(files);

                // Notify renderer that index is ready
                if (this.mainWindow && !this.mainWindow.isDestroyed()) {
                    this.mainWindow.webContents.send('index-ready', semanticIndex.getStats());
                }
            });

            // Start watching
            await this.fileWatcher.watchProject(projectPath);
        } catch (error) {
            console.error('Error initializing LaTeX services:', error);
        }
    }

    async indexAllFiles(files) {
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

    async stop() {
        if (this.fileWatcher) {
            await this.fileWatcher.stopWatching();
            this.fileWatcher = null;
        }
        semanticIndex.clear();
    }

    getSemanticIndex() {
        return semanticIndex;
    }
}

module.exports = new LatexManager(); // Export as singleton
