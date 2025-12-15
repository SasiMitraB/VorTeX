
import TableEditor from '../../../src/editor/TableEditor';

describe('TableEditor Logic', () => {
    let editor;
    let mockSpreadsheet;

    beforeEach(() => {
        // Setup minimal DOM
        document.body.innerHTML = `
      <div id="modal-overlay" style="display:none">
        <div id="xspreadsheet"></div>
        <button id="te-insert"></button>
        <button id="te-cancel"></button>
        <button id="modal-close"></button>
      </div>
    `;

        // Mock window.x_spreadsheet
        mockSpreadsheet = {
            getData: jest.fn(),
            loadData: jest.fn()
        };
        window.x_spreadsheet = jest.fn(() => mockSpreadsheet);

        editor = new TableEditor();
    });

    const setupData = (rowsObj) => {
        mockSpreadsheet.getData.mockReturnValue([{
            rows: rowsObj,
            styles: [
                { font: { bold: true } },   // Style 0: Bold
                { font: { italic: true } }  // Style 1: Italic
            ]
        }]);
        editor.open(() => { }); // Initialize spreadsheet
        editor.spreadsheet = mockSpreadsheet; // Ensure ref is set
    };

    test('returns empty string if no data', () => {
        setupData({});
        expect(editor.generateLatex()).toBe('');
    });

    test('generates simple table', () => {
        setupData({
            0: { cells: { 0: { text: 'A' }, 1: { text: 'B' } } },
            1: { cells: { 0: { text: '1' }, 1: { text: '2' } } }
        });

        const latex = editor.generateLatex();
        expect(latex).toContain('\\begin{tabular}{c c}');
        expect(latex).toContain('A & B \\\\');
        expect(latex).toContain('1 & 2 \\\\');
    });

    test('filters empty rows and columns', () => {
        // Row 0: Empty
        // Row 1: Col 1 has data (Col 0 empty)
        setupData({
            0: { cells: { 0: { text: '' } } },
            1: { cells: { 1: { text: 'Data' } } }
        });

        // Should only output 1 column (c) and 1 row
        const latex = editor.generateLatex();
        expect(latex).toContain('\\begin{tabular}{c}');
        expect(latex).toContain('Data \\\\');
        // Shouldn't see empty cells from row 0 or col 0
    });

    test('applies formatting and escapes characters', () => {
        setupData({
            0: {
                cells: {
                    0: { text: 'Bold', style: 0 },
                    1: { text: 'Italic', style: 1 },
                    2: { text: '100% & $Money' }
                }
            }
        });

        const latex = editor.generateLatex();
        expect(latex).toContain('\\textbf{Bold}');
        expect(latex).toContain('\\textit{Italic}');
        expect(latex).toContain('100\\% \\& \\$Money');
    });

    test('handles skipping completely empty rows in middle', () => {
        // Row 0: Data
        // Row 1: Empty
        // Row 2: Data
        setupData({
            0: { cells: { 0: { text: 'Row1' } } },
            1: { cells: { 0: { text: '' } } },
            2: { cells: { 0: { text: 'Row3' } } }
        });

        const latex = editor.generateLatex();
        expect(latex.match(/\\\\/g).length).toBe(2); // Only 2 rows
        expect(latex).toContain('Row1');
        expect(latex).toContain('Row3');
    });
});
