import { getWordRange } from '../utils.js';

export async function getRefCompletions(monaco, query, currentFile, position) {
  try {
    const results = await window.latexServices.fuzzySearch(query, 'labels', currentFile);
    
    const suggestions = results.map((result, index) => ({
      label: result.obj.key,
      kind: monaco.languages.CompletionItemKind.Reference,
      detail: `${result.obj.filename}:${result.obj.line}`,
      documentation: {
        value: `Label defined in **${result.obj.filename}** at line ${result.obj.line}`
      },
      insertText: result.obj.key,
      sortText: String(index).padStart(4, '0'),
      range: getWordRange(monaco, position, query.length)
    }));
    
    return { suggestions };
  } catch (error) {
    console.error('Error getting ref completions:', error);
    return { suggestions: [] };
  }
}

export async function getCiteCompletions(monaco, query, currentFile, position, afterComma) {
  try {
    const results = await window.latexServices.fuzzySearch(query, 'citations', currentFile);
    
    const suggestions = results.map((result, index) => {
      const entry = result.obj;
      const fields = entry.fields || {};
      
      // Build documentation
      let docParts = [];
      if (fields.title) docParts.push(`**${fields.title}**`);
      if (fields.author) docParts.push(`*${fields.author}*`);
      if (fields.year) docParts.push(`(${fields.year})`);
      if (fields.journal) docParts.push(fields.journal);
      
      return {
        label: entry.key,
        kind: monaco.languages.CompletionItemKind.Text,
        detail: `@${entry.entryType} - ${fields.author || 'Unknown author'}`,
        documentation: {
          value: docParts.join('\n\n') || entry.key
        },
        insertText: entry.key,
        sortText: String(index).padStart(4, '0'),
        range: getWordRange(monaco, position, query.length)
      };
    });
    
    return { suggestions };
  } catch (error) {
    console.error('Error getting cite completions:', error);
    return { suggestions: [] };
  }
}
