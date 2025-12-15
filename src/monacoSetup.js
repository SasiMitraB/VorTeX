import { state } from './state.js';
import { registerSnippetManager } from './editor/snippetManager.js';
import { defineTheme } from './editor/theme.js';
import { registerLanguages } from './editor/languages.js';
import { registerCompletionProvider, disposeCompletionProvider } from './editor/completionProvider.js';

export async function initMonaco() {
  return new Promise((resolve) => {
    require(['vs/editor/editor.main'], function () {
      state.monaco = monaco;

      defineTheme(monaco);
      registerLanguages(monaco);
      registerCompletionProvider(monaco);

      state.editors.left = monaco.editor.create(document.getElementById('editor-left'), {
        value: '',
        language: 'plaintext',
        automaticLayout: true
      });
      registerSnippetManager(state.editors.left, monaco);

      state.editors.right = monaco.editor.create(document.getElementById('editor-right'), {
        value: '',
        language: 'plaintext',
        automaticLayout: true
      });
      registerSnippetManager(state.editors.right, monaco);

      resolve();
    });
  });
}

export { disposeCompletionProvider };
