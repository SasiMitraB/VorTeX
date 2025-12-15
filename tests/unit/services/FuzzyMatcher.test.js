const { searchLabels, searchBibEntries } = require('../../../src/services/FuzzyMatcher.js');

describe('FuzzyMatcher.js', () => {
    const mockLabels = [
        { type: 'label', key: 'fig:test', filename: '/path/to/main.tex' },
        { type: 'label', key: 'eq:alpha', filename: '/path/to/other.tex' },
    ];

    const mockBibs = [
        { type: 'bibentry', key: 'doe2023', fields: { title: 'Deep Learning', author: 'John Doe' } },
    ];

    test('searchLabels finds exact matches', () => {
        const results = searchLabels('fig:test', mockLabels);
        expect(results.length).toBeGreaterThan(0);
        expect(results[0].obj.key).toBe('fig:test');
    });

    test('searchLabels fuzzy matching', () => {
        const results = searchLabels('fi', mockLabels);
        expect(results.some(r => r.obj.key === 'fig:test')).toBe(true);
    });

    test('searchBibEntries finds by title', () => {
        const results = searchBibEntries('Deep', mockBibs);
        expect(results.length).toBe(1);
        expect(results[0].obj.key).toBe('doe2023');
    });
});
