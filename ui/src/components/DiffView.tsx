// Unified diff of a file at a revision against the working copy (or unsaved buffer).

import { useEffect, useState } from "react";
import { GitCompare, RotateCw } from "lucide-react";
import { commands, type DiffRow, type FileDiff } from "../bindings";
import { getModel } from "../editor/models";
import { relativeTo } from "../lib/paths";
import { useApp } from "../store";

const MAX_ROWS = 4000;
const encoder = new TextEncoder();

/** Splits `text` into [plain, emphasized, plain, ...] using UTF-8 byte ranges. */
function segments(text: string, ranges: { start: number; end: number }[]) {
  if (!ranges.length) return [{ text, em: false }];
  const bytes = encoder.encode(text);
  const decode = (a: number, b: number) => new TextDecoder().decode(bytes.slice(a, b));
  const out: { text: string; em: boolean }[] = [];
  let at = 0;
  for (const r of ranges) {
    if (r.start > at) out.push({ text: decode(at, r.start), em: false });
    out.push({ text: decode(r.start, r.end), em: true });
    at = r.end;
  }
  if (at < bytes.length) out.push({ text: decode(at, bytes.length), em: false });
  return out;
}

function Row({ row }: { row: DiffRow }) {
  if (row.type === "skipped")
    return <div className="diff-skipped">⋯ {row.lines} unchanged {row.lines === 1 ? "line" : "lines"}</div>;
  const sign = row.kind === "added" ? "+" : row.kind === "removed" ? "−" : " ";
  return (
    <div className={`diff-line ${row.kind}`}>
      <span className="diff-no">{row.oldNo ?? ""}</span>
      <span className="diff-no">{row.newNo ?? ""}</span>
      <span className="diff-sign">{sign}</span>
      <span className="diff-text">
        {row.text === ""
          ? " "
          : segments(row.text, row.emphasis).map((s, i) => (s.em ? <mark key={i}>{s.text}</mark> : <span key={i}>{s.text}</span>))}
      </span>
    </div>
  );
}

export function DiffView({ path, rev, label, visible }: { path: string; rev: string | null; label: string; visible: boolean }) {
  const project = useApp((s) => s.project?.path);
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    if (!visible) return;
    // Unsaved edits count as the working copy.
    const model = getModel(path);
    commands
      .gitDiff(path, rev, model ? model.getValue() : null)
      .then((d) => (setDiff(d), setError(null)))
      .catch((e) => setError(String(e)));
  }, [path, rev, nonce, visible]);

  return (
    <div className="diff-view" style={{ display: visible ? undefined : "none" }}>
      <div className="pane-toolbar">
        <GitCompare size={14} className="accent" />
        <span className="pane-toolbar-title">{relativeTo(project, path)}</span>
        <span className="muted ellipsis">{label} ↔ Working copy</span>
        <span className="spacer" />
        {diff && (
          <span className="mono">
            <span className="added-text">+{diff.added}</span> <span className="removed-text">−{diff.removed}</span>
          </span>
        )}
        <button className="icon-btn" title="Refresh" onClick={() => setNonce((n) => n + 1)}>
          <RotateCw size={14} />
        </button>
      </div>
      <div className="diff-scroll">
        {error && <div className="empty-state">{error}</div>}
        {!error && !diff && <div className="empty-state">Loading diff…</div>}
        {diff && diff.rows.length === 0 && <div className="empty-state">No differences</div>}
        {diff?.rows.slice(0, MAX_ROWS).map((row, i) => <Row key={i} row={row} />)}
        {diff && diff.rows.length > MAX_ROWS && <div className="diff-skipped">Diff truncated after {MAX_ROWS} lines</div>}
      </div>
    </div>
  );
}
