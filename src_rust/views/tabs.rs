use crate::state::{PaneSide, Tab, TabType};
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

pub fn render_tab_bar(
    tabs: &[Tab],
    active_tab_id: Option<&str>,
    pane_side: PaneSide,
    on_switch_tab: impl Fn(String, PaneSide, &mut Window, &mut App) + 'static + Clone,
    on_close_tab: impl Fn(String, PaneSide, &mut Window, &mut App) + 'static + Clone,
    on_new_tab: impl Fn(PaneSide, &mut Window, &mut App) + 'static + Clone,
    on_split_pane: impl Fn(&mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let on_new = on_new_tab.clone();
    let on_split = on_split_pane.clone();

    div()
        .h(px(32.0))
        .bg(Theme::bg_tab_bar())
        .border_b_1()
        .border_color(Theme::border_subtle())
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .h_full()
                .overflow_hidden()
                .children(tabs.iter().map(|tab| {
                    let is_active = active_tab_id == Some(&tab.id);
                    let tab_id = tab.id.clone();
                    let tab_id_close = tab.id.clone();
                    let on_sw = on_switch_tab.clone();
                    let on_cl = on_close_tab.clone();

                    let icon = match tab.tab_type {
                        TabType::Text => "λ",
                        TabType::Pdf => "📄",
                    };

                    div()
                        .h_full()
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .border_r_1()
                        .border_color(Theme::border_subtle())
                        .bg(if is_active { Theme::bg_tab_active() } else { Theme::bg_tab_inactive() })
                        .when(is_active, |d| d.border_b_2().border_color(Theme::border_focus()))
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_sw(tab_id.clone(), pane_side, window, cx);
                        })
                        .child(
                            div()
                                .text_xs()
                                .text_color(if is_active { Theme::accent_blue() } else { Theme::text_dim() })
                                .child(icon),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(if is_active { Theme::text_bright() } else { Theme::text_primary() })
                                .child(format!("{}{}", tab.name, if tab.dirty { " •" } else { "" })),
                        )
                        .child(
                            div()
                                .w(px(16.0))
                                .h(px(16.0))
                                .rounded_sm()
                                .flex()
                                .justify_center()
                                .items_center()
                                .hover(|h| h.bg(Theme::bg_hover()))
                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                    on_cl(tab_id_close.clone(), pane_side, window, cx);
                                })
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(Theme::text_muted())
                                        .child("×"),
                                ),
                        )
                }))
                .child(
                    // New Tab Button
                    div()
                        .h_full()
                        .px_2()
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .hover(|h| h.bg(Theme::bg_hover()))
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_new(pane_side, window, cx);
                        })
                        .child(
                            div()
                                .text_xs()
                                .text_color(Theme::text_muted())
                                .child("+"),
                        ),
                ),
        )
        .child(
            // Split view toggle button
            div()
                .px_2()
                .cursor_pointer()
                .hover(|h| h.bg(Theme::bg_hover()))
                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                    on_split(window, cx);
                })
                .child(
                    div()
                        .text_xs()
                        .text_color(Theme::text_dim())
                        .child("◫ Split"),
                ),
        )
}
