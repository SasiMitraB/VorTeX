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
- **Split panes with tab pools:** drag any tab between left/right panes; each pane keeps its own tabs. Close all tabs in a pane to hide it.
- **PDF viewing:** opening a `.pdf` renders it inline (per-pane iframe) while text files use Monaco.
- **Drag-and-drop tabs:** works across panes for both text and PDF tabs; drop zones stay inside the editor area.
- **Dark theme:** Atom One Dark-inspired Monaco theme.
- **Language support:** basic highlighting for LaTeX (`.tex`) and BibTeX (`.bib`) plus common web/dev languages.
- **Material UI:** toolbar/buttons styled with MDC Web.

## Project structure

- `index.html` — shell, toolbar, split layout, loads modules (type=module)
- `style.css` — layout, tabs, split panes, drop-zone and PDF iframe styling
- `renderer.js` — entry point wiring everything together
- `src/state.js` — shared state (monaco instance, panes, tabs)
- `src/monacoSetup.js` — Monaco init, theme, language registrations
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
