# VorTeX

A modern, minimal Electron editor that feels current—not 20 years old. Built around Monaco, with split panes, tab dragging, and PDF preview.

## Getting started

1) Install dependencies

```bash
npm install
```

2) Run the app

```bash
npm start
```

## Features (current)

- **Folder explorer:** choose a folder on startup; collapsible tree on the left (toggle/hide) with folders/files; click a file to open it.
- **Enhanced Explorer:**
    - Context menu (Right-click) for file operations (New File, Rename, Delete).
    - File icons based on extension.
    - Outline view for LaTeX files (shows sections/subsections).
- **Split panes with tab pools:** drag any tab between left/right panes; each pane keeps its own tabs. Close all tabs in a pane to hide it.
- **PDF viewing:** opening a `.pdf` renders it inline (per-pane iframe) while text files use Monaco.
- **Drag-and-drop tabs:** works across panes for both text and PDF tabs; drop zones stay inside the editor area.
- **Dark theme:** Atom One Dark-inspired Monaco theme.
- **Language support:** basic highlighting for LaTeX (`.tex`) and BibTeX (`.bib`) plus common web/dev languages.
- **Smart LaTeX Editing:**
    - **Auto-completion:** References (`\ref`), citations (`\cite`), environments (`\begin`), and commands.
    - **Smart Labels:** Auto-suggests labels based on context (e.g., `eq:` for equations, `fig:` for figures).
    - **Auto-itemization:** Pressing Enter in a list environment (`itemize`, `enumerate`) automatically inserts `\item`.
    - **Math Support:** Auto-closing `$` delimiters and Unicode symbol picker (trigger with `\` in math mode).
    - **File Paths:** Auto-complete paths in `\input`, `\include`, `\includegraphics`.
    - **Smart Braces:** Auto-closing braces for commands like `\section`, `\ref`.
- **Material UI:** toolbar/buttons styled with MDC Web.

## Project structure

- `index.html` — shell, toolbar, split layout, loads modules (type=module)
- `style.css` — layout, tabs, split panes, drop-zone and PDF iframe styling
- `renderer.js` — entry point wiring everything together
- `src/state.js` — shared state (monaco instance, panes, tabs)
- `src/monacoSetup.js` — Monaco init, orchestrates editor modules
- `src/editor/` — Modular editor logic:
    - `theme.js` — Editor theme definition
    - `languages.js` — Language configuration
    - `completionProvider.js` — Auto-completion logic
    - `snippetManager.js` — Custom keybindings and snippets
    - `completions/` — Specific completion providers (refs, envs, commands, files)
- `src/services/` — Backend services:
    - `BibtexParser.js` — Parses .bib files
    - `LatexParser.js` — Parses .tex files for structure and labels
    - `SemanticIndex.js` — Indexes project content for auto-completion
    - `FileWatcher.js` — Watches for file changes
- `src/tabs.js` — tab creation/rendering, switching, close/open/save logic
- `src/paneContent.js` — per-pane content swapping between Monaco and PDF iframe
- `src/dragDrop.js` — drag/drop zones, tab movement between panes
- `src/dom.js` — DOM helpers (drop zones, PDF pointer toggles)
- `src/utils.js` — language detection
- `src/explorer.js` — folder picker and collapsible tree rendering

## Explorer usage

- On launch you'll be prompted to pick a folder (or use the “Open Folder” button in the toolbar).
- The Explorer pane can be toggled with the “Explorer” button or the × in its header.
- Click folders to expand/collapse; click files to open them in the active pane.

## Usage notes

- Start empty: no tab opens by default; open/create files as needed.
- PDFs are read-only in this view; Save/Save As is skipped for them.
- Dragging a tab over a pane shows an overlay within the editor area; drop to move/split.
