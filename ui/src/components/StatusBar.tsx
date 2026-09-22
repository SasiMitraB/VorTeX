import { AlertCircle, AlertTriangle, CheckCircle2, Loader2 } from "lucide-react";
import { useApp } from "../store";

export function StatusBar() {
  const status = useApp((s) => s.status);
  const building = useApp((s) => s.building);
  const build = useApp((s) => s.lastBuild);
  const sync = useApp((s) => s.syncHint);
  const stats = useApp((s) => s.stats);
  const cursor = useApp((s) => s.cursor);
  const errors = build?.diagnostics.filter((d) => d.severity === "error").length ?? 0;
  const warnings = build?.diagnostics.filter((d) => d.severity === "warning").length ?? 0;

  return (
    <footer className="status-bar">
      <span className="status-item">
        {building ? (
          <Loader2 size={12} className="spin" />
        ) : build ? (
          build.success ? (
            <CheckCircle2 size={12} className="ok" />
          ) : (
            <AlertCircle size={12} className="err" />
          )
        ) : null}
        <span className="ellipsis">{status ?? "Ready"}</span>
        {build && !building && <span className="muted">{(build.durationMs / 1000).toFixed(2)} s</span>}
      </span>
      {build && (errors > 0 || warnings > 0) && (
        <span className="status-item" title="Errors and warnings from the last build">
          <AlertCircle size={12} className={errors ? "err" : "muted"} /> {errors}
          <AlertTriangle size={12} className={warnings ? "warn" : "muted"} /> {warnings}
        </span>
      )}
      {sync && <span className="status-item muted">{sync}</span>}
      <span className="spacer" />
      {stats && (
        <span className="status-item muted">
          {stats.labels} labels · {stats.bibentries} bib entries · {stats.filesIndexed} files
        </span>
      )}
      {cursor && (
        <span className="status-item muted">
          Ln {cursor.line}, Col {cursor.col} · {cursor.lines} lines
        </span>
      )}
    </footer>
  );
}
