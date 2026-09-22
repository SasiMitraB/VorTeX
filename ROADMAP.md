# VorTeX Roadmap

Target: **v0.1 alpha on Tauri.** The GPUI frontend has been deleted (after M0, instead of at M7); git history before the migration is the reference for its behaviour.

Sizing: **S** ≈ an evening, **M** ≈ a few days, **L** ≈ a week or more.

---

## Order of work

The three lists below are **not** worked through in order. Some bugs live in code that the migration throws away, so fixing them in GPUI would be wasted effort.

| Phase | What | Where |
|---|---|---|
| **0. Prepare** | Fix backend bugs (Part 1a) and extract `vortex-core` (M0) | Current GPUI repo |
| **1. Migrate** | Tauri migration M1–M7 (Part 3) | New Tauri app |
| **2. Alpha** | Alpha features (Part 2a), then release | Tauri app |
| **3. After alpha** | IDE/workflow features (Part 2b) | Tauri app |

The rule: **backend fixes happen now** because they carry over unchanged. **UI bugs are not fixed in GPUI.** They become acceptance checks for the new interface (Part 1b).

---

## Decisions

| Area | Choice | Why |
|---|---|---|
| Shell | **Tauri v2** | Keeps all Rust services; small bundle; strict permission and CSP model |
| Frontend | **TypeScript + React + Vite** | The frontend stack AI assistants know best; large ecosystem |
| Editor | **Monaco** (`monaco-editor`, bundled; no CDN loader) | Chosen over CodeMirror 6 when the UI was built: find/replace, multi-cursor, folding, IME, suggest/hover/marker widgets, undo grouping and diffing come built in, and it matches the VS Code feel. Only the editor features are imported (`ui/src/editor/monaco.ts`), not its ~80 languages |
| LaTeX language | A Monarch tokenizer in `ui/src/editor/latex.ts` | Highlighting only; the Rust parser stays the source of truth for structure |
| PDF viewer | **pdf.js**, bundled locally | Vector zoom (no blur), text selection, in-PDF search, real page coordinates for SyncTeX |
| Rust ↔ TS API | **tauri-specta** (typed command bindings) | Keeps Rust and TS types from drifting apart as AI edits both sides |
| Theme | Catppuccin as CSS variables + `prefers-color-scheme` | Replaces the `defaults read` polling thread |
| Kept in Rust | Everything in `services/` | This is the product; the interface is replaceable |

### Offline guarantee (a hard requirement for every phase)

- [x] No network plugins: never add `tauri-plugin-http`, `tauri-plugin-updater`, or anything that pulls in `reqwest`/`hyper`.
- [x] CI check: fail the build if `cargo tree` for the app contains `reqwest`, `hyper`, `ureq`, `isahc` or `curl`. Run it right after scaffolding (M1) to see what Tauri itself brings in.
  - Result: Tauri brings in only `http` (types, no I/O) and `url` (a parser). `tauri-plugin-fs` comes in as a dependency of the dialog plugin; none of its permissions are granted. `scripts/check-offline.sh` also checks the capability file.
- [x] Every JS dependency, font, pdf.js worker and icon is bundled by Vite. No CDN URLs anywhere; add a lint for `https://` in `ui/src`. (Lint in `scripts/check-offline.sh`; re-check when pdf.js lands in M4.)
- [x] A strict CSP in `tauri.conf.json`: `default-src 'self'`, and `connect-src` limited to Tauri IPC. Check the exact directives pdf.js workers and the asset protocol need. (`connect-src` also allows the asset protocol so pdf.js can fetch the PDF; `worker-src 'self' blob:`. Verify both in M4.)
- [x] **Spike (S):** macOS App Sandbox without the `network.client` entitlement. This is the strongest guarantee, but the sandbox may block running `latexmk`/`synctex`/`latexdiff` (child processes inherit the sandbox and TeX Live sits outside it). If it can't work, rely on the three checks above and write down why.
  - **Result: it can't work.** An ad-hoc-signed probe `.app` with only `com.apple.security.app-sandbox` got `Operation not permitted` when spawning `latexmk`, `synctex` and `latexdiff` (from `/Library/TeX/texbin`). `/usr/bin/git` refused with "xcrun: cannot be used within an App Sandbox", and project folders outside the container were unreadable without user-granted bookmarks. VorTeX ships unsandboxed and relies on the three checks above.
- [x] Add a short "What VorTeX never does" section to the README: no network access, no telemetry, no AI. Grammar checking is rule-based and runs locally. VorTeX checks your writing; it never writes for you.

