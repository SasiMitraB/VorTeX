# VorTeX

An offline LaTeX editor for macOS, built with **Rust** and **[Tauri](https://tauri.app/)**.

Everything that understands LaTeX runs in Rust: the semantic index, builds and their diagnostics, SyncTeX, git, grammar checking (Harper), and equation previews (MuPDF). The interface is a small React app in the system webview, using [Monaco](https://microsoft.github.io/monaco-editor/) for editing and [pdf.js](https://mozilla.github.io/pdf.js/) for viewing PDFs. It's bundled with the app, so there is no Electron and nothing is downloaded at runtime.

### What VorTeX never does

- **No network access.** The app has no HTTP client, loads nothing from the internet, and CI fails if a network library or a remote URL sneaks in (see [Offline guarantee](#offline-guarantee)).
- **No telemetry.**
- **No AI.** Grammar checking is rule-based and runs locally. VorTeX checks your writing; it never writes for you.

---

## Table of Contents

- [Features](#features)
  - [Editor](#editor)
  - [Autocomplete](#autocomplete)
  - [Equation Hover Preview](#equation-hover-preview)
  - [Grammar Checking](#grammar-checking)
  - [Building & PDF Viewing](#building--pdf-viewing)
  - [SyncTeX (Forward & Inverse Search)](#synctex-forward--inverse-search)
  - [Source Control](#source-control)
  - [Compare Versions (latexdiff)](#compare-versions-latexdiff)
  - [Visual Table Editor](#visual-table-editor)
  - [Sidebar: Explorer, Outline, Labels & TODOs](#sidebar-explorer-outline-labels--todos)
  - [Workspace, Tabs & Split Panes](#workspace-tabs--split-panes)
  - [Project Selector](#project-selector)
  - [Semantic Index & File Watching](#semantic-index--file-watching)
  - [Themes & Icons](#themes--icons)
- [Keyboard Shortcuts](#keyboard-shortcuts)
- [Requirements](#requirements)
- [Getting Started](#getting-started)
- [Data & Configuration](#data--configuration)
- [Project Structure](#project-structure)
- [Testing](#testing)
- [Offline Guarantee](#offline-guarantee)
- [License](#license)

---

## Features

### Editor

- **Monaco editor** with line numbers, current-line highlight, soft wrap, a breadcrumb (`project › file › line`), find and replace (⌘F), multi-cursor editing, and folding of `\begin`…`\end` blocks.
- **LaTeX syntax highlighting** for commands, `\begin`/`\end` and environment names, sectioning commands, references and citations, math (`$…$`, `$$…$$`, `\[…\]`, `\(…\)`), braces/brackets, and `%` comments.
- **Smart editing**:
  - Auto-closing `$`, `{`, `(`, `[`, with type-over of existing closers.
  - Wrap a selection by typing `$`, `{`, `(`, `[`, or `"`.
  - Enter keeps indentation and continues `\item` lists (Enter on an empty `\item` ends the list).
  - Tab / Shift-Tab indent and unindent, duplicate line or selection (⌘D), toggle `%` comments (⌘/).
- **Unicode → LaTeX replacement**: typing symbols such as `→ ⇒ α β θ λ π ∑ ∫ ≤ ≥ ≠ ± ∞` inserts the matching command.
- **Undo/redo** that groups edits, and a dirty dot on tabs with unsaved changes (it clears again if you undo back to the saved text).
- **Git gutter markers** showing added, modified, and deleted lines relative to `HEAD` (see [Source Control](#source-control)).
- **Build errors in the editor**: errors and warnings from the last build are underlined on the lines they refer to, including in `\input` files.

### Autocomplete

- **Citations**: `\cite`, `\citep`, `\citet`, and other `\cite`-family commands (17 variants) fuzzy-match your BibTeX entries, showing *Author et al., Year* plus title/journal.
- **References**: `\ref`, `\eqref`, `\pageref`, `\autoref`, `\cref`, `\Cref` fuzzy-match labels across the project, showing `file:line`, with labels in the current file ranked higher.
- **Environments**: typing `\begin{` offers templates (`figure`, `table`, `equation`, `align`, `itemize`, `enumerate`, `matrix`, and more) that insert the matching `\end{…}`.
- **Command snippets**: typing `\` followed by letters suggests common commands.
- Navigate with ↑/↓, accept with Enter or Tab, dismiss with Esc.

### Equation Hover Preview

Hover over any math to see it rendered in a popup:

- Supports inline math (`$…$`, `\(…\)`), display math (`$$…$$`, `\[…\]`), and environments such as `equation`, `align`, `gather`, `multline`, `flalign`, `alignat`, `eqnarray`, and their starred forms. `$` inside verbatim/listing environments and comments is ignored.
- **Uses your document's macros**: `\newcommand`, `\DeclareMathOperator`, `\def`, and whitelisted math/font packages are pulled from the buffer, `main.tex`, and `\input`/`\include`d files. If that preamble fails, it falls back to a minimal `amsmath`/`amssymb` preamble.
- Compiled with `pdflatex` into a `standalone` document (no shell escape, 10 s timeout) and rasterized with MuPDF at high DPI on a transparent background, with text colored to match the current theme.
- Results are cached on disk and in memory. LaTeX errors are cached too, so broken equations are not recompiled on every hover.

### Grammar Checking

- **Local and private**: powered by [harper-core](https://github.com/Automattic/harper), running in-process with no network access.
- **Checks as you type**: runs in the background 750 ms after you stop typing.
- **Understands LaTeX**: before checking, the source is converted to plain prose with a byte-accurate source map, so issues underline the right text in your `.tex` file:
  - Math, verbatim, and listing environments, tabular column specs, and structural commands (`\label`, `\usepackage`, `\includegraphics`, …) are skipped.
  - Inline math is turned into readable Unicode text.
  - `\cite` becomes *Author (Year)* using your bibliography, and `\ref`-style commands become a placeholder word.
  - Text inside `\textbf`, `\emph`, `\footnote`, section titles, and captions is still checked.
- Issues get an amber underline; hover to read the explanation.
- The dialect is configurable (American, British, Canadian, Australian, Indian); the default is British. Source layout such as indentation and table alignment is not flagged, and neither is math.

### Building & PDF Viewing

- **One-key build** (⌘B) with `latexmk` (SyncTeX enabled). Modified files are saved first.
- **The engine is chosen automatically**: a `% !TEX program = …` comment wins; otherwise XeLaTeX is used when `fontspec`, `unicode-math`, or `polyglossia` is loaded, and pdfLaTeX in all other cases. The toolbar shows which one.
- **Which file is built**: a `%!TEX root = …` magic comment in the active file, else the active file if it contains `\documentclass`, else the project's `main.tex`, else the first root-level `.tex` file with `\documentclass`. ⌘B in a chapter file builds the whole document.
- **Incremental builds**: auxiliary files are kept, so latexmk only reruns what changed. *Clean and Build* (⌘⇧B) starts from scratch. Auxiliary files are hidden in the Explorer.
- **Diagnostics**: the TeX log is parsed into errors, warnings (undefined references and citations, …) and bad boxes, each with its file and line. The status bar shows the result, the real build time and error/warning counts.
- Builds run in the background. The PDF opens in the right pane, or reloads if it is already open (also when another tool rebuilds it).
- **PDF viewer** (pdf.js):
  - Continuous vertical scrolling; pages render as they come into view, sharp at any zoom.
  - Selectable text.
  - Zoom from 50% to 300%, fit to width (the default when a page doesn't fit), and click the percentage to reset to 100%.
  - Reload, and open in your system's default PDF viewer.

### SyncTeX (Forward & Inverse Search)

- **Forward search** (⌘J or the *Sync PDF* button): jumps from the cursor position to the matching spot in the PDF and highlights it.
- **Inverse search**: click anywhere in the PDF to open the source file at the matching line and column. Dragging to select text does not jump.
- Both require the `synctex` command-line tool and a `.synctex.gz` file next to the PDF (VorTeX builds produce one automatically).

### Source Control

A **Source Control** panel in the sidebar, backed by your system `git`:

- The current branch and a list of changed files, each with a status letter (M/A/D/R/U), or *Working tree clean*.
- **History** for the active file (last 50 commits, following renames).
- **Diff tabs**:
  - Click a changed file to diff `HEAD` against your working copy (including unsaved edits), or click a commit to diff that commit against it.
  - Unified view with old/new line numbers, word-level highlighting of changes, a +N/−N summary, and collapsed unchanged regions.
- **Gutter markers** in the editor: green for added lines, blue for modified lines, and a red notch where lines were deleted. They update shortly after each edit.
- Refreshes automatically on save, when you switch files, and when files change on disk. A badge on the sidebar ribbon shows how many files have changed.
- The panel is read-only: staging, committing, and pushing are done outside VorTeX.

### Compare Versions (latexdiff)

Generate a **change-tracked PDF** between any two versions of your document (Build → *Compare Versions (latexdiff)…*, or from the Source Control panel):

- Pick an **old version** (any commit) and a **new version** (the working copy or any commit) from the project's history. The default is latest commit → working copy.
- **The main document is found automatically**, in this order:
  1. A `%!TEX root = …` magic comment.
  2. The active file, if it contains `\documentclass`.
  3. `main.tex`.
  4. The first root-level `.tex` file containing `\documentclass`.
- **The engine is chosen automatically**: a `% !TEX program = …` comment wins. Otherwise XeLaTeX is used when `fontspec`, `unicode-math`, or `polyglossia` is loaded, and pdfLaTeX in all other cases.
- **Multi-file projects are handled**: each version is exported to a temporary directory and flattened with `latexdiff --flatten`.
- **Automatic retry**: if the first compile fails, VorTeX tries again with safer markup options (whole-math markup, no graphics or citation markup).
- **Output**: additions are underlined in blue and deletions struck out in red. The result is saved as `<name>-diff-<old>-<new>.pdf` next to your main file and opened in the preview pane. Only the PDF is kept; temporary files are discarded.
- **Errors**: if compilation fails, the relevant errors from the LaTeX log are shown.

### Visual Table Editor

Open with ⌘⌥T or Insert → *Table…*:

- **Type directly into the cells**, caption, and label; paste tab-separated rows from a spreadsheet into a cell to fill the grid.
- **Editing an existing table**: with the cursor inside a `table`/`tabular` environment, that table is parsed and loaded, and *Apply* replaces the original source. Otherwise *Insert* adds a new table at the cursor.
- **Supported LaTeX**: `l`/`c`/`r`/`p{…}` columns, `\hline`, booktabs rules, `\multicolumn`, `\multirow`, captions, and labels.
- **Controls**:
  - Add and remove rows and columns.
  - Cycle the alignment of each column.
  - Toggle booktabs styling.
  - Merge a selected range of cells (Shift-click the second corner), and unmerge them.
- **Live LaTeX code preview** of the generated `tabular`. It is wrapped in a `table` float when a caption or label is set.

### Sidebar: Explorer, Outline, Labels & TODOs

An icon ribbon switches between three panels (toggle the sidebar with ⌘\\):

- **Explorer**: a project file tree with collapsible folders and per-file-type icons. Hidden files, `node_modules`, `target`, and LaTeX's auxiliary files are skipped.
- **Outline & Tasks**:
  - **Project outline**: every `\part` … `\subparagraph` heading across all project files, indented by level, with the current file highlighted. Click to jump.
  - **Labels**: all `\label`s grouped by prefix (section, equation, table, figure, other), with filter chips and click-to-jump.
  - **TODO & Notes**: comments tagged `TODO`, `FIXME`, `NOTE`, `BUG`, `HACK`, `XXX`, `IDEA`, `OPTIMIZE`, `REVIEW`, plus `[ ]` / `[x]` checkboxes, gathered from across the project.
- **Source Control**: see [above](#source-control).

### Workspace, Tabs & Split Panes

- **Two resizable panes**, each with its own tab bar. The *Split* button opens the active tab in the right pane; source and PDF usually sit side by side. The sidebar is resizable too.
- **Tabs for different content**: `.tex`/`.bib`/text files, PDFs, and diff views, each with its own icon.
- **Toolbar**: back to projects, sidebar toggle, *Sync PDF*, and *Build*.
- **Status bar**:
  - Build status messages.
  - A SyncTeX location hint.
  - Index statistics (labels, bibliography entries, files indexed).
  - Cursor position and line count.
- **Native menu bar** (macOS) with File, Edit, Insert, View, Build, and Window menus.
- **Native file dialogs**: Open File, Open Folder, and Save As for untitled tabs.
- **Unsaved changes are never lost silently**: closing a tab, switching projects, closing the window, or quitting asks Save / Don't Save / Cancel.

### Project Selector

- Choose a **projects folder**; every subfolder appears as a project card.
- Cards show a **thumbnail of the compiled PDF's first page** (rendered with MuPDF and cached), or a *LaTeX Project* badge if there is no PDF yet.
- Opening a project automatically opens its main document, and VorTeX reopens the last project on launch.
- Switch projects any time with ⌘⇧P.

### Semantic Index & File Watching

- **Background indexing**: when you open a project, every `.tex` and `.bib` file is indexed for labels, citations, references, sections, TODOs, tables, and bibliography sources. Files referenced via `\bibliography` / `\addbibresource` are loaded automatically.
- **Incremental and persistent**: the index is content-hashed, updated on save, and cached to disk so the next launch starts quickly.
- **File watcher**: native OS file events (FSEvents on macOS), debounced to 300 ms, watch the project for `.tex`/`.bib` changes and keep the outline, labels, and TODOs up to date, including changes made by other tools.
- **BibTeX parsing** is handled by a built-in parser that decodes LaTeX accents.

### Themes & Icons

- **Light and dark themes** based on Catppuccin Latte and Mocha.
- **Auto mode** follows the macOS system appearance live. Cycle Auto → Light → Dark with ⌘⇧T; your choice is remembered.
- Syntax highlighting, diff colors, and rendered equation previews all adapt to the active theme.
- **Icons**: [Lucide](https://lucide.dev/) icons (`lucide-react`), bundled with the app.

---

## Keyboard Shortcuts

On macOS these use ⌘; on other platforms ⌘ maps to Ctrl.

### Application

| Shortcut | Action |
|---|---|
| ⌘N | New tab |
| ⌘O | Open file |
| ⌘⇧O | Open folder |
| ⌘S | Save |
| ⌘W | Close tab |
| ⌘⇧P | Show projects |
| ⌘B | Build |
| ⌘⇧B | Clean and build |
| ⌘J | Sync PDF to cursor (forward SyncTeX) |
| ⌘⌥T | Insert / edit table |
| ⌘\\ | Toggle sidebar |
| ⌘⇧T | Cycle theme (Auto / Light / Dark) |
| ⌘M | Minimize |
| ⌘H / ⌘⌥H | Hide / Hide others |
| ⌘Q | Quit |

### Editor

| Shortcut | Action |
|---|---|
| ⌘Z | Undo |
| ⌘⇧Z / ⌘Y | Redo |
| ⌘X / ⌘C / ⌘V | Cut / Copy / Paste |
| ⌘A | Select all |
| ⌘F | Find (and replace) |
| ⌘/ | Toggle comment |
| ⌘D | Duplicate line or selection |
| Tab / ⇧Tab | Indent / Unindent |
| ⌥← / ⌥→ | Move by word |
| ⌘← / ⌘→ | Line start / end |
| ⌥↑ / ⌥↓ | Move line up / down |
| ⌘↑ / ⌘↓ | Document start / end |

---

## Requirements

**To build VorTeX**

- Rust (stable) and Cargo. MuPDF and Harper are compiled in through their crates.
- Node.js 20 or later and npm, for the interface and the Tauri CLI.

**Runtime tools** (found automatically in standard MacTeX / TeX Live / Homebrew locations)

| Tool | Used for | Required? |
|---|---|---|
| `latexmk` + `pdflatex` (TeX Live / MacTeX) | Building PDFs, equation previews | Required for building |
| `xelatex` / `lualatex` | Documents that need a Unicode engine | If your document needs them |
| `synctex` | Forward & inverse search | Optional |
| `git` | Source Control panel, gutter markers, diffs | Optional |
| `latexdiff`, `tar` | Compare Versions | Optional |

VorTeX is currently developed and tested on **macOS 12 or later**.

---

## Getting Started

```bash
npm install       # the interface's dependencies and the Tauri CLI

npm run dev       # run with hot reload (starts Vite, then the app)
npm run build     # release bundle: VorTeX.app and a .dmg in target/release/bundle/
```

On first launch, choose a projects folder. Each subfolder is treated as a LaTeX project.

To build the release binary with Cargo directly, pass `--features tauri/custom-protocol` (as `npm run build` does); without it the binary looks for the dev server and shows an empty window.

---

## Data & Configuration

VorTeX stores its state in `~/.vortex-editor/`:

| Path | Contents |
|---|---|
| `config.json` | Projects folder, current project, theme preference, grammar dialect |
| `index.json` | Cached semantic index |
| `math_cache/` | Rendered equation previews |
| `thumbnails/` | Project card thumbnails |

You can safely delete the cache files and directories; they are regenerated on demand. (`pdf_cache/`, left by earlier versions, is no longer used.)

---

## Project Structure

```
├── Cargo.toml                   # Cargo workspace
├── package.json                 # Tauri CLI + npm workspace (ui/)
├── crates/vortex-core/          # Everything that understands LaTeX; no UI code
│   ├── src/
│   │   ├── backend.rs               # One open project: index, watcher, matcher
│   │   ├── project.rs               # Main-document and engine detection
│   │   ├── compiler.rs              # latexmk builds
│   │   ├── build_log.rs             # TeX log → file/line diagnostics
│   │   ├── completion.rs            # What to complete at the cursor, and the edits
│   │   ├── synctex.rs               # Forward/inverse search via the synctex CLI
│   │   ├── git.rs                   # Status, history, file contents, line diffs
│   │   ├── latexdiff.rs             # Change-tracked PDFs between versions
│   │   ├── math_preview.rs          # Equation detection, compilation & rasterization
│   │   ├── grammar_checker.rs       # Harper-based grammar linting
│   │   ├── grammar_preprocess.rs    # LaTeX → prose conversion with source mapping
│   │   ├── table_parser.rs          # LaTeX table parsing & generation
│   │   ├── table_editor.rs          # Table editing operations
│   │   ├── latex_parser.rs          # Labels, refs, cites, sections, TODOs
│   │   ├── bibtex_parser.rs         # BibTeX parser & accent decoder
│   │   ├── semantic_index.rs        # Thread-safe project index with disk caching
│   │   ├── fuzzy_matcher.rs         # Fuzzy search with active-file boost
│   │   ├── file_watcher.rs          # Debounced OS file-system watcher
│   │   ├── fs_utils.rs              # File tree, project scanning, thumbnails
│   │   ├── settings.rs              # config.json
│   │   └── text.rs                  # UTF-16 column conversions for the editor
│   └── tests/                       # Integration tests (+ real TeX logs in fixtures/)
├── src-tauri/                   # The desktop app
│   ├── src/commands/                # #[tauri::command]s over vortex-core
│   ├── src/events.rs                # Events sent to the UI
│   ├── src/menu.rs                  # Native menu bar
│   ├── capabilities/default.json    # What the webview may call
│   └── tauri.conf.json              # Window, CSP, bundle settings
├── ui/                          # React + TypeScript interface (Vite)
│   └── src/
│       ├── bindings.ts              # Generated typed API (do not edit)
│       ├── store.ts / actions.ts    # App state and behaviour
│       ├── editor/                  # Monaco setup, LaTeX language, completion, hovers
│       ├── pdf/                     # pdf.js viewer
│       └── components/              # Workspace, sidebar, panes, dialogs
└── scripts/check-offline.sh     # The offline guarantee checks
```

The TypeScript API in `ui/src/bindings.ts` is generated from the Rust command signatures by [tauri-specta](https://github.com/specta-rs/tauri-specta). After changing a command or an event, regenerate it with `npm run bindings`.

---

## Testing

```bash
cargo test --workspace   # Rust: vortex-core and the app layer
npm run typecheck        # TypeScript
```

The Rust suite covers:

- BibTeX and LaTeX parsing, the semantic index, and fuzzy matching.
- Builds (engine selection, incremental builds, diagnostics) and the log parser, against real TeX logs.
- Main-document detection, SyncTeX, and PDF project previews.
- Git integration and latexdiff.
- Completion, math preview detection and preamble building, and grammar checking.
- Table parsing, generation, and editing operations.
- That every command is registered, permitted, and exported to TypeScript.

Tests that need TeX tools skip themselves when the tools are not installed.

---

## Offline Guarantee

`npm run check:offline` (also run in CI) fails if:

- the app's Rust dependency tree contains an HTTP client (`reqwest`, `hyper`, `ureq`, `isahc`, `curl`, …);
- the interface references a remote URL (a CDN script, font, or image);
- the capability file grants the webview anything beyond events, window dragging, and file dialogs.

The window also runs under a strict Content Security Policy that only allows the app's own files and its IPC. The macOS App Sandbox would be a stronger guarantee, but it blocks running `latexmk`, `synctex`, `latexdiff`, and `git`, so VorTeX is not sandboxed (see `ROADMAP.md`).

---

## License

MIT. Icons are from [Lucide](https://lucide.dev/) under the ISC license. Monaco (MIT) and pdf.js (Apache-2.0) are bundled under their own licenses.
