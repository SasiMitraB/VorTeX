#![allow(dead_code)]

use crate::theme::Theme;
use gpui::prelude::*;
use gpui::*;

pub struct TableEditorModal {
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<Vec<String>>,
    pub is_open: bool,
    pub on_insert: Option<Box<dyn Fn(String) + Send + Sync + 'static>>,
}

impl Default for TableEditorModal {
    fn default() -> Self {
        let rows = 4;
        let cols = 3;
        let mut cells = Vec::new();
        for r in 0..rows {
            let mut row_vec = Vec::new();
            for c in 0..cols {
                if r == 0 {
                    row_vec.push(format!("Header {}", c + 1));
                } else {
                    row_vec.push(format!("Data {},{}", r, c + 1));
                }
            }
            cells.push(row_vec);
        }

        Self {
            rows,
            cols,
            cells,
            is_open: false,
            on_insert: None,
        }
    }
}

impl TableEditorModal {
    pub fn add_row(&mut self) {
        if self.rows < 15 {
            self.rows += 1;
            self.cells.push(vec![String::new(); self.cols]);
        }
    }

    pub fn remove_row(&mut self) {
        if self.rows > 1 {
            self.rows -= 1;
            self.cells.pop();
        }
    }

    pub fn add_col(&mut self) {
        if self.cols < 10 {
            self.cols += 1;
            for row in self.cells.iter_mut() {
                row.push(String::new());
            }
        }
    }

    pub fn remove_col(&mut self) {
        if self.cols > 1 {
            self.cols -= 1;
            for row in self.cells.iter_mut() {
                row.pop();
            }
        }
    }

    pub fn set_cell(&mut self, row: usize, col: usize, text: String) {
        if row < self.rows && col < self.cols {
            self.cells[row][col] = text;
        }
    }

    pub fn generate_latex(&self) -> String {
        let col_spec = vec!["c"; self.cols].join(" ");
        let mut latex = format!("\\begin{{tabular}}{{{}}}\n\\toprule\n", col_spec);

        for (r, row) in self.cells.iter().enumerate() {
            let row_escaped: Vec<String> = row.iter().map(|c| escape_latex(c)).collect();
            latex.push_str("  ");
            latex.push_str(&row_escaped.join(" & "));
            latex.push_str(" \\\\\n");

            if r == 0 && self.rows > 1 {
                latex.push_str("  \\midrule\n");
            }
        }

        latex.push_str("\\bottomrule\n\\end{tabular}");
        latex
    }
}

fn escape_latex(text: &str) -> String {
    text.replace('\\', "\\textbackslash{}")
        .replace('&', "\\&")
        .replace('%', "\\%")
        .replace('$', "\\$")
        .replace('#', "\\#")
        .replace('_', "\\_")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('~', "\\textasciitilde{}")
        .replace('^', "\\textasciicircum{}")
}

pub fn render_table_editor_modal(
    modal: &TableEditorModal,
    scroll_handle: &ScrollHandle,
    on_insert: impl Fn(String, &mut Window, &mut App) + 'static + Clone,
    on_close: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_change_cell: impl Fn(usize, usize, String, &mut Window, &mut App) + 'static + Clone,
    on_add_row: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_remove_row: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_add_col: impl Fn(&mut Window, &mut App) + 'static + Clone,
    on_remove_col: impl Fn(&mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let latex_preview = modal.generate_latex();
    let on_ins = on_insert.clone();
    let on_cls1 = on_close.clone();
    let on_cls2 = on_close.clone();

    div()
        .absolute()
        .inset_0()
        .bg(hsla(0.0, 0.0, 0.0, 0.6))
        .flex()
        .justify_center()
        .items_center()
        .child(
            div()
                .w(px(720.0))
                .bg(Theme::bg_modal())
                .border_1()
                .border_color(Theme::border_subtle())
                .rounded_lg()
                .shadow_lg()
                .flex()
                .flex_col()
                .overflow_hidden()
                // Header
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
                                .text_base()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(Theme::text_bright())
                                .child("Visual LaTeX Table Generator"),
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
                // Controls bar
                .child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(Theme::bg_panel())
                        .border_b_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .text_xs()
                                .text_color(Theme::text_muted())
                                .child(format!("Grid: {} rows × {} columns", modal.rows, modal.cols)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    button_small("+ Row", on_add_row)
                                )
                                .child(
                                    button_small("- Row", on_remove_row)
                                )
                                .child(
                                    button_small("+ Col", on_add_col)
                                )
                                .child(
                                    button_small("- Col", on_remove_col)
                                ),
                        ),
                )
                // Matrix Grid View
                .child(
                    div()
                        .id("table_matrix_scroll")
                        .px_4()
                        .py_3()
                        .max_h(px(260.0))
                        .overflow_y_scroll()
                        .track_scroll(scroll_handle)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(modal.cells.iter().enumerate().map(|(r, row)| {
                            let on_ch = on_change_cell.clone();
                            div()
                                .flex()
                                .gap_1()
                                .children(row.iter().enumerate().map(|(_c, val)| {
                                    let val_clone = val.clone();
                                    let on_ch_inner = on_ch.clone();
                                    let _ = on_ch_inner;
                                    div()
                                        .flex_1()
                                        .h(px(28.0))
                                        .px_2()
                                        .bg(if r == 0 { Theme::bg_card() } else { Theme::bg_editor() })
                                        .border_1()
                                        .border_color(Theme::border_subtle())
                                        .rounded_sm()
                                        .flex()
                                        .items_center()
                                        .text_xs()
                                        .text_color(if r == 0 { Theme::accent_blue() } else { Theme::text_primary() })
                                        .font_weight(if r == 0 { FontWeight::BOLD } else { FontWeight::NORMAL })
                                        .child(if val.is_empty() { "—".to_string() } else { val_clone })
                                }))
                        })),
                )
                // LaTeX Code Preview
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
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(Theme::text_dim())
                                .child("LATEX PREVIEW"),
                        )
                        .child(
                            div()
                                .h(px(70.0))
                                .p_2()
                                .bg(Theme::bg_editor())
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
                // Footer buttons
                .child(
                    div()
                        .px_4()
                        .py_3()
                        .bg(Theme::bg_titlebar())
                        .border_t_1()
                        .border_color(Theme::border_subtle())
                        .flex()
                        .justify_end()
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
                                .hover(|h| h.bg(hsla(207.0 / 360.0, 0.82, 0.75, 1.0)))
                                .rounded_md()
                                .cursor_pointer()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(Theme::bg_titlebar())
                                .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                    on_ins(latex_preview.clone(), window, cx);
                                })
                                .child("Insert Table"),
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
