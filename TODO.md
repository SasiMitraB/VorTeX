# VorTeX TODO: features other editors have

Features that **TeXstudio**, **LaTeX Workshop** (VS Code) or **Texifier** have and VorTeX doesn't yet.
Ordered by what's needed first: writing a whole paper in VorTeX (the alpha), then open-sourcing it.

Who has it: **TS** = TeXstudio, **LW** = LaTeX Workshop, **Tx** = Texifier.

---

## P0: Needed to write a whole paper in VorTeX (alpha)

### Navigation and search

- [ ] **Project-wide find and replace**: regex, across all `.tex` and `.bib` files, with results you can click. *(TS, LW)*
- [ ] **Go to definition**: `\ref` → `\label`, `\cite` → `.bib` entry, `\input`/`\include` → file, a macro → its `\newcommand`. The index already has the data. *(LW)*
- [ ] **Find all references** to a label or cite key, and **rename a label** everywhere it's used. *(LW)*
- [ ] **Hover on `\ref`** shows what it points to (section title, equation preview, figure caption); **hover on `\cite`** shows the bib entry. Hover currently only previews math. *(LW, TS)*
- [ ] **Hover on `\includegraphics`** shows the image. *(LW, TS)*

### Completion

- [ ] **File path completion** in `\input`, `\include`, `\includegraphics` and `\bibliography`. *(LW, TS)*
- [ ] **Commands and environments from loaded packages**, plus macros defined in the project with `\newcommand`/`\DeclareMathOperator`. Completion currently knows a fixed list. *(LW, TS)*

### Editing

- [ ] **Rename the `\begin{…}`/`\end{…}` pair together**, select the current environment, close the open environment, wrap a selection in an environment. *(LW, TS)*
- [ ] **Formatting shortcuts** for `\textbf`, `\emph`, `\textit` and so on. Note that ⌘B is already Build. *(LW, TS)*

### Checks without compiling

- [ ] **Undefined and duplicate labels, unknown cite keys** marked in the editor as you type. *(TS)*
- [ ] **Syntax problems**: unbalanced braces, mismatched `\begin`/`\end`, unknown commands. *(TS; LW via ChkTeX)*
- [ ] **Grammar controls**: per-project dictionary, "ignore this rule", languages other than English. *(TS via Hunspell and LanguageTool; LW via LTeX)*

### PDF viewer

- [ ] **Search inside the PDF**. *(LW, TS, Tx)*
- [ ] **Dark mode for PDF pages** (invert or recolour). *(LW)*
- [ ] **Two-page spread** and presentation mode. *(TS, LW)*

### Not losing work

- [ ] **Autosave and crash recovery** of unsaved buffers. *(TS, LW, Tx)*
- [ ] **Restore open tabs and split layout** per project. *(TS, LW, Tx)*
- [ ] **Build on save** (optional). *(LW; Tx typesets live)*
- [ ] **Settings screen**: font and size, word wrap, grammar dialect, engine override, build on save. Today these live in `~/.vortex-editor/config.json` or aren't settable. *(all three)*

---

## P1: Before open-sourcing

### Getting started

- [ ] **Requirements check on first launch**: find TeX Live/MacTeX, `latexmk`, `synctex`, `git` and `latexdiff`, and explain how to install whatever is missing. *(Tx bundles its own TeX)*
- [ ] **Signed and notarized `.dmg`**.
- [ ] **Linux and Windows builds**. Tauri supports both; most of the work is TeX path discovery and CI. *(TS, LW)*
- [ ] **New project from a template**: article, beamer, and journal classes (MNRAS, AASTeX, A&A, …). *(TS, Tx)*

### Building

- [ ] **Build problems panel**: a clickable list of errors and warnings, plus a raw log tab. Today they're only underlined in the editor and counted in the status bar. *(LW, TS)*
- [ ] **Configurable builds**: biber, makeglossaries, makeindex, `-shell-escape`, output directory, extra `latexmk` options. *(LW recipes, TS)*

### Writing workflow

- [ ] **Bibliography browser**: search `.bib` entries and insert citations from a picker, edit entries, find duplicates. Support Zotero (Better BibTeX auto-export) and BibDesk files. *(Tx, LW, TS)*
- [ ] **Drag an image in to insert a figure** (`figure` environment with `\includegraphics`, caption and label). *(TS, Tx)*
- [ ] **Word count per section** (in progress: `ui/src/editor/wordcount.ts`). *(LW via texcount, TS, Tx)*
- [ ] **Git: stage and commit** from the Source Control panel. Push and pull are out, since they need the network. *(LW via VS Code)*
- [ ] **Formatting**: `latexindent` for `.tex`; sort and align `.bib` entries. *(LW)*
- [ ] **Customizable keyboard shortcuts**. *(LW, TS)*

---

## P2: Nice to have

- [ ] **Math symbol palette**: clickable symbols, with recently used ones first. *(TS, LW snippet view)*
- [ ] **Live math preview panel** that follows the cursor, as well as the hover. *(LW, TS)*
- [ ] **Instant preview while typing**, without a full build. This is Texifier's headline feature and hard to do; TeXstudio's "preview selection" is a cheaper middle ground. *(Tx, TS)*
- [ ] **Table source helpers**: align the `&` columns in source, add or remove a column. *(TS)*
- [ ] **User snippets**, and `@`-shortcuts for Greek letters (`@a` → `\alpha`). *(LW)*
- [ ] **Folding by section**. *(LW, TS)*
- [ ] **Section operations from the outline**: move, indent or outdent, delete. *(TS)*
- [ ] **Bookmarks** in the source. *(TS)*
- [ ] **Package documentation** (`texdoc`) from the editor. *(LW, TS)*
- [ ] **Text analysis**: word frequency, repeated words. *(TS)*
- [ ] **Focus mode** and writing goals. *(Tx)*
- [ ] **Export to HTML/ODT** via TeX4ht. *(TS)*
- [ ] **User scripts and macros**. *(TS)*

---

## Not planned: conflicts with VorTeX's principles

- **Real-time collaboration and cloud sync** (Overleaf, Texifier's iCloud): needs network access.
- **Downloading packages on demand** (Texifier, Tectonic): needs network access. A fully offline bundled TeX might be possible later.
- **AI writing help**: VorTeX checks your writing; it never writes for you.
- **iPad/iOS app** (Texifier).
