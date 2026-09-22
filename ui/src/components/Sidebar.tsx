import { useState, type ReactNode } from "react";
import {
  ChevronDown,
  ChevronRight,
  File,
  FileCode,
  FileImage,
  FileText,
  Files,
  Folder,
  FolderOpen,
  GitBranch,
  GitCommitHorizontal,
  GitCompare,
  Heading,
  Image,
  ListTodo,
  ListTree,
  RotateCw,
  Sigma,
  Table,
  Tag,
} from "lucide-react";
import type { LabelKind, TreeNode } from "../bindings";
import { activeTab, set, useApp, type SidebarTab } from "../store";
import { openDiff, openPath, refreshGit, refreshHistory } from "../actions";
import { reveal } from "../editor/registry";
import { basename, extname } from "../lib/paths";

const useActivePath = () =>
  useApp((s) => {
    const t = activeTab(s);
    return t && t.kind !== "diff" ? t.path : null;
  });

async function jump(file: string, line: number) {
  await openPath(file, "left");
  reveal(file, line);
}

export function Ribbon() {
  const tab = useApp((s) => s.sidebarTab);
  const visible = useApp((s) => s.sidebarVisible);
  const changes = useApp((s) => s.git?.changes.length ?? 0);
  const item = (id: SidebarTab, icon: ReactNode, title: string, badge?: number) => (
    <button
      className={`ribbon-btn ${visible && tab === id ? "active" : ""}`}
      title={title}
      onClick={() => {
        set((s) => ({ sidebarTab: id, sidebarVisible: !(s.sidebarVisible && s.sidebarTab === id) }));
        if (id === "git") queueMicrotask(() => void refreshHistory());
      }}
    >
      {icon}
      {!!badge && <span className="badge">{badge > 99 ? "99+" : badge}</span>}
    </button>
  );
  return (
    <nav className="ribbon">
      {item("explorer", <Files size={18} />, "Explorer")}
      {item("outline", <ListTree size={18} />, "Outline & Tasks")}
      {item("git", <GitBranch size={18} />, "Source Control", changes)}
    </nav>
  );
}

export function Sidebar() {
  const tab = useApp((s) => s.sidebarTab);
  return (
    <aside className="sidebar">
      {tab === "explorer" && <Explorer />}
      {tab === "outline" && <OutlinePanel />}
      {tab === "git" && <SourceControl />}
    </aside>
  );
}

