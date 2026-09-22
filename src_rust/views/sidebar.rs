use crate::backend::{LabelItem, SectionItem, TodoItem};
use crate::icons::{file_icon, icon, IconName};
use crate::services::git::{ChangeKind, Commit};
use crate::state::{GitPanelState, LabelTypeFilter, SidebarTab};
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;
use std::collections::HashSet;

// ─────────────────────────────────────────────────────────────────────────────
// Label classification helpers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelKind {
    Section,
    Equation,
    Table,
    Figure,
    Other,
}

fn classify_label(key: &str) -> LabelKind {
    let prefix = key.split(':').next().unwrap_or("").to_lowercase();
    match prefix.as_str() {
        "sec" | "sub" | "subsec" | "chap" | "ch" | "part" | "app" => LabelKind::Section,
        "eq" | "eqn" => LabelKind::Equation,
        "tab" | "tbl" | "table" => LabelKind::Table,
        "fig" | "figure" | "subfig" => LabelKind::Figure,
        _ => LabelKind::Other,
    }
}

fn label_kind_icon(kind: LabelKind) -> IconName {
    match kind {
        LabelKind::Section => IconName::Heading,
        LabelKind::Equation => IconName::Sigma,
        LabelKind::Table => IconName::Table,
        LabelKind::Figure => IconName::Image,
        LabelKind::Other => IconName::Tag,
    }
}

fn label_kind_matches_filter(kind: LabelKind, filter: LabelTypeFilter) -> bool {
    match filter {
        LabelTypeFilter::All => true,
        LabelTypeFilter::Section => kind == LabelKind::Section,
        LabelTypeFilter::Equation => kind == LabelKind::Equation,
        LabelTypeFilter::Table => kind == LabelKind::Table,
        LabelTypeFilter::Figure => kind == LabelKind::Figure,
        LabelTypeFilter::Other => kind == LabelKind::Other,
    }
}

