console.log('Renderer script loading...');

import { state, setActivePane } from './src/state.js';
import { initMonaco, disposeCompletionProvider } from './src/monacoSetup.js';
import { hideDropZones } from './src/dom.js';
import { setupDropZones, setupGlobalDragHandlers } from './src/dragDrop.js';
import { createNewTab, openFiles, saveActive, saveAsActive } from './src/tabs.js';
import { initExplorer, chooseFolderAndLoad, toggleExplorer } from './src/explorer.js';
import TableEditor from './src/editor/TableEditor.js';

console.log('All imports loaded successfully');

function setupButtons() {
  const openBtn = document.getElementById('openBtn');
  if (openBtn) openBtn.onclick = openFiles;
  const newBtn = document.getElementById('newBtn');
  if (newBtn) newBtn.onclick = () => createNewTab();
  const saveBtn = document.getElementById('saveBtn');
  if (saveBtn) saveBtn.onclick = saveActive;
  const saveAsBtn = document.getElementById('saveAsBtn');
  if (saveAsBtn) saveAsBtn.onclick = saveAsActive;
  const openFolderBtn = document.getElementById('openFolderBtn');
  if (openFolderBtn) openFolderBtn.onclick = chooseFolderAndLoad;
  const toggleExplorerBtn = document.getElementById('toggleExplorerBtn');
  if (toggleExplorerBtn) toggleExplorerBtn.onclick = toggleExplorer;
}

function setupKeyboardShortcuts() {
  window.addEventListener('keydown', (e) => {
    const isSave = (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's';
    if (isSave) {
      e.preventDefault();
      saveActive();
    }
  });
}

function setupPaneClickFocus() {
  document.addEventListener('click', (e) => {
    if (e.target.closest('#editor-pane-left') || e.target.id === 'editor-left' || e.target.id === 'tabbar-left') {
      setActivePane('left');
      hideDropZones();
      if (state.editors.left && state.paneState.left.activeId) {
        state.editors.left.focus();
      }
    } else if (e.target.closest('#editor-pane-right') || e.target.id === 'editor-right' || e.target.id === 'tabbar-right') {
      setActivePane('right');
      hideDropZones();
      if (state.editors.right && state.paneState.right.activeId) {
        state.editors.right.focus();
      }
    }
  });
}

async function bootstrap() {
  await initMonaco();
  setupButtons();
  setupKeyboardShortcuts();
  setupPaneClickFocus();
  setupDropZones();
  setupGlobalDragHandlers();
  initExplorer();

  // Load the current project folder automatically
  const currentProject = await window.api.configGet('currentProject');
  if (currentProject) {
    console.log('Loading project:', currentProject);
    const { loadFolder } = await import('./src/explorer.js');
    await loadFolder(currentProject);

    // Initialize LaTeX services subscription
    initLatexServicesListeners();
  }

  try {
    if (window.mdc && window.mdc.ripple) {
      document.querySelectorAll('.mdc-button').forEach(b => window.mdc.ripple.MDCRipple.attachTo(b));
    }
  } catch (e) {
    // ignore if MDC not loaded
  }

  // Initialize Table Editor
  const tableEditor = new TableEditor();

  if (window.menuAPI) {
    window.menuAPI.onInsertTable(() => {
      tableEditor.open((latex) => {
        // Insert into active editor
        const activePane = state.activePane;
        const editor = state.editors[activePane];
        if (editor) {
          const position = editor.getPosition();
          editor.executeEdits('table-insert', [{
            range: new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column),
            text: latex,
            forceMoveMarkers: true
          }]);
          editor.focus();
        }
      });
    });
  }
}

/**
 * Initialize listeners for LaTeX services events
 */
function initLatexServicesListeners() {
  if (!window.latexServices) {
    console.warn('LaTeX services not available');
    return;
  }

  // Listen for file changes from the file watcher
  window.latexServices.onFileChange((event) => {
    console.log('File change event:', event.type, event.path);
    // The main process handles the indexing automatically
    // This is just for UI updates if needed
  });

  // Listen for index ready event
  window.latexServices.onIndexReady((stats) => {
    console.log('Semantic index ready:', stats);
    // Could show a status indicator here
  });
}

/**
 * Clean up LaTeX services when switching projects
 */
function cleanupLatexServices() {
  if (window.latexServices) {
    window.latexServices.removeAllListeners();
  }
  disposeCompletionProvider();
}

window.addEventListener('DOMContentLoaded', bootstrap);

console.log('Renderer script loaded');
