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
    div()
        .h(px(24.0))
        .bg(Theme::bg_titlebar())
        .border_t_1()
        .border_color(Theme::border_subtle())
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .text_xs()
        .child(
            // Left: status message or active file
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .text_color(if status_msg.is_some() { Theme::accent_green() } else { Theme::text_dim() })
                        .child(status_msg.unwrap_or_else(|| active_file.unwrap_or("Ready")).to_string()),
                ),
        )
        .child(
            // Right: index stats & cursor info
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_color(Theme::text_dim())
                        .child(format!("{} labels", stats.labels))
                        .child("•")
                        .child(format!("{} bib entries", stats.bibentries))
                        .child("•")
                        .child(format!("{} files indexed", stats.files_indexed)),
                )
                .child(
                    div()
                        .text_color(Theme::text_muted())
                        .font_family(".AppleSystemUIFontMonospaced")
                        .child(format!("Ln {}, Col {} ({} lines)", cursor_row + 1, cursor_col + 1, total_lines)),
                )
                .child(
                    div()
                        .text_color(Theme::accent_blue())
                        .child("LaTeX (UTF-8)"),
                ),
        )
}
