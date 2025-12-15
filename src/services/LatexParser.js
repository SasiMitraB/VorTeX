const latexUtensils = require('latex-utensils');
const latexParser = latexUtensils.latexParser;
const path = require('path');

// Cache for parsed results
const parseCache = new Map();

/**
 * Parse a LaTeX file and extract semantic elements
 * @param {string} filepath - Path to the file
 * @param {string} content - File content
 * @returns {Promise<{labels: Array, citations: Array, refs: Array, bibliographies: Array}>}
 */
async function parseLatexFile(filepath, content) {
  try {
    // Check cache
    const cacheKey = `${filepath}:${hashContent(content)}`;
    if (parseCache.has(cacheKey)) {
      return parseCache.get(cacheKey);
    }

    const result = {
      labels: [],
      citations: [],
      refs: [],
      bibliographies: [],
      sections: []
    };

    // Parse with latex-utensils
    let ast;
    try {
      ast = latexParser.parse(content, { enableMathCharacterLocation: false });
    } catch (parseError) {
      // If full parse fails, try line-by-line regex extraction
      console.warn(`LaTeX parse warning for ${filepath}:`, parseError.message);
      return extractWithRegex(filepath, content);
    }

    // Walk the AST
    walkAst(ast, filepath, result);

    // Cache the result
    parseCache.set(cacheKey, result);

    // Limit cache size
    if (parseCache.size > 100) {
      const firstKey = parseCache.keys().next().value;
      parseCache.delete(firstKey);
    }

    return result;
  } catch (error) {
    console.error(`Error parsing LaTeX file ${filepath}:`, error);
    return { labels: [], citations: [], refs: [], bibliographies: [], sections: [] };
  }
}

/**
 * Walk the AST and extract semantic elements
 */
function walkAst(node, filepath, result) {
  if (!node) return;

  // Handle arrays
  if (Array.isArray(node)) {
    node.forEach(child => walkAst(child, filepath, result));
    return;
  }

  // Handle objects with content
  if (typeof node === 'object') {
    // Check for command nodes
    if (node.kind === 'command') {
      const cmdName = node.name?.toLowerCase() || '';
      const line = node.location?.start?.line || 0;

      // Extract \label{...}
      if (cmdName === 'label') {
        const key = extractArgument(node);
        if (key) {
          result.labels.push({
            type: 'label',
            key,
            file: filepath,
            filename: path.basename(filepath),
            line
          });
        }
      }

      // Extract \cite{...}, \citep{...}, \citet{...}, \autocite{...}
      if (['cite', 'citep', 'citet', 'autocite', 'parencite', 'textcite'].includes(cmdName)) {
        const keys = extractCitationKeys(node);
        if (keys.length > 0) {
          result.citations.push({
            type: 'citation',
            keys,
            file: filepath,
            filename: path.basename(filepath),
            line
          });
        }
      }

      // Extract \ref{...}, \eqref{...}, \cref{...}, \autoref{...}
      if (['ref', 'eqref', 'cref', 'autoref', 'pageref'].includes(cmdName)) {
        const key = extractArgument(node);
        if (key) {
          result.refs.push({
            type: 'ref',
            key,
            file: filepath,
            filename: path.basename(filepath),
            line
          });
        }
      }

      // Extract \bibliography{...} and \addbibresource{...}
      if (['bibliography', 'addbibresource'].includes(cmdName)) {
        const bibFile = extractArgument(node);
        if (bibFile) {
          result.bibliographies.push({
            type: 'bibliography',
            file: bibFile,
            sourceFile: filepath,
            line
          });
        }
      }

      // Extract sections
      if (['section', 'subsection', 'subsubsection', 'paragraph', 'subparagraph', 'chapter', 'part'].includes(cmdName)) {
        const title = extractArgument(node);
        if (title) {
          result.sections.push({
            type: 'section',
            level: getSectionLevel(cmdName),
            title,
            file: filepath,
            line
          });
        }
      }
    }

    // Recurse into children
    for (const key of Object.keys(node)) {
      if (key !== 'location' && node[key] && typeof node[key] === 'object') {
        walkAst(node[key], filepath, result);
      }
    }
  }
}

