import { detectLanguageFromPath } from '../../src/utils.js';

describe('utils.js', () => {
    describe('detectLanguageFromPath', () => {
        test('returns correct language for known extensions', () => {
            expect(detectLanguageFromPath('file.js')).toBe('javascript');
            expect(detectLanguageFromPath('file.tex')).toBe('latex');
            expect(detectLanguageFromPath('file.pdf')).toBe('plaintext'); // Default fallback for unknown map
        });

        test('case insensitivity', () => {
            expect(detectLanguageFromPath('FILE.JS')).toBe('javascript');
        });

        test('plaintext default', () => {
            expect(detectLanguageFromPath('file.unknown')).toBe('plaintext');
        });
    });
});