fn label_kind_color(kind: LabelKind) -> gpui::Hsla {
    match kind {
        LabelKind::Section => Theme::accent_blue(),
        LabelKind::Equation => Theme::accent_mauve(),
        LabelKind::Table => Theme::accent_teal(),
        LabelKind::Figure => Theme::accent_peach(),
        LabelKind::Other => Theme::text_dim(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

pub fn render_sidebar(
    current_tab: SidebarTab,
    file_tree: &[crate::backend::TreeNode],
    expanded_folders: &HashSet<String>,
    outline_sections: &[SectionItem],
    outline_labels: &[LabelItem],
    label_filter: LabelTypeFilter,
    todos: &[TodoItem],
    current_file_path: Option<&str>,
    tree_scroll_handle: &ScrollHandle,
    outline_scroll_handle: &ScrollHandle,
    todo_scroll_handle: &ScrollHandle,
    on_select_tab: impl Fn(SidebarTab, &mut Window, &mut App) + 'static + Clone,
    on_set_label_filter: impl Fn(LabelTypeFilter, &mut Window, &mut App) + 'static + Clone,
    on_toggle_folder: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_open_file: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_jump_to_location: impl Fn(String, usize, &mut Window, &mut App) + 'static + Clone,
    on_open_table: impl Fn(String, usize, usize, &mut Window, &mut App) + 'static + Clone,
    git: &GitPanelState,
    on_git_refresh: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_git_open_change: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_git_open_commit: impl Fn(Commit, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    div()
        .h_full()
        .flex()
        .flex_row()
        .overflow_hidden()
        // 1. Far Left Ribbon (Activity Bar)
        .child(render_ribbon_bar(current_tab, git.changes.len(), on_select_tab))
        // 2. Sidebar Panel Content
        .child(
            div()
                .w(px(250.0))
                .h_full()
                .bg(Theme::bg_sidebar())
                .border_r_1()
                .border_color(Theme::border_subtle())
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(match current_tab {
                    SidebarTab::Explorer => render_explorer_panel(
                        file_tree,
                        expanded_folders,
                        current_file_path,
                        tree_scroll_handle,
                        on_toggle_folder,
                        on_open_file,
                    )
                    .into_any_element(),
                    SidebarTab::OutlineTodos => render_outline_todos_panel(
                        outline_sections,
                        outline_labels,
                        label_filter,
                        todos,
                        current_file_path,
                        outline_scroll_handle,
                        todo_scroll_handle,
                        on_set_label_filter,
                        on_jump_to_location,
                        on_open_table,
                    )
                    .into_any_element(),
                    SidebarTab::Git => render_git_panel(git, on_git_refresh, on_git_open_change, on_git_open_commit)
                        .into_any_element(),
                }),
        )
}

// ─────────────────────────────────────────────────────────────────────────────
// Ribbon (icon-only activity bar)
// ─────────────────────────────────────────────────────────────────────────────

fn render_ribbon_bar(
    current_tab: SidebarTab,
    git_change_count: usize,
    on_select_tab: impl Fn(SidebarTab, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let on_tab1 = on_select_tab.clone();
    let on_tab2 = on_select_tab.clone();
    let on_tab3 = on_select_tab.clone();

    div()
        .w(px(46.0))
        .h_full()
        .bg(Theme::bg_ribbon())
        .border_r_1()
        .border_color(Theme::border_subtle())
        .flex()
        .flex_col()
        .items_center()
        .py_3()
        .gap_3()
        // Ribbon Tab 1: File Explorer
        .child(
            render_ribbon_button(
                IconName::Files,
                "Explorer",
                current_tab == SidebarTab::Explorer,
                move |window, cx| {
                    on_tab1(SidebarTab::Explorer, window, cx);
                },
            ),
        )
        // Ribbon Tab 2: Project Outline & TODOs
        .child(
            render_ribbon_button(
                IconName::ListTree,
                "Outline & Tasks",
                current_tab == SidebarTab::OutlineTodos,
                move |window, cx| {
                    on_tab2(SidebarTab::OutlineTodos, window, cx);
                },
            ),
        )
        // Ribbon Tab 3: Source Control, with a badge counting changed files
        .child(
            div()
                .relative()
                .child(render_ribbon_button(
                    IconName::GitBranch,
                    "Source Control",
                    current_tab == SidebarTab::Git,
                    move |window, cx| {
                        on_tab3(SidebarTab::Git, window, cx);
                    },
                ))
                .when(git_change_count > 0, |d| {
                    d.child(
                        div()
                            .absolute()
                            .top(px(1.0))
                            .right(px(-2.0))
                            .min_w(px(15.0))
                            .h(px(15.0))
                            .px(px(3.0))
                            .rounded_full()
                            .bg(Theme::accent_blue())
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(9.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(Theme::text_inverted())
                            .child(if git_change_count > 99 { "99+".to_string() } else { git_change_count.to_string() }),
                    )
                }),
        )
}

// ─────────────────────────────────────────────────────────────────────────────
// Source Control panel
// ─────────────────────────────────────────────────────────────────────────────

fn change_kind_color(kind: ChangeKind) -> Hsla {
    match kind {
        ChangeKind::Modified | ChangeKind::Renamed => Theme::accent_blue(),
        ChangeKind::Added | ChangeKind::Untracked => Theme::accent_green(),
        ChangeKind::Deleted => Theme::accent_red(),
        ChangeKind::Conflicted => Theme::accent_peach(),
    }
}

fn git_section_header(section_icon: IconName, title: String) -> Div {
    div()
        .px_3()
        .pt_3()
        .pb_1()
        .flex()
        .items_center()
        .gap_1p5()
        .child(icon(section_icon).size(px(11.0)).text_color(Theme::text_dim()))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(Theme::text_dim())
                .min_w_0()
                .truncate()
                .child(title),
        )
}

fn git_note(text: &'static str) -> Div {
    div().px_3().py_1().text_xs().text_color(Theme::text_dim()).child(text)
}

fn render_git_panel(
    git: &GitPanelState,
    on_refresh: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_open_change: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_open_commit: impl Fn(Commit, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let header = div()
        .px_3()
        .py_2p5()
        .bg(Theme::bg_titlebar())
        .border_b_1()
        .border_color(Theme::border_subtle())
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(icon(IconName::GitBranch).size(px(12.0)).text_color(Theme::text_dim()))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(Theme::text_muted())
                        .child("SOURCE CONTROL"),
                ),
        )
        .child(
            div()
                .id("git_refresh")
                .size(px(20.0))
                .rounded_md()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|h| h.bg(Theme::bg_hover()))
                .on_click(move |_, window, cx| on_refresh(window, cx))
                .child(
                    icon(if git.refreshing { IconName::LoaderCircle } else { IconName::RotateCw })
                        .size(px(12.0))
                        .text_color(Theme::text_muted()),
                ),
        );

    let mut content: Vec<AnyElement> = Vec::new();
    if !git.loaded {
        content.push(git_note("Loading…").into_any_element());
    } else if git.root.is_none() {
        content.push(git_note("This project is not a git repository.").pt_3().into_any_element());
    } else {
        // Branch
        content.push(
            div()
                .mx_3()
                .mt_3()
                .px_2()
                .py_1p5()
                .rounded_md()
                .bg(Theme::bg_card())
                .flex()
                .items_center()
                .gap_1p5()
                .child(icon(IconName::GitBranch).size(px(13.0)).text_color(Theme::accent_mauve()))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(Theme::text_bright())
                        .child(git.branch.clone().unwrap_or_else(|| "detached HEAD".to_string())),
                )
                .into_any_element(),
        );

        // Changed files
        content.push(git_section_header(IconName::GitCompare, format!("CHANGES ({})", git.changes.len())).into_any_element());
        if git.changes.is_empty() {
            content.push(git_note("Working tree clean").into_any_element());
        }
        for change in &git.changes {
            // Untracked directories are reported with a trailing slash
            let is_dir = change.path.ends_with('/');
            let trimmed = change.path.trim_end_matches('/');
            let (dir, name) = match trimmed.rsplit_once('/') {
                Some((d, n)) => (d.to_string(), n.to_string()),
                None => (String::new(), trimmed.to_string()),
            };
            let (file_ic, file_color) = if is_dir {
                (IconName::Folder, Theme::accent_yellow())
            } else {
                file_icon(&name)
            };
            let kind_color = change_kind_color(change.kind);
            let rel = change.path.clone();
            let on_open = on_open_change.clone();
            content.push(
                div()
                    .id(SharedString::from(format!("git_change_{}", change.path)))
                    .h(px(24.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .cursor_pointer()
                    .hover(|h| h.bg(Theme::bg_hover()))
                    .on_click(move |_, window, cx| on_open(rel.clone(), window, cx))
                    .child(icon(file_ic).size(px(13.0)).text_color(file_color))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .overflow_hidden()
                            // Name keeps its width first; the folder gets whatever is left.
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_xs()
                                    .text_color(if change.kind == ChangeKind::Deleted {
                                        Theme::text_dim()
                                    } else {
                                        Theme::text_primary()
                                    })
                                    .when(change.kind == ChangeKind::Deleted, |d| d.line_through())
                                    .child(name),
                            )
                            .child(div().flex_1().min_w_0().truncate().text_xs().text_color(Theme::text_dim()).child(dir)),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .font_family(".AppleSystemUIFontMonospaced")
                            .font_weight(FontWeight::BOLD)
                            .text_color(kind_color)
                            .child(change.kind.letter()),
                    )
                    .into_any_element(),
            );
        }

        // History of the active file
        match &git.history_file {
            None => {
                content.push(git_section_header(IconName::History, "HISTORY".to_string()).into_any_element());
                content.push(git_note("Open a file to see its history.").into_any_element());
            }
            Some(file) => {
                let file_name = file.rsplit('/').next().unwrap_or(file).to_string();
                content.push(git_section_header(IconName::History, format!("HISTORY · {file_name}")).into_any_element());
                if git.history.is_empty() {
                    content.push(git_note("No commits for this file yet.").into_any_element());
                }
                for commit in &git.history {
                    let on_open = on_open_commit.clone();
                    let c = commit.clone();
                    content.push(
                        div()
                            .id(SharedString::from(format!("git_commit_{}", commit.hash)))
                            .px_3()
                            .py_1p5()
                            .flex()
                            .gap_2()
                            .cursor_pointer()
                            .hover(|h| h.bg(Theme::bg_hover()))
                            .on_click(move |_, window, cx| on_open(c.clone(), window, cx))
                            .child(
                                div()
                                    .pt(px(2.0))
                                    .child(icon(IconName::GitCommitHorizontal).size(px(13.0)).text_color(Theme::accent_peach())),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(Theme::text_primary())
                                            .truncate()
                                            .child(commit.subject.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_1p5()
                                            .text_xs()
                                            .text_color(Theme::text_dim())
                                            .child(
                                                div()
                                                    .font_family(".AppleSystemUIFontMonospaced")
                                                    .text_color(Theme::accent_blue())
                                                    .child(commit.short_hash.clone()),
                                            )
                                            .child(div().min_w_0().truncate().child(format!(
                                                "{} · {}",
                                                commit.author, commit.relative_date
                                            ))),
                                    ),
                            )
                            .into_any_element(),
                    );
                }
            }
        }
    }

    div()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(header)
        .child(
            div()
                .id("git_panel_scroll")
                .flex_1()
                .overflow_y_scroll()
                .track_scroll(&git.scroll_handle)
                .pb_3()
                .children(content),
        )
}

fn render_ribbon_button(
    ribbon_icon: IconName,
    tooltip: &'static str,
    is_active: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    div()
        .id(ElementId::Name(format!("ribbon_btn_{}", tooltip).into()))
        .w(px(36.0))
        .h(px(36.0))
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .when(is_active, |d| {
            d.bg(Theme::bg_active())
                .border_l_2()
                .border_color(Theme::accent_blue())
        })
        .when(!is_active, |d| {
            d.hover(|h| h.bg(Theme::bg_hover()))
        })
        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
            on_click(window, cx);
        })
        .child(
            icon(ribbon_icon)
                .size(px(18.0))
                .text_color(if is_active { Theme::text_bright() } else { Theme::text_muted() }),
        )
}

