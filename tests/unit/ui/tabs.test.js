import { setPaneContent } from '../../../src/paneContent.js';

// Mock state
jest.mock('../../../src/state.js', () => ({
    state: {
        editors: { left: { setModel: jest.fn() }, right: { setModel: jest.fn() } }
    }
}));

describe('paneContent.js', () => {
    beforeEach(() => {
        // Setup basic DOM
        document.body.innerHTML = `
      <div id="editor-left" style="visibility: visible;"></div>
      <iframe id="pdf-frame-left" style="display: none;"></iframe>
    `;
        jest.clearAllMocks();
    });

    test('switching to PDF tab shows iframe and hides editor', () => {
        const pdfTab = { type: 'pdf', pdfUrl: 'file:///test.pdf' };

        setPaneContent('left', pdfTab);

        const iframe = document.getElementById('pdf-frame-left');
        const editor = document.getElementById('editor-left');

        expect(iframe.style.display).toBe('block');
        expect(iframe.src).toBe('file:///test.pdf');
        expect(editor.style.visibility).toBe('hidden');
        expect(editor.style.pointerEvents).toBe('none');
    });

    test('switching to text tab shows editor and hides iframe', () => {
        const textTab = { type: 'text', model: {} };

        // First set to PDF
        document.getElementById('pdf-frame-left').style.display = 'block';

        setPaneContent('left', textTab);

        const iframe = document.getElementById('pdf-frame-left');
        const editor = document.getElementById('editor-left');

        expect(iframe.style.display).toBe('none');
        expect(editor.style.visibility).toBe('visible');
        expect(editor.style.pointerEvents).toBe('auto');
    });

    test('correctly handles file paths for PDF', () => {
        const pdfTab = { type: 'pdf', path: '/abs/path/to/doc.pdf' };

        setPaneContent('left', pdfTab);

        const iframe = document.getElementById('pdf-frame-left');
        expect(iframe.src).toBe('file:///abs/path/to/doc.pdf');
    });
});
