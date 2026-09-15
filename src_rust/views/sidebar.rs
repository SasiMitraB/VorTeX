use crate::backend::{LabelItem, SectionItem, TodoItem};
use crate::state::{LabelTypeFilter, SidebarTab};
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

fn label_kind_symbol(kind: LabelKind) -> &'static str {
    match kind {
        LabelKind::Section => "§",
        LabelKind::Equation => "∑",
        LabelKind::Table => "⊞",
        LabelKind::Figure => "⬡",
        LabelKind::Other => "⬧",
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
) -> impl IntoElement {
    div()
        .h_full()
        .flex()
        .flex_row()
        .overflow_hidden()
        // 1. Far Left Ribbon (Activity Bar)
        .child(render_ribbon_bar(current_tab, on_select_tab))
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
                }),
        )
}

// ─────────────────────────────────────────────────────────────────────────────
// Ribbon (icon-only activity bar)
// ─────────────────────────────────────────────────────────────────────────────

fn render_ribbon_bar(
    current_tab: SidebarTab,
    on_select_tab: impl Fn(SidebarTab, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let on_tab1 = on_select_tab.clone();
    let on_tab2 = on_select_tab.clone();

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
                "▣",
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
                "≡",
                "Outline & Tasks",
                current_tab == SidebarTab::OutlineTodos,
                move |window, cx| {
                    on_tab2(SidebarTab::OutlineTodos, window, cx);
                },
            ),
        )
}

fn render_ribbon_button(
    icon: &'static str,
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
            div()
                .text_base()
                .text_color(if is_active { Theme::text_bright() } else { Theme::text_muted() })
                .child(icon),
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
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(Theme::text_muted())
                        .child("FILE EXPLORER"),
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
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(Theme::text_muted())
                                        .child("PROJECT OUTLINE"),
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
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(Theme::text_muted())
                                        .child("TODO & NOTES"),
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
    let filters: &[(&str, LabelTypeFilter)] = &[
        ("All", LabelTypeFilter::All),
        ("§ Sec", LabelTypeFilter::Section),
        ("∑ Eq", LabelTypeFilter::Equation),
        ("⊞ Tab", LabelTypeFilter::Table),
        ("⬡ Fig", LabelTypeFilter::Figure),
        ("⬧ Other", LabelTypeFilter::Other),
    ];

    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap_1()
        .children(filters.iter().map(|(label, filter)| {
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
                        .child(
                            div()
                                .text_xs()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .font_weight(FontWeight::BOLD)
                                .text_color(sym_color)
                                .child("§"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(if is_in_current_file {
                                    Theme::text_bright()
                                } else {
                                    Theme::text_secondary()
                                })
                                .overflow_hidden()
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
        let symbol = label_kind_symbol(kind);
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
                                .text_xs()
                                .text_color(color)
                                .child(symbol),
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
                                .overflow_hidden()
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

    let (icon, icon_color) = if is_folder {
        if is_expanded {
            ("📂", Theme::accent_yellow())
        } else {
            ("📁", Theme::accent_yellow())
        }
    } else {
        let ext = node.name.rsplit('.').next().unwrap_or("").to_lowercase();
        match ext.as_str() {
            "tex" => ("λ", Theme::accent_blue()),
            "bib" => ("❝", Theme::accent_purple()),
            "pdf" => ("📄", Theme::accent_red()),
            "md" => ("📝", Theme::accent_cyan()),
            "py" => ("🐍", Theme::accent_yellow()),
            "js" | "ts" => ("⚡", Theme::accent_yellow()),
            "json" => ("{}", Theme::accent_orange()),
            _ => ("•", Theme::text_dim()),
        }
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
                .gap_2()
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
                .child(
                    div()
                        .text_xs()
                        .text_color(icon_color)
                        .child(icon),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if is_active { Theme::text_bright() } else { Theme::text_primary() })
                        .child(node.name.clone()),
                ),
        )
        .children(children_elements)
        .into_any_element()
}
