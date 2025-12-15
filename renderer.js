console.log('Renderer script loading...');

import { state, setActivePane } from './src/state.js';
import { initMonaco } from './src/monacoSetup.js';
import { hideDropZones } from './src/dom.js';
import { setupDropZones, setupGlobalDragHandlers } from './src/dragDrop.js';
import { createNewTab, openFiles, saveActive, saveAsActive } from './src/tabs.js';
import { initExplorer, chooseFolderAndLoad, toggleExplorer } from './src/explorer.js';

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
  }

  try {
    if (window.mdc && window.mdc.ripple) {
      document.querySelectorAll('.mdc-button').forEach(b => window.mdc.ripple.MDCRipple.attachTo(b));
    }
  } catch (e) {
    // ignore if MDC not loaded
  }
}

window.addEventListener('DOMContentLoaded', bootstrap);

console.log('Renderer script loaded');
