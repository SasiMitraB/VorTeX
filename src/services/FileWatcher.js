const chokidar = require('chokidar');
const path = require('path');
const EventEmitter = require('events');

class FileWatcher extends EventEmitter {
  constructor() {
    super();
    this.watcher = null;
    this.watchedFiles = new Set();
    this.debounceTimers = new Map();
    this.debounceDelay = 300;
    this.currentProjectPath = null;
  }

  /**
   * Start watching a project directory for .tex and .bib files
   * @param {string} projectPath - The project directory to watch
   */
  async watchProject(projectPath) {
    // Stop any existing watcher
    await this.stopWatching();

    this.currentProjectPath = projectPath;
    this.watchedFiles.clear();

    // Configure chokidar to watch .tex and .bib files
    this.watcher = chokidar.watch(['**/*.tex', '**/*.bib'], {
      cwd: projectPath,
      ignored: [
        '**/node_modules/**',
        '**/.git/**',
        '**/build/**',
        '**/dist/**',
        '**/out/**',
        '**/*.aux',
        '**/*.log',
        '**/*.synctex.gz'
      ],
      persistent: true,
      ignoreInitial: false,
      awaitWriteFinish: {
        stabilityThreshold: 200,
        pollInterval: 100
      }
    });

    // Handle file events
    this.watcher
      .on('add', (relativePath) => this.handleFileEvent('add', relativePath))
      .on('change', (relativePath) => this.handleFileEvent('change', relativePath))
      .on('unlink', (relativePath) => this.handleFileEvent('unlink', relativePath))
      .on('error', (error) => {
        console.error('FileWatcher error:', error);
        this.emit('error', error);
      })
      .on('ready', () => {
        console.log(`FileWatcher ready. Watching ${this.watchedFiles.size} files in ${projectPath}`);
        this.emit('ready', Array.from(this.watchedFiles));
      });
  }

  /**
   * Handle file events with debouncing
   */
  handleFileEvent(eventType, relativePath) {
    const fullPath = path.join(this.currentProjectPath, relativePath);

    // Clear any existing debounce timer for this file
    if (this.debounceTimers.has(fullPath)) {
      clearTimeout(this.debounceTimers.get(fullPath));
    }

    // For unlink events, process immediately
    if (eventType === 'unlink') {
      this.watchedFiles.delete(fullPath);
      this.emit('file-change', {
        type: 'deleted',
        path: fullPath,
        relativePath
      });
      return;
    }

    // Debounce add/change events
    const timer = setTimeout(() => {
      this.debounceTimers.delete(fullPath);

      if (eventType === 'add') {
        this.watchedFiles.add(fullPath);
        this.emit('file-change', {
          type: 'added',
          path: fullPath,
          relativePath
        });
      } else if (eventType === 'change') {
        this.emit('file-change', {
          type: 'changed',
          path: fullPath,
          relativePath
        });
      }
    }, this.debounceDelay);

    this.debounceTimers.set(fullPath, timer);
  }

  /**
   * Stop watching and clean up
   */
  async stopWatching() {
    // Clear all debounce timers
    for (const timer of this.debounceTimers.values()) {
      clearTimeout(timer);
    }
    this.debounceTimers.clear();

    if (this.watcher) {
      await this.watcher.close();
      this.watcher = null;
    }

    this.watchedFiles.clear();
    this.currentProjectPath = null;
  }

  /**
   * Get list of currently watched files
   */
  getWatchedFiles() {
    return Array.from(this.watchedFiles);
  }

  /**
   * Check if a specific file is being watched
   */
  isWatching(filePath) {
    return this.watchedFiles.has(filePath);
  }
}

module.exports = FileWatcher;
