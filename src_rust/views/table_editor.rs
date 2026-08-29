#![allow(dead_code)]

use crate::services::table_parser::{
    generate_latex_table, parse_latex_table, ColAlign, TableModel,
};
use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;
use std::ops::Range;

#[derive(Clone)]
pub struct TableSpreadsheet {
    pub model: TableModel,
    pub is_open: bool,
    pub active_cell: Option<(usize, usize)>,
    pub editing_cell: Option<(usize, usize)>,
    pub edit_buffer: String,
    pub caption_buffer: String,
    pub label_buffer: String,
    pub selected_range: Option<((usize, usize), (usize, usize))>,
    pub status_msg: Option<String>,
}

impl Default for TableSpreadsheet {
    fn default() -> Self {
        let model = TableModel::default();
        let caption_buffer = model.caption.clone().unwrap_or_default();
        let label_buffer = model.label.clone().unwrap_or_default();

        Self {
            model,
            is_open: false,
            active_cell: Some((0, 0)),
            editing_cell: None,
            edit_buffer: String::new(),
            caption_buffer,
            label_buffer,
            selected_range: None,
            status_msg: None,
        }
    }
}

impl TableSpreadsheet {
    pub fn default_clone(&self) -> Self {
        self.clone()
    }

    pub fn load_from_latex(&mut self, source: &str, span: Option<Range<usize>>, file: Option<String>) {
        if let Some(mut parsed) = parse_latex_table(source) {
            parsed.source_span = span;
            parsed.source_file = file;
            self.caption_buffer = parsed.caption.clone().unwrap_or_default();
            self.label_buffer = parsed.label.clone().unwrap_or_default();
            self.model = parsed;
            self.active_cell = Some((0, 0));
            self.editing_cell = None;
            self.edit_buffer.clear();
            self.selected_range = None;
            self.status_msg = Some("Loaded table from source".to_string());
        } else {
            *self = Self::default();
            self.model.source_span = span;
            self.model.source_file = file;
            self.status_msg = Some("Created new blank table".to_string());
        }
    }

    pub fn add_row(&mut self) {
        self.model.add_row();
        self.status_msg = Some(format!("Row added (total {})", self.model.num_rows()));
    }

    pub fn remove_row(&mut self) {
        if self.model.num_rows() > 1 {
            let target = self.active_cell.map(|(r, _)| r).unwrap_or(self.model.num_rows() - 1);
            self.model.remove_row(target);
            if let Some((r, c)) = self.active_cell {
                self.active_cell = Some((r.min(self.model.num_rows().saturating_sub(1)), c));
            }
            self.status_msg = Some(format!("Row removed (total {})", self.model.num_rows()));
        }
    }

    pub fn add_col(&mut self) {
        self.model.add_col();
        self.status_msg = Some(format!("Column added (total {})", self.model.num_cols()));
    }

    pub fn remove_col(&mut self) {
        if self.model.num_cols() > 1 {
            let target = self.active_cell.map(|(_, c)| c).unwrap_or(self.model.num_cols() - 1);
            self.model.remove_col(target);
            if let Some((r, c)) = self.active_cell {
                self.active_cell = Some((r, c.min(self.model.num_cols().saturating_sub(1))));
            }
            self.status_msg = Some(format!("Column removed (total {})", self.model.num_cols()));
        }
    }

    pub fn set_cell(&mut self, r: usize, c: usize, val: String) {
        self.model.set_cell(r, c, val);
    }

    pub fn start_edit(&mut self, r: usize, c: usize) {
        if let Some(cell) = self.model.get_cell(r, c) {
            self.active_cell = Some((r, c));
            self.editing_cell = Some((r, c));
            self.edit_buffer = cell.content.clone();
        }
    }

    pub fn commit_edit(&mut self) {
        if let Some((r, c)) = self.editing_cell {
            let buf = self.edit_buffer.clone();
            self.model.set_cell(r, c, buf);
            self.editing_cell = None;
            self.edit_buffer.clear();
        }
    }

    pub fn cancel_edit(&mut self) {
        self.editing_cell = None;
        self.edit_buffer.clear();
    }

    pub fn cycle_col_align(&mut self, c: usize) {
        if let Some(col) = self.model.columns.get_mut(c) {
            col.alignment = match col.alignment {
                ColAlign::Left => ColAlign::Center,
                ColAlign::Center => ColAlign::Right,
                ColAlign::Right => ColAlign::Left,
                ColAlign::Paragraph(_) => ColAlign::Left,
            };
        }
    }

