const path = require('path');
const fs = require('fs').promises;
const os = require('os');
const { parseLatexFile } = require('./LatexParser');
const { parseBibFile } = require('./BibtexParser');

const INDEX_VERSION = 1;
const INDEX_DIR = path.join(os.homedir(), '.latex-editor');
const INDEX_FILE = path.join(INDEX_DIR, 'index.json');

class SemanticIndex {
  constructor() {
    // Main index storage
    this.labels = new Map();      // key -> label object
    this.bibentries = new Map();  // key -> bibentry object
    this.citations = new Map();   // file -> array of citation objects
    this.refs = new Map();        // file -> array of ref objects
    this.sections = new Map();    // file -> array of section objects
    this.bibliographies = [];     // array of bibliography file references
    
    // File tracking
    this.fileHashes = new Map();  // filepath -> hash of content
    this.currentProject = null;
    
    // Save debouncing
    this.saveTimeout = null;
    this.isDirty = false;
  }

  /**
   * Initialize the index for a project
   * @param {string} projectPath - Path to the project
   */
  async initProject(projectPath) {
    this.currentProject = projectPath;
    await this.loadFromDisk();
  }

  /**
   * Update the index for a file
   * @param {string} filepath - Path to the file
   * @param {string} content - File content
   */
  async updateFile(filepath, content) {
    try {
      const hash = this.hashContent(content);
      
      // Skip if content hasn't changed
      if (this.fileHashes.get(filepath) === hash) {
        return;
      }
      
      this.fileHashes.set(filepath, hash);
      
      // Remove old entries from this file first
      this.removeFileEntries(filepath);

      const ext = path.extname(filepath).toLowerCase();

      if (ext === '.tex') {
        const parsed = await parseLatexFile(filepath, content);
        
        // Add labels
        for (const label of parsed.labels) {
          this.labels.set(`${filepath}:${label.key}`, label);
        }
        
        // Store citations for this file
        if (parsed.citations.length > 0) {
          this.citations.set(filepath, parsed.citations);
        }
        
        // Store refs for this file
        if (parsed.refs.length > 0) {
          this.refs.set(filepath, parsed.refs);
        }

        // Store sections for this file
        if (parsed.sections.length > 0) {
          this.sections.set(filepath, parsed.sections);
        }
        
        // Track bibliography files
        for (const bib of parsed.bibliographies) {
          if (!this.bibliographies.find(b => b.file === bib.file && b.sourceFile === bib.sourceFile)) {
            this.bibliographies.push(bib);
          }
        }
      } else if (ext === '.bib') {
        const parsed = await parseBibFile(filepath, content);
        
        // Add bib entries
        for (const entry of parsed) {
          this.bibentries.set(entry.key, entry);
        }
      }

      this.scheduleSave();
    } catch (error) {
      console.error(`Error updating index for ${filepath}:`, error);
    }
  }

  /**
   * Remove all entries associated with a file
   * @param {string} filepath - Path to the file
   */
  removeFile(filepath) {
    this.removeFileEntries(filepath);
    this.fileHashes.delete(filepath);
    this.scheduleSave();
  }

  /**
   * Remove entries without removing file hash
   */
  removeFileEntries(filepath) {
    // Remove labels from this file
    for (const [key, label] of this.labels) {
      if (label.file === filepath) {
        this.labels.delete(key);
      }
    }
    
    // Remove bib entries from this file
    for (const [key, entry] of this.bibentries) {
      if (entry.file === filepath) {
        this.bibentries.delete(key);
      }
    }
    
    // Remove citations and refs
    this.citations.delete(filepath);
    this.refs.delete(filepath);
    this.sections.delete(filepath);
    
    // Remove bibliography references from this file
    this.bibliographies = this.bibliographies.filter(b => b.sourceFile !== filepath);
  }

  /**
   * Get all labels
   * @returns {Array} - Array of label objects
   */
  getAllLabels() {
    return Array.from(this.labels.values());
  }

  /**
   * Get all citation keys (from bib entries)
   * @returns {Array} - Array of bibentry objects
   */
  getAllCitations() {
    return Array.from(this.bibentries.values());
  }

  /**
   * Get labels from a specific file
   * @param {string} filepath - Path to the file
   * @returns {Array} - Array of labels from that file
   */
  getLabelsForFile(filepath) {
    return this.getAllLabels().filter(l => l.file === filepath);
  }

