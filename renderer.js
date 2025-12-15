import { state, setActivePane } from './src/state.js';
import { initMonaco } from './src/monacoSetup.js';
import { hideDropZones } from './src/dom.js';
import { setupDropZones, setupGlobalDragHandlers } from './src/dragDrop.js';
import { createNewTab, openFiles, saveActive, saveAsActive } from './src/tabs.js';

function setupButtons() {
  document.getElementById('openBtn').onclick = openFiles;
  document.getElementById('newBtn').onclick = () => createNewTab();
  document.getElementById('saveBtn').onclick = saveActive;
  document.getElementById('saveAsBtn').onclick = saveAsActive;
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

  try {
    if (window.mdc && window.mdc.ripple) {
      document.querySelectorAll('.mdc-button').forEach(b => window.mdc.ripple.MDCRipple.attachTo(b));
    }
  } catch (e) {
    // ignore if MDC not loaded
  }
}

window.addEventListener('DOMContentLoaded', bootstrap);
