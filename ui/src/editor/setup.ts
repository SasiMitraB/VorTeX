// One-time Monaco setup: worker, LaTeX language, themes, completion and math hover.

import * as monaco from "./monaco";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import { convertFileSrc } from "@tauri-apps/api/core";
import { commands, type CompletionKind } from "../bindings";
import { LATEX, registerLatex, themeTextHex } from "./latex";
import { pathOf } from "./models";
import { installModelFeatures } from "./features";
import { effectiveTheme, get } from "../store";

self.MonacoEnvironment = { getWorker: () => new EditorWorker() };

const KIND: Record<CompletionKind, monaco.languages.CompletionItemKind> = {
  reference: monaco.languages.CompletionItemKind.Reference,
  citation: monaco.languages.CompletionItemKind.Text,
  environment: monaco.languages.CompletionItemKind.Module,
  command: monaco.languages.CompletionItemKind.Function,
};

/** Escapes text for a snippet, then marks the cursor position with `$0`. */
function snippet(text: string, cursor: number) {
  const esc = (s: string) => s.replace(/[\\$}]/g, "\\$&");
  return esc(text.slice(0, cursor)) + "$0" + esc(text.slice(cursor));
}

function registerCompletion() {
  monaco.languages.registerCompletionItemProvider(LATEX, {
    triggerCharacters: ["\\", "{", ","],
    async provideCompletionItems(model, position) {
      const line = model.getLineContent(position.lineNumber);
      const result = await commands.complete(line, position.column - 1, pathOf(model)).catch(() => null);
      if (!result) return { suggestions: [] };
      const range = new monaco.Range(position.lineNumber, result.from + 1, position.lineNumber, result.to + 1);
      return {
        suggestions: result.items.map((item, i) => ({
          label: { label: item.label, description: item.detail ?? undefined },
          kind: KIND[item.kind],
          detail: item.detail ?? undefined,
          documentation: item.documentation ?? undefined,
          insertText: snippet(item.insertText, item.cursorOffset),
          insertTextRules:
            monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet |
            monaco.languages.CompletionItemInsertTextRule.KeepWhitespace,
          // Rust already ranked these; keep that order and match on the key itself.
          sortText: String(i).padStart(4, "0"),
          filterText: item.label.replace(/^\\/, ""),
          range,
        })),
      };
    },
  });
}

const images = new Map<string, Promise<string>>();

/** The PNG as a data: URL (Monaco's hover markdown won't load custom-scheme URLs). */
function dataUrl(path: string) {
  let p = images.get(path);
  if (!p) {
    p = fetch(convertFileSrc(path))
      .then((r) => r.blob())
      .then(
        (blob) =>
          new Promise<string>((resolve, reject) => {
            const reader = new FileReader();
            reader.onload = () => resolve(reader.result as string);
            reader.onerror = reject;
            reader.readAsDataURL(blob);
          }),
      );
    images.set(path, p);
  }
  return p;
}

function registerMathHover() {
  monaco.languages.registerHoverProvider(LATEX, {
    async provideHover(model, position) {
      const text = model.getValue();
      const at = await commands.mathAt(text, position.lineNumber - 1, position.column - 1).catch(() => null);
      if (!at) return null;
      const range = new monaco.Range(at.fromLine + 1, at.fromCol + 1, at.toLine + 1, at.toCol + 1);
      const color = themeTextHex(effectiveTheme(get()));
      try {
        const png = await commands.mathRender(text, pathOf(model), at.snippet, color);
        const src = await dataUrl(png.pngPath);
        // 1pt ≈ 1.33 CSS px; a little larger reads better in a hover.
        const w = Math.round((png.widthPt ?? 0) * 1.6);
        const h = Math.round((png.heightPt ?? 0) * 1.6);
        return {
          range,
          contents: [
            { value: `**${at.label}**` },
            { value: `<img src="${src}" width="${w}" height="${h}">`, supportHtml: true },
          ],
        };
      } catch (e) {
        return { range, contents: [{ value: `**${at.label}**` }, { value: `Could not render: ${String(e)}` }] };
      }
    },
  });
}

function registerKeybindings() {
  const { KeyMod, KeyCode } = monaco;
  monaco.editor.addKeybindingRules([
    // App shortcuts that Monaco would otherwise take.
    { keybinding: KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.KeyO, command: "-editor.action.quickOutline" },
    { keybinding: KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.KeyP, command: "-editor.action.quickCommand" },
    // ⌘D duplicates the line or selection, as in the GPUI editor.
    { keybinding: KeyMod.CtrlCmd | KeyCode.KeyD, command: "-editor.action.addSelectionToNextFindMatch" },
    { keybinding: KeyMod.CtrlCmd | KeyCode.KeyD, command: "editor.action.copyLinesDownAction" },
  ]);
}

let done = false;

export function setupMonaco() {
  if (done) return;
  done = true;
  registerLatex();
  registerCompletion();
  registerMathHover();
  registerKeybindings();
  installModelFeatures();
}