---

## Part 1: Bugs

### 1a. Fix now, in `services/` (these carry over to Tauri)

| # | Bug | Where | Fix | Size |
|---|---|---|---|---|
| B1 | The build always uses pdflatex (`latexmk -pdf`), so `fontspec`/`unicode-math` documents fail | `services/compiler.rs:73-118` | Reuse the engine detection from `latexdiff.rs:92-111` (`% !TEX program`, then package sniffing) and pass `-pdf`/`-xelatex`/`-lualatex` | S |
| B2 | The build ignores `%!TEX root`; ⌘B in a chapter file compiles that file on its own | `main.rs:741-770` | Move main-document detection from `latexdiff.rs:33` into a shared `project::main_document()` and use it for both | S |
| B3 | `latexmk -C` before every build turns off incremental builds (3–4 full passes each time) | `services/compiler.rs` | Remove the pre-clean and add a separate "Clean build" command | S |
| B4 | Build output is captured but thrown away; there are no error diagnostics | `services/compiler.rs` | Return a `BuildResult { success, diagnostics: Vec<{file, line, severity, message}>, raw_log }` by generalizing `latexdiff::log_errors`. Unit-test it against real `.log` files (undefined control sequence, missing file, overfull box warnings, citation/reference warnings) | M |
| B5 | The file watcher drops `.pdf` events, so the PDF auto-reload branch probably never runs | `services/file_watcher.rs`, `main.rs:1119-1123` | Confirm, then allow `.pdf` through (debounced) | S |
| B6 | Grammar dialect is fixed to British English | `services/grammar_checker.rs:52-60` | Make the dialect a parameter (US/UK/CA/AU) | S |
| B7 | `tokio` and `biblatex` are declared but never used | `Cargo.toml` | Remove them (Tauri brings its own tokio) | S |
| B8 | Stale "pdftoppm/pdfinfo" comments and error strings | `services/pdf_renderer.rs` | Rename them to MuPDF | S |