    pub fn toggle_booktabs(&mut self) {
        self.model.booktabs = !self.model.booktabs;
    }

    pub fn merge_selection(&mut self, r1: usize, c1: usize, r2: usize, c2: usize) {
        self.model.merge_cells(r1, c1, r2, c2);
        self.status_msg = Some("Cells merged".to_string());
    }

    pub fn unmerge_cell(&mut self, r: usize, c: usize) {
        self.model.unmerge_cell(r, c);
        self.status_msg = Some("Cell unmerged".to_string());
    }

    pub fn paste_clipboard(&mut self, text: &str) {
        let (start_r, start_c) = self.active_cell.unwrap_or((0, 0));
        let rows: Vec<&str> = text.split('\n').filter(|s| !s.trim().is_empty()).collect();

        if rows.is_empty() {
            return;
        }

        let is_multi_cell = rows.len() > 1 || rows[0].contains('\t');
        if is_multi_cell {
            for (dr, row_str) in rows.iter().enumerate() {
                let r = start_r + dr;
                while r >= self.model.num_rows() {
                    self.model.add_row();
                }

                let cells: Vec<&str> = row_str.split('\t').collect();
                for (dc, cell_str) in cells.iter().enumerate() {
                    let c = start_c + dc;
                    while c >= self.model.num_cols() {
                        self.model.add_col();
                    }
                    self.model.set_cell(r, c, cell_str.trim().to_string());
                }
            }
            self.status_msg = Some(format!("Pasted multi-cell content from clipboard"));
        } else {
            self.model.set_cell(start_r, start_c, text.trim().to_string());
            self.status_msg = Some("Pasted into active cell".to_string());
        }
    }

    pub fn sync_caption_label(&mut self) {
        self.model.caption = if self.caption_buffer.trim().is_empty() {
            None
        } else {
            Some(self.caption_buffer.trim().to_string())
        };
        self.model.label = if self.label_buffer.trim().is_empty() {
            None
        } else {
            Some(self.label_buffer.trim().to_string())
        };
    }

    pub fn generate_latex(&self) -> String {
        let mut model_clone = self.model.clone();
        model_clone.caption = if self.caption_buffer.trim().is_empty() {
            None
        } else {
            Some(self.caption_buffer.trim().to_string())
        };
        model_clone.label = if self.label_buffer.trim().is_empty() {
            None
        } else {
            Some(self.label_buffer.trim().to_string())
        };
        generate_latex_table(&model_clone)
    }
}

