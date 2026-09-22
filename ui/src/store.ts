// App state. Components read it with `useApp(selector)`; `actions.ts` changes it.
// Editor text lives in Monaco models (editor/models.ts), not here.

import { create } from "zustand";
import type {
  BuildResult,
  Commit,
  GitStatus,
  IndexStats,
  LabelEntry,
  LabelKind,
  MainDocument,
  ProjectInfo,
  ProjectItem,
  SectionItem,
  Settings,
  SynctexRect,
  TodoItem,
  TreeNode,
} from "./bindings";

export type PaneId = "left" | "right";

export type Tab =
  | { id: string; kind: "text"; key: string; path: string | null; title: string }
  | { id: string; kind: "pdf"; path: string; title: string }
  | { id: string; kind: "diff"; path: string; rev: string | null; label: string; title: string };

export type Pane = { tabs: Tab[]; activeId: string | null };

export type SidebarTab = "explorer" | "outline" | "git";

export type Dialog = null | "latexdiff" | "table";

export interface AppState {
  view: "projects" | "workspace";
  settings: Settings | null;
  projects: ProjectItem[];
  projectsLoading: boolean;
  /** Why listing or opening a project failed (shown on the project selector). */
  projectsError: string | null;
  project: ProjectInfo | null;
  tree: TreeNode[];
  expanded: Record<string, boolean>;
  stats: IndexStats | null;
  outline: SectionItem[];
  labels: LabelEntry[];
  todos: TodoItem[];
  labelFilter: LabelKind | "all";

  panes: Record<PaneId, Pane>;
  activePane: PaneId;
  /** Unsaved models, by model key. */
  dirty: Record<string, boolean>;

  sidebarVisible: boolean;
  sidebarTab: SidebarTab;

  building: boolean;
  lastBuild: BuildResult | null;
  mainDoc: MainDocument | null;
  status: string | null;
  syncHint: string | null;
  cursor: { line: number; col: number; lines: number } | null;

  git: GitStatus | null;
  history: Commit[];
  historyFile: string | null;

  /** Bumped when a PDF changes on disk so viewers reload it. */
  pdfVersion: Record<string, number>;
  pdfHighlight: { path: string; rect: SynctexRect; nonce: number } | null;

  systemDark: boolean;
  dialog: Dialog;
}

export const useApp = create<AppState>()(() => ({
  view: "projects",
  settings: null,
  projects: [],
  projectsLoading: false,
  projectsError: null,
  project: null,
  tree: [],
  expanded: {},
  stats: null,
  outline: [],
  labels: [],
  todos: [],
  labelFilter: "all",
  panes: { left: { tabs: [], activeId: null }, right: { tabs: [], activeId: null } },
  activePane: "left",
  dirty: {},
  sidebarVisible: true,
  sidebarTab: "explorer",
  building: false,
  lastBuild: null,
  mainDoc: null,
  status: null,
  syncHint: null,
  cursor: null,
  git: null,
  history: [],
  historyFile: null,
  pdfVersion: {},
  pdfHighlight: null,
  systemDark: window.matchMedia("(prefers-color-scheme: dark)").matches,
  dialog: null,
}));

export const set = useApp.setState;
export const get = useApp.getState;

export function activeTab(state: AppState = get(), pane: PaneId = state.activePane): Tab | null {
  const p = state.panes[pane];
  return p.tabs.find((t) => t.id === p.activeId) ?? null;
}

/** The theme actually shown. */
export function effectiveTheme(state: Pick<AppState, "settings" | "systemDark">): "light" | "dark" {
  const pref = state.settings?.themePreference ?? "auto";
  return pref === "auto" ? (state.systemDark ? "dark" : "light") : pref;
}