function Section({ title, count, children, actions }: { title: string; count?: number; children: ReactNode; actions?: ReactNode }) {
  const [open, setOpen] = useState(true);
  return (
    <section className="side-section">
      <header onClick={() => setOpen(!open)}>
        {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        <span>{title}</span>
        {count !== undefined && <span className="count">{count}</span>}
        <span className="spacer" />
        <span onClick={(e) => e.stopPropagation()}>{actions}</span>
      </header>
      {open && <div className="side-body">{children}</div>}
    </section>
  );
}

// ── Explorer ────────────────────────────────────────────────────────────────

export function fileIcon(path: string, size = 14) {
  const ext = extname(path);
  if (ext === "tex" || ext === "sty" || ext === "cls") return <FileCode size={size} className="icon-tex" />;
  if (ext === "bib") return <FileText size={size} className="icon-bib" />;
  if (ext === "pdf") return <FileText size={size} className="icon-pdf" />;
  if (["png", "jpg", "jpeg", "svg", "gif", "eps"].includes(ext)) return <FileImage size={size} className="icon-img" />;
  return <File size={size} className="muted" />;
}

function TreeItem({ node, depth, activePath }: { node: TreeNode; depth: number; activePath: string | null }) {
  const open = useApp((s) => !!s.expanded[node.path]);
  const pad = { paddingLeft: 8 + depth * 14 };
  if (node.type === "folder")
    return (
      <>
        <div className="tree-row" style={pad} onClick={() => set((s) => ({ expanded: { ...s.expanded, [node.path]: !open } }))}>
          {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          {open ? <FolderOpen size={14} className="icon-folder" /> : <Folder size={14} className="icon-folder" />}
          <span className="ellipsis">{node.name}</span>
        </div>
        {open && node.children?.map((c) => <TreeItem key={c.path} node={c} depth={depth + 1} activePath={activePath} />)}
      </>
    );
  return (
    <div className={`tree-row ${activePath === node.path ? "selected" : ""}`} style={pad} onClick={() => void openPath(node.path)}>
      <span className="chevron-space" />
      {fileIcon(node.path)}
      <span className="ellipsis">{node.name}</span>
    </div>
  );
}

function Explorer() {
  const tree = useApp((s) => s.tree);
  const name = useApp((s) => s.project?.name ?? "");
  const activePath = useActivePath();
  return (
    <Section title={name.toUpperCase()}>
      {tree.map((n) => (
        <TreeItem key={n.path} node={n} depth={0} activePath={activePath} />
      ))}
    </Section>
  );
}

// ── Outline & tasks ─────────────────────────────────────────────────────────

const LABEL_KINDS: { kind: LabelKind; label: string; icon: ReactNode }[] = [
  { kind: "section", label: "Sec", icon: <Heading size={12} /> },
  { kind: "equation", label: "Eq", icon: <Sigma size={12} /> },
  { kind: "table", label: "Tab", icon: <Table size={12} /> },
  { kind: "figure", label: "Fig", icon: <Image size={12} /> },
  { kind: "other", label: "Other", icon: <Tag size={12} /> },
];

function OutlinePanel() {
  const outline = useApp((s) => s.outline);
  const labels = useApp((s) => s.labels);
  const todos = useApp((s) => s.todos);
  const filter = useApp((s) => s.labelFilter);
  const activePath = useActivePath();
  const minLevel = Math.min(...outline.map((s) => s.level), 99);
  const shown = labels.filter((l) => filter === "all" || l.kind === filter);

  return (
    <>
      <Section title="OUTLINE" count={outline.length}>
        {outline.length === 0 && <div className="side-empty">No sections</div>}
        {outline.map((s, i) => (
          <div
            key={i}
            className={`tree-row ${s.file === activePath ? "current-file" : ""}`}
            style={{ paddingLeft: 10 + (s.level - minLevel) * 12 }}
            onClick={() => void jump(s.file, s.line)}
            title={`${basename(s.file)}:${s.line}`}
          >
            <Heading size={12} className="muted" />
            <span className="ellipsis">{s.title}</span>
          </div>
        ))}
      </Section>
      <Section title="LABELS" count={labels.length}>
        <div className="chips">
          <button className={`chip ${filter === "all" ? "on" : ""}`} onClick={() => set({ labelFilter: "all" })}>
            All
          </button>
          {LABEL_KINDS.map((k) => (
            <button key={k.kind} className={`chip ${filter === k.kind ? "on" : ""}`} onClick={() => set({ labelFilter: k.kind })}>
              {k.icon}
              {k.label}
            </button>
          ))}
        </div>
        {shown.length === 0 && <div className="side-empty">No labels</div>}
        {shown.map((l) => (
          <div key={`${l.file}:${l.line}:${l.key}`} className="tree-row" onClick={() => void jump(l.file, l.line)}>
            <span className={`label-kind ${l.kind}`}>{LABEL_KINDS.find((k) => k.kind === l.kind)?.icon}</span>
            <span className="ellipsis mono">{l.key}</span>
            <span className="spacer" />
            <span className="muted small">
              {l.filename}:{l.line}
            </span>
          </div>
        ))}
      </Section>
      <Section title="TODO & NOTES" count={todos.length}>
        {todos.length === 0 && <div className="side-empty">Nothing to do</div>}
        {todos.map((t, i) => (
          <div key={i} className="tree-row todo" onClick={() => void jump(t.file, t.line)}>
            <ListTodo size={12} className={`todo-${t.tag.toLowerCase()}`} />
            <span className={`todo-tag todo-${t.tag.toLowerCase()}`}>{t.tag}</span>
            <span className="ellipsis">{t.text}</span>
            <span className="spacer" />
            <span className="muted small">
              {t.filename}:{t.line}
            </span>
          </div>
        ))}
      </Section>
    </>
  );
}

// ── Source control ──────────────────────────────────────────────────────────

function SourceControl() {
  const git = useApp((s) => s.git);
  const history = useApp((s) => s.history);
  const historyFile = useApp((s) => s.historyFile);
  const project = useApp((s) => s.project);

  if (!project) return null;
  if (!git)
    return (
      <Section title="SOURCE CONTROL">
        <div className="side-empty">This project is not in a git repository.</div>
      </Section>
    );

  return (
    <>
      <Section
        title="CHANGES"
        count={git.changes.length}
        actions={
          <>
            <button className="icon-btn" title="Compare versions (latexdiff)" onClick={() => set({ dialog: "latexdiff" })}>
              <GitCompare size={13} />
            </button>
            <button className="icon-btn" title="Refresh" onClick={() => refreshGit()}>
              <RotateCw size={13} />
            </button>
          </>
        }
      >
        <div className="branch">
          <GitBranch size={13} /> {git.branch ?? "detached"}
        </div>
        {git.changes.length === 0 && <div className="side-empty">Working tree clean</div>}
        {git.changes.map((c) => (
          <div key={c.path} className="tree-row" title={c.path} onClick={() => openDiff(`${git.root}/${c.path}`, null, "HEAD")}>
            {fileIcon(c.path)}
            <span className="ellipsis">{basename(c.path)}</span>
            <span className="muted small ellipsis">{c.path.includes("/") ? c.path.slice(0, c.path.lastIndexOf("/")) : ""}</span>
            <span className="spacer" />
            <span className={`change-letter ${c.kind}`}>{letter(c.kind)}</span>
          </div>
        ))}
      </Section>
      <Section title={historyFile ? `HISTORY · ${basename(historyFile)}` : "HISTORY"} count={history.length}>
        {!historyFile && <div className="side-empty">Open a file to see its history</div>}
        {historyFile && history.length === 0 && <div className="side-empty">No commits</div>}
        {historyFile &&
          history.map((c) => (
            <div
              key={c.hash}
              className="commit-row"
              title={`${c.shortHash} · ${c.author}`}
              onClick={() => openDiff(historyFile, c.hash, `${c.shortHash} · ${c.subject}`)}
            >
              <GitCommitHorizontal size={13} className="muted" />
              <div className="commit-text">
                <div className="ellipsis">{c.subject}</div>
                <div className="muted small">
                  <span className="mono">{c.shortHash}</span> · {c.author} · {c.relativeDate}
                </div>
              </div>
            </div>
          ))}
      </Section>
    </>
  );
}

function letter(kind: string) {
  return { modified: "M", added: "A", deleted: "D", renamed: "R", untracked: "U", conflicted: "!" }[kind] ?? "?";
}
