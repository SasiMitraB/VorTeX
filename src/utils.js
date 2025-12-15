export function detectLanguageFromPath(path) {
  const ext = path.split('.').pop().toLowerCase();
  switch (ext) {
    case 'js': return 'javascript';
    case 'ts': return 'typescript';
    case 'json': return 'json';
    case 'md': return 'markdown';
    case 'tex': return 'latex';
    case 'bib': return 'bibtex';
    case 'py': return 'python';
    case 'html': return 'html';
    case 'css': return 'css';
    default: return 'plaintext';
  }
}
