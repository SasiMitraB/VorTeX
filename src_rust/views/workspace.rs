use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

pub fn render_toolbar(
    project_name: Option<String>,
    sidebar_visible: bool,
    is_building: bool,
    _theme_label: &str,
    on_back_to_projects: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_toggle_sidebar: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_open_file: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_open_folder: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_new_file: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_save: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_build: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_insert_table: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_sync_pdf: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_toggle_theme: impl Fn(&mut Window, &mut App) + 'static + Clone,
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
    let on_theme = on_toggle_theme.clone();
    let name = project_name.unwrap_or_else(|| "VorTeX".to_string());
    let is_dark = Theme::mode().is_dark();

    div()
        .h(px(46.0))
        .bg(Theme::bg_titlebar())
        .border_b_1()
        .border_color(Theme::border_subtle())
        .pl(if cfg!(target_os = "macos") { px(82.0) } else { px(12.0) })
        .pr_3()
        .flex()
        .items_center()
        .justify_between()
        .child(
            // Left cluster: Sidebar toggle, Project Breadcrumb, File Actions
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    // Sidebar Toggler (outline/explorer toggle)
                    div()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(Theme::bg_card())
                        .hover(|h| h.bg(Theme::bg_hover()))
                        .border_1()
                        .border_color(Theme::border_subtle())
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .text_xs()
                        .text_color(if sidebar_visible { Theme::accent_blue() } else { Theme::text_dim() })
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_toggle(window, cx);
                        })
                        .child(if sidebar_visible { "◧" } else { "◻" }),
                )
                .child(
                    // Project breadcrumb chip: ● ProjectName ▾
                    div()
                        .px_2p5()
                        .py_1()
                        .rounded_md()
                        .bg(Theme::bg_card())
                        .hover(|h| h.bg(Theme::bg_hover()))
                        .border_1()
                        .border_color(Theme::border_subtle())
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_back(window, cx);
                        })
                        .child(
                            div()
                                .w(px(6.0))
                                .h(px(6.0))
                                .rounded_full()
                                .bg(Theme::accent_green()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(Theme::text_bright())
                                .child(name),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(Theme::text_dim())
                                .child("▾"),
                        ),
                )
                .child(
                    div().w(px(1.0)).h(px(16.0)).bg(Theme::border_subtle()).mx_1()
                )
                .child(
                    cat_toolbar_btn("Open", on_op_f)
                )
                .child(
                    cat_toolbar_btn("Folder", on_op_dir)
                )
                .child(
                    cat_toolbar_btn("＋ New", on_new)
                )
                .child(
                    cat_toolbar_btn("Save", on_s)
                )
                .child(
                    cat_toolbar_btn("⊞ Table", on_tbl)
                ),
        )
        .child(
            // Right cluster: Engine badge, Clean Build Button (Mocha Blue), Theme Pill, Sync PDF
            div()
                .flex()
                .items_center()
                .gap_2p5()
                .child(
                    // Engine badge: ● LuaTeX ▾
                    div()
                        .px_2p5()
                        .py_1()
                        .rounded_md()
                        .bg(Theme::bg_card())
                        .border_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .child(
                            div()
                                .w(px(5.0))
                                .h(px(5.0))
                                .rounded_full()
                                .bg(Theme::accent_green()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .text_color(Theme::text_secondary())
                                .child("LuaTeX"),
                        ),
                )
                // Distinctive Blue Build Button with ⌘B badge
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .bg(if is_building { Theme::accent_yellow() } else { Theme::accent_blue() })
                        .hover(|h| h.opacity(0.92))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            if !is_building {
                                on_b(window, cx);
                            }
                        })
                        .child(
                            div()
                                .text_xs()
                                .text_color(Theme::text_inverted())
                                .child(if is_building { "◷" } else { "▶" }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(Theme::text_inverted())
                                .child(if is_building { "Building…" } else { "Build" }),
                        )
                        .child(
                            div()
                                .px_1()
                                .py_0p5()
                                .rounded_xs()
                                .bg(Theme::modal_backdrop())
                                .text_xs()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .text_color(Theme::accent_blue())
                                .child("⌘B"),
                        ),
                )
                // Sync PDF Button
                .child(
                    div()
                        .px_2p5()
                        .py_1()
                        .rounded_md()
                        .bg(Theme::bg_card())
                        .hover(|h| h.bg(Theme::bg_hover()))
                        .border_1()
                        .border_color(Theme::border_subtle())
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_xs()
                        .text_color(Theme::accent_blue())
                        .font_weight(FontWeight::MEDIUM)
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_sync(window, cx);
                        })
                        .child("⇄ Sync PDF"),
                )
                // Catppuccin Theme Pill: Mocha (Dark) / Latte (Light)
                .child(
                    div()
                        .px_1()
                        .py_0p5()
                        .rounded_md()
                        .bg(Theme::bg_card())
                        .border_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .items_center()
                        .gap_0p5()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_theme(window, cx);
                        })
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded_sm()
                                .when(is_dark, |d| {
                                    d.bg(Theme::bg_editor())
                                        .text_color(Theme::accent_mauve())
                                        .font_weight(FontWeight::SEMIBOLD)
                                })
                                .when(!is_dark, |d| {
                                    d.text_color(Theme::text_dim())
                                })
                                .text_xs()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .w(px(5.0))
                                        .h(px(5.0))
                                        .rounded_full()
                                        .bg(if is_dark { Theme::accent_mauve() } else { Theme::text_dim() }),
                                )
                                .child("Mocha"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded_sm()
                                .when(!is_dark, |d| {
                                    d.bg(Theme::bg_editor())
                                        .text_color(Theme::accent_mauve())
                                        .font_weight(FontWeight::SEMIBOLD)
                                })
                                .when(is_dark, |d| {
                                    d.text_color(Theme::text_dim())
                                })
                                .text_xs()
                                .child("Latte"),
                        ),
                ),
        )
}

fn cat_toolbar_btn(label: &'static str, on_click: impl Fn(&mut Window, &mut App) + 'static + Clone) -> impl IntoElement {
    div()
        .px_2p5()
        .py_1()
        .bg(Theme::bg_card())
        .hover(|h| h.bg(Theme::bg_hover()))
        .border_1()
        .border_color(Theme::border_subtle())
        .rounded_md()
        .cursor_pointer()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(Theme::text_secondary())
        .items_center()
        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
            on_click(window, cx);
        })
        .child(label)
}

