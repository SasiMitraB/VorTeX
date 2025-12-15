import { state, setActivePane } from './state.js';
import { createNewTab } from './tabs.js';
import { hideDropZones } from './dom.js';

let currentRoot = null;
let isCollapsed = false;

const explorerPane = () => document.getElementById('explorer-pane');
const explorerToggleBtn = () => document.getElementById('explorer-toggle');
const explorerChangeBtn = () => document.getElementById('explorer-change');
const explorerTree = () => document.getElementById('explorer-tree');

function renderTree(nodes, container) {
  container.innerHTML = '';
  const ul = document.createElement('ul');
  ul.className = 'tree-root';
  nodes.forEach((node) => ul.appendChild(renderNode(node)));
  container.appendChild(ul);
}

function renderNode(node) {
  const li = document.createElement('li');
  li.className = `tree-node ${node.type}`;
  if (node.type === 'folder') li.classList.add('collapsed');
  li.dataset.path = node.path;
  li.dataset.type = node.type;

  const row = document.createElement('div');
  row.className = 'tree-row';

  const caret = document.createElement('span');
  caret.className = node.type === 'folder' ? 'caret' : 'caret spacer';
  caret.innerText = '▸';
  row.appendChild(caret);

  const icon = document.createElement('span');
  icon.className = 'node-icon material-icons';
  icon.innerText = iconForNode(node);
  row.appendChild(icon);

  const label = document.createElement('span');
  label.className = 'label';
  label.innerText = node.name;
  row.appendChild(label);

  row.onclick = async (e) => {
    e.stopPropagation();
    hideDropZones();
    if (node.type === 'folder') {
      li.classList.toggle('collapsed');
    } else {
      // open file in active pane
      const file = await window.api.readFileAt(node.path);
      createNewTab(file.content, file.path, state.activePane);
    }
  };

  li.appendChild(row);

  if (node.type === 'folder') {
    const childrenContainer = document.createElement('ul');
    childrenContainer.className = 'children';
    (node.children || []).forEach((child) => childrenContainer.appendChild(renderNode(child)));
    li.appendChild(childrenContainer);
  }

  return li;
}

function iconForNode(node) {
  if (node.type === 'folder') return 'folder';
  const ext = (node.name.split('.').pop() || '').toLowerCase();
  switch (ext) {
    case 'js': return 'javascript';
    case 'ts': return 'description';
    case 'json': return 'data_object';
    case 'md': return 'article';
    case 'tex': return 'functions';
    case 'bib': return 'menu_book';
    case 'py': return 'code';
    case 'html': return 'language';
    case 'css': return 'style';
    case 'pdf': return 'picture_as_pdf';
    default: return 'insert_drive_file';
  }
}

export async function chooseFolderAndLoad() {
  const folder = await window.api.openFolder();
  if (!folder) return;
  currentRoot = folder;
  await loadFolder(folder);
}

export async function loadFolder(folderPath) {
  const treeData = await window.api.readTree(folderPath);
  const treeEl = explorerTree();
  if (treeEl) renderTree(treeData, treeEl);
  const rootLabel = document.getElementById('explorer-root');
  if (rootLabel) rootLabel.innerText = folderPath;
  const pane = explorerPane();
  if (pane) {
    pane.classList.remove('collapsed');
    isCollapsed = false;
  }
}

export function toggleExplorer() {
  const pane = explorerPane();
  if (!pane) return;
  isCollapsed = !isCollapsed;
  
  const reopenBtn = document.getElementById('explorer-reopen');
  const icon = reopenBtn?.querySelector('.material-icons');
  
  if (isCollapsed) {
    pane.classList.add('collapsed');
    if (icon) icon.textContent = 'chevron_right';
  } else {
    pane.classList.remove('collapsed');
    if (icon) icon.textContent = 'chevron_left';
  }
}

export function initExplorer() {
  if (explorerChangeBtn()) explorerChangeBtn().onclick = chooseFolderAndLoad;
  
  // Toggle button (arrow that moves with pane)
  const reopenBtn = document.getElementById('explorer-reopen');
  if (reopenBtn) reopenBtn.onclick = toggleExplorer;
  
  // Back to projects button
  const backBtn = document.getElementById('back-to-projects');
  if (backBtn) {
    backBtn.onclick = async () => {
      // Clear current project and navigate back to project selector
      await window.api.configSet('currentProject', null);
      window.location.href = 'project-selector.html';
    };
  }
}
