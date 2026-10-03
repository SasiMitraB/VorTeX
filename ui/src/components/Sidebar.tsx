import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type DragEvent,
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
} from "react";
import {
  ChevronDown,
  ChevronRight,
  ChevronsDownUp,
  Columns2,
  Copy,
  ExternalLink,
  File,
  FileCode,
  FileImage,
  FilePlus,
  FileText,
  Files,
  Folder,
  FolderOpen,
  FolderPlus,
  GitBranch,
  GitCommitHorizontal,
  GitCompare,
  Heading,
  Image,
  ListTodo,
  ListTree,
  Pencil,
  RotateCw,
  Sigma,
  Table,
  Tag,
  Trash,
} from "lucide-react";
import type { LabelKind, TreeNode } from "../bindings";
import { activeTab, get, set, useApp, type SidebarTab } from "../store";
import {
  copyPath,
  createEntry,
  deleteEntry,
  moveEntry,
  openDiff,
  openPath,
  refreshGit,
  refreshHistory,
  refreshTree,
  renameEntry,
  report,
  revealInFileManager,
} from "../actions";
import { reveal } from "../editor/registry";
import { basename, dirname, extname } from "../lib/paths";
import { ContextMenu, type MenuItem } from "./ContextMenu";

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

function Section({
  title,
  count,
  children,
  actions,
  className = "",
}: {
  title: string;
  count?: number;
  children: ReactNode;
  actions?: ReactNode;
  className?: string;
}) {
  const [open, setOpen] = useState(true);
  return (
    <section className={`side-section ${className}`}>
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

type Editing = { mode: "file" | "folder"; parent: string } | { mode: "rename"; path: string } | null;

interface ExplorerUi {
  focused: string | null;
  menuTarget: string | null;
  editing: Editing;
  dropTarget: string | null;
  focus: (path: string) => void;
  stopEditing: () => void;
  openMenu: (e: MouseEvent, node: TreeNode | null) => void;
  dragOver: (e: DragEvent, folder: string) => void;
  drop: (e: DragEvent, folder: string) => void;
}

const ExplorerCtx = createContext<ExplorerUi>(null!);
const DRAG_TYPE = "application/x-vortex-path";
const isMac = navigator.userAgent.includes("Mac");

function findNode(nodes: TreeNode[], path: string): TreeNode | null {
  for (const n of nodes) {
    if (n.path === path) return n;
    if (n.children && path.startsWith(n.path + "/")) return findNode(n.children, path);
  }
  return null;
}

/** The rows currently on screen, top to bottom. */
function visibleNodes(nodes: TreeNode[], expanded: Record<string, boolean>, out: TreeNode[] = []): TreeNode[] {
  for (const n of nodes) {
    out.push(n);
    if (n.children && expanded[n.path]) visibleNodes(n.children, expanded, out);
  }
  return out;
}

const toggleFolder = (path: string) => set((s) => ({ expanded: { ...s.expanded, [path]: !s.expanded[path] } }));

/** Inline name box for new files/folders and renames. */
function NameInput({
  depth,
  icon,
  initial,
  submit,
}: {
  depth: number;
  icon: ReactNode;
  initial: string;
  submit: (name: string) => Promise<string | null>;
}) {
  const { stopEditing } = useContext(ExplorerCtx);
  const ref = useRef<HTMLInputElement>(null);
  const [error, setError] = useState<string | null>(null);
  const state = useRef<"idle" | "busy" | "done">("idle");

  useEffect(() => {
    const el = ref.current!;
    el.focus();
    const dot = initial.lastIndexOf(".");
    el.setSelectionRange(0, dot > 0 ? dot : initial.length);
  }, [initial]);

  const commit = async (onBlur: boolean) => {
    if (state.current !== "idle") return;
    const name = ref.current!.value.trim();
    if (!name || name === initial) {
      state.current = "done";
      return stopEditing();
    }
    state.current = "busy";
    const err = await submit(name);
    if (!err || onBlur) {
      if (err) report(err);
      state.current = "done";
      return stopEditing();
    }
    state.current = "idle";
    setError(err);
  };

  return (
    <div className="tree-row editing" style={{ paddingLeft: 8 + depth * 14 }}>
      <span className="chevron-space" />
      {icon}
      <div className="name-input">
        <input
          ref={ref}
          defaultValue={initial}
          spellCheck={false}
          className={error ? "invalid" : ""}
          onChange={() => setError(null)}
          onBlur={() => void commit(true)}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === "Enter") void commit(false);
            if (e.key === "Escape") {
              state.current = "done";
              stopEditing();
            }
          }}
        />
        {error && <div className="name-error">{error}</div>}
      </div>
    </div>
  );
}