// ─────────────────────────────────────────────────────────────────────────────
// File Explorer panel
// ─────────────────────────────────────────────────────────────────────────────

fn render_explorer_panel(
    file_tree: &[crate::backend::TreeNode],
    expanded_folders: &HashSet<String>,
    current_file_path: Option<&str>,
    tree_scroll_handle: &ScrollHandle,
    on_toggle_folder: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_open_file: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden()
        // Explorer Header
        .child(
            div()
                .px_3()
                .py_2p5()
                .bg(Theme::bg_titlebar())
                .border_b_1()
                .border_color(Theme::border_subtle())
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .child(icon(IconName::Files).size(px(12.0)).text_color(Theme::text_dim()))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(Theme::text_muted())
                                .child("FILE EXPLORER"),
                        ),
                )
                .child(
                    div()
                        .px_1p5()
                        .py_0p5()
                        .rounded_full()
                        .bg(Theme::bg_card())
                        .text_xs()
                        .font_family(".AppleSystemUIFontMonospaced")
                        .text_color(Theme::text_dim())
                        .child(format!("{}", file_tree.len())),
                ),
        )
        // File Tree Scroll Area
        .child(
            div()
                .id("sidebar_file_tree_scroll")
                .flex_1()
                .overflow_y_scroll()
                .track_scroll(tree_scroll_handle)
                .py_1()
                .children(file_tree.iter().map(|node| {
                    render_tree_node(
                        node,
                        expanded_folders,
                        current_file_path,
                        0,
                        on_toggle_folder.clone(),
                        on_open_file.clone(),
                    )
                })),
        )
}

