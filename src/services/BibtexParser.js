const bibtexParse = require('bibtex-parser');
const path = require('path');

/**
 * Parse a BibTeX file and extract bibliography entries
 * @param {string} filepath - Path to the file
 * @param {string} content - File content
 * @returns {Promise<Array>} - Array of bibliography entries
 */
async function parseBibFile(filepath, content) {
  try {
    // Pre-process content to handle common encoding issues
    const cleanedContent = preprocessBibContent(content);

    // Parse with bibtex-parser (it exports the parse function directly)
    let entries;
    try {
      entries = bibtexParse(cleanedContent);
    } catch (parseError) {
      console.warn(`BibTeX parse warning for ${filepath}:`, parseError.message);
      // Try fallback regex parsing
      return extractWithRegex(filepath, content);
    }

    const result = [];

    // Process each entry
    for (const [key, entry] of Object.entries(entries)) {
      // Skip string definitions and other non-entry items
      if (key.startsWith('@')) continue;

      result.push({
        type: 'bibentry',
        key: key,
        entryType: entry.type?.toLowerCase() || 'misc',
        file: filepath,
        filename: path.basename(filepath),
        fields: {
          title: cleanField(entry.TITLE || entry.title),
          author: cleanField(entry.AUTHOR || entry.author),
          year: cleanField(entry.YEAR || entry.year),
          journal: cleanField(entry.JOURNAL || entry.journal),
          booktitle: cleanField(entry.BOOKTITLE || entry.booktitle),
          publisher: cleanField(entry.PUBLISHER || entry.publisher),
          doi: cleanField(entry.DOI || entry.doi),
          url: cleanField(entry.URL || entry.url)
        }
      });
    }

    return result;
  } catch (error) {
    console.error(`Error parsing BibTeX file ${filepath}:`, error);
    return [];
  }
}

/**
 * Pre-process BibTeX content to handle encoding issues
 */
function preprocessBibContent(content) {
  return content
    // Normalize line endings
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    // Handle some common LaTeX accents that might cause issues
    .replace(/\\"{([aeiouAEIOU])}/g, '$1')
    .replace(/\\'([aeiouAEIOU])/g, '$1')
    .replace(/\\`([aeiouAEIOU])/g, '$1')
    .replace(/\\~([nN])/g, '$1')
    .replace(/\\c{([cC])}/g, '$1');
}

/**
 * Clean a BibTeX field value
 */
function cleanField(value) {
  if (!value) return '';

  return value
    // Remove surrounding braces
    .replace(/^\{+|\}+$/g, '')
    // Remove LaTeX commands
    .replace(/\\[a-zA-Z]+\{([^}]*)\}/g, '$1')
    .replace(/\\[a-zA-Z]+/g, '')
    // Remove extra braces
    .replace(/\{([^{}]*)\}/g, '$1')
    // Clean up whitespace
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Fallback regex-based extraction for malformed BibTeX
 */
function extractWithRegex(filepath, content) {
  const result = [];

  // Match @type{key, ... }
  const entryRegex = /@(\w+)\s*\{\s*([^,\s]+)\s*,([^@]*?)(?=\n\s*@|\n*$)/gs;

  let match;
  while ((match = entryRegex.exec(content)) !== null) {
    const entryType = match[1].toLowerCase();
    const key = match[2].trim();
    const body = match[3];

    // Skip comments and strings
    if (entryType === 'comment' || entryType === 'string' || entryType === 'preamble') {
      continue;
    }

    const fields = extractFields(body);

    result.push({
      type: 'bibentry',
      key: key,
      entryType: entryType,
      file: filepath,
      filename: path.basename(filepath),
      fields: {
        title: cleanField(fields.title),
        author: cleanField(fields.author),
        year: cleanField(fields.year),
        journal: cleanField(fields.journal),
        booktitle: cleanField(fields.booktitle),
        publisher: cleanField(fields.publisher),
        doi: cleanField(fields.doi),
        url: cleanField(fields.url)
      }
    });
  }

  return result;
}

/**
 * Extract fields from BibTeX entry body
 */
function extractFields(body) {
  const fields = {};
  const fieldRegex = /(\w+)\s*=\s*(?:\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}|"([^"]*)"|(\d+))/g;

  let match;
  while ((match = fieldRegex.exec(body)) !== null) {
    const fieldName = match[1].toLowerCase();
    const value = match[2] || match[3] || match[4] || '';
    fields[fieldName] = value;
  }

  return fields;
}

/**
 * Format a bibliography entry for display
 */
function formatBibEntry(entry) {
  const parts = [];

  if (entry.fields.author) {
    parts.push(entry.fields.author);
  }

  if (entry.fields.year) {
    parts.push(`(${entry.fields.year})`);
  }

  if (entry.fields.title) {
    parts.push(`"${entry.fields.title}"`);
  }

  if (entry.fields.journal) {
    parts.push(entry.fields.journal);
  } else if (entry.fields.booktitle) {
    parts.push(entry.fields.booktitle);
  }

  return parts.join('. ') || entry.key;
}

module.exports = {
  parseBibFile,
  formatBibEntry
};
