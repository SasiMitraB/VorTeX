export function getCommandCompletions(monaco, query, position) {
  const commands = [
    { cmd: 'ref', snippet: 'ref{$1}', doc: 'Reference a label' },
    { cmd: 'eqref', snippet: 'eqref{$1}', doc: 'Reference an equation' },
    { cmd: 'cite', snippet: 'cite{$1}', doc: 'Citation' },
    { cmd: 'citep', snippet: 'citep{$1}', doc: 'Parenthetical citation' },
    { cmd: 'citet', snippet: 'citet{$1}', doc: 'Textual citation' },
    { cmd: 'label', snippet: 'label{$1}', doc: 'Create a label' },
    { cmd: 'section', snippet: 'section{$1}', doc: 'Section heading' },
    { cmd: 'subsection', snippet: 'subsection{$1}', doc: 'Subsection heading' },
    { cmd: 'subsubsection', snippet: 'subsubsection{$1}', doc: 'Subsubsection heading' },
    { cmd: 'textbf', snippet: 'textbf{$1}', doc: 'Bold text' },
    { cmd: 'textit', snippet: 'textit{$1}', doc: 'Italic text' },
    { cmd: 'emph', snippet: 'emph{$1}', doc: 'Emphasized text' },
    { cmd: 'frac', snippet: 'frac{$1}{$2}', doc: 'Fraction' },
    { cmd: 'sqrt', snippet: 'sqrt{$1}', doc: 'Square root' },
    { cmd: 'sum', snippet: 'sum_{$1}^{$2}', doc: 'Summation' },
    { cmd: 'int', snippet: 'int_{$1}^{$2}', doc: 'Integral' },
    { cmd: 'includegraphics', snippet: 'includegraphics[width=${1:0.8}\\textwidth]{$2}', doc: 'Include image' },
    { cmd: 'input', snippet: 'input{$1}', doc: 'Input file' },
    { cmd: 'include', snippet: 'include{$1}', doc: 'Include file' },
    { cmd: 'begin', snippet: 'begin{$1}\n\t$2\n\\\\end{$1}', doc: 'Begin environment' }
  ];
  
  const queryLower = query.toLowerCase();
  const filtered = commands.filter(c => c.cmd.toLowerCase().startsWith(queryLower));
  
  const suggestions = filtered.map((cmd, index) => ({
    label: '\\' + cmd.cmd,
    kind: monaco.languages.CompletionItemKind.Keyword,
    detail: cmd.doc,
    insertText: '\\' + cmd.snippet,
    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
    sortText: String(index).padStart(4, '0'),
    range: {
      startLineNumber: position.lineNumber,
      startColumn: position.column - query.length - 1, // Include the backslash
      endLineNumber: position.lineNumber,
      endColumn: position.column
    }
  }));
  
  return { suggestions };
}
