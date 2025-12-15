export function registerLanguages(monaco) {
  monaco.languages.register({ id: 'latex' });
  monaco.languages.setLanguageConfiguration('latex', {
    wordPattern: /[a-zA-Z0-9_\:\-\.]+/
  });
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
  monaco.languages.setLanguageConfiguration('bibtex', {
    wordPattern: /[a-zA-Z0-9_\:\-\.]+/
  });
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
}
