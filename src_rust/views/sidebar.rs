use crate::backend::{SectionItem, TreeNode};
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;
use std::collections::HashSet;

pub fn render_sidebar(
    file_tree: &[TreeNode],
    expanded_folders: &HashSet<String>,
    outline_sections: &[SectionItem],
    current_file_path: Option<&str>,
    tree_scroll_handle: &ScrollHandle,
    outline_scroll_handle: &ScrollHandle,
    on_toggle_folder: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_open_file: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_jump_to_line: impl Fn(usize, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    div()
        .w(px(240.0))
        .h_full()
        .bg(Theme::bg_sidebar())
        .border_r_1()
        .border_color(Theme::border_subtle())
        .flex()
        .flex_col()
        .overflow_hidden()
        // Explorer Section Header
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
                        .child("EXPLORER"),
                ),
        )
        // File Tree Area
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
        // Outline Section Header
        .child(
            div()
                .px_3()
                .py_2()
                .bg(Theme::bg_titlebar())
                .border_t_1()
                .border_b_1()
                .border_color(Theme::border_subtle())
                .flex()
                .items_center()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(Theme::text_dim())
                        .child("DOCUMENT OUTLINE"),
                ),
        )
        // Outline List
        .child(
            div()
                .id("sidebar_outline_scroll")
                .h(px(180.0))
                .overflow_y_scroll()
                .track_scroll(outline_scroll_handle)
                .py_1()
                .children(if outline_sections.is_empty() {
                    vec![
                        div()
                            .px_3()
                            .py_2()
                            .text_xs()
                            .text_color(Theme::text_dim())
                            .child("No outline sections found")
                            .into_any_element(),
                    ]
                } else {
                    outline_sections.iter().map(|sec| {
                        let line_num = sec.line;
                        let on_jump = on_jump_to_line.clone();
                        let indent_px = px((sec.level.saturating_sub(1) as f32) * 12.0 + 8.0);

                        div()
                            .px_2()
                            .py_1()
                            .pl(indent_px)
                            .flex()
                            .items_center()
                            .gap_2()
                            .cursor_pointer()
                            .hover(|h| h.bg(Theme::bg_hover()))
                            .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                on_jump(line_num, window, cx);
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(Theme::accent_purple())
                                    .child("§"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(Theme::text_primary())
                                    .child(sec.title.clone()),
                            )
                            .into_any_element()
                    }).collect()
                }),
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
