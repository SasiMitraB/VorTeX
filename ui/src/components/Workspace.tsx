import { useRef, useState } from "react";
import { ArrowLeft, Loader2, PanelLeft, PanelLeftClose, Play, ScanSearch } from "lucide-react";
import { useApp } from "../store";
import { build, showProjects, syncPdf } from "../actions";
import { Pane } from "./Pane";
import { Ribbon, Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";
import { LatexDiffDialog } from "./LatexDiffDialog";
import { TableEditor } from "./TableEditor";

/** A drag handle; `onDrag` gets the pointer's x in the parent's coordinates. */
function Resizer({ onDrag }: { onDrag: (x: number, width: number) => void }) {
  const ref = useRef<HTMLDivElement>(null);
  return (
    <div
      ref={ref}
      className="resizer"
      onPointerDown={(e) => {
        const parent = ref.current!.parentElement!;
        const rect = parent.getBoundingClientRect();
        ref.current!.setPointerCapture(e.pointerId);
        const move = (ev: PointerEvent) => onDrag(ev.clientX - rect.left, rect.width);
        const up = () => {
          ref.current?.removeEventListener("pointermove", move);
          ref.current?.removeEventListener("pointerup", up);
        };
        ref.current!.addEventListener("pointermove", move);
        ref.current!.addEventListener("pointerup", up);
      }}
    />
  );
}

function Toolbar() {
  const project = useApp((s) => s.project);
  const building = useApp((s) => s.building);
  const sidebar = useApp((s) => s.sidebarVisible);
  const engine = useApp((s) => s.lastBuild?.engine ?? s.mainDoc?.engine);
  const engineName = engine && { pdflatex: "pdfLaTeX", xelatex: "XeLaTeX", lualatex: "LuaLaTeX" }[engine];
  return (
    <header className="toolbar" data-tauri-drag-region>
      <div className="toolbar-left">
        <button className="tool-btn" title="Projects (⌘⇧P)" onClick={showProjects}>
          <ArrowLeft size={14} /> Projects
        </button>
        <button
          className="icon-btn"
          title="Toggle sidebar (⌘\)"
          onClick={() => useApp.setState((s) => ({ sidebarVisible: !s.sidebarVisible }))}
        >
          {sidebar ? <PanelLeftClose size={15} /> : <PanelLeft size={15} />}
        </button>
      </div>
      <div className="toolbar-title" data-tauri-drag-region>
        {project?.name}
      </div>
      <div className="toolbar-right">
        {engineName && (
          <span className="engine-chip" title="TeX engine for the main document">
            {engineName}
          </span>
        )}
        <button className="tool-btn" title="Sync PDF to cursor (⌘J)" onClick={() => void syncPdf()}>
          <ScanSearch size={14} /> Sync PDF
        </button>
        <button className="tool-btn primary" title="Build (⌘B)" disabled={building} onClick={() => void build()}>
          {building ? <Loader2 size={14} className="spin" /> : <Play size={14} />} {building ? "Building…" : "Build"}
        </button>
      </div>
    </header>
  );
}

export function Workspace() {
  const sidebarVisible = useApp((s) => s.sidebarVisible);
  const hasRight = useApp((s) => s.panes.right.tabs.length > 0);
  const dialog = useApp((s) => s.dialog);
  const [sidebarWidth, setSidebarWidth] = useState(260);
  const [split, setSplit] = useState(0.5);

  return (
    <div className="workspace">
      <Toolbar />
      <div className="workspace-body">
        <Ribbon />
        {sidebarVisible && (
          <>
            <div style={{ width: sidebarWidth }} className="sidebar-slot">
              <Sidebar />
            </div>
            <Resizer onDrag={(x) => setSidebarWidth(Math.min(520, Math.max(180, x - 44)))} />
          </>
        )}
        <div className="panes">
          <div className="pane-slot" style={{ flexBasis: hasRight ? `${split * 100}%` : "100%" }}>
            <Pane pane="left" />
          </div>
          {hasRight && (
            <>
              <Resizer onDrag={(x, w) => setSplit(Math.min(0.8, Math.max(0.2, x / w)))} />
              <div className="pane-slot" style={{ flexBasis: `${(1 - split) * 100}%` }}>
                <Pane pane="right" />
              </div>
            </>
          )}
        </div>
      </div>
      <StatusBar />
      {dialog === "latexdiff" && <LatexDiffDialog />}
      {dialog === "table" && <TableEditor />}
    </div>
  );
}
