
class TableEditor {
    constructor() {
        this.overlay = document.getElementById('modal-overlay');
        this.container = document.getElementById('xspreadsheet');
        this.btnInsert = document.getElementById('te-insert');
        this.btnCancel = document.getElementById('te-cancel');
        this.btnClose = document.getElementById('modal-close');
        this.onInsert = null;
        this.spreadsheet = null;

        this.init();
    }

    init() {
        this.btnInsert.onclick = () => {
            const latex = this.generateLatex();
            if (this.onInsert) this.onInsert(latex);
            this.close();
        };

        this.btnCancel.onclick = () => this.close();
        if (this.btnClose) this.btnClose.onclick = () => this.close();
    }

    open(onInsertCallback) {
        this.onInsert = onInsertCallback;
        this.overlay.style.display = 'flex';

        // Clear container and create new spreadsheet
        this.container.innerHTML = '';

        if (window.x_spreadsheet) {
            this.spreadsheet = window.x_spreadsheet(this.container, {
                mode: 'edit',
                showToolbar: true,
                showGrid: true,
                showContextmenu: true,
                view: {
                    height: () => 360,
                    width: () => 680
                },
                row: {
                    len: 10,
                    height: 25
                },
                col: {
                    len: 10,
                    width: 80
                },
                style: {
                    bgcolor: '#282c34',
                    align: 'left',
                    valign: 'middle',
                    textwrap: false,
                    strike: false,
                    underline: false,
                    color: '#abb2bf',
                    font: {
                        name: 'Helvetica',
                        size: 10,
                        bold: false,
                        italic: false,
                    },
                },
            });

            // Hack: x-data-spreadsheet exposes some style configs via global or we need to patch css for headers.
            // The `style` config above applies to CELLS.
            // For Headers (A,B,C...) and Sidebar (1,2,3...), we might need CSS overrides for canvas drawing? 
            // x-data-spreadsheet draws everything on canvas.
            // Looking at the library, it might not support full dark mode theme config easily via options.
            // But we can try setting default cell styles.

            // Let's load data with default styles.
            this.spreadsheet.loadData({
                styles: [
                    { bgcolor: '#282c34', color: '#abb2bf' }
                ]
            });
        } else {
            console.error('x-data-spreadsheet not loaded');
        }
    }

    close() {
        this.overlay.style.display = 'none';
        this.spreadsheet = null;
    }

    generateLatex() {
        if (!this.spreadsheet) return '';

        // Get data from spreadsheet
        const data = this.spreadsheet.getData();
        if (!data || data.length === 0 || !data[0].rows) return '';

        const sheetData = data[0];
        const rows = sheetData.rows || {};

        // First pass: find all cells with content
        const usedRows = new Set();
        const usedCols = new Set();

        Object.keys(rows).forEach(r => {
            const rowIdx = parseInt(r);
            const cells = rows[r].cells || {};

            Object.keys(cells).forEach(c => {
                const colIdx = parseInt(c);
                const cell = cells[c];

                // Check if cell has actual content
                if (cell && cell.text !== undefined && cell.text !== null && String(cell.text).trim() !== '') {
                    usedRows.add(rowIdx);
                    usedCols.add(colIdx);
                }
            });
        });

        if (usedRows.size === 0 || usedCols.size === 0) return '';

        // Convert to sorted arrays
        const rowIndices = [...usedRows].sort((a, b) => a - b);
        const colIndices = [...usedCols].sort((a, b) => a - b);

        const numCols = colIndices.length;

        // Build column spec (default center)
        const colSpec = Array(numCols).fill('c').join(' ');

        let latex = `\\begin{tabular}{${colSpec}}\n\\toprule\n`;

        rowIndices.forEach((r, rowIndex) => {
            const rowData = rows[r]?.cells || {};
            const rowLatex = [];

            colIndices.forEach(c => {
                const cell = rowData[c];
                let content = '';

                if (cell && cell.text !== undefined) {
                    content = this.escapeLatex(String(cell.text));

                    // Check style
                    if (cell.style !== undefined && sheetData.styles) {
                        const style = sheetData.styles[cell.style];
                        if (style) {
                            if (style.font?.bold) content = `\\textbf{${content}}`;
                            if (style.font?.italic) content = `\\textit{${content}}`;
                        }
                    }
                }

                rowLatex.push(content);
            });

            latex += '  ' + rowLatex.join(' & ') + ' \\\\\n';

            if (rowIndex === 0) latex += '  \\midrule\n';
        });

        latex += '  \\bottomrule\n\\end{tabular}';
        return latex;
    }

    escapeLatex(text) {
        return text
            .replace(/\\/g, '\\textbackslash{}')
            .replace(/([&%$#_{}])/g, '\\$1')
            .replace(/~/g, '\\textasciitilde{}')
            .replace(/\^/g, '\\textasciicircum{}');
    }
}

export default TableEditor;
