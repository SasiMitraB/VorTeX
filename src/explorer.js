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
  const ext = node.type === 'folder' ? 'folder' : (node.name.split('.').pop() || '').toLowerCase();
  icon.className = `node-icon material-icons ${ext}`;
  icon.innerText = iconForNode(node);
  row.appendChild(icon);

  const label = document.createElement('span');
  label.className = 'label';
  label.innerText = node.name;
  row.appendChild(label);

  // Context Menu
  row.oncontextmenu = (e) => {
    e.preventDefault();
    e.stopPropagation();
    showContextMenu(e.clientX, e.clientY, node);
  };

  row.onclick = async (e) => {
    e.stopPropagation();
    hideDropZones();
    if (node.type === 'folder') {
      li.classList.toggle('collapsed');
    } else {
      // open file in active pane
      const file = await window.api.readFileAt(node.path);
      createNewTab(file.content, file.path, state.activePane);
      
      // Update outline if it's a tex file
      if (node.path.endsWith('.tex')) {
        updateOutline(node.path);
      }
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

  // Hide context menu on click elsewhere
  document.addEventListener('click', () => {
    const menu = document.getElementById('context-menu');
    if (menu) menu.style.display = 'none';
  });
}

/**
 * Update the outline view for a file
 */
export async function updateOutline(filePath) {
  const outlineTree = document.getElementById('outline-tree');
  if (!outlineTree) return;
  
  outlineTree.innerHTML = '';
  
  if (!filePath || !filePath.endsWith('.tex')) {
    outlineTree.innerHTML = '<div style="padding: 8px; color: var(--muted); font-size: 12px;">No outline available</div>';
    return;
  }

  try {
    const sections = await window.latexServices.getSections(filePath);
    
    if (!sections || sections.length === 0) {
      outlineTree.innerHTML = '<div style="padding: 8px; color: var(--muted); font-size: 12px;">No sections found</div>';
      return;
    }

    sections.forEach(section => {
      const el = document.createElement('div');
      el.className = `outline-node level-${section.level}`;
      el.innerText = section.title;
      el.title = section.title;
      el.onclick = () => {
        // Navigate to section
        const activeEditor = state.editors[state.activePane];
        if (activeEditor) {
          activeEditor.revealLineInCenter(section.line);
          activeEditor.setPosition({ lineNumber: section.line, column: 1 });
          activeEditor.focus();
        }
      };
      outlineTree.appendChild(el);
    });
  } catch (error) {
    console.error('Error updating outline:', error);
  }
}

/**
 * Show context menu
 */
function showContextMenu(x, y, node) {
  const menu = document.getElementById('context-menu');
  if (!menu) return;

  menu.style.display = 'block';
  menu.style.left = `${x}px`;
  menu.style.top = `${y}px`;

  // Setup actions
  const items = menu.querySelectorAll('.menu-item');
  items.forEach(item => {
    item.onclick = async (e) => {
      e.stopPropagation();
      menu.style.display = 'none';
      const action = item.dataset.action;
      
      switch (action) {
        case 'copy-path':
          await navigator.clipboard.writeText(node.path);
          break;
        case 'copy-relative-path':
          // Assuming currentRoot is the project root
          const relPath = node.path.replace(currentRoot, '').replace(/^\//, '');
          await navigator.clipboard.writeText(relPath);
          break;
        case 'delete':
          if (confirm(`Are you sure you want to delete ${node.name}?`)) {
            // Implement delete logic via API (needs to be added to preload/main)
            console.log('Delete not implemented yet');
          }
          break;
        case 'rename':
          const newName = prompt('Enter new name:', node.name);
          if (newName && newName !== node.name) {
            // Implement rename logic via API
            console.log('Rename not implemented yet');
          }
          break;
        // Cut/Copy would require clipboard state management
      }
    };
  });
}
