import { useEffect, useRef } from "react";
import * as monaco from "./monaco";
import { effectiveTheme, set, useApp, type PaneId, type Tab } from "../store";
import { getModel } from "./models";
import { applyPendingReveal, registerEditor, saveViewState, takeViewState } from "./registry";

type TextTab = Extract<Tab, { kind: "text" }>;

/** Typed symbol → LaTeX command (as in the GPUI editor). */
const UNICODE: Record<string, string> = {
  "→": "\\rightarrow ", "⇒": "\\Rightarrow ", "α": "\\alpha ", "β": "\\beta ", "θ": "\\theta ",
  "λ": "\\lambda ", "π": "\\pi ", "∑": "\\sum ", "∫": "\\int ", "≤": "\\le ", "≥": "\\ge ",
  "≠": "\\neq ", "±": "\\pm ", "∞": "\\infty ",
};

function install(editor: monaco.editor.IStandaloneCodeEditor) {
  // Unicode replacement right after a symbol is typed or pasted on its own.
  editor.onDidChangeModelContent((e) => {
    if (e.isUndoing || e.isRedoing || e.isFlush || e.changes.length !== 1) return;
    const c = e.changes[0];
    const cmd = UNICODE[c.text];
    if (!cmd || c.rangeLength !== 0) return;
    queueMicrotask(() => {
      const at = new monaco.Range(
        c.range.startLineNumber,
        c.range.startColumn,
        c.range.startLineNumber,
        c.range.startColumn + c.text.length,
      );
      editor.executeEdits("unicode", [{ range: at, text: cmd }]);
    });
  });

  // Enter continues `\item` lists; Enter on an empty `\item` ends the list.
  editor.addCommand(
    monaco.KeyCode.Enter,
    () => {
      const model = editor.getModel();
      const pos = editor.getPosition();
      const sel = editor.getSelection();
      if (!model || !pos || !sel?.isEmpty()) return editor.trigger("keyboard", "type", { text: "\n" });
      const line = model.getLineContent(pos.lineNumber);
      const atEnd = pos.column === line.length + 1;
      if (atEnd && /^\s*\\item\s*$/.test(line)) {
        const indent = line.match(/^\s*/)![0];
        editor.executeEdits("item", [
          { range: new monaco.Range(pos.lineNumber, 1, pos.lineNumber, line.length + 1), text: indent },
        ]);
        return;
      }
      editor.trigger("keyboard", "type", { text: "\n" });
      if (/^\s*\\item\b/.test(line) && atEnd) editor.trigger("keyboard", "type", { text: "\\item " });
    },
    "editorTextFocus && !suggestWidgetVisible && !inSnippetMode && !editorReadonly",
  );
}

export function EditorPane({ pane, tab }: { pane: PaneId; tab: TextTab | null }) {
  const host = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const shownTab = useRef<string | null>(null);
  const theme = useApp((s) => effectiveTheme(s));

  useEffect(() => {
    const editor = monaco.editor.create(host.current!, {
      model: null,
      automaticLayout: true,
      theme: `vortex-${theme}`,
      fontFamily: '"SF Mono", ui-monospace, Menlo, monospace',
      fontSize: 13,
      lineHeight: 21,
      wordWrap: "on",
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      renderLineHighlight: "line",
      wordBasedSuggestions: "off",
      quickSuggestions: { other: true, comments: false, strings: true },
      suggest: { showWords: false, preview: false },
      tabSize: 2,
      unicodeHighlight: { ambiguousCharacters: false, invisibleCharacters: false },
      fixedOverflowWidgets: true,
      hover: { delay: 350 },
      lineDecorationsWidth: 10,
      padding: { top: 8 },
      stickyScroll: { enabled: false },
      smoothScrolling: true,
    });
    install(editor);
    editorRef.current = editor;
    registerEditor(pane, editor);

    const cursor = () => {
      const pos = editor.getPosition();
      const model = editor.getModel();
      if (pos && model) set({ cursor: { line: pos.lineNumber, col: pos.column, lines: model.getLineCount() } });
    };
    editor.onDidChangeCursorPosition(cursor);
    editor.onDidFocusEditorText(() => {
      set({ activePane: pane });
      cursor();
    });
    return () => {
      registerEditor(pane, null);
      editor.dispose();
    };
    // Created once per pane; the model is swapped below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    monaco.editor.setTheme(`vortex-${theme}`);
  }, [theme]);

  // Show the active tab's model, keeping each tab's scroll and cursor.
  useEffect(() => {
    const editor = editorRef.current!;
    if (shownTab.current && shownTab.current !== tab?.id) saveViewState(shownTab.current, editor);
    const model = tab ? getModel(tab.key) ?? null : null;
    if (editor.getModel() !== model) {
      editor.setModel(model);
      const view = tab && takeViewState(tab.id);
      if (view) editor.restoreViewState(view);
    }
    shownTab.current = tab?.id ?? null;
    if (tab && model) {
      applyPendingReveal(tab.key, editor);
      const pos = editor.getPosition();
      set({ cursor: { line: pos?.lineNumber ?? 1, col: pos?.column ?? 1, lines: model.getLineCount() } });
    }
  }, [tab?.id, tab?.key]);

  return <div ref={host} className="editor-host" />;
}
