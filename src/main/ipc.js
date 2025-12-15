const { app } = require('electron');
const path = require('path');
const fs = require('fs').promises;
const latexManager = require('./latexManager');
const { searchLabels, searchBibEntries } = require('../services/FuzzyMatcher');

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

function registerIpcHandlers(ipcMain, dialog, store) {
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

    ipcMain.handle('dialog:openCSV', async () => {
        const result = await dialog.showOpenDialog({
            properties: ['openFile'],
            filters: [{ name: 'CSV Files', extensions: ['csv'] }]
        });
        if (result.canceled || !result.filePaths.length) return null;
        const content = await fs.readFile(result.filePaths[0], 'utf-8');
        return { path: result.filePaths[0], content };
    });

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

    // LaTeX Services IPC Handlers
    ipcMain.handle('latex:getCompletionData', async (event, type, currentFile) => {
        try {
            const semanticIndex = latexManager.getSemanticIndex();
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

    ipcMain.handle('latex:fuzzySearch', async (event, query, type, currentFile) => {
        try {
            const semanticIndex = latexManager.getSemanticIndex();
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

    ipcMain.handle('latex:reindexFile', async (event, filePath) => {
        try {
            const semanticIndex = latexManager.getSemanticIndex();
            const content = await fs.readFile(filePath, 'utf-8');
            await semanticIndex.updateFile(filePath, content);
            return { success: true, stats: semanticIndex.getStats() };
        } catch (error) {
            console.error('Error reindexing file:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('latex:getStats', async () => {
        return latexManager.getSemanticIndex().getStats();
    });

    ipcMain.handle('latex:initProject', async (event, projectPath) => {
        await latexManager.stop();
        await latexManager.initialize(projectPath);
        return { success: true };
    });
}

module.exports = { registerIpcHandlers };