function NewEntryInput({ parent, mode, depth }: { parent: string; mode: "file" | "folder"; depth: number }) {
  return (
    <NameInput
      depth={depth}
      initial=""
      icon={mode === "file" ? <File size={14} className="muted" /> : <Folder size={14} className="icon-folder" />}
      submit={(name) => createEntry(parent, name, mode)}
    />
  );
}

function TreeItem({ node, depth, activePath }: { node: TreeNode; depth: number; activePath: string | null }) {
  const ui = useContext(ExplorerCtx);
  const open = useApp((s) => !!s.expanded[node.path]);
  const isFolder = node.type === "folder";
  const pad = { paddingLeft: 8 + depth * 14 };

  if (ui.editing?.mode === "rename" && ui.editing.path === node.path)
    return (
      <NameInput
        depth={depth}
        initial={node.name}
        icon={isFolder ? <Folder size={14} className="icon-folder" /> : fileIcon(node.path)}
        submit={(name) => renameEntry(node.path, `${dirname(node.path)}/${name}`)}
      />
    );

  const dropFolder = isFolder ? node.path : dirname(node.path);
  const classes = [
    "tree-row",
    activePath === node.path && "selected",
    ui.focused === node.path && "focused",
    ui.menuTarget === node.path && "menu-target",
    isFolder && ui.dropTarget === node.path && "drop-target",
  ].filter(Boolean);

  const row = (
    <div
      className={classes.join(" ")}
      style={pad}
      data-path={node.path}
      title={node.name}
      draggable
      onClick={() => {
        ui.focus(node.path);
        if (isFolder) toggleFolder(node.path);
        else void openPath(node.path);
      }}
      onContextMenu={(e) => ui.openMenu(e, node)}
      onDragStart={(e) => {
        e.dataTransfer.setData(DRAG_TYPE, node.path);
        e.dataTransfer.effectAllowed = "move";
      }}
      onDragOver={(e) => ui.dragOver(e, dropFolder)}
      onDrop={(e) => ui.drop(e, dropFolder)}
    >
      {isFolder ? open ? <ChevronDown size={12} /> : <ChevronRight size={12} /> : <span className="chevron-space" />}
      {isFolder ? (
        open ? (
          <FolderOpen size={14} className="icon-folder" />
        ) : (
          <Folder size={14} className="icon-folder" />
        )
      ) : (
        fileIcon(node.path)
      )}
      <span className="ellipsis">{node.name}</span>
    </div>
  );
  if (!isFolder) return row;
  const creating = ui.editing && ui.editing.mode !== "rename" && ui.editing.parent === node.path ? ui.editing.mode : null;
  return (
    <>
      {row}
      {open && (
        <>
          {creating && <NewEntryInput parent={node.path} mode={creating} depth={depth + 1} />}
          {node.children?.map((c) => <TreeItem key={c.path} node={c} depth={depth + 1} activePath={activePath} />)}
        </>
      )}
    </>
  );
}

