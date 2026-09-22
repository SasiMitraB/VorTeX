use crate::icons::{icon, IconName};
use crate::services::git::Commit;
use crate::state::LatexDiffDialog;
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

/// Commit picker for the latexdiff PDF: old version on the left, new on the right.
pub fn render_latexdiff_dialog(
    dialog: &LatexDiffDialog,
    on_select_old: impl Fn(usize, &mut Window, &mut App) + 'static + Clone,
    on_select_new: impl Fn(Option<usize>, &mut Window, &mut App) + 'static + Clone,
    on_generate: impl Fn(&mut Window, &mut App) + 'static,
    on_close: impl Fn(&mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let on_close_x = on_close.clone();
    let can_generate = !dialog.running && dialog.old.is_some() && dialog.main_rel.is_some();

    let header = div()
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(Theme::border_subtle())
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(icon(IconName::FileDiff).size(px(16.0)).text_color(Theme::accent_blue()))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(Theme::text_bright())
                        .child("Compare Versions"),
                )
                .when_some(dialog.main_rel.clone(), |d, main| {
                    d.child(div().min_w_0().truncate().text_xs().text_color(Theme::text_dim()).child(main))
                }),
        )
        .child(
            div()
                .id("latexdiff_close")
                .size(px(22.0))
                .flex_shrink_0()
                .rounded_md()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|h| h.bg(Theme::bg_hover()))
                .on_click(move |_, window, cx| on_close_x(window, cx))
                .child(icon(IconName::X).size(px(13.0)).text_color(Theme::text_muted())),
        );

    let body: AnyElement = if dialog.loading {
        note("Loading history…")
    } else if dialog.repo_root.is_none() {
        note("This project is not a git repository.")
    } else if dialog.main_rel.is_none() {
        note("No main .tex document (with \\documentclass) found in this project.")
    } else if dialog.commits.is_empty() {
        note("This project has no commits yet.")
    } else {
        let old_rows = dialog.commits.iter().enumerate().map(|(i, c)| {
            // The old version must be older than the new one (commits are newest first).
            let enabled = dialog.new.map_or(true, |n| i > n);
            let on_sel = on_select_old.clone();
            commit_row(("latexdiff_old", i), c, dialog.old == Some(i), enabled, move |w, cx| on_sel(i, w, cx))
        });
        let on_wc = on_select_new.clone();
        let working_copy = option_row(
            ("latexdiff_new_wc", 0),
            "Working copy",
            "Current files, including uncommitted changes",
            dialog.new.is_none(),
            true,
            move |w, cx| on_wc(None, w, cx),
        );
        let new_rows = dialog.commits.iter().enumerate().map(|(i, c)| {
            let enabled = dialog.old.map_or(true, |o| i < o);
            let on_sel = on_select_new.clone();
            commit_row(("latexdiff_new", i), c, dialog.new == Some(i), enabled, move |w, cx| on_sel(Some(i), w, cx))
        });

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .child(column("OLD VERSION", "latexdiff_old_scroll", &dialog.old_scroll, old_rows.collect()))
            .child(div().w(px(1.0)).bg(Theme::border_subtle()))
            .child(column(
                "NEW VERSION",
                "latexdiff_new_scroll",
                &dialog.new_scroll,
                std::iter::once(working_copy).chain(new_rows).collect(),
            ))
            .into_any_element()
    };

    let status: AnyElement = if dialog.running {
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .text_xs()
            .text_color(Theme::text_muted())
            .child(icon(IconName::LoaderCircle).size(px(12.0)).text_color(Theme::accent_blue()))
            .child("Running latexdiff and compiling…")
            .into_any_element()
    } else if let Some(err) = dialog.error.clone() {
        div().text_xs().text_color(Theme::accent_red()).child(err).into_any_element()
    } else {
        div()
            .text_xs()
            .text_color(Theme::text_dim())
            .child("Additions are underlined in blue, deletions struck out in red. Only the PDF is saved.")
            .into_any_element()
    };

    let footer = div()
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .border_t_1()
        .border_color(Theme::border_subtle())
        .child(div().flex_1().min_w_0().child(status))
        .child(
            div()
                .flex()
                .flex_shrink_0()
                .gap_2()
                .child(
                    div()
                        .id("latexdiff_cancel")
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .cursor_pointer()
                        .text_xs()
                        .text_color(Theme::text_secondary())
                        .hover(|h| h.bg(Theme::bg_hover()))
                        .on_click(move |_, window, cx| on_close(window, cx))
                        .child(if dialog.running { "Close" } else { "Cancel" }),
                )
                .child(
                    div()
                        .id("latexdiff_generate")
                        .px_4()
                        .py_1()
                        .rounded_md()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .when(can_generate, |d| {
                            d.bg(Theme::accent_blue())
                                .text_color(Theme::text_inverted())
                                .cursor_pointer()
                                .hover(|h| h.bg(Theme::border_focus()))
                                .on_click(move |_, window, cx| on_generate(window, cx))
                        })
                        .when(!can_generate, |d| d.bg(Theme::bg_hover()).text_color(Theme::text_dim()))
                        .child("Generate PDF"),
                ),
        );

    div()
        .id("latexdiff_backdrop")
        .absolute()
        .inset_0()
        .occlude()
        .bg(Theme::modal_backdrop())
        .flex()
        .justify_center()
        .items_center()
        .child(
            div()
                .w(px(760.0))
                .h(px(540.0))
                .bg(Theme::bg_modal())
                .border_1()
                .border_color(Theme::border_subtle())
                .rounded_lg()
                .shadow_lg()
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(header)
                .child(body)
                .child(footer),
        )
}