/**
 * Extract the first argument from a command node
 */
function extractArgument(node) {
  if (!node.args || node.args.length === 0) return null;

  const arg = node.args[0];
  if (!arg || !arg.content) return null;

  return extractTextContent(arg.content);
}

/**
 * Extract multiple citation keys (handles comma-separated)
 */
function extractCitationKeys(node) {
  const argText = extractArgument(node);
  if (!argText) return [];

  return argText.split(',').map(k => k.trim()).filter(k => k.length > 0);
}

/**
 * Extract text content from AST nodes
 */
function extractTextContent(content) {
  if (!content) return '';

  if (typeof content === 'string') return content;

  if (Array.isArray(content)) {
    return content.map(extractTextContent).join('');
  }

  if (content.kind === 'text.string') {
    return content.content || '';
  }

  if (content.content) {
    return extractTextContent(content.content);
  }

  return '';
}

/**
 * Fallback regex-based extraction for malformed LaTeX
 */
function extractWithRegex(filepath, content) {
  const result = {
    labels: [],
    citations: [],
    refs: [],
    bibliographies: [],
    sections: []
  };

  const lines = content.split('\n');

  lines.forEach((line, index) => {
    const lineNum = index + 1;

    // Extract \label{...}
    const labelMatches = line.matchAll(/\\label\s*\{([^}]+)\}/g);
    for (const match of labelMatches) {
      result.labels.push({
        type: 'label',
        key: match[1],
        file: filepath,
        filename: path.basename(filepath),
        line: lineNum
      });
    }

    // Extract \cite variants
    const citeMatches = line.matchAll(/\\(?:cite|citep|citet|autocite|parencite|textcite)\s*\{([^}]+)\}/g);
    for (const match of citeMatches) {
      const keys = match[1].split(',').map(k => k.trim()).filter(k => k);
      if (keys.length > 0) {
        result.citations.push({
          type: 'citation',
          keys,
          file: filepath,
          filename: path.basename(filepath),
          line: lineNum
        });
      }
    }

    // Extract \ref variants
    const refMatches = line.matchAll(/\\(?:ref|eqref|cref|autoref|pageref)\s*\{([^}]+)\}/g);
    for (const match of refMatches) {
      result.refs.push({
        type: 'ref',
        key: match[1],
        file: filepath,
        filename: path.basename(filepath),
        line: lineNum
      });
    }

    // Extract bibliography references
    const bibMatches = line.matchAll(/\\(?:bibliography|addbibresource)\s*\{([^}]+)\}/g);
    for (const match of bibMatches) {
      result.bibliographies.push({
        type: 'bibliography',
        file: match[1],
        sourceFile: filepath,
        line: lineNum
      });
    }

    // Extract sections
    const sectionMatches = line.matchAll(/\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)\*?\s*\{([^}]+)\}/g);
    for (const match of sectionMatches) {
      result.sections.push({
        type: 'section',
        level: getSectionLevel(match[1]),
        title: match[2],
        file: filepath,
        line: lineNum
      });
    }
  });

  return result;
}

function getSectionLevel(cmd) {
  switch (cmd) {
    case 'part': return 0;
    case 'chapter': return 1;
    case 'section': return 2;
    case 'subsection': return 3;
    case 'subsubsection': return 4;
    case 'paragraph': return 5;
    case 'subparagraph': return 6;
    default: return 2;
  }
}

/**
 * Simple hash for cache key
 */
function hashContent(content) {
  let hash = 0;
  for (let i = 0; i < content.length; i++) {
    const char = content.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash;
  }
  return hash.toString(16);
}

/**
 * Clear the parse cache
 */
function clearCache() {
  parseCache.clear();
}

module.exports = {
  parseLatexFile,
  clearCache
};
