export function defineTheme(monaco) {
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
}
