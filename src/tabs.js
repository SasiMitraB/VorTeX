import { state, newTabId, setActivePane } from './state.js';
import { setPaneContent } from './paneContent.js';
import { hideDropZones, setPdfPointerEvents } from './dom.js';
import { detectLanguageFromPath } from './utils.js';
import { updateOutline } from './explorer.js';

function makeTabElement(tab, pane) {
  const el = document.createElement('div');
  el.className = 'tab';
  el.dataset.id = tab.id;
  el.dataset.pane = pane;
  el.innerText = tab.name + (tab.dirty ? ' *' : '');

  el.draggable = true;
  el.addEventListener('dragstart', () => {
    state.draggedTab = { tab, fromPane: pane };
    el.classList.add('dragging');
    setPdfPointerEvents(false);
  });
  el.addEventListener('dragend', () => {
    el.classList.remove('dragging');
    state.draggedTab = null;
    setPdfPointerEvents(true);
  });

  const close = document.createElement('span');
  close.className = 'close';
  close.innerText = '×';
  close.onclick = (e) => { e.stopPropagation(); closeTab(tab.id, pane); };
  close.onmousedown = (e) => e.stopPropagation();
  el.appendChild(close);
  el.onclick = (e) => {
    e.stopPropagation();
    hideDropZones();
    setActivePane(pane);
    switchTab(tab.id, pane);
  };
  return el;
}

export function renderTabs(pane) {
  const barId = pane === 'left' ? 'tabbar-left' : 'tabbar-right';
  const bar = document.getElementById(barId);
  bar.innerHTML = '';
  state.paneTabs[pane].forEach(t => {
    const el = makeTabElement(t, pane);
    if (t.id === state.paneState[pane].activeId) el.classList.add('active');
    bar.appendChild(el);
  });
}

export function switchTab(id, pane = state.activePane) {
  const tab = state.paneTabs[pane].find(t => t.id === id);
  if (!tab) return;
  hideDropZones();
  state.paneState[pane].activeId = id;
  setActivePane(pane);
  setPaneContent(pane, tab);
  if (state.editors[pane] && tab.type !== 'pdf') {
    state.editors[pane].focus();
  }
  
  // Update outline if it's a tex file
  if (tab.path && tab.path.endsWith('.tex')) {
    updateOutline(tab.path);
  } else {
    updateOutline(null);
  }

  renderTabs('left');
  renderTabs('right');
}

export function closeTab(id, pane = state.activePane) {
  const idx = state.paneTabs[pane].findIndex(t => t.id === id);
  if (idx === -1) return;
  const tab = state.paneTabs[pane][idx];
  if (tab.dirty) {
    if (!confirm('Discard changes?')) return;
  }
  if (tab.model) tab.model.dispose();
  state.paneTabs[pane].splice(idx, 1);

  if (state.paneState[pane].activeId === id) {
    if (state.paneTabs[pane].length > 0) {
      switchTab(state.paneTabs[pane][Math.max(0, idx - 1)].id, pane);
    } else {
      state.paneState[pane].activeId = null;
      setPaneContent(pane, null);
      if (pane === 'right') {
        document.getElementById('editor-pane-right').style.display = 'none';
        setActivePane('left');
      } else if (pane === 'left' && state.paneTabs['right'].length === 0) {
        state.paneState.left.activeId = null;
        setPaneContent('left', null);
      }
      renderTabs(pane);
    }
  } else {
    renderTabs(pane);
  }
}

export function createNewTab(content = '', path = null, targetPane = state.activePane) {
  const id = newTabId();
  const name = path ? path.split('/').pop() : `Untitled-${state.nextUntitled++}`;
  const isPdf = path && path.toLowerCase().endsWith('.pdf');

  if (isPdf) {
    const tab = { id, path, name, model: null, dirty: false, type: 'pdf', pdfUrl: path ? `file://${path}` : null };
    state.paneTabs[targetPane].push(tab);
    switchTab(id, targetPane);
    return tab;
  }

  const model = state.monaco.editor.createModel(content, 'plaintext');
  const tab = { id, path, name, model, dirty: false, type: 'text' };
  if (path) {
    const lang = detectLanguageFromPath(path);
    state.monaco.editor.setModelLanguage(model, lang);
  }
  model.onDidChangeContent(() => {
    tab.dirty = true;
    renderTabs('left');
    renderTabs('right');
  });
  state.paneTabs[targetPane].push(tab);
  switchTab(id, targetPane);
  return tab;
}

export async function openFiles() {
  const files = await window.api.openFiles();
  if (!files || files.length === 0) return;
  files.forEach(f => createNewTab(f.content, f.path));
}

export async function saveActive() {
  const tab = state.paneTabs[state.activePane].find(t => t.id === state.paneState[state.activePane].activeId);
  if (!tab) return;
  if (tab.type === 'pdf') return;
  const content = tab.model.getValue();
  if (tab.path) {
    await window.api.writeFile(tab.path, content);
    tab.dirty = false;
    renderTabs('left');
    renderTabs('right');
  } else {
    await saveAsActive();
  }
}

export async function saveAsActive() {
  const tab = state.paneTabs[state.activePane].find(t => t.id === state.paneState[state.activePane].activeId);
  if (!tab) return;
  if (tab.type === 'pdf') return;
  const content = tab.model.getValue();
  const res = await window.api.saveAs(tab.name, content);
  if (!res || res.canceled) return;
  tab.path = res.filePath;
  tab.name = res.filePath.split('/').pop();
  tab.dirty = false;
  const lang = detectLanguageFromPath(tab.path);
  state.monaco.editor.setModelLanguage(tab.model, lang);
  renderTabs('left');
  renderTabs('right');
}
