use crate::backend::{SectionItem, TodoItem, TreeNode};
use crate::state::SidebarTab;
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;
use std::collections::HashSet;

pub fn render_sidebar(
    current_tab: SidebarTab,
    file_tree: &[TreeNode],
    expanded_folders: &HashSet<String>,
    outline_sections: &[SectionItem],
    todos: &[TodoItem],
    current_file_path: Option<&str>,
    tree_scroll_handle: &ScrollHandle,
    outline_scroll_handle: &ScrollHandle,
    todo_scroll_handle: &ScrollHandle,
    on_select_tab: impl Fn(SidebarTab, &mut Window, &mut App) + 'static + Clone,
    on_toggle_folder: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_open_file: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_jump_to_location: impl Fn(String, usize, &mut Window, &mut App) + 'static + Clone,
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
                        todos,
                        current_file_path,
                        outline_scroll_handle,
                        todo_scroll_handle,
                        on_jump_to_location,
                    )
                    .into_any_element(),
                }),
        )
}

fn render_ribbon_bar(
    current_tab: SidebarTab,
    on_select_tab: impl Fn(SidebarTab, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let on_tab1 = on_select_tab.clone();
    let on_tab2 = on_select_tab.clone();

    div()
        .w(px(46.0))
        .h_full()
        .bg(Theme::bg_titlebar())
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
                .child(icon),
        )
}

fn render_explorer_panel(
    file_tree: &[TreeNode],
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
                .py_2()
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
                        .text_color(Theme::text_dim())
                        .child("FILE EXPLORER"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(Theme::text_dim())
                        .child(format!("{} items", file_tree.len())),
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

fn render_outline_todos_panel(
    outline_sections: &[SectionItem],
    todos: &[TodoItem],
    current_file_path: Option<&str>,
    outline_scroll_handle: &ScrollHandle,
    todo_scroll_handle: &ScrollHandle,
    on_jump_to_location: impl Fn(String, usize, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden()
        // ==========================================
        // UPPER HALF: Project Outline
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
                        .py_2()
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
                                        .text_color(Theme::text_dim())
                                        .child("PROJECT OUTLINE"),
                                ),
                        )
                        .child(
                            div()
                                .px_1p5()
                                .py_0p5()
                                .bg(Theme::bg_card())
                                .rounded_sm()
                                .text_xs()
                                .text_color(Theme::accent_purple())
                                .child(format!("{}", outline_sections.len())),
                        ),
                )
                // Outline Scroll List
                .child(
                    div()
                        .id("sidebar_outline_scroll")
                        .flex_1()
                        .overflow_y_scroll()
                        .track_scroll(outline_scroll_handle)
                        .py_1()
                        .children(if outline_sections.is_empty() {
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
                                            .child("No outline sections found"),
                                    )
                                    .into_any_element(),
                            ]
                        } else {
                            outline_sections
                                .iter()
                                .map(|sec| {
                                    let sec_file = sec.file.clone();
                                    let line_num = sec.line;
                                    let on_jump = on_jump_to_location.clone();
                                    let indent_px = px((sec.level.saturating_sub(1) as f32) * 10.0 + 8.0);
                                    let is_in_current_file = current_file_path.map(|p| p == sec.file).unwrap_or(false);

                                    div()
                                        .px_2()
                                        .py_1()
                                        .pl(indent_px)
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .cursor_pointer()
                                        .hover(|h| h.bg(Theme::bg_hover()))
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
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(if sec.level <= 1 {
                                                            Theme::accent_blue()
                                                        } else if sec.level == 2 {
                                                            Theme::accent_purple()
                                                        } else {
                                                            Theme::accent_cyan()
                                                        })
                                                        .child("§"),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(if is_in_current_file {
                                                            Theme::text_bright()
                                                        } else {
                                                            Theme::text_primary()
                                                        })
                                                        .overflow_hidden()
                                                        .child(sec.title.clone()),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(Theme::text_dim())
                                                .child(format!(":{}", sec.line)),
                                        )
                                        .into_any_element()
                                })
                                .collect()
                        }),
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
        // BOTTOM HALF: TODO List
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
                        .py_2()
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
                                        .text_color(Theme::text_dim())
                                        .child("TODO LIST"),
                                ),
                        )
                        .child(
                            div()
                                .px_1p5()
                                .py_0p5()
                                .bg(Theme::bg_card())
                                .rounded_sm()
                                .text_xs()
                                .text_color(Theme::accent_yellow())
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
                        .py_1()
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
                                            .child("No TODOs found"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(Theme::text_dim())
                                            .child("(e.g. % TODO: ... or % FIXME: ...)"),
                                    )
                                    .into_any_element(),
                            ]
                        } else {
                            todos
                                .iter()
                                .map(|todo| {
                                    let todo_file = todo.file.clone();
                                    let line_num = todo.line;
                                    let on_jump = on_jump_to_location.clone();

                                    let tag_color = match todo.tag.as_str() {
                                        "FIXME" | "BUG" => Theme::accent_red(),
                                        "NOTE" | "IDEA" => Theme::accent_green(),
                                        "HACK" | "XXX" => Theme::accent_orange(),
                                        "DONE" => Theme::accent_green(),
                                        _ => Theme::accent_yellow(),
                                    };

                                    div()
                                        .px_2()
                                        .py_1p5()
                                        .mx_1()
                                        .my_0p5()
                                        .rounded_sm()
                                        .flex()
                                        .flex_col()
                                        .gap_0p5()
                                        .cursor_pointer()
                                        .hover(|h| h.bg(Theme::bg_hover()))
                                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                            on_jump(todo_file.clone(), line_num, window, cx);
                                        })
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .px_1()
                                                        .py_0p5()
                                                        .rounded_xs()
                                                        .bg(Theme::bg_active())
                                                        .text_xs()
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(tag_color)
                                                        .child(todo.tag.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(Theme::text_dim())
                                                        .child(format!("{}:{}", todo.filename, todo.line)),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(Theme::text_primary())
                                                .child(todo.text.clone()),
                                        )
                                        .into_any_element()
                                })
                                .collect()
                        }),
                ),
        )
}

fn render_tree_node(
    node: &TreeNode,
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
