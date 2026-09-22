use crate::services::fs_utils::get_or_create_pdf_thumbnail;
use crate::icons::{icon, IconName};
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

#[allow(dead_code)]
pub fn render_pdf_preview(
    pdf_path: &str,
    on_open_external: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let path_str = pdf_path.to_string();
    let on_ext = on_open_external.clone();
    let filename = std::path::Path::new(pdf_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Document.pdf")
        .to_string();

    let thumb_img = get_or_create_pdf_thumbnail(pdf_path);

    div()
        .size_full()
        .bg(Theme::pdf_backing_canvas())
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_4()
        .p_6()
        .child(
            div()
                .w(px(520.0))
                .bg(Theme::bg_card())
                .border_1()
                .border_color(Theme::pdf_page_border())
                .rounded_xl()
                .shadow_lg()
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    if let Some(img_path) = thumb_img {
                        div()
                            .h(px(320.0))
                            .w_full()
                            .bg(Theme::bg_sidebar())
                            .overflow_hidden()
                            .flex()
                            .items_center()
                            .justify_center()
                            .p_3()
                            .child(
                                img(std::path::PathBuf::from(&img_path))
                                    .w_full()
                                    .h_full()
                                    .object_fit(gpui::ObjectFit::Contain),
                            )
                    } else {
                        div()
                            .h(px(140.0))
                            .w_full()
                            .bg(Theme::bg_sidebar())
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w(px(56.0))
                                    .h(px(56.0))
                                    .bg(Theme::bg_active())
                                    .rounded_full()
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .child(icon(IconName::FileText).size(px(26.0)).text_color(Theme::accent_red())),
                            )
                    }
                )
                .child(
                    div()
                        .p_5()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(Theme::text_bright())
                                        .child(filename),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(Theme::text_muted())
                                        .child(path_str.clone()),
                                ),
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
                                    on_ext(path_str.clone(), window, cx);
                                })
                                .child("Open in System PDF Viewer"),
                        ),
                ),
        )
}
