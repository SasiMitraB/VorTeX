use crate::services::synctex::SynctexForwardResult;
use crate::icons::{icon, IconName};
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

#[derive(Debug, Clone)]
pub struct SynctexHighlight {
    pub page: usize, // 1-based
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl From<SynctexForwardResult> for SynctexHighlight {
    fn from(r: SynctexForwardResult) -> Self {
        // Prefer box origin h/v if width/height present, else use x/y
        let (hx, hy) = if r.width > 0.0 && r.height > 0.0 {
            (r.h, r.v)
        } else {
            (r.x, r.y)
        };
        Self {
            page: r.page,
            x: hx,
            y: hy,
            width: if r.width > 0.0 { r.width } else { 80.0 },
            height: if r.height > 0.0 { r.height } else { 12.0 },
        }
    }
}

pub fn render_pdf_viewer(
    pdf_path: &str,
    page_images: &[String],
    page_count: usize,
    zoom: f32,
    highlight: Option<SynctexHighlight>,
    scroll_handle: &ScrollHandle,
    page_width_pts: f32,
    page_height_pts: f32,
    sidebar_visible: bool,
    has_right_pane: bool,
    is_right_pane: bool,
    on_zoom_in: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_zoom_out: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_zoom_reset: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_reload: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_open_external: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_inverse_request: impl Fn(String, usize, f32, f32, &mut Window, &mut App) + 'static + Clone,
    is_rendering: bool,
) -> impl IntoElement {
    let pdf_path_owned = pdf_path.to_string();
    let filename = std::path::Path::new(pdf_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Document.pdf")
        .to_string();

    let display_w = page_width_pts * zoom;
    let display_h = page_height_pts * zoom;
    let gap = 16.0;

    let has_pages = !page_images.is_empty() && page_count > 0;
    let scroll_handle_clone = scroll_handle.clone();
    let zoom_pct = (zoom * 100.0).round() as i32;

    let highlight_clone = highlight.clone();
    let page_width_pts_c = page_width_pts;
    let page_height_pts_c = page_height_pts;
    let pdf_path_for_click = pdf_path_owned.clone();

    // Precompute target scroll offset if highlight exists (for auto-scroll button)
    let highlight_page = highlight.as_ref().map(|h| h.page);

    div()
        .size_full()
        .bg(Theme::pdf_backing_canvas())
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            // Viewer toolbar HUD
            div()
                .h(px(34.0))
                .w_full()
                .bg(Theme::bg_titlebar())
                .border_b_1()
                .border_color(Theme::border_subtle())
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(Theme::text_bright())
                                .child(filename.clone()),
                        )
                        .child(
                            div()
                                .text_color(Theme::border_subtle())
                                .child("•"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .text_color(Theme::text_dim())
                                .child(format!("{} pages", page_count)),
                        )
                        .when_some(highlight_page, |d, p| {
                            d.child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .bg(Theme::bg_card())
                                    .rounded_full()
                                    .text_xs()
                                    .font_family(".AppleSystemUIFontMonospaced")
                                    .text_color(Theme::accent_teal())
                                    .child(format!("100% Synced (p.{})", p)),
                            )
                        })
                        .when(is_rendering, |d| {
                            d.child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .bg(Theme::accent_yellow())
                                    .rounded_full()
                                    .text_xs()
                                    .text_color(Theme::text_inverted())
                                    .child("Rendering..."),
                            )
                        }),
                )
                .child(
                    // Floating glass zoom controls & quick action icons
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(viewer_btn(IconName::Minus, on_zoom_out.clone()))
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .bg(Theme::bg_card())
                                .border_1()
                                .border_color(Theme::border_subtle())
                                .rounded_md()
                                .text_xs()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .text_color(Theme::accent_blue())
                                .font_weight(FontWeight::MEDIUM)
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                    on_zoom_reset(window, cx);
                                })
                                .child(format!("{}%", zoom_pct)),
                        )
                        .child(viewer_btn(IconName::Plus, on_zoom_in.clone()))
                        .child(
                            div().w(px(1.0)).h(px(14.0)).bg(Theme::border_subtle()).mx_1()
                        )
                        .child(viewer_btn(IconName::RotateCw, on_reload.clone()))
                        .child(viewer_btn(IconName::ExternalLink, on_open_external.clone())),
                ),
        )
        .child(if has_pages {
            div()
                .id(SharedString::from(format!(
                    "pdf_viewer_scroll_{}_{}",
                    pdf_path,
                    if is_right_pane { "right" } else { "left" }
                )))
                .flex_1()
                .size_full()
                .bg(Theme::pdf_backing_canvas())
                .overflow_y_scroll()
                .track_scroll(scroll_handle)
                .p_4()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(gap))
                        .pb_8()
                        .children(page_images.iter().enumerate().map(|(idx, img_path)| {
                            let page_num = idx + 1;
                            let is_highlight_page = highlight_clone.as_ref().map(|h| h.page == page_num).unwrap_or(false);
                            let hl = highlight_clone.clone();
                            let pdf_path_click = pdf_path_for_click.clone();
                            let on_inv_req = on_inverse_request.clone();
                            let scroll_h = scroll_handle_clone.clone();

                            let page_disp_w = display_w;
                            let page_disp_h = display_h;
                            let pw_pts = page_width_pts_c;
                            let ph_pts = page_height_pts_c;

                            // Highlight rect display coords (points * zoom)
                            let hl_rect = if is_highlight_page {
                                hl.as_ref().map(|h| {
                                    let left = h.x * zoom;
                                    let top = h.y * zoom;
                                    let w = h.width * zoom;
                                    let hgt = h.height * zoom;
                                    (left, top, w, hgt)
                                })
                            } else {
                                None
                            };

                            let img_path_buf = std::path::PathBuf::from(img_path);
                            let page_img_exists = img_path_buf.is_file();

                            div()
                                .relative()
                                .w(px(page_disp_w))
                                .h(px(page_disp_h))
                                .bg(Theme::bg_card())
                                .border_1()
                                .border_color(Theme::pdf_page_border())
                                .rounded_md()
                                .shadow_md()
                                .overflow_hidden()
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, window, cx| {
                                    // Inverse SyncTeX: click inside page -> source
                                    // Compute click offset within page
                                    // We use window size + known layout geometry to estimate page origin
                                    let mouse_x = f32::from(event.position.x);
                                    let mouse_y = f32::from(event.position.y);

                                    let win_size = window.viewport_size();
                                    let win_w = f32::from(win_size.width);
                                    // let win_h = f32::from(win_size.height);

                                    let sidebar_w = if sidebar_visible { 296.0 } else { 0.0 };
                                    let pane_w = if has_right_pane {
                                        (win_w - sidebar_w).max(200.0) / 2.0
                                    } else {
                                        (win_w - sidebar_w).max(200.0)
                                    };
                                    // Viewer toolbar + app toolbar + tab bar heights before scroll container
                                    let chrome_top = 46.0 + 34.0 + 36.0; // app toolbar 46, tab bar 34, viewer toolbar 36

                                    // Scroll offset (positive scrolled amount)
                                    let scroll_y = (-f32::from(scroll_h.offset().y)).max(0.0);

                                    // Page origin within window
                                    // Scroll container top is at chrome_top
                                    // Each page: index * (h+gap) + padding top 16
                                    let padding_top = 16.0;
                                    let page_offset_in_scroll = (idx as f32) * (page_disp_h + gap) + padding_top;
                                    let page_top_in_window = chrome_top + page_offset_in_scroll - scroll_y;

                                    // Horizontal centering: page is centered within its pane
                                    let pane_left = if is_right_pane {
                                        sidebar_w + pane_w + 2.0 // divider 2px
                                    } else {
                                        sidebar_w
                                    };
                                    // Scroll container has p_4 (16) padding on both sides, but centered pages ignore that
                                    // Compute page left as pane_left + (pane_w - page_disp_w)/2
                                    let page_left_in_window = pane_left + ((pane_w - page_disp_w) / 2.0).max(16.0);

                                    let x_in_page = mouse_x - page_left_in_window;
                                    let y_in_page = mouse_y - page_top_in_window;

                                    // Clamp to page bounds
                                    if x_in_page < 0.0 || x_in_page > page_disp_w || y_in_page < 0.0 || y_in_page > page_disp_h {
                                        // Click outside this page (maybe gap) - ignore
                                        return;
                                    }

                                    // Convert display px to PDF points: point = display / zoom
                                    let pdf_x = x_in_page / zoom;
                                    let pdf_y = y_in_page / zoom;

                                    // Clamp to page dims
                                    let pdf_x = pdf_x.clamp(0.0, pw_pts);
                                    let pdf_y = pdf_y.clamp(0.0, ph_pts);

                                    // Async inverse search via main thread (non-blocking)
                                    on_inv_req(pdf_path_click.clone(), page_num, pdf_x, pdf_y, window, cx);
                                })
                                .child(if page_img_exists {
                                    div()
                                        .w_full()
                                        .h_full()
                                        .child(
                                            img(img_path_buf)
                                                .w_full()
                                                .h_full()
                                                .object_fit(ObjectFit::Contain)
                                        )
                                        .into_any_element()
                                } else {
                                    div()
                                        .w_full()
                                        .h_full()
                                        .bg(Theme::bg_panel())
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_xs()
                                        .text_color(Theme::text_muted())
                                        .child(format!("Page {}", page_num))
                                        .into_any_element()
                                })
                                .when_some(hl_rect, |d, (left, top, w, hgt)| {
                                    d.child(
                                        div()
                                            .absolute()
                                            .left(px(left))
                                            .top(px(top))
                                            .w(px(w.max(20.0)))
                                            .h(px(hgt.max(8.0)))
                                            .bg(Theme::pdf_synctex_highlight())
                                            .border_1()
                                            .border_color(Theme::accent_blue())
                                            .rounded_sm()
                                    )
                                })
                                .child(
                                    // Page number badge
                                    div()
                                        .absolute()
                                        .bottom_2()
                                        .right_2()
                                        .px_2()
                                        .py_1()
                                        .bg(hsla(0.0, 0.0, 0.0, 0.55))
                                        .rounded_sm()
                                        .text_xs()
                                        .text_color(Theme::text_bright())
                                        .child(format!("{}", page_num))
                                )
                        }))
                )
                .into_any_element()
        } else {
            // Empty / loading state
            div()
                .flex_1()
                .w_full()
                .bg(Theme::pdf_backing_canvas())
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
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
                .child(
                    div()
                        .text_sm()
                        .text_color(Theme::text_dim())
                        .child(if is_rendering {
                            "Rendering PDF pages... (background)"
                        } else if page_count == 0 {
                            "No PDF generated yet. Build to preview."
                        } else {
                            "PDF not rendered. Click Retry."
                        }),
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
                            on_reload(window, cx);
                        })
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .child(icon(IconName::RotateCw).size(px(12.0)).text_color(Theme::text_inverted()))
                        .child("Retry Render"),
                )
                .into_any_element()
        })
}

fn viewer_btn(
    btn_icon: IconName,
    on_click: impl Fn(&mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    div()
        .size(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(Theme::bg_card())
        .hover(|h| h.bg(Theme::bg_hover()))
        .border_1()
        .border_color(Theme::border_subtle())
        .rounded_md()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
            on_click(window, cx);
        })
        .child(icon(btn_icon).size(px(13.0)).text_color(Theme::text_primary()))
}
