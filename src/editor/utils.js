import { state } from '../state.js';

/**
 * Get current file path from active editor state
 */
export function getCurrentFilePath() {
  const activePane = state.activePane;
  const paneState = state.paneState[activePane];
  if (paneState && paneState.activeId) {
    const tab = state.paneTabs[activePane].find(t => t.id === paneState.activeId);
    if (tab && tab.path) {
      return tab.path;
    }
  }
  return null;
}

/**
 * Get word range for replacement
 */
export function getWordRange(monaco, position, queryLength) {
  return {
    startLineNumber: position.lineNumber,
    startColumn: position.column - queryLength,
    endLineNumber: position.lineNumber,
    endColumn: position.column
  };
}
