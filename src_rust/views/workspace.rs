use crate::icons::{icon, IconName};
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

/// Slim title bar. File, edit, insert and theme commands live in the native
/// menu bar (see `crate::actions::app_menus`); only the frequently used
/// build/sync controls and project navigation stay here.
pub fn render_toolbar(
    project_name: Option<String>,
    sidebar_visible: bool,
    is_building: bool,
    on_back_to_projects: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_toggle_sidebar: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_build: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_sync_pdf: impl Fn(&mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let on_back = on_back_to_projects.clone();
    let on_toggle = on_toggle_sidebar.clone();
    let on_b = on_build.clone();
    let on_sync = on_sync_pdf.clone();
    let name = project_name.unwrap_or_else(|| "VorTeX".to_string());

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
            // Left cluster: Sidebar toggle, Project Breadcrumb
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    // Sidebar Toggler (outline/explorer toggle)
                    div()
                        .size(px(26.0))
                        .rounded_md()
                        .hover(|h| h.bg(Theme::bg_hover()))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_toggle(window, cx);
                        })
                        .child(
                            icon(if sidebar_visible { IconName::PanelLeftClose } else { IconName::PanelLeft })
                                .size(px(16.0))
                                .text_color(if sidebar_visible { Theme::accent_blue() } else { Theme::text_muted() }),
                        ),
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
                        .child(icon(IconName::FolderOpen).size(px(13.0)).text_color(Theme::accent_yellow()))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(Theme::text_bright())
                                .child(name),
                        )
                        .child(icon(IconName::ChevronDown).size(px(12.0)).text_color(Theme::text_dim())),
                )
        )
        .child(
            // Right cluster: Engine badge, Sync PDF, Build
            div()
                .flex()
                .items_center()
                .gap_2p5()
                .child(
                    // Engine badge: ● LuaTeX
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
                        .gap_1p5()
                        .text_xs()
                        .text_color(Theme::text_secondary())
                        .font_weight(FontWeight::MEDIUM)
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_sync(window, cx);
                        })
                        .child(icon(IconName::ArrowRightLeft).size(px(13.0)).text_color(Theme::accent_blue()))
                        .child("Sync PDF"),
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
                            icon(if is_building { IconName::LoaderCircle } else { IconName::Play })
                                .size(px(12.0))
                                .text_color(Theme::text_inverted()),
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
        )
}

