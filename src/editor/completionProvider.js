import { getCurrentFilePath } from './utils.js';
import { getRefCompletions, getCiteCompletions } from './completions/references.js';
import { getEnvironmentCompletions } from './completions/environments.js';
import { getCommandCompletions } from './completions/commands.js';
import { getFileCompletions } from './completions/files.js';

let completionProviderDisposable = null;

export function registerCompletionProvider(monaco) {
  if (completionProviderDisposable) {
    completionProviderDisposable.dispose();
  }

  completionProviderDisposable = monaco.languages.registerCompletionItemProvider('latex', {
    triggerCharacters: ['\\', '{', ','],
    
    provideCompletionItems: async (model, position) => {
      const lineContent = model.getLineContent(position.lineNumber);
      const textUntilPosition = lineContent.substring(0, position.column - 1);
      
      const currentFile = getCurrentFilePath();
      
      // Check for \ref{, \eqref{, \cref{, \autoref{
      const refMatch = textUntilPosition.match(/\\(ref|eqref|cref|autoref|pageref)\{([^}]*)$/);
      if (refMatch) {
        const query = refMatch[2];
        return await getRefCompletions(monaco, query, currentFile, position);
      }
      
      // Check for \cite{, \citep{, \citet{, etc. (including comma-separated)
      const citeMatch = textUntilPosition.match(/\\(cite|citep|citet|autocite|parencite|textcite)\{([^}]*)$/);
      if (citeMatch) {
        const citeContent = citeMatch[2];
        const lastCommaIndex = citeContent.lastIndexOf(',');
        const query = lastCommaIndex >= 0 ? citeContent.substring(lastCommaIndex + 1).trim() : citeContent;
        return await getCiteCompletions(monaco, query, currentFile, position, lastCommaIndex >= 0);
      }
      
      // Check for \begin{ - offer environment completions with snippets
      const beginMatch = textUntilPosition.match(/\\begin\{([^}]*)$/);
      if (beginMatch) {
        const query = beginMatch[1];
        return getEnvironmentCompletions(monaco, query, position);
      }
      
      // Check for backslash commands
      const cmdMatch = textUntilPosition.match(/\\([a-zA-Z]*)$/);
      if (cmdMatch) {
        const query = cmdMatch[1];
        return getCommandCompletions(monaco, query, position);
      }

      // Feature 6: File Path Suggestions
      const fileMatch = textUntilPosition.match(/\\(input|include|includegraphics|bibliography|addbibresource)\{([^}]*)$/);
      if (fileMatch) {
        const cmd = fileMatch[1];
        const query = fileMatch[2];
        return await getFileCompletions(monaco, query, cmd, position);
      }

      // Feature 8: Section Label Suggestion (on new line)
      if (position.column === 1 && position.lineNumber > 1) {
        const prevLine = model.getLineContent(position.lineNumber - 1);
        const sectionMatch = prevLine.match(/\\(section|subsection|subsubsection)\{([^}]+)\}/);
        if (sectionMatch) {
          const title = sectionMatch[2];
          const slug = title.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
          const label = `sec:${slug}`;
          
          return {
            suggestions: [{
              label: `\\label{${label}}`,
              kind: monaco.languages.CompletionItemKind.Snippet,
              detail: 'Auto-generated section label',
              insertText: `\\label{${label}}`,
              range: {
                startLineNumber: position.lineNumber,
                startColumn: 1,
                endLineNumber: position.lineNumber,
                endColumn: 1
              }
            }]
          };
        }
      }
      
      return { suggestions: [] };
    }
  });
}

export function disposeCompletionProvider() {
  if (completionProviderDisposable) {
    completionProviderDisposable.dispose();
    completionProviderDisposable = null;
  }
}