fn note(msg: &str) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .px_6()
        .text_sm()
        .text_color(Theme::text_dim())
        .child(msg.to_string())
        .into_any_element()
}

fn column(title: &'static str, id: &'static str, scroll: &ScrollHandle, rows: Vec<AnyElement>) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .pt_3()
                .pb_1p5()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(Theme::text_muted())
                .child(title),
        )
        .child(
            div()
                .id(id)
                .flex_1()
                .overflow_y_scroll()
                .track_scroll(scroll)
                .px_2()
                .pb_2()
                .children(rows),
        )
}

fn commit_row(
    id: (&'static str, usize),
    commit: &Commit,
    selected: bool,
    enabled: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> AnyElement {
    option_row(
        id,
        &commit.subject,
        &format!("{} · {} · {}", commit.short_hash, commit.author, commit.relative_date),
        selected,
        enabled,
        on_click,
    )
}

fn option_row(
    id: (&'static str, usize),
    title: &str,
    detail: &str,
    selected: bool,
    enabled: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(ElementId::NamedInteger(id.0.into(), id.1 as u64))
        .px_2()
        .py_1p5()
        .mb_0p5()
        .rounded_md()
        .flex()
        .items_center()
        .gap_2()
        .border_1()
        .border_color(if selected { Theme::accent_blue() } else { gpui::transparent_black() })
        .when(selected, |d| d.bg(Theme::bg_active()))
        .when(!enabled, |d| d.opacity(0.35))
        .when(enabled && !selected, |d| d.cursor_pointer().hover(|h| h.bg(Theme::bg_hover())))
        .when(enabled, |d| d.on_click(move |_, window, cx| on_click(window, cx)))
        .child(
            div()
                .size(px(12.0))
                .flex_shrink_0()
                .rounded_full()
                .border_1()
                .border_color(if selected { Theme::accent_blue() } else { Theme::text_dim() })
                .flex()
                .items_center()
                .justify_center()
                .when(selected, |d| d.child(div().size(px(6.0)).rounded_full().bg(Theme::accent_blue()))),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .text_xs()
                        .text_color(if selected { Theme::text_bright() } else { Theme::text_primary() })
                        .child(title.to_string()),
                )
                .child(div().truncate().text_xs().text_color(Theme::text_dim()).child(detail.to_string())),
        )
        .into_any_element()
}