// ─────────────────────────────────────────────────────────────────────────────
// Outline + TODOs panel
// ─────────────────────────────────────────────────────────────────────────────

fn render_outline_todos_panel(
    outline_sections: &[SectionItem],
    outline_labels: &[LabelItem],
    label_filter: LabelTypeFilter,
    todos: &[TodoItem],
    current_file_path: Option<&str>,
    outline_scroll_handle: &ScrollHandle,
    todo_scroll_handle: &ScrollHandle,
    on_set_label_filter: impl Fn(LabelTypeFilter, &mut Window, &mut App) + 'static + Clone,
    on_jump_to_location: impl Fn(String, usize, &mut Window, &mut App) + 'static + Clone,
    on_open_table: impl Fn(String, usize, usize, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    // Pre-count filtered labels for the badge
    let filtered_label_count = outline_labels
        .iter()
        .filter(|l| label_kind_matches_filter(classify_label(&l.key), label_filter))
        .count();
    let total_outline_count = outline_sections.len() + filtered_label_count;

    div()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden()
        // ==========================================
        // 1. TOP: Project Outline (sections + labels)
        // ==========================================
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .overflow_hidden()
                // Header row
                .child(
                    div()
                        .px_3()
                        .py_2p5()
                        .bg(Theme::bg_titlebar())
                        .border_b_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        // Title + badge row
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1p5()
                                        .child(icon(IconName::ListTree).size(px(12.0)).text_color(Theme::text_dim()))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(Theme::text_muted())
                                                .child("PROJECT OUTLINE"),
                                        ),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .bg(Theme::bg_card())
                                        .rounded_full()
                                        .text_xs()
                                        .font_family(".AppleSystemUIFontMonospaced")
                                        .text_color(Theme::accent_blue())
                                        .child(format!("{}", total_outline_count)),
                                ),
                        )
                        // Label type filter chips
                        .child(render_label_filter_bar(
                            label_filter,
                            on_set_label_filter,
                        )),
                )
                // Outline Scroll List
                .child(
                    div()
                        .id("sidebar_outline_scroll")
                        .flex_1()
                        .overflow_y_scroll()
                        .track_scroll(outline_scroll_handle)
                        .px_1p5()
                        .py_1p5()
                        .children(render_outline_items(
                            outline_sections,
                            outline_labels,
                            label_filter,
                            current_file_path,
                            on_jump_to_location.clone(),
                            on_open_table,
                        )),
                ),
        )
        // ==========================================
        // DIVIDER
        // ==========================================
        .child(
            div()
                .h(px(1.0))
                .bg(Theme::border_subtle()),
        )
        // ==========================================
        // 2. BOTTOM: TODO & Notes Cards
        // ==========================================
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .overflow_hidden()
                // Header
                .child(
                    div()
                        .px_3()
                        .py_2p5()
                        .bg(Theme::bg_titlebar())
                        .border_b_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1p5()
                                        .child(icon(IconName::ListTodo).size(px(12.0)).text_color(Theme::text_dim()))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(Theme::text_muted())
                                                .child("TODO & NOTES"),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .bg(Theme::bg_card())
                                .rounded_full()
                                .text_xs()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .text_color(Theme::accent_peach())
                                .child(format!("{}", todos.len())),
                        ),
                )
                // TODO Scroll List
                .child(
                    div()
                        .id("sidebar_todo_scroll")
                        .flex_1()
                        .overflow_y_scroll()
                        .track_scroll(todo_scroll_handle)
                        .px_2()
                        .py_2()
                        .gap_2()
                        .children(if todos.is_empty() {
                            vec![
                                div()
                                    .px_3()
                                    .py_4()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(Theme::text_dim())
                                            .child("No notes or TODOs found"),
                                    )
                                    .into_any_element(),
                            ]
                        } else {
                            todos
                                .iter()
                                .map(|todo| {
                                    let is_note = todo.tag == "NOTE" || todo.tag == "IDEA";
                                    let (tag_bg, tag_color) = if is_note {
                                        (Theme::bg_card(), Theme::accent_teal())
                                    } else {
                                        (Theme::bg_card(), Theme::accent_peach())
                                    };

                                    let t_file = todo.file.clone();
                                    let t_line = todo.line;
                                    let t_tag = todo.tag.clone();
                                    let t_text = todo.text.clone();
                                    let t_fname = todo.filename.clone();
                                    let on_jump = on_jump_to_location.clone();

                                    div()
                                        .p_2p5()
                                        .my_1()
                                        .rounded_md()
                                        .bg(Theme::bg_card())
                                        .border_1()
                                        .border_color(Theme::border_subtle())
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .cursor_pointer()
                                        .hover(|h| h.border_color(Theme::border_focus()))
                                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                            on_jump(t_file.clone(), t_line, window, cx);
                                        })
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .px_1p5()
                                                        .py_0p5()
                                                        .rounded_xs()
                                                        .bg(tag_bg)
                                                        .text_xs()
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(tag_color)
                                                        .child(t_tag),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_family(".AppleSystemUIFontMonospaced")
                                                        .text_color(Theme::text_dim())
                                                        .child(format!("{}:{}", t_fname, t_line)),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(Theme::text_secondary())
                                                .child(t_text),
                                        )
                                        .into_any_element()
                                })
                                .collect()
                        }),
                ),
        )
}

