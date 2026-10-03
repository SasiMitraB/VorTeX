// Everything the app does in response to the user, the menu, and backend events.

import { ask, message, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { commands, events, type Settings, type ThemePreference } from "./bindings";
import { activeTab, get, set, type PaneId, type Tab } from "./store";
import * as models from "./editor/models";
import { activeEditor, editorOf, forgetTab, reveal } from "./editor/registry";
import { refreshAllGutters, showBuildDiagnostics } from "./editor/features";
import { basename, dirname, isImage, isPdf, relativeTo } from "./lib/paths";

let tabSeq = 0;
const newId = () => `tab-${++tabSeq}`;

export function report(e: unknown) {
  set({ status: String(e) });
}

function debounced(ms: number, fn: () => Promise<void> | void) {
  let t: number | undefined;
  return () => {
    window.clearTimeout(t);
    t = window.setTimeout(() => void fn(), ms);
  };
}

// ── Tabs ────────────────────────────────────────────────────────────────────

const allTabs = () => [...get().panes.left.tabs, ...get().panes.right.tabs];

function findTab(pred: (t: Tab) => boolean): { pane: PaneId; tab: Tab } | null {
  for (const pane of ["left", "right"] as const) {
    const tab = get().panes[pane].tabs.find(pred);
    if (tab) return { pane, tab };
  }
  return null;
}

function addTab(pane: PaneId, tab: Tab, focus = true) {
  set((s) => ({
    panes: { ...s.panes, [pane]: { tabs: [...s.panes[pane].tabs, tab], activeId: tab.id } },
    ...(focus ? { activePane: pane } : {}),
  }));
}

export function activateTab(pane: PaneId, id: string, focus = true) {
  set((s) => ({
    panes: { ...s.panes, [pane]: { ...s.panes[pane], activeId: id } },
    ...(focus ? { activePane: pane } : {}),
  }));
  if (focus && activeTab(get(), pane)?.kind === "text") queueMicrotask(() => editorOf(pane)?.focus());
  refreshForActiveFile();
}

type SaveChoice = "save" | "discard" | "cancel";

async function askToSave(names: string[]): Promise<SaveChoice> {
  const what = names.length === 1 ? `“${names[0]}”` : `${names.length} files`;
  const r = await message(`Do you want to save the changes you made to ${what}?`, {
    title: "Unsaved changes",
    kind: "warning",
    buttons: { yes: "Save", no: "Don't Save", cancel: "Cancel" },
  });
  if (r === "Yes" || r === "Save") return "save";
  if (r === "No" || r === "Don't Save") return "discard";
  return "cancel";
}

export async function closeTab(pane: PaneId, id: string) {
  const tab = get().panes[pane].tabs.find((t) => t.id === id);
  if (!tab) return;
  if (tab.kind === "text") {
    const shared = allTabs().some((t) => t.kind === "text" && t.key === tab.key && t.id !== id);
    if (!shared) {
      if (get().dirty[tab.key]) {
        const choice = await askToSave([tab.title]);
        if (choice === "cancel") return;
        if (choice === "save" && !(await saveTab(tab))) return;
      }
      models.disposeModel(tab.key);
    }
  }
  forgetTab(id);
  set((s) => {
    const tabs = s.panes[pane].tabs;
    const i = tabs.findIndex((t) => t.id === id);
    const rest = tabs.filter((t) => t.id !== id);
    const activeId =
      s.panes[pane].activeId === id ? (rest[Math.min(i, rest.length - 1)]?.id ?? null) : s.panes[pane].activeId;
    const otherPane: PaneId = pane === "left" ? "right" : "left";
    return {
      panes: { ...s.panes, [pane]: { tabs: rest, activeId } },
      activePane: rest.length === 0 && s.panes[otherPane].tabs.length ? otherPane : s.activePane,
    };
  });
  refreshForActiveFile();
}

export function closeActiveTab() {
  const s = get();
  const tab = activeTab(s);
  if (tab) void closeTab(s.activePane, tab.id);
}

/** Opens a file (or switches to its tab). PDFs go to the right pane by default. */
export async function openPath(path: string, pane?: PaneId, focus = true) {
  const existing = findTab((t) => (t.kind === "text" || t.kind === "pdf") && t.path === path);
  if (existing && (!pane || existing.pane === pane)) {
    activateTab(existing.pane, existing.tab.id, focus);
    return;
  }
  if (isPdf(path)) return openPdf(path, pane ?? "right", focus);
  try {
    if (!models.getModel(path)) models.createModel(path, await commands.readFile(path));
    addTab(pane ?? get().activePane, { id: newId(), kind: "text", key: path, path, title: basename(path) }, focus);
    refreshForActiveFile();
  } catch (e) {
    report(e);
  }
}

export async function openPdf(path: string, pane: PaneId = "right", focus = true) {
  const existing = findTab((t) => t.kind === "pdf" && t.path === path);
  if (existing) return activateTab(existing.pane, existing.tab.id, focus);
  const project = get().project?.path;
  if (!project || !path.startsWith(project + "/")) await commands.grantFileAccess(path).catch(report);
  addTab(pane, { id: newId(), kind: "pdf", path, title: basename(path) }, focus);
}

export function openDiff(path: string, rev: string | null, label: string) {
  const existing = findTab((t) => t.kind === "diff" && t.path === path && t.rev === rev);
  if (existing) return activateTab(existing.pane, existing.tab.id);
  const pane: PaneId = get().panes.right.tabs.length ? "right" : get().activePane;
  addTab(pane, { id: newId(), kind: "diff", path, rev, label, title: `${basename(path)} (${rev ? rev.slice(0, 7) : "HEAD"})` });
}

export function newTab() {
  const { key, title } = models.nextUntitledKey();
  models.createModel(key, "");
  addTab(get().activePane, { id: newId(), kind: "text", key, path: null, title });
}

/** The Split button: shows the active tab in the other pane too. */
export function splitActive() {
  const s = get();
  const tab = activeTab(s);
  if (!tab) return;
  const other: PaneId = s.activePane === "left" ? "right" : "left";
  const copy = { ...tab, id: newId() } as Tab;
  addTab(other, copy);
}

// ── Projects ────────────────────────────────────────────────────────────────

export async function updateSettings(patch: Partial<Settings>) {
  const current = get().settings;
  if (!current) return;
  const next = { ...current, ...patch };
  set({ settings: next });
  await commands.setSettings(next).catch(report);
}

export async function loadProjects() {
  if (!get().settings?.projectsFolder) return set({ projects: [] });
  set({ projectsLoading: true, projectsError: null });
  try {
    set({ projects: await commands.listProjects() });
  } catch (e) {
    set({ projectsError: String(e) });
  } finally {
    set({ projectsLoading: false });
  }
}

export async function chooseProjectsFolder() {
  const folder = await open({ directory: true, title: "Choose your projects folder" });
  if (typeof folder !== "string") return;
  await updateSettings({ projectsFolder: folder });
  set({ view: "projects" });
  await loadProjects();
}

/** Closes every tab, asking about unsaved changes first. False if the user cancelled. */
async function closeAll(): Promise<boolean> {
  if (!(await resolveUnsaved())) return false;
  for (const [key] of models.allModels()) models.disposeModel(key);
  allTabs().forEach((t) => forgetTab(t.id));
  set({ panes: { left: { tabs: [], activeId: null }, right: { tabs: [], activeId: null } }, activePane: "left" });
  return true;
}

export async function openProject(path: string) {
  if (get().project?.path !== path && !(await closeAll())) return;
  try {
    const info = await commands.openProject(path);
    set({ project: info, tree: info.tree, stats: info.stats, view: "workspace", status: null, lastBuild: null });
    set((s) => ({ settings: s.settings && { ...s.settings, currentProject: info.path } }));
    const main = await commands.mainDocument(null);
    set({ mainDoc: main });
    if (main && !allTabs().length) await openPath(main.path, "left");
    refreshIndex();
    refreshGit();
  } catch (e) {
    set({ view: "projects" });
    await loadProjects();
    set({ projectsError: `Could not open ${basename(path)}: ${String(e)}` });
  }
}

export function showProjects() {
  set({ view: "projects" });
  void loadProjects();
}

export async function openFolderDialog() {
  const folder = await open({ directory: true, title: "Open a LaTeX project folder" });
  if (typeof folder === "string") await openProject(folder);
}

export async function openFileDialog() {
  const file = await open({
    title: "Open File",
    defaultPath: get().project?.path,
    filters: [{ name: "LaTeX and PDF", extensions: ["tex", "bib", "sty", "cls", "txt", "pdf"] }],
  });
  if (typeof file === "string") await openPath(file);
}

// ── Index, git, tree ────────────────────────────────────────────────────────

export const refreshIndex = debounced(200, async () => {
  if (!get().project) return;
  try {
    const [outline, labels, todos, stats] = await Promise.all([
      commands.outline(),
      commands.labels(),
      commands.todos(null),
      commands.indexStats(),
    ]);
    set({ outline, labels, todos, stats });
  } catch (e) {
    report(e);
  }
});

export const refreshGit = debounced(250, async () => {
  if (!get().project) return;
  try {
    set({ git: await commands.gitStatus() });
    await refreshHistory();
  } catch (e) {
    report(e);
  }
});

export async function refreshHistory() {
  const tab = activeTab();
  const file = tab?.kind === "text" ? tab.path : null;
  if (get().sidebarTab !== "git" || !file) return set({ history: [], historyFile: file });
  if (!get().git) return set({ history: [], historyFile: file });
  set({ history: await commands.gitHistory(file, 50).catch(() => []), historyFile: file });
}

export const refreshTree = debounced(300, async () => {
  if (get().project) set({ tree: await commands.fileTree().catch(() => get().tree) });
});

// ── File operations (explorer) ──────────────────────────────────────────────

const within = (p: string, root: string) => p === root || p.startsWith(root + "/");

function checkName(name: string): string | null {
  if (!name) return "A name is required";
  if (name === "." || name === "..") return `“${name}” is not a valid name`;
  if (/[/\\]/.test(name)) return "Names can’t contain slashes";
  return null;
}

function expand(folder: string) {
  set((s) => ({ expanded: { ...s.expanded, [folder]: true } }));
}

/** Creates `name` in `parent` and opens it (files) or expands it (folders). Returns an error message on failure. */
export async function createEntry(parent: string, name: string, kind: "file" | "folder"): Promise<string | null> {
  const bad = checkName(name);
  if (bad) return bad;
  const path = `${parent}/${name}`;
  try {
    await (kind === "file" ? commands.createFile(path) : commands.createFolder(path));
  } catch (e) {
    return String(e);
  }
  if (parent !== get().project?.path) expand(parent);
  if (kind === "folder") expand(path);
  set({ tree: await commands.fileTree().catch(() => get().tree) });
  if (kind === "file" && !isImage(path)) {
    await openPath(path);
    reveal(path, 1);
  }
  refreshGit();
  return null;
}

/** Points open tabs (and expanded folders) at their new location after a rename or move. */
function retarget(from: string, to: string) {
  const move = (p: string) => (within(p, from) ? to + p.slice(from.length) : p);
  for (const t of allTabs()) if (t.kind === "text" && t.path && within(t.path, from)) models.rekey(t.key, move(t.path));
  const fix = (t: Tab): Tab => {
    if (t.kind === "diff" || !t.path || !within(t.path, from)) return t;
    const path = move(t.path);
    return t.kind === "text" ? { ...t, key: path, path, title: basename(path) } : { ...t, path, title: basename(path) };
  };
  set((s) => ({
    panes: {
      left: { ...s.panes.left, tabs: s.panes.left.tabs.map(fix) },
      right: { ...s.panes.right, tabs: s.panes.right.tabs.map(fix) },
    },
    expanded: Object.fromEntries(Object.entries(s.expanded).map(([k, v]) => [move(k), v])),
  }));
}

/** Renames or moves `from` to `to`. Returns an error message on failure. */
export async function renameEntry(from: string, to: string): Promise<string | null> {
  if (from === to) return null;
  if (within(to, from)) return "Can’t move a folder into itself";
  const bad = checkName(basename(to));
  if (bad) return bad;
  try {
    await commands.renamePath(from, to);
  } catch (e) {
    return String(e);
  }
  retarget(from, to);
  set({ tree: await commands.fileTree().catch(() => get().tree) });
  refreshForActiveFile();
  refreshGit();
  return null;
}

/** Drag and drop: moves `from` into `folder`. */
export async function moveEntry(from: string, folder: string) {
  const to = `${folder}/${basename(from)}`;
  if (to === from) return;
  const err = await renameEntry(from, to);
  if (err) report(err);
  else if (folder !== get().project?.path) expand(folder);
}

/** Closes tabs without asking to save (their files are gone). */
function dropTabs(pred: (t: Tab) => boolean) {
  const gone = allTabs().filter(pred);
  if (!gone.length) return;
  gone.forEach((t) => forgetTab(t.id));
  for (const t of gone) if (t.kind === "text" && !allTabs().some((o) => o.kind === "text" && o.key === t.key && !pred(o))) models.disposeModel(t.key);
  set((s) => {
    const prune = (p: (typeof s.panes)["left"]) => {
      const i = p.tabs.findIndex((t) => t.id === p.activeId);
      const tabs = p.tabs.filter((t) => !pred(t));
      const kept = tabs.some((t) => t.id === p.activeId);
      return { tabs, activeId: kept ? p.activeId : (tabs[Math.min(Math.max(i, 0), tabs.length - 1)]?.id ?? null) };
    };
    const panes = { left: prune(s.panes.left), right: prune(s.panes.right) };
    const other: PaneId = s.activePane === "left" ? "right" : "left";
    return { panes, activePane: panes[s.activePane].tabs.length || !panes[other].tabs.length ? s.activePane : other };
  });
  refreshForActiveFile();
}

/** Moves a file or folder to the Trash after confirming. */
export async function deleteEntry(path: string, isFolder: boolean) {
  const name = basename(path);
  const unsaved = models.dirtyKeys().filter((k) => within(k, path)).length;
  const warning = unsaved ? `\n\n${unsaved === 1 ? "An open file has" : `${unsaved} open files have`} unsaved changes that will be lost.` : "";
  const ok = await ask(`Move ${isFolder ? "the folder " : ""}“${name}”${isFolder ? " and its contents" : ""} to the Trash?${warning}`, {
    title: "Delete",
    kind: "warning",
    okLabel: "Move to Trash",
    cancelLabel: "Cancel",
  });
  if (!ok) return;
  try {
    await commands.deletePath(path);
  } catch (e) {
    return report(e);
  }
  dropTabs((t) => t.kind !== "diff" && !!t.path && within(t.path, path));
  set((s) => ({ status: `Moved ${relativeTo(s.project?.path, path)} to the Trash` }));
  set({ tree: await commands.fileTree().catch(() => get().tree) });
  refreshGit();
}

export async function copyPath(path: string, relative: boolean) {
  const text = relative ? relativeTo(get().project?.path, path) : path;
  try {
    await navigator.clipboard.writeText(text);
    set({ status: `Copied ${text}` });
  } catch (e) {
    report(e);
  }
}

/** Shows a folder (or a file's folder) in Finder / the file manager. */
export function revealInFileManager(path: string, isFolder: boolean) {
  void commands.openExternal(isFolder ? path : dirname(path)).catch(report);
}

const refreshMainDoc = debounced(150, async () => {
  const tab = activeTab();
  const active = tab?.kind === "text" ? tab.path : null;
  if (get().project) set({ mainDoc: await commands.mainDocument(active).catch(() => null) });
});

function refreshForActiveFile() {
  refreshMainDoc();
  void refreshHistory();
}

// ── Saving ──────────────────────────────────────────────────────────────────

/** Saves a text tab (Save As for untitled buffers). False if cancelled or failed. */
async function saveTab(tab: Extract<Tab, { kind: "text" }>): Promise<boolean> {
  const model = models.getModel(tab.key);
  if (!model) return false;
  let path = tab.path;
  if (!path) {
    const chosen = await saveDialog({
      title: "Save As",
      defaultPath: `${get().project?.path ?? ""}/${tab.title}.tex`,
      filters: [{ name: "LaTeX Document", extensions: ["tex"] }],
    });
    if (!chosen) return false;
    path = chosen;
  }
  try {
    await commands.writeFile(path, model.getValue());
  } catch (e) {
    report(e);
    return false;
  }
  if (!tab.path) {
    models.rekey(tab.key, path);
    const title = basename(path);
    set((s) => ({
      panes: {
        left: { ...s.panes.left, tabs: s.panes.left.tabs.map((t) => (t.kind === "text" && t.key === tab.key ? { ...t, key: path!, path, title } : t)) },
        right: { ...s.panes.right, tabs: s.panes.right.tabs.map((t) => (t.kind === "text" && t.key === tab.key ? { ...t, key: path!, path, title } : t)) },
      },
    }));
    refreshTree();
  }
  models.markSaved(path);
  set({ status: `Saved ${relativeTo(get().project?.path, path)}` });
  refreshGit();
  return true;
}

export async function saveActive() {
  const tab = activeTab();
  if (tab?.kind === "text") await saveTab(tab);
}

/** Saves every modified file that has a path (before building). */
export async function saveAllFiles() {
  for (const key of models.dirtyKeys()) {
    const tab = allTabs().find((t) => t.kind === "text" && t.key === key);
    if (tab?.kind === "text" && tab.path) await saveTab(tab);
  }
}

/** Asks about all unsaved buffers. False if the user cancelled. */
async function resolveUnsaved(): Promise<boolean> {
  const keys = models.dirtyKeys();
  if (!keys.length) return true;
  const tabs = keys
    .map((k) => allTabs().find((t) => t.kind === "text" && t.key === k))
    .filter((t): t is Extract<Tab, { kind: "text" }> => !!t);
  const choice = await askToSave(tabs.map((t) => t.title));
  if (choice === "cancel") return false;
  if (choice === "save") for (const t of tabs) if (!(await saveTab(t))) return false;
  return true;
}

export async function quit() {
  if (await resolveUnsaved()) await commands.quit();
}

// ── Build & SyncTeX ─────────────────────────────────────────────────────────

function bumpPdf(path: string) {
  set((s) => ({ pdfVersion: { ...s.pdfVersion, [path]: (s.pdfVersion[path] ?? 0) + 1 } }));
}

export async function build(clean = false) {
  if (get().building || !get().project) return;
  await saveAllFiles();
  const tab = activeTab();
  const active = tab?.kind === "text" ? tab.path : null;
  set({ building: true, status: "Building…" });
  try {
    const result = clean ? await commands.cleanBuild(active) : await commands.build(active);
    set({ lastBuild: result, status: result.message });
    showBuildDiagnostics(result.diagnostics);
    if (result.pdfPath) bumpPdf(result.pdfPath);
    // latexmk leaves an up-to-date PDF alone (pdfPath is then null); show it anyway.
    const pdf = result.pdfPath ?? (result.success ? (await commands.mainDocument(active).catch(() => null))?.pdfPath : null);
    if (pdf) await openPdf(pdf, "right", false);
    refreshTree();
    refreshMainDoc();
  } catch (e) {
    report(e);
  } finally {
    set({ building: false });
  }
}

export async function syncPdf() {
  const tab = activeTab();
  const editor = activeEditor();
  if (tab?.kind !== "text" || !tab.path || !editor) return set({ status: "SyncTeX: put the cursor in a .tex file" });
  const line = editor.getPosition()?.lineNumber ?? 1;
  try {
    const main = await commands.mainDocument(tab.path);
    if (!main) return set({ status: "SyncTeX: no main document found" });
    const rect = await commands.synctexForward(tab.path, line, main.pdfPath);
    if (!rect) return set({ status: `SyncTeX: no result for line ${line}. Build first.` });
    await openPdf(main.pdfPath, get().activePane === "right" ? "left" : "right", false);
    set({ pdfHighlight: { path: main.pdfPath, rect, nonce: Date.now() }, syncHint: `SyncTeX → page ${rect.page} (line ${line})` });
  } catch (e) {
    report(e);
  }
}

/** A click in the PDF: opens the source at that spot. */
export async function inverseSearch(pdf: string, page: number, x: number, y: number) {
  try {
    const loc = await commands.synctexInverse(pdf, page, x, y);
    if (!loc) return set({ syncHint: "SyncTeX ← no source here" });
    const pdfTab = findTab((t) => t.kind === "pdf" && t.path === pdf);
    const pane: PaneId = pdfTab?.pane === "left" ? "right" : "left";
    await openPath(loc.file, pane);
    reveal(loc.file, loc.line, loc.column + 1);
    set({ syncHint: `SyncTeX ← ${relativeTo(get().project?.path, loc.file)}:${loc.line}` });
  } catch (e) {
    report(e);
  }
}

// ── Theme, dialogs, menu ────────────────────────────────────────────────────

export function cycleTheme() {
  const order: ThemePreference[] = ["auto", "light", "dark"];
  const current = get().settings?.themePreference ?? "auto";
  void updateSettings({ themePreference: order[(order.indexOf(current) + 1) % order.length] });
}

function editorAction(id: string, fallback?: string) {
  const editor = activeEditor();
  if (editor) {
    editor.focus();
    editor.trigger("menu", id, null);
  } else if (fallback) {
    document.execCommand(fallback);
  }
}

const inWorkspace = () => get().view === "workspace";

export function handleMenu(cmd: import("./bindings").MenuCommand) {
  switch (cmd) {
    case "quit":
    case "closeWindow":
      return void quit();
    case "newTab":
      return inWorkspace() && newTab();
    case "openFile":
      return void openFileDialog();
    case "openFolder":
      return void openFolderDialog();
    case "save":
      return void saveActive();
    case "closeTab":
      return closeActiveTab();
    case "showProjects":
      return showProjects();
    case "changeProjectsFolder":
      return void chooseProjectsFolder();
    case "undo":
      return editorAction("undo", "undo");
    case "redo":
      return editorAction("redo", "redo");
    case "find":
      return editorAction("actions.find");
    case "toggleComment":
      return editorAction("editor.action.commentLine");
    case "insertTable":
      return inWorkspace() && set({ dialog: "table" });
    case "toggleSidebar":
      return set((s) => ({ sidebarVisible: !s.sidebarVisible }));
    case "cycleTheme":
      return cycleTheme();
    case "build":
      return void build();
    case "cleanBuild":
      return void build(true);
    case "syncPdf":
      return void syncPdf();
    case "compareVersions":
      return inWorkspace() && set({ dialog: "latexdiff" });
    case "prepareSubmission":
      return inWorkspace() && set({ dialog: "submission" });
  }
}

// ── Startup ─────────────────────────────────────────────────────────────────

export async function init() {
  const settings = await commands.getSettings();
  set({ settings });

  void events.menuCommand.listen((e) => handleMenu(e.payload));
  void events.indexUpdated.listen((e) => {
    set({ stats: e.payload });
    refreshIndex();
  });
  void events.gitChanged.listen(() => refreshGit());
  void events.fsChanged.listen(async ({ payload }) => {
    if (payload.kind === "pdf") return bumpPdf(payload.path);
    if (payload.kind === "git") return refreshAllGutters();
    refreshTree();
    // Keep unmodified open files in sync with edits made by other tools.
    if (payload.type !== "deleted" && models.getModel(payload.path)) {
      const text = await commands.readFile(payload.path).catch(() => null);
      if (text !== null) models.reloadIfClean(payload.path, text);
    }
  });
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", (e) => set({ systemDark: e.matches }));

  if (settings.currentProject) await openProject(settings.currentProject);
  else await loadProjects();
}
