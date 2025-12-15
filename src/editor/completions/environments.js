import { getWordRange } from '../utils.js';
import { state } from '../../state.js';

export function getEnvironmentCompletions(monaco, query, position) {
  // Check if label already exists in next 3 lines
  const model = state.editors[state.activePane].getModel();
  let hasLabel = false;
  if (model) {
    for (let i = 1; i <= 3; i++) {
      if (position.lineNumber + i <= model.getLineCount()) {
        const line = model.getLineContent(position.lineNumber + i);
        if (line.includes('\\label{')) {
          hasLabel = true;
          break;
        }
      }
    }
  }

  const environments = [
    {
      name: 'equation',
      snippet: hasLabel ? 'equation}\n\t$1\n\\\\end{equation' : 'equation}\n\t$1\n\\\\label{eq:$2}\n\\\\end{equation',
      doc: 'Numbered equation' + (hasLabel ? '' : ' with label')
    },
    {
      name: 'equation*',
      snippet: 'equation*}\n\t$1\n\\\\end{equation*',
      doc: 'Unnumbered equation'
    },
    {
      name: 'align',
      snippet: hasLabel ? 'align}\n\t$1 &= $2 \\\\\\\\\n\t$3 &= $4\n\\\\end{align' : 'align}\n\t$1 &= $2 \\\\\\\\\n\t$3 &= $4\n\\\\label{eq:$5}\n\\\\end{align',
      doc: 'Aligned equations' + (hasLabel ? '' : ' with label')
    },
    {
      name: 'figure',
      snippet: hasLabel ? 'figure}[${1:htbp}]\n\t\\\\centering\n\t\\\\includegraphics[width=${2:0.8}\\\\textwidth]{${3:filename}}\n\t\\\\caption{${4:Caption}}\n\\\\end{figure' : 'figure}[${1:htbp}]\n\t\\\\centering\n\t\\\\includegraphics[width=${2:0.8}\\\\textwidth]{${3:filename}}\n\t\\\\caption{${4:Caption}}\n\t\\\\label{fig:$5}\n\\\\end{figure',
      doc: 'Figure environment' + (hasLabel ? '' : ' with label')
    },
    {
      name: 'table',
      snippet: hasLabel ? 'table}[${1:htbp}]\n\t\\\\centering\n\t\\\\caption{${2:Caption}}\n\t\\\\begin{tabular}{${3:cc}}\n\t\t\\\\hline\n\t\t$4 \\\\\\\\\n\t\t\\\\hline\n\t\\\\end{tabular}\n\\\\end{table' : 'table}[${1:htbp}]\n\t\\\\centering\n\t\\\\caption{${2:Caption}}\n\t\\\\label{tab:$3}\n\t\\\\begin{tabular}{${4:cc}}\n\t\t\\\\hline\n\t\t$5 \\\\\\\\\n\t\t\\\\hline\n\t\\\\end{tabular}\n\\\\end{table',
      doc: 'Table environment' + (hasLabel ? '' : ' with label')
    },
    {
      name: 'itemize',
      snippet: 'itemize}\n\t\\\\item $1\n\t\\\\item $2\n\\\\end{itemize',
      doc: 'Bulleted list'
    },
    {
      name: 'enumerate',
      snippet: 'enumerate}\n\t\\\\item $1\n\t\\\\item $2\n\\\\end{enumerate',
      doc: 'Numbered list'
    },
    {
      name: 'theorem',
      snippet: hasLabel ? 'theorem}\n\t$1\n\\\\end{theorem' : 'theorem}\n\t$1\n\\\\label{thm:$2}\n\\\\end{theorem',
      doc: 'Theorem environment' + (hasLabel ? '' : ' with label')
    },
    {
      name: 'proof',
      snippet: 'proof}\n\t$1\n\\\\end{proof',
      doc: 'Proof environment'
    },
    {
      name: 'abstract',
      snippet: 'abstract}\n\t$1\n\\\\end{abstract',
      doc: 'Abstract environment'
    }
  ];
  
  const queryLower = query.toLowerCase();
  const filtered = environments.filter(e => e.name.toLowerCase().includes(queryLower));
  
  const suggestions = filtered.map((env, index) => ({
    label: env.name,
    kind: monaco.languages.CompletionItemKind.Snippet,
    detail: env.doc,
    insertText: env.snippet,
    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
    sortText: String(index).padStart(4, '0'),
    range: getWordRange(monaco, position, query.length)
  }));
  
  return { suggestions };
}