// ─────────────────────────────────────────────────────────────────────────────
// Label filter bar (chips row)
// ─────────────────────────────────────────────────────────────────────────────

fn render_label_filter_bar(
    current_filter: LabelTypeFilter,
    on_set_filter: impl Fn(LabelTypeFilter, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let filters: &[(Option<IconName>, &str, LabelTypeFilter)] = &[
        (None, "All", LabelTypeFilter::All),
        (Some(IconName::Heading), "Sec", LabelTypeFilter::Section),
        (Some(IconName::Sigma), "Eq", LabelTypeFilter::Equation),
        (Some(IconName::Table), "Tab", LabelTypeFilter::Table),
        (Some(IconName::Image), "Fig", LabelTypeFilter::Figure),
        (Some(IconName::Tag), "Other", LabelTypeFilter::Other),
    ];

    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap_1()
        .children(filters.iter().map(|(chip_icon, label, filter)| {
            let f = *filter;
            let is_active = current_filter == f;
            let on_click = on_set_filter.clone();
            div()
                .id(ElementId::Name(format!("label_filter_{:?}", f).into()))
                .px_1p5()
                .py_0p5()
                .rounded_full()
                .cursor_pointer()
                .text_xs()
                .font_family(".AppleSystemUIFontMonospaced")
                .when(is_active, |d| {
                    d.bg(Theme::accent_blue())
                        .text_color(Theme::bg_sidebar())
                })
                .when(!is_active, |d| {
                    d.bg(Theme::bg_card())
                        .text_color(Theme::text_dim())
                        .hover(|h| h.bg(Theme::bg_hover()).text_color(Theme::text_primary()))
                })
                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                    on_click(f, window, cx);
                })
                .flex()
                .items_center()
                .gap_1()
                .when_some(*chip_icon, |d, name| {
                    d.child(icon(name).size(px(11.0)).text_color(if is_active {
                        Theme::bg_sidebar()
                    } else {
                        Theme::text_dim()
                    }))
                })
                .child(*label)
        }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Outline items (sections interleaved with labels)
// ─────────────────────────────────────────────────────────────────────────────

fn render_outline_items(
    outline_sections: &[SectionItem],
    outline_labels: &[LabelItem],
    label_filter: LabelTypeFilter,
    current_file_path: Option<&str>,
    on_jump_to_location: impl Fn(String, usize, &mut Window, &mut App) + 'static + Clone,
    _on_open_table: impl Fn(String, usize, usize, &mut Window, &mut App) + 'static + Clone,
) -> Vec<AnyElement> {
    let mut items: Vec<AnyElement> = Vec::new();

    // ── Section items ────────────────────────────────────────────────────────
    for sec in outline_sections {
        let sec_file = sec.file.clone();
        let line_num = sec.line;
        let on_jump = on_jump_to_location.clone();
        let indent_px = px((sec.level.saturating_sub(1) as f32) * 12.0 + 8.0);
        let is_in_current_file = current_file_path.map(|p| p == sec.file).unwrap_or(false);

        let sym_color = if is_in_current_file {
            Theme::accent_blue()
        } else if sec.level <= 1 {
            Theme::accent_blue()
        } else if sec.level == 2 {
            Theme::accent_mauve()
        } else {
            Theme::accent_teal()
        };

        items.push(
            div()
                .px_2()
                .py_1p5()
                .pl(indent_px)
                .my_0p5()
                .rounded_md()
                .flex()
                .items_center()
                .justify_between()
                .cursor_pointer()
                .when(is_in_current_file, |d| {
                    d.bg(Theme::bg_card())
                        .border_l_2()
                        .border_color(Theme::accent_blue())
                })
                .when(!is_in_current_file, |d| {
                    d.hover(|h| h.bg(Theme::bg_hover()))
                })
                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                    on_jump(sec_file.clone(), line_num, window, cx);
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .overflow_hidden()
                        .child(icon(IconName::Hash).size(px(12.0)).text_color(sym_color))
                        .child(
                            div()
                                .text_xs()
                                .text_color(if is_in_current_file {
                                    Theme::text_bright()
                                } else {
                                    Theme::text_secondary()
                                })
                                .min_w_0()
                                .truncate()
                                .child(sec.title.clone()),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(".AppleSystemUIFontMonospaced")
                        .text_color(if is_in_current_file { Theme::accent_blue() } else { Theme::text_dim() })
                        .child(format!(":{}", sec.line)),
                )
                .into_any_element(),
        );
    }

    // ── Label items (filtered) ───────────────────────────────────────────────
    // Sort labels by file then line for a predictable order
    let mut sorted_labels: Vec<&LabelItem> = outline_labels
        .iter()
        .filter(|l| label_kind_matches_filter(classify_label(&l.key), label_filter))
        .collect();
    sorted_labels.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));

    if sorted_labels.is_empty() && outline_sections.is_empty() {
        items.push(
            div()
                .px_3()
                .py_4()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(Theme::text_dim())
                        .child("No outline items found"),
                )
                .into_any_element(),
        );
        return items;
    }

    // Separator between sections and labels (only when both exist)
    if !outline_sections.is_empty() && !sorted_labels.is_empty() {
        items.push(
            div()
                .mx_2()
                .my_1()
                .h(px(1.0))
                .bg(Theme::border_subtle())
                .into_any_element(),
        );
        items.push(
            div()
                .px_2()
                .py_0p5()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(Theme::text_dim())
                        .child("LABELS"),
                )
                .into_any_element(),
        );
    }

    for label in sorted_labels {
        let kind = classify_label(&label.key);
        let kind_icon = label_kind_icon(kind);
        let color = label_kind_color(kind);
        let lbl_file = label.file.clone();
        let lbl_line = label.line;
        let on_jump = on_jump_to_location.clone();
        let is_in_current_file = current_file_path.map(|p| p == label.file).unwrap_or(false);
        let display_key = label.key.clone();
        let filename = label.filename.clone();

        items.push(
            div()
                .px_2()
                .py_1p5()
                .pl(px(16.0))
                .my_0p5()
                .rounded_md()
                .flex()
                .items_center()
                .justify_between()
                .cursor_pointer()
                .when(is_in_current_file, |d| {
                    d.bg(Theme::bg_card())
                        .border_l_2()
                        .border_color(color)
                })
                .when(!is_in_current_file, |d| {
                    d.hover(|h| h.bg(Theme::bg_hover()))
                })
                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                    on_jump(lbl_file.clone(), lbl_line, window, cx);
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .overflow_hidden()
                        // Type symbol badge
                        .child(
                            div()
                                .w(px(16.0))
                                .h(px(16.0))
                                .rounded_xs()
                                .bg(Theme::bg_card())
                                .flex()
                                .items_center()
                                .justify_center()
                                .flex_shrink_0()
                                .child(icon(kind_icon).size(px(11.0)).text_color(color)),
                        )
                        // Label key text
                        .child(
                            div()
                                .text_xs()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .text_color(if is_in_current_file {
                                    Theme::text_bright()
                                } else {
                                    Theme::text_secondary()
                                })
                                .min_w_0()
                                .truncate()
                                .child(display_key),
                        ),
                )
                // Right: filename:line
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_0p5()
                        .flex_shrink_0()
                        .child(
                            div()
                                .text_xs()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .text_color(Theme::text_dim())
                                .child(format!("{}:{}", filename, lbl_line)),
                        ),
                )
                .into_any_element(),
        );
    }

    items
}

