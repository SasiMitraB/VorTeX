import { getWordRange, getCurrentFilePath } from '../utils.js';

export async function getFileCompletions(monaco, query, cmd, position) {
  try {
    const currentProject = await window.api.configGet('currentProject');
    if (!currentProject) return { suggestions: [] };
    
    const tree = await window.api.readTree(currentProject);
    const files = flattenTree(tree);
    
    // Filter based on command
    let extensions = [];
    if (cmd === 'includegraphics') {
      extensions = ['.png', '.jpg', '.jpeg', '.pdf', '.eps', '.svg'];
    } else if (cmd === 'bibliography' || cmd === 'addbibresource') {
      extensions = ['.bib'];
    } else {
      extensions = ['.tex'];
    }
    
    const filteredFiles = files.filter(f => {
      const ext = f.name.substring(f.name.lastIndexOf('.')).toLowerCase();
      return extensions.includes(ext);
    });
    
    // Calculate relative paths
    const currentFile = getCurrentFilePath();
    const currentDir = currentFile ? currentFile.substring(0, currentFile.lastIndexOf('/')) : currentProject;
    
    const suggestions = filteredFiles.map((f, index) => {
      // Simple relative path calculation
      let relPath = f.path.replace(currentDir + '/', '');
      if (relPath.startsWith(currentDir)) {
        // It's in a parent directory or different branch
        // Fallback to project relative
        relPath = f.path.replace(currentProject + '/', '');
      }
      
      // Remove extension for input/include if standard
      if ((cmd === 'input' || cmd === 'include') && relPath.endsWith('.tex')) {
        relPath = relPath.substring(0, relPath.length - 4);
      }
      
      return {
        label: relPath,
        kind: monaco.languages.CompletionItemKind.File,
        detail: f.path,
        insertText: relPath,
        sortText: String(index).padStart(4, '0'),
        range: getWordRange(monaco, position, query.length)
      };
    });
    
    return { suggestions };
  } catch (error) {
    console.error('Error getting file completions:', error);
    return { suggestions: [] };
  }
}

function flattenTree(nodes) {
  let result = [];
  for (const node of nodes) {
    if (node.type === 'file') {
      result.push(node);
    } else if (node.type === 'folder' && node.children) {
      result = result.concat(flattenTree(node.children));
    }
  }
  return result;
}
