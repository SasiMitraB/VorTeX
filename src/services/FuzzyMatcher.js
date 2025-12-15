const fuzzysortRaw = require('fuzzysort');
const fuzzysort = fuzzysortRaw.default || fuzzysortRaw;

/**
 * Default options for fuzzy search
 */
const defaultOptions = {
  threshold: -1000,
  limit: 10,
  allowTypo: true
};

/**
 * Perform fuzzy search on candidates
 * @param {string} query - Search query
 * @param {Array} candidates - Array of candidate objects
 * @param {Object} options - Search options
 * @returns {Array} - Sorted matching results
 */
function fuzzySearch(query, candidates, options = {}) {
  if (!query || query.length === 0) {
    // Return all candidates if no query, limited by limit option
    const limit = options.limit || defaultOptions.limit;
    return candidates.slice(0, limit).map(c => ({
      obj: c,
      score: 0,
      highlighted: null
    }));
  }

  const mergedOptions = { ...defaultOptions, ...options };

  // Determine which keys to search based on candidate type
  const keys = mergedOptions.keys || determineKeys(candidates[0]);

  const results = fuzzysort.go(query, candidates, {
    keys,
    threshold: mergedOptions.threshold,
    limit: mergedOptions.limit,
    allowTypo: mergedOptions.allowTypo
  });

  return results.map(result => ({
    obj: result.obj,
    score: result.score,
    highlighted: formatHighlighted(result, keys)
  }));
}

/**
 * Determine search keys based on object type
 */
function determineKeys(sample) {
  if (!sample) return ['key'];

  if (sample.type === 'label') {
    return ['key', 'filename'];
  }

  if (sample.type === 'bibentry') {
    return ['key', 'fields.title', 'fields.author'];
  }

  return ['key'];
}

/**
 * Format highlighted matches for display
 */
function formatHighlighted(result, keys) {
  const highlighted = {};

  for (let i = 0; i < keys.length; i++) {
    const key = keys[i];
    const match = result[i];

    if (match && match.highlight) {
      if (typeof fuzzysort.highlight === 'function') {
        highlighted[key] = fuzzysort.highlight(match, '<mark>', '</mark>');
      } else {
        // Fallback: Use simple string replacement or just return target
        // For now, just return target to prevent crash
        highlighted[key] = match.target;
      }
    }
  }

  return highlighted;
}

/**
 * Search labels with prioritization for current file
 * @param {string} query - Search query
 * @param {Array} labels - Array of label objects
 * @param {string} currentFile - Current file path (to prioritize)
 * @returns {Array} - Sorted matching results
 */
function searchLabels(query, labels, currentFile = null) {
  // If we have a current file, boost results from that file
  let results = fuzzySearch(query, labels, {
    keys: ['key', 'filename'],
    limit: 15 // Get more results for re-sorting
  });

  if (currentFile) {
    // Sort by: current file first, then by score
    results.sort((a, b) => {
      const aIsCurrentFile = a.obj.file === currentFile;
      const bIsCurrentFile = b.obj.file === currentFile;

      if (aIsCurrentFile && !bIsCurrentFile) return -1;
      if (!aIsCurrentFile && bIsCurrentFile) return 1;

      // Same file status, sort by score (higher is better, scores are negative)
      return b.score - a.score;
    });
  }

  return results.slice(0, 10);
}

/**
 * Search bibliography entries
 * @param {string} query - Search query
 * @param {Array} bibentries - Array of bibentry objects
 * @returns {Array} - Sorted matching results
 */
function searchBibEntries(query, bibentries) {
  return fuzzySearch(query, bibentries, {
    keys: ['key', 'fields.title', 'fields.author'],
    limit: 10
  });
}

/**
 * Filter labels by prefix (e.g., 'fig:', 'eq:', 'sec:')
 * @param {Array} labels - Array of label objects
 * @param {string} prefix - Prefix to filter by (optional)
 * @returns {Array} - Filtered labels
 */
function filterByPrefix(labels, prefix) {
  if (!prefix) return labels;
  return labels.filter(l => l.key.startsWith(prefix));
}

/**
 * Extract prefix from partial key
 * e.g., 'fig:re' -> 'fig:'
 */
function extractPrefix(query) {
  const colonIndex = query.indexOf(':');
  if (colonIndex > 0) {
    return query.substring(0, colonIndex + 1);
  }
  return null;
}

module.exports = {
  fuzzySearch,
  searchLabels,
  searchBibEntries,
  filterByPrefix,
  extractPrefix
};
