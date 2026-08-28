# VorTeX

A high-performance, GPU-accelerated LaTeX editor built natively in **Rust** with **GPUI**.

---

## Highlights

- **100% Native Rust Architecture**: Zero Node.js or Electron dependencies. Single self-contained native binary.
- **GPU-Accelerated Rendering**: 120+ FPS rendering powered by GPUI.
- **In-Process Semantic Indexing**: Real-time background AST parsing, label indexing, citation tracking, and section hierarchy mapping.
- **Smart LaTeX Editing & Autocomplete**:
  - **Environments**: Type `\begin{` to get instant multi-line environment templates (`figure`, `table`, `equation`, `align`, `itemize`, `enumerate`, `matrix`, etc.).
  - **References & Citations**: Instant fuzzy suggestions for `\ref{...}`, `\eqref{...}`, `\cite{...}`, and `\citep{...}`.
  - **Commands & Math**: Fast auto-closing delimiters (`$`, `(`, `{`, `[`), Unicode math auto-replacements (`\rightarrow`, `\alpha`, `\sum`, `\int`), and smart itemization (`\item`).
- **Split Panes & Tab Pools**: Multi-pane editing with split views and PDF preview integration.
- **Native OS File Watcher**: Incremental kernel-level file monitoring (`FSEvents` on macOS).

---

## Getting Started

### Prerequisites

Ensure you have Rust and Cargo installed:
```bash
source "$HOME/.cargo/env"
```

### Running the App

```bash
cargo run
```

### Running Tests

```bash
cargo test
```

### Building for Release

```bash
cargo build --release
```

---

## Project Structure

```
├── Cargo.toml            # Rust crate manifest & dependencies
├── src_rust/
│   ├── main.rs           # Application entry point & GPUI window setup
│   ├── lib.rs            # Library root exposing backend, services, state, views
│   ├── backend.rs        # In-process BackendClient orchestrator
│   ├── state.rs          # App state, tabs, panes, and project data models
│   ├── theme.rs          # UI design system & color palettes
│   ├── services/         # Native service layer
│   │   ├── latex_parser.rs   # LaTeX AST & semantic element extraction
│   │   ├── bibtex_parser.rs  # BibTeX parser & accent decoder
│   │   ├── fuzzy_matcher.rs  # Skim-based fuzzy search with active-file boosting
│   │   ├── file_watcher.rs   # OS kernel file system event monitor
│   │   ├── semantic_index.rs # Thread-safe in-memory semantic index with disk caching
│   │   └── fs_utils.rs       # Recursive tree builder, project scanner, and config store
│   └── views/            # GPUI UI Components
│       ├── editor/           # LaTeX editor with syntax highlighting & autocomplete
│       ├── pdf_preview.rs    # PDF preview panel
│       ├── project_selector.rs # Recent projects and project browser
│       ├── sidebar.rs        # File tree & section outline
│       ├── status_bar.rs     # Index status & document metadata
│       ├── table_editor.rs   # Interactive LaTeX table editor modal
│       ├── tabs.rs           # Tab bar with drag & drop
│       └── workspace.rs      # Workspace toolbar and split pane layout
└── tests/                # Native Rust integration test suite
    ├── backend_client_test.rs
    ├── bibtex_parser_test.rs
    ├── editor_completion_test.rs
    ├── fuzzy_matcher_test.rs
    ├── latex_parser_test.rs
    └── semantic_index_test.rs
```

---

## License

MIT
