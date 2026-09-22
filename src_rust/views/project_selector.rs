use crate::backend::ProjectItem;
use crate::icons::{icon, IconName};
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
                                        .text_color(Theme::text_inverted())
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
                                .flex()
                                .items_center()
                                .gap_3()
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
                                        .child(icon(IconName::FolderSearch).size(px(15.0)).text_color(Theme::accent_blue()))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(Theme::text_primary())
                                                .child(if projects_folder.is_some() {
                                                    "Change Projects Folder"
                                                } else {
                                                    "Choose Projects Folder"
                                                }),
                                        ),
                                ),
                        ),
                )
                .child(
                    if let Some(folder) = projects_folder {
                        div()
                            .text_xs()
                            .text_color(Theme::text_dim())
                            .child(format!("Scanning: {}", folder))
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    },
                ),
        )
        .child(
            // Projects Grid or Empty State
            div()
                .w_full()
                .max_w(px(900.0))
                .flex()
                .flex_col()
                .gap_4()
                .children(if projects_folder.is_none() {
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
                                icon(IconName::FolderSearch).size(px(36.0)).text_color(Theme::accent_blue()),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(Theme::text_bright())
                                    .child("Choose your LaTeX Projects Directory"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(Theme::text_muted())
                                    .child("Select a parent directory containing all your LaTeX project folders"),
                            )
                            .child(
                                div()
                                    .px_4()
                                    .py_2()
                                    .bg(Theme::accent_blue())
                                    .hover(|h| h.bg(Theme::border_focus()))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(Theme::text_inverted())
                                    .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                        on_ch2(window, cx);
                                    })
                                    .child("Select Folder"),
                            )
                            .into_any_element(),
                    ]
                } else if projects.is_empty() {
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
                                icon(IconName::Folder).size(px(30.0)).text_color(Theme::text_dim()),
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
                                    .hover(|h| h.bg(Theme::border_focus()))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(Theme::text_inverted())
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
                                    .flex_1()
                                    .min_w(px(220.0))
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
                                                .w_full()
                                                .h(px(210.0))
                                                .bg(Theme::bg_sidebar())
                                                .relative()
                                                .overflow_hidden()
                                                .child(
                                                    img(std::path::PathBuf::from(&img_path))
                                                        .absolute()
                                                        .top_0()
                                                        .left_0()
                                                        .w_full(),
                                                )
                                                .child(
                                                    div()
                                                        .absolute()
                                                        .top_2()
                                                        .right_2()
                                                        .px_2()
                                                        .py_0p5()
                                                        .bg(Theme::modal_backdrop())
                                                        .border_1()
                                                        .border_color(Theme::border_subtle())
                                                        .rounded_md()
                                                        .text_xs()
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(Theme::text_bright())
                                                        .child("PDF Preview"),
                                                )
                                        } else {
                                            div()
                                                .w_full()
                                                .h(px(175.0))
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
                                                        .child(
                                                            icon(if has_pdf { IconName::FileText } else { IconName::FileCode })
                                                                .size(px(22.0))
                                                                .text_color(if has_pdf { Theme::accent_red() } else { Theme::accent_blue() }),
                                                        ),
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
                                                            .flex()
                                                            .items_center()
                                                            .gap_1()
                                                            .text_xs()
                                                            .text_color(if has_pdf { Theme::accent_green() } else { Theme::text_dim() })
                                                            .child(
                                                                icon(if has_pdf { IconName::CircleCheck } else { IconName::FileCode })
                                                                    .size(px(12.0))
                                                                    .text_color(if has_pdf { Theme::accent_green() } else { Theme::text_dim() }),
                                                            )
                                                            .child(if has_pdf { "PDF" } else { "TeX" }),
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
