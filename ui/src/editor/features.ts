// Background features per model: grammar squiggles, git gutter and build errors.

import * as monaco from "./monaco";
import { commands, type Diagnostic, type GutterMarkers } from "../bindings";
import { allModels, getModel, onModelChange, onModelCreated, pathOf } from "./models";
import { LATEX } from "./latex";

const timers = new Map<string, { grammar?: number; gutter?: number }>();
const gutterDecorations = new Map<monaco.editor.ITextModel, string[]>();
/** Markers from the last build, by file, so files opened later show them too. */
let buildMarkers = new Map<string, monaco.editor.IMarkerData[]>();

function debounce(key: string, kind: "grammar" | "gutter", ms: number, fn: () => void) {
  const t = timers.get(key) ?? {};
  window.clearTimeout(t[kind]);
  t[kind] = window.setTimeout(fn, ms);
  timers.set(key, t);
}

async function checkGrammar(model: monaco.editor.ITextModel) {
  if (model.isDisposed() || model.getLanguageId() !== LATEX || pathOf(model)?.endsWith(".bib")) return;
  const version = model.getVersionId();
  try {
    const issues = await commands.grammarCheck(model.getValue(), null);
    if (model.isDisposed() || model.getVersionId() !== version) return;
    monaco.editor.setModelMarkers(
      model,
      "grammar",
      issues.map((i) => ({
        severity: monaco.MarkerSeverity.Warning,
        message: i.message,
        startLineNumber: i.line + 1,
        startColumn: i.from + 1,
        endLineNumber: i.line + 1,
        endColumn: Math.max(i.to, i.from + 1) + 1,
        source: "Grammar",
      })),
    );
  } catch {
    // A failed check leaves the previous markers.
  }
}

function gutterDecorationsFor(m: GutterMarkers): monaco.editor.IModelDeltaDecoration[] {
  const out: monaco.editor.IModelDeltaDecoration[] = [];
  m.lines.forEach((change, row) => {
    if (change)
      out.push({
        range: new monaco.Range(row + 1, 1, row + 1, 1),
        options: { linesDecorationsClassName: change === "added" ? "git-added" : "git-modified" },
      });
  });
  for (const after of m.removedAfter) {
    const line = after === null ? 1 : after + 1;
    out.push({
      range: new monaco.Range(line, 1, line, 1),
      options: { linesDecorationsClassName: after === null ? "git-removed-top" : "git-removed" },
    });
  }
  return out;
}

async function updateGutter(model: monaco.editor.ITextModel) {
  const path = pathOf(model);
  if (!path || model.isDisposed()) return;
  try {
    const markers = await commands.gitLineMarkers(path, model.getValue());
    if (model.isDisposed()) return;
    const old = gutterDecorations.get(model) ?? [];
    gutterDecorations.set(model, model.deltaDecorations(old, gutterDecorationsFor(markers)));
  } catch {
    // Not in a repository.
  }
}

/** Recomputes git gutters for every open file (after a commit, checkout, ...). */
export function refreshAllGutters() {
  for (const [key, model] of allModels()) debounce(key, "gutter", 50, () => updateGutter(model));
}

/** Shows build errors and warnings on the files they belong to. */
export function showBuildDiagnostics(diagnostics: Diagnostic[]) {
  const byFile = new Map<string, monaco.editor.IMarkerData[]>();
  for (const d of diagnostics) {
    if (!d.file || d.line === null) continue;
    const list = byFile.get(d.file) ?? [];
    list.push({
      severity:
        d.severity === "error"
          ? monaco.MarkerSeverity.Error
          : d.severity === "warning"
            ? monaco.MarkerSeverity.Warning
            : monaco.MarkerSeverity.Info,
      message: d.message,
      startLineNumber: d.line,
      startColumn: 1,
      endLineNumber: d.line,
      endColumn: 1000,
      source: "LaTeX",
    });
    byFile.set(d.file, list);
  }
  buildMarkers = byFile;
  for (const [key, model] of allModels()) monaco.editor.setModelMarkers(model, "latex", byFile.get(key) ?? []);
}

export function installModelFeatures() {
  onModelCreated((key, model) => {
    const markers = buildMarkers.get(key);
    if (markers) monaco.editor.setModelMarkers(model, "latex", markers);
  });
  onModelChange((key, model) => {
    debounce(key, "grammar", 750, () => checkGrammar(model));
    debounce(key, "gutter", 200, () => updateGutter(model));
  });
}

export { getModel };