// Helper stub (unused but avoids dead code warning)


// ─────────────────────────────────────────────────────────────────────────────
// Tree node renderer (file explorer)
// ─────────────────────────────────────────────────────────────────────────────

fn render_tree_node(
    node: &crate::backend::TreeNode,
    expanded_folders: &HashSet<String>,
    current_file_path: Option<&str>,
    depth: usize,
    on_toggle_folder: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_open_file: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
) -> AnyElement {
    let is_folder = node.node_type == "folder";
    let is_expanded = is_folder && expanded_folders.contains(&node.path);
    let is_active = current_file_path == Some(&node.path);

    let node_path = node.path.clone();
    let on_toggle = on_toggle_folder.clone();
    let on_open = on_open_file.clone();

    let indent_px = px(depth as f32 * 14.0 + 8.0);

    let (node_icon, icon_color) = if is_folder {
        let folder_icon = if is_expanded { IconName::FolderOpen } else { IconName::Folder };
        (folder_icon, Theme::accent_yellow())
    } else {
        file_icon(&node.name)
    };

    let mut children_elements = Vec::new();
    if is_expanded {
        if let Some(ref children) = node.children {
            for child in children {
                children_elements.push(render_tree_node(
                    child,
                    expanded_folders,
                    current_file_path,
                    depth + 1,
                    on_toggle_folder.clone(),
                    on_open_file.clone(),
                ));
            }
        }
    }

    div()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(24.0))
                .pl(indent_px)
                .pr_2()
                .flex()
                .items_center()
                .gap_1p5()
                .cursor_pointer()
                .when(is_active, |d| d.bg(Theme::bg_active()))
                .when(!is_active, |d| d.hover(|h| h.bg(Theme::bg_hover())))
                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                    if is_folder {
                        on_toggle(node_path.clone(), window, cx);
                    } else {
                        on_open(node_path.clone(), window, cx);
                    }
                })
                // Disclosure chevron (folders) or an equally wide spacer (files) keeps names aligned
                .child(if is_folder {
                    icon(if is_expanded { IconName::ChevronDown } else { IconName::ChevronRight })
                        .size(px(12.0))
                        .text_color(Theme::text_dim())
                        .into_any_element()
                } else {
                    div().w(px(12.0)).flex_none().into_any_element()
                })
                .child(icon(node_icon).size(px(14.0)).text_color(icon_color))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(if is_active { Theme::text_bright() } else { Theme::text_primary() })
                        .child(node.name.clone()),
                ),
        )
        .children(children_elements)
        .into_any_element()
}