pub fn render_table_editor_modal(
    sheet: &TableSpreadsheet,
    scroll_handle: &ScrollHandle,
    on_insert: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_close: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_select_cell: impl Fn(usize, usize, &mut Window, &mut App) + 'static + Clone,
    _on_edit_cell: impl Fn(usize, usize, &mut Window, &mut App) + 'static + Clone,
    _on_change_cell_text: impl Fn(usize, usize, String, &mut Window, &mut App) + 'static + Clone,
    on_add_row: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_remove_row: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_add_col: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_remove_col: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_cycle_align: impl Fn(usize, &mut Window, &mut App) + 'static + Clone,
    on_toggle_booktabs: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_merge_right: impl Fn(usize, usize, &mut Window, &mut App) + 'static + Clone,
    on_merge_down: impl Fn(usize, usize, &mut Window, &mut App) + 'static + Clone,
    on_unmerge: impl Fn(usize, usize, &mut Window, &mut App) + 'static + Clone,
    _on_update_caption: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    _on_update_label: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let latex_preview = sheet.generate_latex();
    let on_ins = on_insert.clone();
    let on_cls1 = on_close.clone();
    let on_cls2 = on_close.clone();

    let is_editing_existing = sheet.model.source_span.is_some();
    let title = if is_editing_existing {
        if let Some(ref file) = sheet.model.source_file {
            format!("Edit Table — {}", file)
        } else {
            "Edit Table in Buffer".to_string()
        }
    } else {
        "Visual LaTeX Spreadsheet & Table Editor".to_string()
    };

    let active_coords = sheet.active_cell.unwrap_or((0, 0));

    div()
        .absolute()
        .inset_0()
        .bg(Theme::modal_backdrop())
        .flex()
        .justify_center()
        .items_center()
        .child(
            div()
                .w(px(860.0))
                .max_h(px(720.0))
                .bg(Theme::bg_modal())
                .border_1()
                .border_color(Theme::border_subtle())
                .rounded_lg()
                .shadow_lg()
                .flex()
                .flex_col()
                .overflow_hidden()
                // ==========================================
                // 1. Modal Header
                // ==========================================
                .child(
                    div()
                        .px_4()
                        .py_3()
                        .bg(Theme::bg_titlebar())
                        .border_b_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_base()
                                        .text_color(Theme::accent_blue())
                                        .child("📊"),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(Theme::text_bright())
                                        .child(title),
                                ),
                        )
                        .child(
                            div()
                                .cursor_pointer()
                                .text_color(Theme::text_muted())
                                .hover(|h| h.text_color(Theme::text_bright()))
                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                    on_cls1(window, cx);
                                })
                                .child("✕"),
                        ),
                )
                // ==========================================
                // 2. Toolbar & Controls Bar
                // ==========================================
                .child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(Theme::bg_panel())
                        .border_b_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(button_small("+ Row", on_add_row.clone()))
                                        .child(button_small("- Row", on_remove_row.clone()))
                                        .child(button_small("+ Col", on_add_col.clone()))
                                        .child(button_small("- Col", on_remove_col.clone())),
                                )
                                .child(
                                    div()
                                        .w(px(1.0))
                                        .h(px(16.0))
                                        .bg(Theme::border_subtle()),
                                )
                                // Merge / Unmerge buttons for active cell
                                .child({
                                    let on_mr = on_merge_right.clone();
                                    let on_md = on_merge_down.clone();
                                    let on_um = on_unmerge.clone();
                                    let (ar, ac) = active_coords;

                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .px_2()
                                                .py_0p5()
                                                .bg(Theme::bg_hover())
                                                .hover(|h| h.bg(Theme::bg_active()))
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .text_xs()
                                                .text_color(Theme::accent_purple())
                                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                                    on_mr(ar, ac, window, cx);
                                                })
                                                .child("Merge Right ➔"),
                                        )
                                        .child(
                                            div()
                                                .px_2()
                                                .py_0p5()
                                                .bg(Theme::bg_hover())
                                                .hover(|h| h.bg(Theme::bg_active()))
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .text_xs()
                                                .text_color(Theme::accent_purple())
                                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                                    on_md(ar, ac, window, cx);
                                                })
                                                .child("Merge Down ⬇"),
                                        )
                                        .child(
                                            div()
                                                .px_2()
                                                .py_0p5()
                                                .bg(Theme::bg_hover())
                                                .hover(|h| h.bg(Theme::bg_active()))
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .text_xs()
                                                .text_color(Theme::text_dim())
                                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                                    on_um(ar, ac, window, cx);
                                                })
                                                .child("Unmerge"),
                                        )
                                }),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child({
                                    let on_bt = on_toggle_booktabs.clone();
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1p5()
                                        .cursor_pointer()
                                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                            on_bt(window, cx);
                                        })
                                        .child(
                                            div()
                                                .w(px(14.0))
                                                .h(px(14.0))
                                                .rounded_xs()
                                                .border_1()
                                                .border_color(Theme::border_subtle())
                                                .bg(if sheet.model.booktabs { Theme::accent_blue() } else { Theme::bg_card() })
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .child(if sheet.model.booktabs { "✓" } else { "" }),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(Theme::text_primary())
                                                .child("Booktabs"),
                                        )
                                })
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(Theme::text_dim())
                                        .child(format!("{} × {}", sheet.model.num_rows(), sheet.model.num_cols())),
                                ),
                        ),
                )
                // ==========================================
                // 3. Caption & Label Bar
                // ==========================================
                .child(
                    div()
                        .px_4()
                        .py_1p5()
                        .bg(Theme::bg_card())
                        .border_b_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(Theme::text_dim())
                                        .child("Caption:"),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .px_2()
                                        .py_0p5()
                                        .bg(Theme::bg_editor())
                                        .border_1()
                                        .border_color(Theme::border_subtle())
                                        .rounded_sm()
                                        .text_xs()
                                        .text_color(Theme::text_bright())
                                        .child(if sheet.caption_buffer.is_empty() {
                                            "Optional table caption...".to_string()
                                        } else {
                                            sheet.caption_buffer.clone()
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .w(px(220.0))
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(Theme::text_dim())
                                        .child("Label:"),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .px_2()
                                        .py_0p5()
                                        .bg(Theme::bg_editor())
                                        .border_1()
                                        .border_color(Theme::border_subtle())
                                        .rounded_sm()
                                        .text_xs()
                                        .text_color(Theme::text_bright())
                                        .child(if sheet.label_buffer.is_empty() {
                                            "tbl:name".to_string()
                                        } else {
                                            sheet.label_buffer.clone()
                                        }),
                                ),
                        ),
                )
                // ==========================================
                // 4. Column Header Bar & 5. Matrix Grid View
                // ==========================================
                .child({
                    let num_cols = sheet.model.columns.len().max(1);
                    let total_avail = 796.0f32;
                    let mut col_widths = vec![140.0f32; num_cols];

                    for c in 0..num_cols {
                        let mut max_len = 8usize;
                        for row in &sheet.model.rows {
                            if let Some(cell) = row.cells.get(c) {
                                if cell.col_span == 1 && !cell.is_shadow {
                                    max_len = max_len.max(cell.content.len());
                                }
                            }
                        }
                        col_widths[c] = ((max_len as f32) * 7.5 + 32.0).clamp(130.0, 360.0);
                    }

                    let total_gap = (num_cols.saturating_sub(1) as f32) * 4.0;
                    let current_sum: f32 = col_widths.iter().sum();
                    if current_sum + total_gap < total_avail {
                        let factor = (total_avail - total_gap) / current_sum;
                        for w in col_widths.iter_mut() {
                            *w *= factor;
                        }
                    }

                    let header_col_widths = col_widths.clone();
                    let grid_col_widths = col_widths.clone();

                    div()
                        .flex()
                        .flex_col()
                        // 4. Column Header Bar
                        .child(
                            div()
                                .px_4()
                                .py_1()
                                .bg(Theme::bg_titlebar())
                                .border_b_1()
                                .border_color(Theme::border_subtle())
                                .flex()
                                .gap_1()
                                .child(
                                    // Gutter spacer
                                    div()
                                        .w(px(28.0))
                                        .flex_shrink_0()
                                        .text_xs()
                                        .text_color(Theme::text_dim())
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child("#"),
                                )
                                .children(sheet.model.columns.iter().enumerate().map(|(c, col)| {
                                    let on_cyc = on_cycle_align.clone();
                                    let align_badge = match col.alignment {
                                        ColAlign::Left => "L ▾",
                                        ColAlign::Center => "C ▾",
                                        ColAlign::Right => "R ▾",
                                        ColAlign::Paragraph(_) => "P ▾",
                                    };
                                    let col_w = header_col_widths[c];

                                    div()
                                        .w(px(col_w))
                                        .flex_shrink_0()
                                        .px_2()
                                        .py_0p5()
                                        .bg(Theme::bg_panel())
                                        .border_1()
                                        .border_color(Theme::border_subtle())
                                        .rounded_sm()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(Theme::text_dim())
                                                .child(format!("Col {}", c + 1)),
                                        )
                                        .child(
                                            div()
                                                .px_1p5()
                                                .py_0p5()
                                                .bg(Theme::bg_hover())
                                                .hover(|h| h.bg(Theme::bg_active()))
                                                .rounded_xs()
                                                .cursor_pointer()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(Theme::accent_cyan())
                                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                                    on_cyc(c, window, cx);
                                                })
                                                .child(align_badge),
                                        )
                                })),
                        )
                        // 5. Interactive Matrix Grid View (Scrollable)
                        .child(
                            div()
                                .id("table_matrix_scroll")
                                .px_4()
                                .py_2()
                                .max_h(px(260.0))
                                .overflow_y_scroll()
                                .track_scroll(scroll_handle)
                                .flex()
                                .flex_col()
                                .gap_1()
                                .children(sheet.model.rows.iter().enumerate().map(|(r, row)| {
                                    let on_sel = on_select_cell.clone();
                                    let row_col_widths = grid_col_widths.clone();

                                    div()
                                        .flex()
                                        .gap_1()
                                        .child(
                                            // Row Number Gutter
                                            div()
                                                .w(px(28.0))
                                                .flex_shrink_0()
                                                .h(px(28.0))
                                                .bg(Theme::bg_titlebar())
                                                .border_1()
                                                .border_color(Theme::border_subtle())
                                                .rounded_sm()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_xs()
                                                .text_color(Theme::text_dim())
                                                .child(format!("{}", r + 1)),
                                        )
                                        .children(row.cells.iter().enumerate().filter_map(|(c, cell)| {
                                            if cell.is_shadow {
                                                return None;
                                            }

                                            let is_active = sheet.active_cell == Some((r, c));
                                            let is_editing = sheet.editing_cell == Some((r, c));
                                            let on_sel_c = on_sel.clone();
                                            let val_clone = cell.content.clone();

                                            let is_header = r == 0;
                                            let end_col = (c + cell.col_span).min(row_col_widths.len());
                                            let span_w: f32 = row_col_widths[c..end_col].iter().sum::<f32>()
                                                + ((end_col.saturating_sub(c).saturating_sub(1)) as f32 * 4.0);
                                            let cell_h = if cell.row_span > 1 {
                                                28.0 * cell.row_span as f32 + ((cell.row_span - 1) as f32 * 4.0)
                                            } else {
                                                28.0
                                            };

                                            let mut cell_el = div()
                                                .w(px(span_w))
                                                .flex_shrink_0()
                                                .h(px(cell_h))
                                                .px_2()
                                                .bg(if is_header { Theme::bg_card() } else { Theme::bg_editor() })
                                                .border_1()
                                                .border_color(if is_active {
                                                    Theme::accent_blue()
                                                } else {
                                                    Theme::border_subtle()
                                                })
                                                .rounded_sm()
                                                .flex()
                                                .items_center()
                                                .cursor_pointer()
                                                .overflow_hidden()
                                                .text_xs()
                                                .text_color(if is_header {
                                                    Theme::accent_blue()
                                                } else {
                                                    Theme::text_primary()
                                                })
                                                .font_weight(if is_header {
                                                    FontWeight::BOLD
                                                } else {
                                                    FontWeight::NORMAL
                                                })
                                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                                    on_sel_c(r, c, window, cx);
                                                });

                                            if is_editing {
                                                cell_el = cell_el
                                                    .bg(Theme::bg_active())
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .overflow_hidden()
                                                            .text_color(Theme::text_bright())
                                                            .child(if sheet.edit_buffer.is_empty() {
                                                                "|".to_string()
                                                            } else {
                                                                format!("{}|", sheet.edit_buffer)
                                                            }),
                                                    );
                                            } else {
                                                cell_el = cell_el.child(
                                                    div()
                                                        .flex_1()
                                                        .overflow_hidden()
                                                        .child(if val_clone.is_empty() {
                                                            "—".to_string()
                                                        } else {
                                                            val_clone
                                                        }),
                                                );
                                            }

                                            Some(cell_el.into_any_element())
                                        }))
                                })),
                        )
                })
                // ==========================================
                // 6. LaTeX Code Preview (Monospace)
                // ==========================================
                .child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(Theme::bg_sidebar())
                        .border_t_1()
                        .border_color(Theme::border_subtle())
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
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(Theme::text_dim())
                                        .child("LATEX CODE PREVIEW"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(Theme::accent_green())
                                        .child(if is_editing_existing {
                                            "Mode: In-place buffer replacement"
                                        } else {
                                            "Mode: Insert at cursor"
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .h(px(80.0))
                                .p_2()
                                .bg(Theme::table_code_preview_bg())
                                .border_1()
                                .border_color(Theme::border_subtle())
                                .rounded_sm()
                                .font_family(".AppleSystemUIFontMonospaced")
                                .text_xs()
                                .text_color(Theme::accent_green())
                                .overflow_hidden()
                                .child(latex_preview.clone()),
                        ),
                )
                // ==========================================
                // 7. Modal Footer Buttons
                // ==========================================
                .child(
                    div()
                        .px_4()
                        .py_3()
                        .bg(Theme::bg_titlebar())
                        .border_t_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .text_xs()
                                .text_color(Theme::text_dim())
                                .child(sheet.status_msg.clone().unwrap_or_else(|| {
                                    "Click cell to select • Double-click or Tab to edit".to_string()
                                })),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    div()
                                        .px_3()
                                        .py_1()
                                        .bg(Theme::bg_panel())
                                        .hover(|h| h.bg(Theme::bg_hover()))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(Theme::text_primary())
                                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                            on_cls2(window, cx);
                                        })
                                        .child("Cancel"),
                                )
                                .child(
                                    div()
                                        .px_4()
                                        .py_1()
                                        .bg(Theme::accent_blue())
                                        .hover(|h| h.bg(Theme::border_focus()))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(Theme::text_inverted())
                                        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                            on_ins(latex_preview.clone(), window, cx);
                                        })
                                        .child(if is_editing_existing {
                                            "Apply Table Changes"
                                        } else {
                                            "Insert Table"
                                        }),
                                ),
                        ),
                ),
        )
}

fn button_small(label: &'static str, on_click: impl Fn(&mut Window, &mut App) + 'static + Clone) -> impl IntoElement {
    div()
        .px_2()
        .py_0p5()
        .bg(Theme::bg_hover())
        .hover(|h| h.bg(Theme::bg_active()))
        .rounded_sm()
        .cursor_pointer()
        .text_xs()
        .text_color(Theme::text_bright())
        .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
            on_click(window, cx);
        })
        .child(label)
}
