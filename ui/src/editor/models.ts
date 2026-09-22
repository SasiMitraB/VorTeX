// One Monaco model per open file (shared by both panes), keyed by path, or
// `untitled:N` for new buffers. Tracks which models have unsaved changes.

import * as monaco from "./monaco";
import { LATEX } from "./latex";
import { extname } from "../lib/paths";
import { get, set } from "../store";

const models = new Map<string, monaco.editor.ITextModel>();
const keys = new WeakMap<monaco.editor.ITextModel, string>();
const saved = new Map<string, number>();
const listeners: Array<(key: string, model: monaco.editor.ITextModel) => void> = [];
const created: Array<(key: string, model: monaco.editor.ITextModel) => void> = [];
let untitled = 0;

const LATEX_EXTS = ["tex", "sty", "cls", "bib", "ltx", "bbx", "cbx", "dtx"];

export function languageFor(path: string | null) {
  return path === null || LATEX_EXTS.includes(extname(path)) ? LATEX : "plaintext";
}

/** Called for every new model and after each edit (features hook in here). */
export function onModelChange(fn: (key: string, model: monaco.editor.ITextModel) => void) {
  listeners.push(fn);
}

/** Called once for every new model. */
export function onModelCreated(fn: (key: string, model: monaco.editor.ITextModel) => void) {
  created.push(fn);
}

export function nextUntitledKey() {
  untitled += 1;
  return { key: `untitled:${untitled}`, title: `Untitled-${untitled}` };
}

export function getModel(key: string) {
  return models.get(key);
}

export function keyOf(model: monaco.editor.ITextModel) {
  return keys.get(model);
}

/** The file path of a model, or null for untitled buffers. */
export function pathOf(model: monaco.editor.ITextModel): string | null {
  const key = keys.get(model);
  return key && !key.startsWith("untitled:") ? key : null;
}

export function createModel(key: string, text: string) {
  const existing = models.get(key);
  if (existing) return existing;
  const model = monaco.editor.createModel(text, languageFor(key.startsWith("untitled:") ? null : key));
  model.updateOptions({ tabSize: 2, insertSpaces: true });
  models.set(key, model);
  keys.set(model, key);
  saved.set(key, model.getAlternativeVersionId());
  model.onDidChangeContent(() => {
    const k = keys.get(model)!;
    const dirty = model.getAlternativeVersionId() !== saved.get(k);
    if (!!get().dirty[k] !== dirty) set((s) => ({ dirty: { ...s.dirty, [k]: dirty } }));
    listeners.forEach((fn) => fn(k, model));
  });
  created.forEach((fn) => fn(key, model));
  listeners.forEach((fn) => fn(key, model));
  return model;
}

export function markSaved(key: string) {
  const model = models.get(key);
  if (!model) return;
  saved.set(key, model.getAlternativeVersionId());
  set((s) => ({ dirty: { ...s.dirty, [key]: false } }));
}

/** Replaces the text of a clean model (the file changed on disk). */
export function reloadIfClean(key: string, text: string) {
  const model = models.get(key);
  if (!model || get().dirty[key] || model.getValue() === text) return;
  model.pushEditOperations([], [{ range: model.getFullModelRange(), text }], () => null);
  saved.set(key, model.getAlternativeVersionId());
}

/** After Save As: the untitled buffer becomes the file at `path`. */
export function rekey(oldKey: string, newKey: string) {
  const model = models.get(oldKey);
  if (!model) return;
  models.delete(oldKey);
  models.set(newKey, model);
  keys.set(model, newKey);
  saved.set(newKey, saved.get(oldKey)!);
  saved.delete(oldKey);
  monaco.editor.setModelLanguage(model, languageFor(newKey));
  set((s) => {
    const { [oldKey]: _, ...dirty } = s.dirty;
    return { dirty };
  });
}

export function disposeModel(key: string) {
  models.get(key)?.dispose();
  models.delete(key);
  saved.delete(key);
  set((s) => {
    const { [key]: _, ...dirty } = s.dirty;
    return { dirty };
  });
}

export function dirtyKeys() {
  return Object.entries(get().dirty)
    .filter(([, d]) => d)
    .map(([k]) => k);
}

export function allModels() {
  return [...models.entries()];
}
