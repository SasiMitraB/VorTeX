// The Monaco editor of each pane, plus cross-component requests to them.

import type * as monaco from "./monaco";
import { activeTab, get, type PaneId } from "../store";
import { getModel } from "./models";

type Editor = monaco.editor.IStandaloneCodeEditor;

const editors: Partial<Record<PaneId, Editor>> = {};
const viewStates = new Map<string, monaco.editor.ICodeEditorViewState | null>();
const pendingReveal = new Map<string, { line: number; col: number }>();

export function registerEditor(pane: PaneId, editor: Editor | null) {
  if (editor) editors[pane] = editor;
  else delete editors[pane];
}

export const editorOf = (pane: PaneId) => editors[pane];

/** The editor of the active pane, when it shows a text tab. */
export function activeEditor(): Editor | undefined {
  const s = get();
  return activeTab(s)?.kind === "text" ? editors[s.activePane] : undefined;
}

export function saveViewState(tabId: string, editor: Editor) {
  viewStates.set(tabId, editor.saveViewState());
}

export function takeViewState(tabId: string) {
  return viewStates.get(tabId) ?? null;
}

export function forgetTab(tabId: string) {
  viewStates.delete(tabId);
}

/** Puts the cursor at 1-based `line`/`col` in the model, now or once an editor shows it. */
export function reveal(key: string, line: number, col = 1) {
  const model = getModel(key);
  const editor = Object.values(editors).find((e) => e && e.getModel() === model);
  if (editor && model) apply(editor, line, col);
  else pendingReveal.set(key, { line, col });
}

export function applyPendingReveal(key: string, editor: Editor) {
  const r = pendingReveal.get(key);
  if (!r) return;
  pendingReveal.delete(key);
  apply(editor, r.line, r.col);
}

function apply(editor: Editor, line: number, col: number) {
  editor.setPosition({ lineNumber: line, column: col });
  editor.revealLineInCenterIfOutsideViewport(line);
  editor.focus();
}
