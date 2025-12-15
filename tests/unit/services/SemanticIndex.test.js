const semanticIndex = require('../../../src/services/SemanticIndex.js');
const { parseLatexFile } = require('../../../src/services/LatexParser.js');

// Mock dependencies
jest.mock('fs', () => ({
    promises: {
        readFile: jest.fn(),
        writeFile: jest.fn(),
        mkdir: jest.fn(),
    },
}));

jest.mock('../../../src/services/LatexParser.js', () => ({
    parseLatexFile: jest.fn(),
}));

describe('SemanticIndex.js', () => {
    beforeEach(() => {
        semanticIndex.clear();
        jest.clearAllMocks();
    });

    test('initProject sets current project', async () => {
        // Mock loadFromDisk strictness
        const fs = require('fs').promises;
        fs.readFile.mockRejectedValue({ code: 'ENOENT' });

        await semanticIndex.initProject('/path/to/proj');
        expect(semanticIndex.currentProject).toBe('/path/to/proj');
    });

    test('updateFile adds labels correctly', async () => {
        parseLatexFile.mockResolvedValue({
            labels: [{ key: 'fig:1', file: 'test.tex' }],
            citations: [],
            refs: [],
            sections: [],
            bibliographies: []
        });

        await semanticIndex.updateFile('test.tex', '\\label{fig:1}');

        expect(semanticIndex.labels.size).toBe(1);
        expect(semanticIndex.getLabelsForFile('test.tex')).toHaveLength(1);
    });
});
