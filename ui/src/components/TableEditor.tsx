// Visual table editor (⌘⌥T). Edits the table around the cursor, or inserts a new one.

import { useEffect, useMemo, useState } from "react";
import { Merge, Minus, Plus, Split } from "lucide-react";
import { commands, type ColAlign, type TableModel, type TableOp } from "../bindings";
import { activeEditor } from "../editor/registry";
import * as monaco from "../editor/monaco";
import { Dialog, closeDialog } from "./Dialog";
import { report } from "../actions";

type Cell = [number, number];

/** The `table` (or bare `tabular`) environment containing `offset`, as [start, end). */
function tableAround(text: string, offset: number): [number, number] | null {
  for (const env of ["table", "tabular"]) {
    const open = new RegExp(`\\\\begin\\{${env}\\*?\\}`, "g");
    let m: RegExpExecArray | null;
    while ((m = open.exec(text))) {
      const endTag = text.indexOf(`\\end{${env}`, m.index);
      if (endTag < 0) break;
      const end = text.indexOf("}", endTag) + 1;
      if (offset >= m.index && offset <= end) return [m.index, end];
      open.lastIndex = end;
    }
  }
  return null;
}

const alignLabel = (a: ColAlign) => (typeof a === "string" ? a[0].toLowerCase() : "p");

export function TableEditor() {
  const editor = useMemo(() => activeEditor(), []);
  const [model, setModel] = useState<TableModel | null>(null);
  const [span, setSpan] = useState<[number, number] | null>(null);
  const [active, setActive] = useState<Cell>([0, 0]);
  const [anchor, setAnchor] = useState<Cell | null>(null);
  const [latex, setLatex] = useState("");

  useEffect(() => {
    const m = editor?.getModel();
    const pos = editor?.getPosition();
    let source = "";
    if (m && pos) {
      const text = m.getValue();
      const found = tableAround(text, m.getOffsetAt(pos));
      if (found) {
        setSpan(found);
        source = text.slice(found[0], found[1]);
      }
    }
    commands.tableLoad(source).then(setModel).catch(report);
  }, [editor]);

  useEffect(() => {
    if (model) void commands.tableLatex(model).then(setLatex);
  }, [model]);

  if (!model) return null;

  const apply = async (op: TableOp) => setModel(await commands.tableApply(model, op));
  const setText = (r: number, c: number, text: string) =>
    setModel({
      ...model,
      rows: model.rows.map((row, i) =>
        i !== r ? row : { ...row, cells: row.cells.map((cell, j) => (j !== c ? cell : { ...cell, content: text })) },
      ),
    });

  const sel = anchor ?? active;
  const [r1, r2] = [Math.min(sel[0], active[0]), Math.max(sel[0], active[0])];
  const [c1, c2] = [Math.min(sel[1], active[1]), Math.max(sel[1], active[1])];
  const inSel = (r: number, c: number) => anchor && r >= r1 && r <= r2 && c >= c1 && c <= c2;
  const activeCell = model.rows[active[0]]?.cells[active[1]];

  function write() {
    const m = editor?.getModel();
    if (!editor || !m) return closeDialog();
    const range = span
      ? monaco.Range.fromPositions(m.getPositionAt(span[0]), m.getPositionAt(span[1]))
      : editor.getSelection() ?? new monaco.Range(1, 1, 1, 1);
    editor.executeEdits("table-editor", [{ range, text: latex, forceMoveMarkers: true }]);
    editor.focus();
    closeDialog();
  }

  return (
    <Dialog
      title={span ? "Edit Table" : "Insert Table"}
      wide
      footer={
        <>
          <label className="check">
            <input type="checkbox" checked={model.booktabs} onChange={() => void apply({ op: "toggleBooktabs" })} />
            booktabs
          </label>
          <span className="spacer" />
          <button onClick={closeDialog}>Cancel</button>
          <button className="primary" disabled={!editor} onClick={write}>
            {span ? "Apply" : "Insert"}
          </button>
        </>
      }
    >
      <div className="table-tools">
        <button onClick={() => void apply({ op: "addRow" })}>
          <Plus size={13} /> Row
        </button>
        <button onClick={() => void apply({ op: "addCol" })}>
          <Plus size={13} /> Column
        </button>
        <button onClick={() => void apply({ op: "removeRow", row: active[0] })} disabled={model.rows.length < 2}>
          <Minus size={13} /> Row
        </button>
        <button onClick={() => void apply({ op: "removeCol", col: active[1] })} disabled={model.columns.length < 2}>
          <Minus size={13} /> Column
        </button>
        <span className="sep" />
        <button
          disabled={!anchor || (r1 === r2 && c1 === c2)}
          title="Shift-click a second cell to select a range"
          onClick={() => {
            void apply({ op: "merge", row1: r1, col1: c1, row2: r2, col2: c2 });
            setAnchor(null);
          }}
        >
          <Merge size={13} /> Merge
        </button>
        <button
          disabled={!activeCell || (activeCell.colSpan < 2 && activeCell.rowSpan < 2)}
          onClick={() => void apply({ op: "unmerge", row: active[0], col: active[1] })}
        >
          <Split size={13} /> Unmerge
        </button>
      </div>

      <div className="table-grid-scroll">
        <table className="table-grid">
          <thead>
            <tr>
              {model.columns.map((col, c) => (
                <th key={c}>
                  <button title="Cycle alignment" onClick={() => void apply({ op: "cycleAlign", col: c })}>
                    {alignLabel(col.alignment)}
                  </button>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {model.rows.map((row, r) => (
              <tr key={r}>
                {row.cells.map((cell, c) =>
                  cell.isShadow ? null : (
                    <td
                      key={c}
                      colSpan={cell.colSpan}
                      rowSpan={cell.rowSpan}
                      className={`${active[0] === r && active[1] === c ? "active" : ""} ${inSel(r, c) ? "selected" : ""}`}
                    >
                      <input
                        value={cell.content}
                        style={{ textAlign: ({ r: "right", c: "center" } as const)[alignLabel(model.columns[c].alignment) as "r" | "c"] ?? "left" }}
                        onChange={(e) => setText(r, c, e.target.value)}
                        onMouseDown={(e) => {
                          if (e.shiftKey) setAnchor(anchor ?? active);
                          else setAnchor(null);
                        }}
                        onFocus={() => setActive([r, c])}
                        onPaste={(e) => {
                          const text = e.clipboardData.getData("text/plain");
                          if (!text.includes("\t") && !text.includes("\n")) return;
                          e.preventDefault();
                          void apply({ op: "paste", row: r, col: c, text });
                        }}
                      />
                    </td>
                  ),
                )}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="table-meta">
        <label>
          Caption
          <input value={model.caption ?? ""} onChange={(e) => setModel({ ...model, caption: e.target.value || null })} />
        </label>
        <label>
          Label
          <input
            className="mono"
            placeholder="tab:results"
            value={model.label ?? ""}
            onChange={(e) => setModel({ ...model, label: e.target.value || null })}
          />
        </label>
      </div>
      <pre className="code-preview">{latex}</pre>
    </Dialog>
  );
}