  /**
   * Get labels NOT from a specific file (for ref suggestions)
   * @param {string} filepath - Current file path
   * @returns {Array} - Labels from other files
   */
  getLabelsExcludingFile(filepath) {
    return this.getAllLabels().filter(l => l.file !== filepath);
  }

  /**
   * Get all unique label keys that are already referenced in a file
   * @param {string} filepath - Path to the file
   * @returns {Set} - Set of referenced label keys
   */
  getReferencedLabels(filepath) {
    const refs = this.refs.get(filepath) || [];
    return new Set(refs.map(r => r.key));
  }

  /**
   * Get all unique citation keys used in a file
   * @param {string} filepath - Path to the file
   * @returns {Set} - Set of citation keys
   */
  getCitedKeys(filepath) {
    const citations = this.citations.get(filepath) || [];
    const keys = new Set();
    for (const cite of citations) {
      cite.keys.forEach(k => keys.add(k));
    }
    return keys;
  }

  /**
   * Get sections for a specific file
   * @param {string} filepath - Path to the file
   * @returns {Array} - Array of sections from that file
   */
  getSectionsForFile(filepath) {
    return this.sections.get(filepath) || [];
  }

  /**
   * Clear the entire index
   */
  clear() {
    this.labels.clear();
    this.bibentries.clear();
    this.citations.clear();
    this.refs.clear();
    this.sections.clear();
    this.bibliographies = [];
    this.fileHashes.clear();
    this.currentProject = null;
  }

  /**
   * Get index statistics
   */
  getStats() {
    return {
      labels: this.labels.size,
      bibentries: this.bibentries.size,
      filesIndexed: this.fileHashes.size,
      project: this.currentProject
    };
  }

  /**
   * Schedule a save to disk (debounced)
   */
  scheduleSave() {
    this.isDirty = true;
    
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }
    
    this.saveTimeout = setTimeout(() => {
      this.saveToDisk();
    }, 2000);
  }

  /**
   * Save the index to disk
   */
  async saveToDisk() {
    if (!this.isDirty || !this.currentProject) return;

    try {
      // Ensure directory exists
      await fs.mkdir(INDEX_DIR, { recursive: true });

      const data = {
        version: INDEX_VERSION,
        project: this.currentProject,
        timestamp: Date.now(),
        labels: Array.from(this.labels.entries()),
        bibentries: Array.from(this.bibentries.entries()),
        citations: Array.from(this.citations.entries()),
        refs: Array.from(this.refs.entries()),
        sections: Array.from(this.sections.entries()),
        bibliographies: this.bibliographies,
        fileHashes: Array.from(this.fileHashes.entries())
      };

      await fs.writeFile(INDEX_FILE, JSON.stringify(data, null, 2), 'utf-8');
      this.isDirty = false;
      console.log('Semantic index saved to disk');
    } catch (error) {
      console.error('Error saving semantic index:', error);
    }
  }

  /**
   * Load the index from disk
   */
  async loadFromDisk() {
    try {
      const content = await fs.readFile(INDEX_FILE, 'utf-8');
      const data = JSON.parse(content);

      // Check version and project match
      if (data.version !== INDEX_VERSION || data.project !== this.currentProject) {
        console.log('Index version mismatch or different project, starting fresh');
        return;
      }

      // Restore the index
      this.labels = new Map(data.labels);
      this.bibentries = new Map(data.bibentries);
      this.citations = new Map(data.citations);
      this.refs = new Map(data.refs);
      this.sections = new Map(data.sections || []);
      this.bibliographies = data.bibliographies || [];
      this.fileHashes = new Map(data.fileHashes);

      console.log(`Loaded semantic index from disk: ${this.labels.size} labels, ${this.bibentries.size} bib entries`);
    } catch (error) {
      if (error.code !== 'ENOENT') {
        console.error('Error loading semantic index:', error);
      }
      // File doesn't exist or error - start fresh
    }
  }

  /**
   * Simple hash for content comparison
   */
  hashContent(content) {
    let hash = 0;
    for (let i = 0; i < content.length; i++) {
      const char = content.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return hash.toString(16);
  }
}

// Singleton instance
const semanticIndex = new SemanticIndex();

module.exports = semanticIndex;
