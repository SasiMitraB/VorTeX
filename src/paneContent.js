import { state } from './state.js';

const pdfFrameId = (pane) => (pane === 'left' ? 'pdf-frame-left' : 'pdf-frame-right');
const editorId = (pane) => (pane === 'left' ? 'editor-left' : 'editor-right');

export function setPaneContent(pane, tab) {
  const editorEl = document.getElementById(editorId(pane));
  const pdfFrame = document.getElementById(pdfFrameId(pane));

  // Hide PDF by default
  if (pdfFrame) {
    pdfFrame.style.display = 'none';
    pdfFrame.src = '';
  }
  if (editorEl) {
    editorEl.style.visibility = 'visible';
    editorEl.style.pointerEvents = 'auto';
  }

  if (!tab) {
    if (state.editors[pane]) state.editors[pane].setModel(null);
    return;
  }

  if (tab.type === 'pdf') {
    if (editorEl) {
      editorEl.style.visibility = 'hidden';
      editorEl.style.pointerEvents = 'none';
    }
    if (state.editors[pane]) state.editors[pane].setModel(null);
    if (pdfFrame) {
      const src = tab.pdfUrl || (tab.path ? `file://${tab.path}` : '');
      pdfFrame.src = src;
      pdfFrame.style.display = 'block';
    }
  } else {
    if (state.editors[pane]) {
      state.editors[pane].setModel(tab.model);
    }
  }
}
