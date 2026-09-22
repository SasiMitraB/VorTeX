use crate::icons::{icon, IconName};
use crate::services::git::{DiffLineKind, DiffRow};
use crate::state::DiffViewState;
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

/// Rows beyond this are not rendered (keeps huge new files responsive).
const MAX_ROWS: usize = 4000;

fn tint(color: Hsla, alpha: f32) -> Hsla {
    Hsla { a: alpha, ..color }
}

/// Read-only unified diff: `spec.base_rev` version (left numbers) → working copy (right numbers).
pub fn render_diff_view(
    view: &DiffViewState,
    on_refresh: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let spec = &view.spec;
    let (added, removed) = view.diff.as_ref().map(|d| (d.added, d.removed)).unwrap_or((0, 0));

    let header = div()
        .h(px(34.0))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .bg(Theme::bg_titlebar())
        .border_b_1()
        .border_color(Theme::border_subtle())
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .overflow_hidden()
                .child(icon(IconName::GitCompare).size(px(14.0)).text_color(Theme::accent_blue()))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(Theme::text_bright())
                        .child(spec.rel_path.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(Theme::text_dim())
                        .overflow_hidden()
                        .child(format!("{}  ↔  Working copy", spec.base_label)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .flex_shrink_0()
                .text_xs()
                .font_family(".AppleSystemUIFontMonospaced")
                .child(div().text_color(Theme::accent_green()).child(format!("+{added}")))
                .child(div().text_color(Theme::accent_red()).child(format!("−{removed}")))
                .child(
                    div()
                        .id("diff_refresh")
                        .size(px(22.0))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|h| h.bg(Theme::bg_hover()))
                        .on_click(move |_, window, cx| on_refresh(window, cx))
                        .child(icon(IconName::RotateCw).size(px(13.0)).text_color(Theme::text_muted())),
                ),
        );

    let body: AnyElement = match (&view.diff, &view.message) {
        (_, Some(msg)) => centered_message(msg.clone()),
        (Some(diff), None) if diff.rows.is_empty() => centered_message("No differences".to_string()),
        (Some(diff), None) => {
            let truncated = diff.rows.len() > MAX_ROWS;
            div()
                .id(SharedString::from(format!("diff_scroll_{}_{}", spec.rel_path, spec.base_rev)))
                .flex_1()
                .overflow_y_scroll()
                .track_scroll(&view.scroll_handle)
                .font_family(".AppleSystemUIFontMonospaced")
                .text_size(px(12.5))
                .children(diff.rows.iter().take(MAX_ROWS).map(render_row))
                .when(truncated, |d| {
                    d.child(skipped_row(format!("Diff truncated after {MAX_ROWS} lines")))
                })
                .into_any_element()
        }
        (None, None) => centered_message("Loading diff…".to_string()),
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(Theme::bg_editor())
        .child(header)
        .child(body)
}

fn centered_message(msg: String) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(Theme::text_dim())
        .child(msg)
        .into_any_element()
}

fn skipped_row(label: String) -> AnyElement {
    div()
        .py_1()
        .px_3()
        .bg(Theme::bg_panel())
        .border_y_1()
        .border_color(Theme::border_subtle())
        .text_xs()
        .text_color(Theme::text_dim())
        .child(label)
        .into_any_element()
}

fn line_number(n: Option<usize>) -> Div {
    div()
        .w(px(44.0))
        .flex_shrink_0()
        .pr_2()
        .flex()
        .justify_end()
        .text_color(Theme::line_num_inactive())
        .child(n.map(|n| n.to_string()).unwrap_or_default())
}

fn render_row(row: &DiffRow) -> AnyElement {
    let (kind, old_no, new_no, text, emphasis) = match row {
        DiffRow::Skipped(n) => {
            let noun = if *n == 1 { "line" } else { "lines" };
            return skipped_row(format!("⋯  {n} unchanged {noun}"));
        }
        DiffRow::Line { kind, old_no, new_no, text, emphasis } => (*kind, *old_no, *new_no, text, emphasis),
    };
    let (sign, row_bg, emph_bg, text_color) = match kind {
        DiffLineKind::Added => ("+", tint(Theme::accent_green(), 0.12), tint(Theme::accent_green(), 0.35), Theme::text_bright()),
        DiffLineKind::Removed => ("−", tint(Theme::accent_red(), 0.12), tint(Theme::accent_red(), 0.35), Theme::text_bright()),
        DiffLineKind::Context => (" ", gpui::transparent_black(), gpui::transparent_black(), Theme::text_secondary()),
    };
    let sign_color = match kind {
        DiffLineKind::Added => Theme::accent_green(),
        DiffLineKind::Removed => Theme::accent_red(),
        DiffLineKind::Context => Theme::text_dim(),
    };
    let highlights: Vec<_> = emphasis
        .iter()
        .map(|r| (r.clone(), HighlightStyle { background_color: Some(emph_bg), ..Default::default() }))
        .collect();
    // An empty StyledText collapses to zero height; keep blank lines one line tall.
    let shown = if text.is_empty() { " ".to_string() } else { text.clone() };

    div()
        .flex()
        .items_start()
        .py(px(1.0))
        .bg(row_bg)
        .child(line_number(old_no))
        .child(line_number(new_no))
        .child(div().w(px(18.0)).flex_shrink_0().text_color(sign_color).child(sign))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pr_3()
                .text_color(text_color)
                .child(StyledText::new(shown).with_highlights(highlights)),
        )
        .into_any_element()
}
