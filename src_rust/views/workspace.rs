use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

pub fn render_toolbar(
    project_name: Option<String>,
    sidebar_visible: bool,
    is_building: bool,
    on_back_to_projects: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_toggle_sidebar: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_open_file: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_open_folder: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_new_file: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_save: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_build: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_insert_table: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_sync_pdf: impl Fn(&mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let on_back = on_back_to_projects.clone();
    let on_toggle = on_toggle_sidebar.clone();
    let on_op_f = on_open_file.clone();
    let on_op_dir = on_open_folder.clone();
    let on_new = on_new_file.clone();
    let on_s = on_save.clone();
    let on_b = on_build.clone();
    let on_tbl = on_insert_table.clone();
    let on_sync = on_sync_pdf.clone();
    let name = project_name.unwrap_or_else(|| "VorTeX Workspace".to_string());

    div()
        .h(px(48.0))
        .bg(Theme::bg_titlebar())
        .border_b_1()
        .border_color(Theme::border_subtle())
        .pl(if cfg!(target_os = "macos") { px(80.0) } else { px(12.0) })
        .pr_3()
        .flex()
        .items_center()
        .justify_between()
        .child(
            // Left toolbar buttons
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(
                    toolbar_btn("‹  Projects", on_back)
                )
                .child(
                    div().w(px(1.0)).h(px(16.0)).bg(Theme::border_subtle()).mx_1()
                )
                .child(
                    toolbar_btn(if sidebar_visible { "▣  Explorer" } else { "□  Explorer" }, on_toggle)
                )
                .child(
                    toolbar_btn("Open folder", on_op_dir)
                )
                .child(
                    toolbar_btn("Open file", on_op_f)
                )
                .child(
                    toolbar_btn("＋  New", on_new)
                )
                .child(
                    toolbar_btn("Save", on_s)
                )
                .child(
                    div()
                        .px_2p5()
                        .py_1()
                        .bg(if is_building { Theme::bg_active() } else { Theme::bg_panel() })
                        .hover(move |h| {
                            if !is_building {
                                h.bg(Theme::bg_hover())
                            } else {
                                h
                            }
                        })
                        .border_1()
                        .border_color(if is_building { Theme::accent_yellow() } else { Theme::accent_green() })
                        .rounded_md()
                        .cursor_pointer()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if is_building { Theme::accent_yellow() } else { Theme::accent_green() })
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            if !is_building {
                                on_b(window, cx);
                            }
                        })
                        .child(if is_building { "Building…" } else { "▶  Build" }),
                )
                .child(
                    toolbar_btn("▦  Table", on_tbl)
                )
                .child(
                    div()
                        .px_2p5()
                        .py_1()
                        .bg(Theme::bg_panel())
                        .hover(|h| h.bg(Theme::bg_hover()))
                        .border_1()
                        .border_color(Theme::border_subtle())
                        .rounded_md()
                        .cursor_pointer()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(Theme::accent_blue())
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_sync(window, cx);
                        })
                        .child("⇄  Sync PDF")
                ),
        )
        .child(
            // Right info
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(Theme::text_bright())
                        .child(name),
                ),
        )
}

fn toolbar_btn(label: &'static str, on_click: impl Fn(&mut Window, &mut App) + 'static + Clone) -> impl IntoElement {
    div()
        .px_2p5()
        .py_1()
        .bg(Theme::bg_panel())
        .hover(|h| h.bg(Theme::bg_hover()))
        .border_1()
        .border_color(Theme::border_subtle())
        .rounded_md()
        .cursor_pointer()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(Theme::text_primary())
        .h(px(30.0))
        .items_center()
        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
            on_click(window, cx);
        })
        .child(label)
}
