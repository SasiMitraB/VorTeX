use crate::backend::ProjectItem;
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

pub fn render_project_selector(
    projects: &[ProjectItem],
    projects_folder: Option<&str>,
    scroll_handle: &ScrollHandle,
    on_select_project: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_change_folder: impl Fn(&mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let on_ch = on_change_folder.clone();

    div()
        .id("project_selector_scroll")
        .size_full()
        .bg(Theme::bg_app())
        .flex()
        .flex_col()
        .items_center()
        .p_8()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .child(
            // Header Section
            div()
                .w_full()
                .max_w(px(900.0))
                .flex()
                .flex_col()
                .gap_4()
                .mb_8()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .w(px(40.0))
                                        .h(px(40.0))
                                        .bg(Theme::accent_blue())
                                        .rounded_lg()
                                        .flex()
                                        .justify_center()
                                        .items_center()
                                        .text_xl()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(Theme::bg_titlebar())
                                        .child("V"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_2xl()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(Theme::text_bright())
                                                .child("VorTeX"),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(Theme::text_muted())
                                                .child("GPU-Accelerated Modern LaTeX Editor"),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .px_4()
                                .py_2()
                                .bg(Theme::bg_card())
                                .hover(|h| h.bg(Theme::bg_hover()))
                                .border_1()
                                .border_color(Theme::border_subtle())
                                .rounded_md()
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .gap_2()
                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                    on_ch(window, cx);
                                })
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(Theme::accent_blue())
                                        .child("📁"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(Theme::text_bright())
                                        .child("Change Projects Folder"),
                                ),
                        ),
                )
                .child(
                    // Folder location label pill
                    div()
                        .px_3()
                        .py_1p5()
                        .bg(Theme::bg_sidebar())
                        .border_1()
                        .border_color(Theme::border_subtle())
                        .rounded_md()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(Theme::accent_yellow())
                                .child("📂"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(Theme::text_muted())
                                .child(projects_folder.unwrap_or("No projects folder selected").to_string()),
                        ),
                ),
        )
        .child(
            // Projects Grid
            div()
                .w_full()
                .max_w(px(900.0))
                .children(if projects.is_empty() {
                    let on_ch2 = on_change_folder.clone();
                    vec![
                        div()
                            .w_full()
                            .py_16()
                            .bg(Theme::bg_card())
                            .border_1()
                            .border_color(Theme::border_subtle())
                            .rounded_xl()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_4()
                            .child(
                                div()
                                    .text_2xl()
                                    .text_color(Theme::text_dim())
                                    .child("📁"),
                            )
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(Theme::text_bright())
                                    .child("No LaTeX projects found in this folder"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(Theme::text_muted())
                                    .child("Select a directory containing your LaTeX project folders"),
                            )
                            .child(
                                div()
                                    .px_4()
                                    .py_2()
                                    .bg(Theme::accent_blue())
                                    .hover(|h| h.bg(hsla(207.0 / 360.0, 0.82, 0.75, 1.0)))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(Theme::bg_titlebar())
                                    .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                        on_ch2(window, cx);
                                    })
                                    .child("Choose Different Folder"),
                            )
                            .into_any_element(),
                    ]
                } else {
                    vec![
                        div()
                            .grid()
                            .w_full()
                            .gap_4()
                            .flex()
                            .flex_wrap()
                            .children(projects.iter().map(|proj| {
                                let proj_path = proj.path.clone();
                                let on_sel = on_select_project.clone();
                                let has_pdf = proj.preview_pdf.is_some();
                                let preview_image = proj.preview_image.clone();

                                div()
                                    .w(px(280.0))
                                    .bg(Theme::bg_card())
                                    .border_1()
                                    .border_color(Theme::border_subtle())
                                    .hover(|h| h.border_color(Theme::accent_blue()).bg(Theme::bg_hover()))
                                    .rounded_xl()
                                    .overflow_hidden()
                                    .cursor_pointer()
                                    .shadow_md()
                                    .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                        on_sel(proj_path.clone(), window, cx);
                                    })
                                    .child(
                                        // Preview thumbnail area
                                        if let Some(img_path) = preview_image {
                                            div()
                                                .h(px(175.0))
                                                .w_full()
                                                .bg(Theme::bg_sidebar())
                                                .relative()
                                                .overflow_hidden()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .child(
                                                    img(img_path)
                                                        .w_full()
                                                        .h_full()
                                                        .object_fit(gpui::ObjectFit::Cover),
                                                )
                                                .child(
                                                    div()
                                                        .absolute()
                                                        .top_2()
                                                        .right_2()
                                                        .px_2()
                                                        .py_0p5()
                                                        .bg(hsla(220.0 / 360.0, 0.2, 0.1, 0.85))
                                                        .border_1()
                                                        .border_color(hsla(0.0, 0.0, 1.0, 0.15))
                                                        .rounded_md()
                                                        .text_xs()
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(Theme::text_bright())
                                                        .child("PDF Preview"),
                                                )
                                        } else {
                                            div()
                                                .h(px(175.0))
                                                .w_full()
                                                .bg(Theme::bg_sidebar())
                                                .flex()
                                                .flex_col()
                                                .items_center()
                                                .justify_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .w(px(48.0))
                                                        .h(px(48.0))
                                                        .rounded_full()
                                                        .bg(Theme::bg_card())
                                                        .border_1()
                                                        .border_color(Theme::border_subtle())
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .text_2xl()
                                                        .text_color(if has_pdf { Theme::accent_red() } else { Theme::accent_blue() })
                                                        .child(if has_pdf { "📄" } else { "λ" }),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(Theme::text_muted())
                                                        .child(if has_pdf { "PDF Available" } else { "LaTeX Project" }),
                                                )
                                        },
                                    )
                                    .child(
                                        // Info section
                                        div()
                                            .p_3()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_color(Theme::text_bright())
                                                            .child(proj.name.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(if has_pdf { Theme::accent_green() } else { Theme::text_dim() })
                                                            .child(if has_pdf { "● PDF" } else { "● Tex" }),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(Theme::text_dim())
                                                    .child(proj.path.clone()),
                                            ),
                                    )
                            }))
                            .into_any_element(),
                    ]
                }),
        )
}
