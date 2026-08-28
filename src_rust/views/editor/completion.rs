use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

pub const ITEM_HEIGHT: f32 = 28.0;
pub const LIST_HEIGHT: f32 = 196.0;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CompletionKind {
    Reference,
    Citation,
    Environment,
    Command,
    File,
    Snippet,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: String,
}

#[derive(Clone)]
pub struct CompletionState {
    pub is_open: bool,
    pub items: Vec<CompletionItem>,
    pub selected_index: usize,
    pub trigger_row: usize,
    pub trigger_col: usize,
    pub query: String,
    pub scroll_handle: ScrollHandle,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self {
            is_open: false,
            items: Vec::new(),
            selected_index: 0,
            trigger_row: 0,
            trigger_col: 0,
            query: String::new(),
            scroll_handle: ScrollHandle::new(),
        }
    }
}

impl std::fmt::Debug for CompletionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompletionState")
            .field("is_open", &self.is_open)
            .field("items_count", &self.items.len())
            .field("selected_index", &self.selected_index)
            .field("trigger_row", &self.trigger_row)
            .field("trigger_col", &self.trigger_col)
            .field("query", &self.query)
            .finish()
    }
}

impl CompletionState {
    pub fn open(&mut self, items: Vec<CompletionItem>, row: usize, col: usize, query: String) {
        self.is_open = !items.is_empty();
        self.items = items;
        self.selected_index = 0;
        self.trigger_row = row;
        self.trigger_col = col;
        self.query = query;
        self.scroll_handle.set_offset(point(px(0.0), px(0.0)));
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.items.clear();
        self.selected_index = 0;
        self.query.clear();
        self.scroll_handle.set_offset(point(px(0.0), px(0.0)));
    }

    pub fn ensure_selected_visible(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let item_top = self.selected_index as f32 * ITEM_HEIGHT;
        let item_bottom = item_top + ITEM_HEIGHT;
        let current_scroll_y = (-f32::from(self.scroll_handle.offset().y)).max(0.0);

        if self.selected_index == 0 {
            self.scroll_handle.set_offset(point(px(0.0), px(0.0)));
        } else if self.selected_index == self.items.len() - 1 {
            let max_scroll = ((self.items.len() as f32 * ITEM_HEIGHT) - LIST_HEIGHT).max(0.0);
            self.scroll_handle.set_offset(point(px(0.0), px(-max_scroll)));
        } else if item_bottom > current_scroll_y + LIST_HEIGHT {
            let new_scroll_y = item_bottom - LIST_HEIGHT;
            self.scroll_handle.set_offset(point(px(0.0), px(-new_scroll_y)));
        } else if item_top < current_scroll_y {
            let new_scroll_y = item_top;
            self.scroll_handle.set_offset(point(px(0.0), px(-new_scroll_y)));
        }
    }

    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
            self.ensure_selected_visible();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.items.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.items.len() - 1;
            } else {
                self.selected_index -= 1;
            }
            self.ensure_selected_visible();
        }
    }

    pub fn current_item(&self) -> Option<&CompletionItem> {
        self.items.get(self.selected_index)
    }
}

pub fn render_completion_popup(
    state: &CompletionState,
    cursor_x: Pixels,
    cursor_y: Pixels,
    on_select: impl Fn(CompletionItem, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let selected_idx = state.selected_index;
    let items = state.items.clone();
    let scroll_handle = state.scroll_handle.clone();

    div()
        .absolute()
        .top(cursor_y + px(20.0))
        .left(cursor_x)
        .w(px(380.0))
        .max_h(px(240.0))
        .bg(Theme::bg_modal())
        .border_1()
        .border_color(Theme::border_subtle())
        .rounded_md()
        .shadow_lg()
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            // Header showing count and hint
            div()
                .h(px(28.0))
                .px_2()
                .py_1()
                .bg(Theme::bg_titlebar())
                .border_b_1()
                .border_color(Theme::border_subtle())
                .flex()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_xs()
                        .text_color(Theme::text_muted())
                        .child(format!("{} suggestions", items.len())),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(Theme::text_dim())
                        .child("Tab / ↵ to insert"),
                ),
        )
        .child(
            // Suggestion list
            div()
                .id("completion_list_scroll")
                .max_h(px(LIST_HEIGHT))
                .overflow_y_scroll()
                .track_scroll(&scroll_handle)
                .children(items.iter().enumerate().map(|(idx, item)| {
                    let is_selected = idx == selected_idx;
                    let item_clone = item.clone();
                    let on_sel = on_select.clone();

                    let (icon, icon_color) = match item.kind {
                        CompletionKind::Reference => ("⚑", Theme::accent_blue()),
                        CompletionKind::Citation => ("❝", Theme::accent_purple()),
                        CompletionKind::Environment => ("⚡", Theme::accent_yellow()),
                        CompletionKind::Command => ("λ", Theme::accent_cyan()),
                        CompletionKind::File => ("📄", Theme::accent_green()),
                        CompletionKind::Snippet => ("✦", Theme::accent_orange()),
                    };

                    div()
                        .h(px(ITEM_HEIGHT))
                        .px_2()
                        .py_1()
                        .flex()
                        .items_center()
                        .justify_between()
                        .cursor_pointer()
                        .when(is_selected, |d| d.bg(Theme::bg_active()))
                        .when(!is_selected, |d| d.hover(|h| h.bg(Theme::bg_hover())))
                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                            on_sel(item_clone.clone(), window, cx);
                        })
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(icon_color)
                                        .child(icon),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(if is_selected {
                                            Theme::text_bright()
                                        } else {
                                            Theme::text_primary()
                                        })
                                        .font_weight(if is_selected {
                                            FontWeight::SEMIBOLD
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .child(item.label.clone()),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(Theme::text_dim())
                                .child(item.detail.clone().unwrap_or_default()),
                        )
                })),
        )
}

