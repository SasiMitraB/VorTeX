import { state } from './state.js';

export async function initMonaco() {
  return new Promise((resolve) => {
    require(['vs/editor/editor.main'], function () {
      state.monaco = monaco;

      monaco.editor.defineTheme('atom-one-dark', {
        base: 'vs-dark',
        inherit: true,
        rules: [
          { token: 'comment', foreground: '5c6370' },
          { token: 'keyword', foreground: 'c678dd' },
          { token: 'number', foreground: 'd19a66' },
          { token: 'string', foreground: '98c379' },
          { token: 'variable', foreground: 'e5c07b' },
          { token: 'type', foreground: '61afef' },
          { token: 'delimiter', foreground: 'abb2bf' },
          { token: 'operator', foreground: 'abb2bf' }
        ],
        colors: {
          'editor.background': '#282c34',
          'editor.foreground': '#abb2bf',
          'editorLineNumber.foreground': '#5c6370',
          'editorLineNumber.activeForeground': '#abb2bf',
          'editor.selectionBackground': '#3e4451',
          'editorCursor.foreground': '#528bff',
          'editor.lineHighlightBackground': '#2c313a'
        }
      });
      monaco.editor.setTheme('atom-one-dark');

      monaco.languages.register({ id: 'latex' });
      monaco.languages.setMonarchTokensProvider('latex', {
        tokenizer: {
          root: [
            [/%.+$/, 'comment'],
            [/\\begin\{[a-zA-Z*]+\}/, 'keyword'],
            [/\\end\{[a-zA-Z*]+\}/, 'keyword'],
            [/\\[a-zA-Z@]+/, 'keyword'],
            [/\$[^$]*\$/, 'string'],
            [/\{[^}]*\}/, 'variable'],
            [/\[[^\]]*\]/, 'variable'],
            [/[0-9]+/, 'number']
          ]
        }
      });

      monaco.languages.register({ id: 'bibtex' });
      monaco.languages.setMonarchTokensProvider('bibtex', {
        tokenizer: {
          root: [
            [/%.*/, 'comment'],
            [/@[a-zA-Z]+\{/, 'keyword'],
            [/[a-zA-Z_][a-zA-Z0-9_]*\s*=/, 'variable'],
            [/"[^"]*"/, 'string'],
            [/\{[^}]*\}/, 'string'],
            [/[0-9]+/, 'number'],
            [/[,=\{\}]/, 'delimiter']
          ]
        }
      });

      state.editors.left = monaco.editor.create(document.getElementById('editor-left'), {
        value: '',
        language: 'plaintext',
        automaticLayout: true
      });

      state.editors.right = monaco.editor.create(document.getElementById('editor-right'), {
        value: '',
        language: 'plaintext',
        automaticLayout: true
      });

      resolve();
    });
  });
}