**Status: B1–B8 done** (now in `crates/vortex-core`).
- B1/B2: `project::main_document()` and `project::detect_engine()` are shared by building and latexdiff. `%!TEX root` is now matched case-insensitively (`% !TeX root`).
- B3: the post-build `latexmk -c` also discarded latexmk's state, so it was removed as well. `compiler::clean()` and the `clean_build` command replace both.
- B4: `build_log::parse()` tracks TeX's file stack, so errors and warnings in `\input` files get the right file. It resolves paths with spaces against the disk and skips echoed source lines. latexmk runs with `max_print_line=10000`, so log lines aren't wrapped. Fixtures in `tests/fixtures/logs/` are real pdfTeX logs. A rebuild of unchanged, failing sources (where latexmk doesn't rerun TeX) still reports the errors from the existing log.
- B5: the watcher reports `.pdf` changes, plus `.git/HEAD` and `.git/index` (for `git-changed`). Events carry a `kind` (`source`/`pdf`/`git`), and only sources are indexed.
- B6: `GrammarDialect` (American/British/Canadian/Australian/Indian). The default stays British, stored as `grammarDialect` in the config.
- B7: `tokio`, `biblatex`, `rfd` and `open` are gone from core.
- B8: those strings lived in `state.rs`/`main.rs`, which were deleted with GPUI.

### 1b. Don't fix in GPUI. Verify they're fixed in Tauri

These are all in `views/` or `main.rs`, which get replaced. Each one becomes an acceptance check in M7.

| # | Bug | How Tauri fixes it |
|---|---|---|
| U1 | No unsaved-changes prompt on close or quit; the tab dirty dot never shows | ✅ Dirty dot follows Monaco's version (undoing back clears it); Save / Don't Save / Cancel on closing a tab, switching projects, closing the window and ⌘Q (Rust intercepts the close and sends `closeWindow`) |
| U2 | The toolbar always says "LuaTeX" | ✅ The engine chip shows `main_document`'s engine, then the last build's |
| U3 | The status bar shows a hardcoded "0.14s" | ✅ Real duration, plus error and warning counts |
| U4 | You can't type into table editor cells, caption or label | ✅ Real inputs; checked by typing into a cell and applying |
| U5 | Inverse search works out click position from hardcoded layout sizes (296px/116px) | ✅ Click position relative to the page element ÷ zoom; drags and text selections don't trigger it |
| U6 | The PDF gets blurry above 100% zoom; all pages render up front | ✅ Re-rendered at zoom × devicePixelRatio, only for pages within 1200px of the view; selectable text layer |
| U7 | The cursor doesn't blink | ✅ Monaco |
| U8 | Undo copies the whole file on every keystroke and doesn't group edits | ✅ Monaco's undo stack |
| U9 | Tables in the outline may not be displayed (`_outline_tables` is unused) | Still not shown, as in GPUI; tables do appear under Labels (`tab:`). The `tables` command exists if a Tables section is wanted |
| U10 | `views/pdf_preview.rs` is dead code | Deleted along with GPUI ✅ |

---

## Part 2: Features

### 2a. Needed for the alpha (build these on Tauri, after M7)

| # | Feature | Notes | Size |
|---|---|---|---|
| F1 | **Build error panel** | Uses B4's diagnostics: a list with file:line that jumps on click, inline markers in the editor, and a raw-log tab | M |
| F2 | **Unsaved-change safety** | U1, plus autosave (configurable) and recovery of unsaved buffers after a crash | M |
| F3 | **Find and replace** | In-file search comes with Monaco (⌘F); project-wide search needs a Rust command (regex, respects `.gitignore`) | M |
| F4 | **File management in the explorer** | New file/folder, rename, delete (to Trash), reveal in Finder | M |
| F5 | **Go to definition** | `\ref` → `\label`, `\cite` → bib entry, `\input`/`\include` → file. The index already has the data | S |
| F6 | **Grammar controls** | Per-project user dictionary, "ignore this rule", dialect setting (B6) | M |
| F7 | **Settings** | Font and size, theme, dialect, autosave, build engine override. Stored in `~/.vortex-editor/config.json` | M |
| F8 | **Packaging** | Signed and notarized `.dmg` via `tauri build`. Include a "requirements check" screen that finds `latexmk`, `synctex`, `git` and `latexdiff` and explains what's missing | M |
| F9 | **Offline guarantee** | Everything in the checklist under Decisions | S–M |

### 2b. After the alpha: what makes VorTeX an IDE

These are the features that set VorTeX apart. They're local, need no AI, and no current editor does them well.

**Project checks** (a "Problems" panel, like PyCharm inspections):
- [ ] Labels that are defined but never referenced; `\ref`s pointing to missing labels; duplicate labels
- [ ] `\cite` keys missing from the `.bib` file; `.bib` entries never cited
- [ ] Figures and `\input` files that are missing on disk
- [ ] Consistency checks (e.g. `Fig.` vs `Figure`, `\ref` vs `\cref` mixed)

**Paper life cycle:**
- [ ] Mark a commit as "submitted version"; one click to latexdiff against it
- [ ] A helper for the response to reviewers (links reviewer comments to the changed regions)
- [ ] Submission prep: flatten, strip comments, check figures, bundle for arXiv or a journal
- [ ] Word and figure counts per section

**Writing workflow:**
- [ ] Git: stage, commit and view history (still no push or pull; that would need network access)
- [ ] BibTeX entry editor and duplicate finder
- [ ] User-defined snippets
- [ ] Code folding by section (environments already fold, via Monaco's `\begin`/`\end` markers)

**Platforms:**
- [ ] Linux (WebKitGTK) and Windows (WebView2) builds; Tauri supports both, so this is mostly TeX path discovery (`compiler.rs:13-35`) and CI

---

## Part 3: Tauri migration

### Target layout

```
VorTeX/
├── Cargo.toml              # workspace
├── crates/
│   └── vortex-core/        # all of services/ + logic moved out of views/ and main.rs; no GPUI or Tauri
├── src-tauri/              # Tauri app: commands, events, menu, window; depends on vortex-core
├── ui/                     # Vite + React + TS frontend
```

### M0: Extract `vortex-core` (in the current repo, before touching Tauri) — M ✅

The services already have no GPUI imports. Only `state.rs` and `theme.rs` use GPUI. Work to do:

- [x] Create the Cargo workspace and move `services/` into `crates/vortex-core`.
- [x] Move logic that lives in the interface layer into core (`completion.rs`, `table_editor.rs`, `latex_parser::label_kind`, `synctex::SynctexRect`; diff hunks were already in `git.rs`):
  - Build-target and main-document selection (`main.rs:741-770`, together with B2)
  - Completion context detection (which command am I in, what prefix?) from `views/editor/completion.rs`
  - The table model (`TableSpreadsheet` in `views/table_editor.rs`), next to `table_parser.rs`
  - Label classification (`sidebar.rs:23-32`)
  - Diff hunk building (from `views/diff_view.rs`), so the frontend only draws the result
- [x] Replace GPUI color types in core APIs with plain values (e.g. the math preview text color as `[u8; 3]`).
- [x] Move service tests into `vortex-core`. Editor, wrap, navigation and icon tests were deleted with GPUI; Monaco replaces that code. Completion and table-editor tests were rewritten against the core APIs.
- [x] **Gate:** `cargo test -p vortex-core` passes, `cargo tree -p vortex-core` has no `gpui`. (The "GPUI app still runs" part was dropped: GPUI was deleted instead.)

### M1: Scaffold — S ✅

- [x] `npm create tauri-app` (Tauri v2, React + TS + Vite) into `src-tauri/` and `ui/`. (Written by hand: Tauri 2.11, React 19, Vite 8, TypeScript 7. A root `package.json` holds the CLI, with `ui/` as an npm workspace.)
- [x] Depend on `vortex-core`; set up the tauri-specta bindings. (`vortex-core`'s `specta` feature derives the types. `ui/src/bindings.ts` is regenerated by `npm run bindings`, and CI fails if it's stale. Errors are thrown, not returned as results. Integers are exported as `number`.)
- [x] Set the CSP and a minimal capabilities file: only the dialog plugin and the specific commands. (`build.rs` lists every command, so Tauri generates per-command permissions. A test keeps `lib.rs`, `build.rs` and `capabilities/default.json` in sync.)
- [x] Add the CI job: `cargo test`, the network-crate check, the no-CDN lint, `npm run typecheck`.
- [x] Do the App Sandbox spike (see the offline guarantee: it can't work).
- [x] **Gate:** an empty window opens, one typed command round-trips, and CI is green. (Checked in the running app: settings, project list, open project, build with diagnostics, and watcher events. CI has not run yet because the workflow isn't pushed.)

### M2: Command and event API — M ✅

Design the whole API once, before any interface work. A thin `#[tauri::command]` layer over `vortex-core`. Long operations run async and never block the IPC thread.

- **Project:** `list_projects`, `open_project`, `file_tree`, `read_file`, `write_file`, `create/rename/delete_path`
- **Index:** `index_stats`, `outline`, `labels`, `todos`, `complete(kind, prefix, file)`
- **Build:** `build(file) -> BuildResult`, `clean_build`
- **SyncTeX:** `synctex_forward(file, line, col) -> {page, rect}`, `synctex_inverse(pdf, page, x, y) -> {file, line, col}`
- **Git:** `git_status`, `git_history(file?)`, `git_diff(path, rev?) -> hunks`, `git_line_markers(path, buffer)`
- **Tools:** `latexdiff(old, new) -> pdf_path`, `grammar_check(text, file) -> diagnostics`, `math_preview(snippet, file, color) -> png`
- **Events** (Rust → UI): `index-updated`, `fs-changed`, `build-started`/`build-finished`, `git-changed`

As built (`src-tauri/src/commands/`; the full signatures are in `ui/src/bindings.ts`):
- All columns crossing the API are **UTF-16 code units** (JavaScript string offsets), so the editor can use them directly. `complete(line, cursor, file)` returns the replacement range and, for each item, the text and cursor offset.
- Additions beyond the list above: `get_settings`/`set_settings` (typed `Settings`, same `config.json` keys as the GPUI app), `close_project`, `create_file`/`create_folder`, `open_external`, `tables`, `main_document(active)` (path, engine and PDF for the toolbar, U2), `latexdiff_options(active)` (dialog defaults), `math_at` + `math_render` (find the span, then render it), and `table_load`/`table_apply(model, op)`/`table_latex`.
- Create, rename and delete are restricted to the open project; delete moves to the Trash. latexdiff revisions must be commit hashes or `HEAD`. Math colours must be `RRGGBB`.
- `open_project` adds the project folder to the asset-protocol scope (for PDFs and images). `~/.vortex-editor/` (thumbnails, math PNGs) is in scope from the start.
- `ui/src/App.tsx` is a temporary API smoke screen; M5 replaces it.

### M3: Editor — L ✅ (Monaco)

- [x] Monaco with a LaTeX Monarch grammar and Catppuccin themes (`ui/src/editor/`). One model per file, shared by both panes; each tab keeps its scroll and cursor.
- [x] Keymap matching today's shortcuts. App shortcuts are native menu accelerators. ⌘D duplicates the line; ⌘⇧O and ⌘⇧P are unbound in Monaco so the app gets them. Differences from GPUI: ⌥↑/↓ move the line (Monaco) instead of jumping by paragraph.
- [x] Autocomplete through `complete` (Rust ranks; Monaco keeps that order). Verified: `\ref{sec:m` → `sec:method}`, consuming the auto-closed brace.
- [x] Auto-close `$ { ( [` and wrap selections (also `"`), `\item` continuation (Enter on an empty `\item` ends the list), Unicode → command replacement, toggle comment (`%`), duplicate line.
- [x] Grammar diagnostics as Monaco markers, debounced 750 ms. Fixing the false positives this exposed (in core): Harper's whitespace rules are off, `&`/`~` become spaces, `\ref` no longer eats the next space, and lints inside inline math are dropped.
- [x] Git gutter as line decorations fed by `git_line_markers` (200 ms), refreshed on `git-changed`.
- [x] Math hover through a Monaco hover provider: `math_at` finds the span, `math_render` makes the PNG, and it is shown as a data: URL (Monaco's hover markdown won't load custom-scheme URLs).

### M4: PDF viewer — M ✅

- [x] pdf.js with a bundled worker; continuous scroll, lazy page rendering, zoom 50–300% (click to reset, fit width, opens at fit-width when 100% doesn't fit), reload, open externally. Only pdf.js's `.textLayer` CSS is used: its full stylesheet also styles generic classes like `.sidebar`.
- [x] SyncTeX both ways (verified: ⌘J highlights the paragraph; clicking a heading opens `chapters/one.tex` at that line).
- [x] Reload on the `fs-changed` event for the PDF (B5), keeping the scroll position.
- [ ] Keep MuPDF in core only for project thumbnails and math rasterizing; remove the per-page PNG cache. (The UI no longer uses `pdf_renderer`; deleting it is left for a cleanup pass.)

### M5: App shell — L ✅

- [x] Resizable two-pane layout with per-pane tabs (text, PDF, diff) and Split.
- [x] Sidebar ribbon with Explorer, Outline & Tasks (outline, labels with filters, TODOs), Source Control (branch, changes, history, badge).
- [x] Status bar: build status and real duration, SyncTeX hint, index stats, cursor position.
- [x] Toolbar: projects, sidebar toggle, detected engine, Sync PDF, Build.
- [x] Project selector with PDF thumbnails and projects-folder picker.
- [x] Native menu bar (Tauri v2 menu API) with the same menus and shortcuts as today.
- [x] Themes: Catppuccin Latte/Mocha as CSS variables; Auto / Light / Dark saved to config.

### M6: Feature parity — M ✅

- [x] Diff view drawn from core hunks: word-level emphasis, collapsed unchanged runs, +/− summary.
- [x] latexdiff dialog: commit pickers, defaults, errors, output opens in the PDF pane.
- [x] Table editor as a React component on top of the core table model: typing works (U4), merge/unmerge, alignment, booktabs, live code preview, apply/insert.

### M7: Cutover — S

- [ ] Go through the parity checklist: every feature in `README.md` works in Tauri.
  - Checked in the running app (dev and release builds): project selector with thumbnails, reopening the last project, Explorer, Outline & Tasks, Source Control, diff tabs, build with diagnostics, PDF viewer, SyncTeX both ways, completion, math hover, grammar, git gutter, table editor, latexdiff, theme cycling, unsaved-changes prompts.
  - Not checked yet: New Tab + Save As, Open File/Folder dialogs, Clean and Build, the Split button, Unicode replacement, `\item` continuation.
  - Known differences: the outline is ordered by file, not by `\input` order (same as GPUI); build artifacts are hidden in the Explorer (GPUI deleted them after each build).
- [ ] Go through the Part 1b acceptance checks (U1–U10).
- [x] Delete `legacy-gpui/` and the `gpui` dependency (done right after M0).
- [x] Update the README (architecture, requirements, project structure) and this roadmap.
- [ ] Tag `v0.1.0-alpha.1` once Part 2a is done.

---

## Risks

| Risk | Mitigation |
|---|---|
| Tauri or a JS dependency quietly adds network code | The CI checks in the offline guarantee; review `Cargo.lock` and `package-lock.json` changes |
| The App Sandbox blocks TeX tools | The M1 spike; fall back to CI checks + CSP |
| Monaco has no LaTeX support | Our Monarch grammar only highlights; structure, completion, math detection and diagnostics all come from Rust |
| SyncTeX coordinate mapping is off by the page scale | Test with a fixture PDF at several zoom levels (extend `pdf_viewer_synctex_test.rs`) |
| The frontend grows as one large file (as `main.rs` did) | One component per panel from the start; state in a small store (e.g. Zustand), not in one root component |