function Explorer() {
  const tree = useApp((s) => s.tree);
  const project = useApp((s) => s.project);
  const root = project?.path ?? "";
  const activePath = useActivePath();
  const treeRef = useRef<HTMLDivElement>(null);
  const [focused, setFocused] = useState<string | null>(null);
  const [editing, setEditing] = useState<Editing>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; node: TreeNode | null } | null>(null);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const closeMenu = useCallback(() => setMenu(null), []);

  // Forget a focused row that no longer exists (deleted, renamed, moved).
  useEffect(() => {
    if (focused && !findNode(tree, focused)) setFocused(null);
  }, [tree, focused]);

  /** The folder new entries go in: the focused folder, the focused file's folder, or the project root. */
  const targetFolder = () => {
    const n = focused ? findNode(tree, focused) : null;
    return n ? (n.type === "folder" ? n.path : dirname(n.path)) : root;
  };

  const startCreate = (mode: "file" | "folder", parent: string) => {
    if (parent !== root) set((s) => ({ expanded: { ...s.expanded, [parent]: true } }));
    set({ sidebarVisible: true, sidebarTab: "explorer" });
    setEditing({ mode, parent });
  };

  const ui: ExplorerUi = {
    focused,
    menuTarget: menu?.node?.path ?? null,
    editing,
    dropTarget,
    focus: setFocused,
    stopEditing: () => {
      setEditing(null);
      // Keep keyboard navigation going, unless focus already moved on (e.g. to a newly opened file).
      if (treeRef.current?.contains(document.activeElement)) treeRef.current.focus();
    },
    openMenu: (e, node) => {
      e.preventDefault();
      e.stopPropagation();
      if (node) setFocused(node.path);
      setMenu({ x: e.clientX, y: e.clientY, node });
    },
    dragOver: (e, folder) => {
      if (!e.dataTransfer.types.includes(DRAG_TYPE)) return;
      e.preventDefault();
      e.stopPropagation();
      e.dataTransfer.dropEffect = "move";
      if (dropTarget !== folder) setDropTarget(folder);
    },
    drop: (e, folder) => {
      const from = e.dataTransfer.getData(DRAG_TYPE);
      setDropTarget(null);
      if (!from) return;
      e.preventDefault();
      e.stopPropagation();
      void moveEntry(from, folder);
    },
  };

  const menuItems = (node: TreeNode | null): MenuItem[] => {
    const reveal = isMac ? "Reveal in Finder" : "Open Containing Folder";
    if (!node)
      return [
        { label: "New File…", icon: <FilePlus size={14} />, run: () => startCreate("file", root) },
        { label: "New Folder…", icon: <FolderPlus size={14} />, run: () => startCreate("folder", root) },
        "separator",
        { label: "Collapse All", icon: <ChevronsDownUp size={14} />, run: () => set({ expanded: {} }) },
        { label: "Refresh", icon: <RotateCw size={14} />, run: () => refreshTree() },
        "separator",
        { label: reveal, icon: <ExternalLink size={14} />, run: () => revealInFileManager(root, true) },
      ];
    const isFolder = node.type === "folder";
    const folder = isFolder ? node.path : dirname(node.path);
    const other = get().activePane === "left" ? "right" : "left";
    return [
      ...(isFolder
        ? []
        : ([
            { label: "Open", run: () => void openPath(node.path) },
            { label: "Open to the Side", icon: <Columns2 size={14} />, run: () => void openPath(node.path, other) },
            "separator",
          ] as MenuItem[])),
      { label: "New File…", icon: <FilePlus size={14} />, run: () => startCreate("file", folder) },
      { label: "New Folder…", icon: <FolderPlus size={14} />, run: () => startCreate("folder", folder) },
      "separator",
      { label: "Rename…", icon: <Pencil size={14} />, shortcut: "↵", run: () => setEditing({ mode: "rename", path: node.path }) },
      {
        label: "Move to Trash",
        icon: <Trash size={14} />,
        shortcut: isMac ? "⌘⌫" : "Del",
        danger: true,
        run: () => void deleteEntry(node.path, isFolder),
      },
      "separator",
      { label: "Copy Path", icon: <Copy size={14} />, run: () => void copyPath(node.path, false) },
      { label: "Copy Relative Path", run: () => void copyPath(node.path, true) },
      { label: reveal, icon: <ExternalLink size={14} />, run: () => revealInFileManager(node.path, isFolder) },
    ];
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if (editing) return;
    const rows = visibleNodes(tree, get().expanded);
    const i = rows.findIndex((n) => n.path === focused);
    const node = rows[i];
    const moveTo = (n: TreeNode | undefined) => {
      if (!n) return;
      setFocused(n.path);
      treeRef.current?.querySelector(`[data-path="${CSS.escape(n.path)}"]`)?.scrollIntoView({ block: "nearest" });
    };
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      return moveTo(i < 0 ? rows[0] : rows[e.key === "ArrowDown" ? i + 1 : i - 1]);
    }
    if (!node) return;
    const isFolder = node.type === "folder";
    const open = !!get().expanded[node.path];
    if (e.key === "ArrowRight" && isFolder) {
      e.preventDefault();
      return open ? moveTo(node.children?.[0]) : toggleFolder(node.path);
    }
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      return isFolder && open ? toggleFolder(node.path) : moveTo(findNode(tree, dirname(node.path)) ?? undefined);
    }
    if (e.key === "Enter" || e.key === "F2") {
      e.preventDefault();
      return setEditing({ mode: "rename", path: node.path });
    }
    if (e.key === " ") {
      e.preventDefault();
      return isFolder ? toggleFolder(node.path) : void openPath(node.path);
    }
    if (e.key === "Delete" || (e.key === "Backspace" && (e.metaKey || e.ctrlKey))) {
      e.preventDefault();
      return void deleteEntry(node.path, isFolder);
    }
  };

  const creatingAtRoot = editing && editing.mode !== "rename" && editing.parent === root ? editing.mode : null;

  return (
    <ExplorerCtx.Provider value={ui}>
      <Section
        title={(project?.name ?? "").toUpperCase()}
        className="explorer"
        actions={
          <>
            <button className="icon-btn" title="New File" onClick={() => startCreate("file", targetFolder())}>
              <FilePlus size={13} />
            </button>
            <button className="icon-btn" title="New Folder" onClick={() => startCreate("folder", targetFolder())}>
              <FolderPlus size={13} />
            </button>
            <button className="icon-btn" title="Refresh" onClick={() => refreshTree()}>
              <RotateCw size={13} />
            </button>
            <button className="icon-btn" title="Collapse All" onClick={() => set({ expanded: {} })}>
              <ChevronsDownUp size={13} />
            </button>
          </>
        }
      >
        <div
          ref={treeRef}
          className={`tree ${dropTarget === root ? "drop-target" : ""}`}
          tabIndex={0}
          onKeyDown={onKeyDown}
          onClick={(e) => e.target === e.currentTarget && setFocused(null)}
          onContextMenu={(e) => ui.openMenu(e, null)}
          onDragOver={(e) => ui.dragOver(e, root)}
          onDragLeave={(e) => !e.currentTarget.contains(e.relatedTarget as Node) && setDropTarget(null)}
          onDrop={(e) => ui.drop(e, root)}
          onDragEnd={() => setDropTarget(null)}
        >
          {creatingAtRoot && <NewEntryInput parent={root} mode={creatingAtRoot} depth={0} />}
          {tree.map((n) => (
            <TreeItem key={n.path} node={n} depth={0} activePath={activePath} />
          ))}
          {tree.length === 0 && !creatingAtRoot && <div className="side-empty">This folder is empty</div>}
        </div>
      </Section>
      {menu && <ContextMenu x={menu.x} y={menu.y} items={menuItems(menu.node)} onClose={closeMenu} />}
    </ExplorerCtx.Provider>
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
                <TagChips tags={c.tags} />
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

export function TagChips({ tags }: { tags: string[] }) {
  if (tags.length === 0) return null;
  return (
    <div className="tag-chips">
      {tags.map((t) => (
        <span key={t} className={`tag-chip ${t.startsWith("submitted/") ? "submitted" : ""}`} title={t}>
          <Tag size={10} /> {t.replace(/^submitted\//, "")}
        </span>
      ))}
    </div>
  );
}

function letter(kind: string) {
  return { modified: "M", added: "A", deleted: "D", renamed: "R", untracked: "U", conflicted: "!" }[kind] ?? "?";
}
