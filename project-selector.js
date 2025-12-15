// Project selector logic
let projectsFolder = null;

async function init() {
  // Check if projects folder is already set
  projectsFolder = await window.api.configGet('projectsFolder');
  
  if (!projectsFolder) {
    // First time - prompt for projects folder
    await promptForProjectsFolder();
  } else {
    // Load projects
    await loadProjects();
  }
  
  // Setup event listeners
  document.getElementById('change-projects-folder')?.addEventListener('click', promptForProjectsFolder);
  document.getElementById('choose-different-folder')?.addEventListener('click', promptForProjectsFolder);
}

async function promptForProjectsFolder() {
  const folder = await window.api.openFolder();
  if (!folder) return;
  
  projectsFolder = folder;
  await window.api.configSet('projectsFolder', folder);
  await loadProjects();
}

async function loadProjects() {
  const loading = document.getElementById('loading');
  const grid = document.getElementById('projects-grid');
  const noProjects = document.getElementById('no-projects');
  const pathLabel = document.getElementById('projects-folder-path');
  
  // Show loading
  loading.style.display = 'flex';
  grid.style.display = 'none';
  noProjects.style.display = 'none';
  
  // Update path label
  if (pathLabel) pathLabel.textContent = projectsFolder || 'No folder selected';
  
  // Scan projects
  const projects = await window.api.scanProjects(projectsFolder);
  
  // Hide loading
  loading.style.display = 'none';
  
  if (projects.length === 0) {
    noProjects.style.display = 'flex';
    return;
  }
  
  // Render projects
  grid.style.display = 'grid';
  grid.innerHTML = '';
  
  projects.forEach(project => {
    const card = createProjectCard(project);
    grid.appendChild(card);
  });
}

function createProjectCard(project) {
  const card = document.createElement('div');
  card.className = 'project-card';
  card.onclick = () => openProject(project.path);
  
  // Preview section
  const preview = document.createElement('div');
  preview.className = 'project-preview';
  
  if (project.previewPdf) {
    // Show PDF preview in iframe
    const iframe = document.createElement('iframe');
    iframe.src = `file://${project.previewPdf}#page=1&view=FitH&toolbar=0&navpanes=0&scrollbar=0`;
    preview.appendChild(iframe);
  } else {
    // Show "no preview" placeholder
    preview.classList.add('no-preview');
    const icon = document.createElement('span');
    icon.className = 'material-icons';
    icon.textContent = 'article';
    const text = document.createElement('p');
    text.textContent = 'No preview available';
    preview.appendChild(icon);
    preview.appendChild(text);
  }
  
  card.appendChild(preview);
  
  // Info section
  const info = document.createElement('div');
  info.className = 'project-info';
  
  const name = document.createElement('h3');
  name.className = 'project-name';
  name.textContent = project.name;
  name.title = project.name;
  
  const path = document.createElement('p');
  path.className = 'project-path';
  path.textContent = project.path;
  path.title = project.path;
  
  info.appendChild(name);
  info.appendChild(path);
  card.appendChild(info);
  
  return card;
}

async function openProject(projectPath) {
  // Store the selected project path
  await window.api.configSet('currentProject', projectPath);
  
  // Navigate to the editor
  window.location.href = 'index.html';
}

// Initialize on load
document.addEventListener('DOMContentLoaded', init);
