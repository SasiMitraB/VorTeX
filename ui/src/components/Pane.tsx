import { Columns2, FileText, GitCompare, X } from "lucide-react";
import { set, useApp, type PaneId, type Tab } from "../store";
import { activateTab, closeTab, splitActive } from "../actions";
import { EditorPane } from "../editor/EditorPane";
import { PdfView } from "../pdf/PdfView";
import { DiffView } from "./DiffView";
import { fileIcon } from "./Sidebar";
import { relativeTo } from "../lib/paths";

function tabIcon(tab: Tab) {
  if (tab.kind === "pdf") return <FileText size={13} className="icon-pdf" />;
  if (tab.kind === "diff") return <GitCompare size={13} className="accent" />;
  return fileIcon(tab.path ?? "untitled.tex", 13);
}

function TabBar({ pane }: { pane: PaneId }) {
  const { tabs, activeId } = useApp((s) => s.panes[pane]);
  const dirty = useApp((s) => s.dirty);
  const isActivePane = useApp((s) => s.activePane === pane);
  return (
    <div className="tab-bar">
      <div className="tabs">
        {tabs.map((t) => {
          const isDirty = t.kind === "text" && dirty[t.key];
          return (
            <div
              key={t.id}
              className={`tab ${t.id === activeId ? "active" : ""} ${isActivePane ? "pane-focused" : ""}`}
              title={t.kind === "diff" ? t.label : (t.path ?? t.title)}
              onMouseDown={(e) => {
                if (e.button === 1) void closeTab(pane, t.id);
                else activateTab(pane, t.id);
              }}
            >
              {tabIcon(t)}
              <span className="tab-title">{t.title}</span>
              <button
                className={`tab-close ${isDirty ? "dirty" : ""}`}
                title="Close"
                onMouseDown={(e) => e.stopPropagation()}
                onClick={() => void closeTab(pane, t.id)}
              >
                <span className="dot" />
                <X size={12} />
              </button>
            </div>
          );
        })}
      </div>
      {pane === "left" && tabs.length > 0 && (
        <button className="icon-btn split-btn" title="Split: show this tab in the right pane" onClick={splitActive}>
          <Columns2 size={14} />
        </button>
      )}
    </div>
  );
}

function Breadcrumb({ tab }: { tab: Extract<Tab, { kind: "text" }> }) {
  const project = useApp((s) => s.project);
  const line = useApp((s) => s.cursor?.line);
  const parts = tab.path ? relativeTo(project?.path, tab.path).split("/") : [tab.title];
  return (
    <div className="breadcrumb">
      {project && <span>{project.name}</span>}
      {parts.map((p, i) => (
        <span key={i} className={i === parts.length - 1 ? "current" : ""}>
          {p}
        </span>
      ))}
      {line && <span className="muted">Ln {line}</span>}
    </div>
  );
}

export function Pane({ pane }: { pane: PaneId }) {
  const { tabs, activeId } = useApp((s) => s.panes[pane]);
  const active = tabs.find((t) => t.id === activeId) ?? null;
  const text = active?.kind === "text" ? active : null;

  return (
    <div className="pane" onMouseDownCapture={() => set({ activePane: pane })}>
      <TabBar pane={pane} />
      <div className="pane-body">
        {text && <Breadcrumb tab={text} />}
        <div className="editor-slot" style={{ display: text ? undefined : "none" }}>
          <EditorPane pane={pane} tab={text} />
        </div>
        {tabs.map((t) =>
          t.kind === "pdf" ? (
            <PdfView key={t.id} path={t.path} visible={t.id === activeId} />
          ) : t.kind === "diff" ? (
            <DiffView key={t.id} path={t.path} rev={t.rev} label={t.label} visible={t.id === activeId} />
          ) : null,
        )}
        {!active && <EmptyPane />}
      </div>
    </div>
  );
}

function EmptyPane() {
  const rows: [string, string][] = [
    ["Open file", "⌘O"],
    ["New tab", "⌘N"],
    ["Build", "⌘B"],
    ["Sync PDF to cursor", "⌘J"],
    ["Toggle sidebar", "⌘\\"],
    ["Show projects", "⌘⇧P"],
  ];
  return (
    <div className="empty-pane">
      <div className="empty-logo">VorTeX</div>
      <dl>
        {rows.map(([what, key]) => (
          <div key={what}>
            <dt>{what}</dt>
            <dd>{key}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}
