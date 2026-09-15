use crate::backend::IndexStats;
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

pub fn render_status_bar(
    active_file: Option<&str>,
    cursor_row: usize,
    cursor_col: usize,
    total_lines: usize,
    stats: &IndexStats,
    status_msg: Option<&str>,
) -> impl IntoElement {
    let file_display = active_file
        .and_then(|p| std::path::Path::new(p).file_name()?.to_str())
        .unwrap_or("Ready");

    div()
        .h(px(25.0))
        .bg(Theme::bg_app())
        .border_t_1()
        .border_color(Theme::border_subtle())
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .text_xs()
        .child(
            // Left cluster: SyncTeX indicator / Status Message
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded_full()
                        .bg(if status_msg.is_some() { Theme::accent_green() } else { Theme::accent_blue() }),
                )
                .child(
                    div()
                        .font_family(".AppleSystemUIFontMonospaced")
                        .text_color(if status_msg.is_some() { Theme::accent_green() } else { Theme::text_secondary() })
                        .child(if let Some(msg) = status_msg {
                            msg.to_string()
                        } else {
                            format!("SyncTeX: {}:{}", file_display, cursor_row + 1)
                        }),
                ),
        )
        .child(
            // Center cluster: Index Stats
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_color(Theme::text_dim())
                .child(format!("{} labels", stats.labels))
                .child(
                    div().text_color(Theme::border_subtle()).child("•")
                )
                .child(format!("{} bib entries", stats.bibentries))
                .child(
                    div().text_color(Theme::border_subtle()).child("•")
                )
                .child(format!("{} files indexed", stats.files_indexed)),
        )
        .child(
            // Right cluster: Cursor Ln/Col, UTF-8, Build Ready
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .text_color(Theme::text_secondary())
                        .font_family(".AppleSystemUIFontMonospaced")
                        .child(format!("Ln {}, Col {} ({} lines)", cursor_row + 1, cursor_col + 1, total_lines)),
                )
                .child(
                    div().text_color(Theme::border_subtle()).child("•")
                )
                .child(
                    div()
                        .text_color(Theme::text_dim())
                        .font_family(".AppleSystemUIFontMonospaced")
                        .child("UTF-8"),
                )
                .child(
                    div().text_color(Theme::border_subtle()).child("•")
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_color(Theme::text_bright())
                        .child(
                            div()
                                .w(px(5.0))
                                .h(px(5.0))
                                .rounded_full()
                                .bg(Theme::accent_green()),
                        )
                        .child(
                            div()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .child("0.14s"),
                        ),
                ),
        )
}

