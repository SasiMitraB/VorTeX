#![allow(dead_code)]

use crate::backend::BackendClient;
use crate::theme::Theme;
use crate::views::editor::completion::{render_completion_popup, CompletionItem, CompletionKind, CompletionState};
use gpui::prelude::*;
use gpui::*;

#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub text: String,
    pub color: Hsla,
    pub is_bold: bool,
    pub is_italic: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum CharClass {
    Whitespace,
    Word,
    Punctuation,
}

fn classify_char(c: char) -> CharClass {
    if c.is_whitespace() {
        CharClass::Whitespace
    } else if c.is_alphanumeric() || c == '_' {
        CharClass::Word
    } else {
        CharClass::Punctuation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualLine {
    pub buffer_row: usize,
    pub wrap_idx: usize,
    pub start_col: usize,
    pub end_col: usize,
}

pub const CHAR_WIDTH: f32 = 8.75;
pub const FONT_SIZE: f32 = 14.0;
pub const LINE_HEIGHT: f32 = 22.0;

pub struct LatexEditor {
    pub focus_handle: FocusHandle,
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub selection_anchor: Option<(usize, usize)>,
    pub selection: Option<((usize, usize), (usize, usize))>,
    pub is_mouse_dragging: bool,
    pub sidebar_visible: bool,
    pub is_right_pane: bool,
    pub has_right_pane: bool,
    pub scroll_top_px: f32,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub char_width: Pixels,
    pub last_wrap_cols: usize,
    pub completion: CompletionState,
    pub backend: BackendClient,
    pub current_file_path: Option<String>,
    pub is_dirty: bool,
    pub undo_stack: Vec<(Vec<String>, usize, usize)>,
    pub redo_stack: Vec<(Vec<String>, usize, usize)>,
}

impl LatexEditor {
    pub fn new(content: &str, file_path: Option<String>, backend: BackendClient, cx: &mut Context<Self>) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|s| s.to_string()).collect()
        };

        Self {
            focus_handle: cx.focus_handle(),
            lines,
            cursor_row: 0,
            cursor_col: 0,
            selection_anchor: None,
            selection: None,
            is_mouse_dragging: false,
            sidebar_visible: true,
            is_right_pane: false,
            has_right_pane: false,
            scroll_top_px: 0.0,
            font_size: px(FONT_SIZE),
            line_height: px(LINE_HEIGHT),
            char_width: px(CHAR_WIDTH),
            last_wrap_cols: 80,
            completion: CompletionState::default(),
            backend,
            current_file_path: file_path,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }


    pub fn set_content(&mut self, content: &str, file_path: Option<String>) {
        self.lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|s| s.to_string()).collect()
        };
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.selection = None;
        self.selection_anchor = None;
        self.is_mouse_dragging = false;
        self.current_file_path = file_path;
        self.is_dirty = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.scroll_top_px = 0.0;
    }

    pub fn get_content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn find_table_at_cursor(&self) -> Option<(String, std::ops::Range<usize>)> {
        let content = self.get_content();
        let mut cursor_byte = 0;
        for (r, line) in self.lines.iter().enumerate() {
            if r == self.cursor_row {
                let col_chars = self.cursor_col.min(line.chars().count());
                let col_bytes: usize = line.chars().take(col_chars).map(|c| c.len_utf8()).sum();
                cursor_byte += col_bytes;
                break;
            } else {
                cursor_byte += line.len() + 1; // +1 for '\n'
            }
        }

        // 1. Check if enclosed within \begin{table} ... \end{table}
        let mut search_idx = 0;
        while let Some(table_pos) = content[search_idx..].find("\\begin{table") {
            let abs_start = search_idx + table_pos;
            if let Some(table_end_rel) = content[abs_start..].find("\\end{table}") {
                let abs_end = abs_start + table_end_rel + "\\end{table}".len();
                if cursor_byte >= abs_start && cursor_byte <= abs_end {
                    let snippet = content[abs_start..abs_end].to_string();
                    return Some((snippet, abs_start..abs_end));
                }
                search_idx = abs_end;
            } else {
                search_idx = abs_start + "\\begin{table".len();
            }
        }

        // 2. Check if enclosed within \begin{tabular} ... \end{tabular}
        search_idx = 0;
        while let Some(tab_pos) = content[search_idx..].find("\\begin{tabular") {
            let abs_start = search_idx + tab_pos;
            if let Some(tab_end_rel) = content[abs_start..].find("\\end{tabular}") {
                let abs_end = abs_start + tab_end_rel + "\\end{tabular}".len();
                if cursor_byte >= abs_start && cursor_byte <= abs_end {
                    let snippet = content[abs_start..abs_end].to_string();
                    return Some((snippet, abs_start..abs_end));
                }
                search_idx = abs_end;
            } else {
                search_idx = abs_start + "\\begin{tabular".len();
            }
        }

        None
    }

    pub fn replace_range(&mut self, range: std::ops::Range<usize>, new_text: &str) {
        self.push_undo();
        let content = self.get_content();
        if range.start <= content.len() && range.end <= content.len() && range.start <= range.end {
            let mut updated = String::with_capacity(content.len().saturating_sub(range.end - range.start) + new_text.len());
            updated.push_str(&content[..range.start]);
            updated.push_str(new_text);
            updated.push_str(&content[range.end..]);

            self.lines = if updated.is_empty() {
                vec![String::new()]
            } else {
                updated.lines().map(|s| s.to_string()).collect()
            };

            let mut accumulated = 0;
            let mut target_r = 0;
            let mut target_c = 0;
            for (r, line) in self.lines.iter().enumerate() {
                let line_len = line.len();
                if accumulated + line_len >= range.start {
                    target_r = r;
                    let byte_offset_in_line = range.start.saturating_sub(accumulated);
                    target_c = line[..byte_offset_in_line.min(line_len)].chars().count();
                    break;
                }
                accumulated += line_len + 1;
            }

            self.cursor_row = target_r.min(self.lines.len().saturating_sub(1));
            self.cursor_col = target_c;
            self.clear_selection();
            self.is_dirty = true;
        }
    }

    pub fn jump_to_line(&mut self, line: usize) {
        let row = line.saturating_sub(1).min(self.lines.len().saturating_sub(1));
        self.cursor_row = row;
        self.cursor_col = 0;
        self.clear_selection();
        let wrap_cols = self.last_wrap_cols.max(20);
        let visual_lines = self.compute_visual_lines(wrap_cols);
        let v_idx = Self::find_visual_line_index(&visual_lines, row, 0);
        self.scroll_top_px = (v_idx as f32 * LINE_HEIGHT - 100.0).max(0.0);
    }

    pub fn calculate_wrap_cols(&self, win_w: f32) -> usize {
        let sidebar_w = if self.sidebar_visible { 240.0 } else { 0.0 };
        let pane_w = if self.has_right_pane {
            (win_w - sidebar_w).max(200.0) / 2.0
        } else {
            (win_w - sidebar_w).max(200.0)
        };
        let gutter_w = 48.0;
        let padding_x = 24.0; // 12px left + 12px right padding
        let scrollbar_margin = 16.0;
        let available_w = (pane_w - gutter_w - padding_x - scrollbar_margin).max(100.0);
        let char_w = CHAR_WIDTH;
        ((available_w / char_w).floor() as usize).max(20)
    }

    pub fn wrap_line_to_segments(line: &str, max_cols: usize) -> Vec<(usize, usize)> {
        let char_count = line.chars().count();
        if char_count == 0 {
            return vec![(0, 0)];
        }
        if char_count <= max_cols || max_cols == 0 {
            return vec![(0, char_count)];
        }

        let chars: Vec<char> = line.chars().collect();
        let mut segments = Vec::new();
        let mut start = 0;

        while start < char_count {
            let remaining = char_count - start;
            if remaining <= max_cols {
                segments.push((start, char_count));
                break;
            }

            let limit = start + max_cols;
            let mut break_point = None;

            // 1. Look for whitespace to break at
            for i in (start + 1..=limit).rev() {
                if chars[i - 1].is_whitespace() {
                    break_point = Some(i);
                    break;
                }
            }

            // 2. If no whitespace, look for punctuation or special LaTeX break points
            if break_point.is_none() {
                for i in (start + 1..=limit).rev() {
                    let ch = chars[i - 1];
                    if ch == ',' || ch == ';' || ch == '}' || ch == ')' || ch == ']' || ch == '>' || ch == '-' {
                        break_point = Some(i);
                        break;
                    }
                }
            }

            // 3. If still no break point, hard break at max_cols
            let end = break_point.unwrap_or(limit);
            segments.push((start, end));
            start = end;
        }

        segments
    }

    pub fn compute_visual_lines(&self, max_cols: usize) -> Vec<VisualLine> {
        let mut visual_lines = Vec::new();
        for (row_idx, line) in self.lines.iter().enumerate() {
            let segments = Self::wrap_line_to_segments(line, max_cols);
            for (wrap_idx, (start_col, end_col)) in segments.into_iter().enumerate() {
                visual_lines.push(VisualLine {
                    buffer_row: row_idx,
                    wrap_idx,
                    start_col,
                    end_col,
                });
            }
        }
        if visual_lines.is_empty() {
            visual_lines.push(VisualLine {
                buffer_row: 0,
                wrap_idx: 0,
                start_col: 0,
                end_col: 0,
            });
        }
        visual_lines
    }

    pub fn find_visual_line_index(visual_lines: &[VisualLine], row: usize, col: usize) -> usize {
        let mut candidate_idx = 0;
        for (idx, vl) in visual_lines.iter().enumerate() {
            if vl.buffer_row == row {
                candidate_idx = idx;
                if col >= vl.start_col && col <= vl.end_col {
                    if col == vl.end_col {
                        if let Some(next_vl) = visual_lines.get(idx + 1) {
                            if next_vl.buffer_row == row && next_vl.start_col == col {
                                return idx + 1;
                            }
                        }
                    }
                    return idx;
                }
            } else if vl.buffer_row > row {
                break;
            }
        }
        candidate_idx
    }

    pub fn slice_spans(spans: &[HighlightSpan], start_col: usize, end_col: usize) -> Vec<HighlightSpan> {
        let mut result = Vec::new();
        let mut current_offset = 0;
        for span in spans {
            let span_chars: Vec<char> = span.text.chars().collect();
            let span_len = span_chars.len();
            let span_end = current_offset + span_len;

            if span_end > start_col && current_offset < end_col {
                let slice_start = start_col.saturating_sub(current_offset);
                let slice_end = (end_col - current_offset).min(span_len);
                if slice_start < slice_end && slice_start < span_chars.len() {
                    let sliced_text: String = span_chars[slice_start..slice_end].iter().collect();
                    if !sliced_text.is_empty() {
                        result.push(HighlightSpan {
                            text: sliced_text,
                            color: span.color,
                            is_bold: span.is_bold,
                            is_italic: span.is_italic,
                        });
                    }
                }
            }
            current_offset = span_end;
        }
        result
    }

    fn push_undo(&mut self) {
        self.undo_stack.push((self.lines.clone(), self.cursor_row, self.cursor_col));
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.is_dirty = true;
    }

    pub fn undo(&mut self) {
        if let Some((prev_lines, r, c)) = self.undo_stack.pop() {
            self.redo_stack.push((self.lines.clone(), self.cursor_row, self.cursor_col));
            self.lines = prev_lines;
            self.cursor_row = r.min(self.lines.len().saturating_sub(1));
            self.cursor_col = c.min(self.current_line_len());
            self.clear_selection();
            self.ensure_cursor_visible();
        }
    }

    pub fn redo(&mut self) {
        if let Some((next_lines, r, c)) = self.redo_stack.pop() {
            self.undo_stack.push((self.lines.clone(), self.cursor_row, self.cursor_col));
            self.lines = next_lines;
            self.cursor_row = r.min(self.lines.len().saturating_sub(1));
            self.cursor_col = c.min(self.current_line_len());
            self.clear_selection();
            self.ensure_cursor_visible();
        }
    }

    pub fn current_line(&self) -> &str {
        self.lines.get(self.cursor_row).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn current_line_mut(&mut self) -> &mut String {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        if self.cursor_row >= self.lines.len() {
            self.cursor_row = self.lines.len() - 1;
        }
        &mut self.lines[self.cursor_row]
    }

    pub fn current_line_len(&self) -> usize {
        self.current_line().chars().count()
    }

    pub fn ensure_cursor_visible(&mut self) {
        let wrap_cols = self.last_wrap_cols.max(20);
        let visual_lines = self.compute_visual_lines(wrap_cols);
        let v_idx = Self::find_visual_line_index(&visual_lines, self.cursor_row, self.cursor_col);
        let cursor_y = v_idx as f32 * LINE_HEIGHT;
        let viewport_h = 550.0;

        if cursor_y < self.scroll_top_px {
            self.scroll_top_px = cursor_y;
        } else if cursor_y > self.scroll_top_px + viewport_h - 44.0 {
            self.scroll_top_px = cursor_y - viewport_h + 44.0;
        }
    }

    // --- Selection and Cursor Navigation Helpers ---

    pub fn normalized_selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let (p1, p2) = self.selection?;
        if p1 == p2 {
            None
        } else if p1.0 < p2.0 || (p1.0 == p2.0 && p1.1 < p2.1) {
            Some((p1, p2))
        } else {
            Some((p2, p1))
        }
    }

    pub fn has_selection(&self) -> bool {
        self.normalized_selection().is_some()
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
    }

    pub fn set_selection(&mut self, anchor: (usize, usize), head: (usize, usize)) {
        if anchor == head {
            self.selection = None;
            self.selection_anchor = None;
        } else {
            self.selection_anchor = Some(anchor);
            self.selection = Some((anchor, head));
        }
        self.cursor_row = head.0;
        self.cursor_col = head.1;
    }

    pub fn select_all(&mut self) {
        if self.lines.is_empty() {
            return;
        }
        let last_row = self.lines.len() - 1;
        let last_col = self.lines[last_row].chars().count();
        self.selection_anchor = Some((0, 0));
        self.selection = Some(((0, 0), (last_row, last_col)));
        self.cursor_row = last_row;
        self.cursor_col = last_col;
        self.ensure_cursor_visible();
    }

    pub fn prev_word_boundary(line: &str, col: usize) -> usize {
        if col == 0 {
            return 0;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut idx = col.min(chars.len());

        // Step 1: Skip trailing whitespace backwards
        while idx > 0 && chars[idx - 1].is_whitespace() {
            idx -= 1;
        }
        if idx == 0 {
            return 0;
        }

        // Step 2: Skip characters of the same class backwards
        let target_class = classify_char(chars[idx - 1]);
        while idx > 0 && classify_char(chars[idx - 1]) == target_class {
            idx -= 1;
        }
        idx
    }

    pub fn next_word_boundary(line: &str, col: usize) -> usize {
        let chars: Vec<char> = line.chars().collect();
        if col >= chars.len() {
            return chars.len();
        }
        let mut idx = col;

        // Step 1: Skip leading whitespace forwards
        while idx < chars.len() && chars[idx].is_whitespace() {
            idx += 1;
        }
        if idx >= chars.len() {
            return chars.len();
        }

        // Step 2: Skip characters of the same class forwards
        let target_class = classify_char(chars[idx]);
        while idx < chars.len() && classify_char(chars[idx]) == target_class {
            idx += 1;
        }
        idx
    }

    pub fn find_word_range_at(line: &str, col: usize) -> (usize, usize) {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return (0, 0);
        }
        let mut idx = col.min(chars.len().saturating_sub(1));
        let mut target_class = classify_char(chars[idx]);

        if target_class == CharClass::Whitespace && idx > 0 && classify_char(chars[idx - 1]) != CharClass::Whitespace {
            idx -= 1;
            target_class = classify_char(chars[idx]);
        }

        let mut start = idx;
        while start > 0 && classify_char(chars[start - 1]) == target_class {
            start -= 1;
        }

        let mut end = idx;
        while end < chars.len() && classify_char(chars[end]) == target_class {
            end += 1;
        }

        (start, end)
    }

    pub fn select_word_at(&mut self, row: usize, col: usize) {
        if row >= self.lines.len() {
            return;
        }
        let (start_col, end_col) = Self::find_word_range_at(&self.lines[row], col);
        self.selection_anchor = Some((row, start_col));
        self.selection = Some(((row, start_col), (row, end_col)));
        self.cursor_row = row;
        self.cursor_col = end_col;
        self.ensure_cursor_visible();
    }

    pub fn select_line_at(&mut self, row: usize) {
        if row >= self.lines.len() {
            return;
        }
        let line_len = self.lines[row].chars().count();
        if row + 1 < self.lines.len() {
            self.selection_anchor = Some((row, 0));
            self.selection = Some(((row, 0), (row + 1, 0)));
            self.cursor_row = row + 1;
            self.cursor_col = 0;
        } else {
            self.selection_anchor = Some((row, 0));
            self.selection = Some(((row, 0), (row, line_len)));
            self.cursor_row = row;
            self.cursor_col = line_len;
        }
        self.ensure_cursor_visible();
    }

    pub fn get_selected_text(&self) -> Option<String> {
        let ((s_row, s_col), (e_row, e_col)) = self.normalized_selection()?;
        if s_row == e_row {
            let line = self.lines.get(s_row)?;
            let chars: Vec<char> = line.chars().collect();
            let start = s_col.min(chars.len());
            let end = e_col.min(chars.len());
            Some(chars[start..end].iter().collect())
        } else {
            let mut result = Vec::new();
            if let Some(line) = self.lines.get(s_row) {
                let chars: Vec<char> = line.chars().collect();
                let start = s_col.min(chars.len());
                result.push(chars[start..].iter().collect::<String>());
            }
            for r in (s_row + 1)..e_row {
                if let Some(line) = self.lines.get(r) {
                    result.push(line.clone());
                }
            }
            if let Some(line) = self.lines.get(e_row) {
                let chars: Vec<char> = line.chars().collect();
                let end = e_col.min(chars.len());
                result.push(chars[..end].iter().collect::<String>());
            }
            Some(result.join("\n"))
        }
    }

    pub fn delete_selection(&mut self) -> bool {
        let Some(((s_row, s_col), (e_row, e_col))) = self.normalized_selection() else {
            return false;
        };
        self.push_undo();

        if s_row == e_row {
            let line = &mut self.lines[s_row];
            let mut chars: Vec<char> = line.chars().collect();
            let start = s_col.min(chars.len());
            let end = e_col.min(chars.len());
            chars.drain(start..end);
            *line = chars.into_iter().collect();
        } else {
            let first_chars: Vec<char> = self.lines[s_row].chars().take(s_col).collect();
            let last_chars: Vec<char> = self.lines[e_row].chars().skip(e_col).collect();
            let merged: String = first_chars.into_iter().chain(last_chars).collect();

            self.lines[s_row] = merged;
            for _ in (s_row + 1)..=e_row {
                if s_row + 1 < self.lines.len() {
                    self.lines.remove(s_row + 1);
                }
            }
        }

        self.cursor_row = s_row;
        let max_col = self.lines.get(s_row).map(|l| l.chars().count()).unwrap_or(0);
        self.cursor_col = s_col.min(max_col);
        self.clear_selection();
        self.ensure_cursor_visible();
        true
    }

    pub fn paste_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.delete_selection();
        self.push_undo();

        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let paste_lines: Vec<&str> = normalized.split('\n').collect();

        if paste_lines.len() == 1 {
            let row = self.cursor_row;
            let col = self.cursor_col;
            let line = &mut self.lines[row];
            let mut chars: Vec<char> = line.chars().collect();
            let insert_chars: Vec<char> = text.chars().collect();
            let count = insert_chars.len();

            if col >= chars.len() {
                chars.extend(insert_chars);
            } else {
                for (idx, ch) in insert_chars.into_iter().enumerate() {
                    chars.insert(col + idx, ch);
                }
            }
            *line = chars.into_iter().collect();
            self.cursor_col = col + count;
        } else {
            let row = self.cursor_row;
            let col = self.cursor_col;
            let line = self.lines[row].clone();
            let chars: Vec<char> = line.chars().collect();

            let before: String = chars[..col.min(chars.len())].iter().collect();
            let after: String = if col < chars.len() {
                chars[col..].iter().collect()
            } else {
                String::new()
            };

            let first_line = format!("{}{}", before, paste_lines[0]);
            self.lines[row] = first_line;

            for (i, middle_line) in paste_lines[1..paste_lines.len() - 1].iter().enumerate() {
                self.lines.insert(row + 1 + i, middle_line.to_string());
            }

            let last_idx = row + paste_lines.len() - 1;
            let last_paste_line = paste_lines[paste_lines.len() - 1];
            let last_line = format!("{}{}", last_paste_line, after);
            self.lines.insert(last_idx, last_line);

            self.cursor_row = last_idx;
            self.cursor_col = last_paste_line.chars().count();
        }

        self.clear_selection();
        self.ensure_cursor_visible();
    }

    pub fn move_cursor(&mut self, new_row: usize, new_col: usize, extend_selection: bool) {
        let max_row = self.lines.len().saturating_sub(1);
        let r = new_row.min(max_row);
        let max_col = self.lines.get(r).map(|l| l.chars().count()).unwrap_or(0);
        let c = new_col.min(max_col);

        if extend_selection {
            let anchor = self.selection_anchor.unwrap_or((self.cursor_row, self.cursor_col));
            self.set_selection(anchor, (r, c));
        } else {
            self.clear_selection();
            self.cursor_row = r;
            self.cursor_col = c;
        }

        self.ensure_cursor_visible();
    }

    pub fn move_left(&mut self, extend_selection: bool, by_word: bool, to_line_start: bool) {
        if !extend_selection && self.has_selection() && !by_word && !to_line_start {
            if let Some(((s_row, s_col), _)) = self.normalized_selection() {
                self.clear_selection();
                self.cursor_row = s_row;
                self.cursor_col = s_col;
                self.ensure_cursor_visible();
                return;
            }
        }

        let line = self.current_line();
        let col = self.cursor_col;
        let row = self.cursor_row;

        if to_line_start {
            let indent = line.chars().take_while(|c| c.is_whitespace()).count();
            let target_col = if col > indent && indent > 0 { indent } else { 0 };
            self.move_cursor(row, target_col, extend_selection);
        } else if by_word {
            if col == 0 {
                if row > 0 {
                    let prev_len = self.lines[row - 1].chars().count();
                    self.move_cursor(row - 1, prev_len, extend_selection);
                }
            } else {
                let target_col = Self::prev_word_boundary(line, col);
                self.move_cursor(row, target_col, extend_selection);
            }
        } else if col > 0 {
            self.move_cursor(row, col - 1, extend_selection);
        } else if row > 0 {
            let prev_len = self.lines[row - 1].chars().count();
            self.move_cursor(row - 1, prev_len, extend_selection);
        }
    }

    pub fn move_right(&mut self, extend_selection: bool, by_word: bool, to_line_end: bool) {
        if !extend_selection && self.has_selection() && !by_word && !to_line_end {
            if let Some((_, (e_row, e_col))) = self.normalized_selection() {
                self.clear_selection();
                self.cursor_row = e_row;
                self.cursor_col = e_col;
                self.ensure_cursor_visible();
                return;
            }
        }

        let line = self.current_line();
        let line_len = self.current_line_len();
        let col = self.cursor_col;
        let row = self.cursor_row;

        if to_line_end {
            self.move_cursor(row, line_len, extend_selection);
        } else if by_word {
            if col >= line_len {
                if row + 1 < self.lines.len() {
                    self.move_cursor(row + 1, 0, extend_selection);
                }
            } else {
                let target_col = Self::next_word_boundary(line, col);
                self.move_cursor(row, target_col, extend_selection);
            }
        } else if col < line_len {
            self.move_cursor(row, col + 1, extend_selection);
        } else if row + 1 < self.lines.len() {
            self.move_cursor(row + 1, 0, extend_selection);
        }
    }

    pub fn move_up(&mut self, extend_selection: bool, to_doc_start: bool, by_paragraph: bool) {
        if to_doc_start {
            self.move_cursor(0, 0, extend_selection);
            return;
        }

        let row = self.cursor_row;
        let col = self.cursor_col;

        if by_paragraph {
            let mut target_row = 0;
            for r in (0..row).rev() {
                if self.lines[r].trim().is_empty() {
                    target_row = r;
                    break;
                }
            }
            self.move_cursor(target_row, col, extend_selection);
        } else if row > 0 {
            self.move_cursor(row - 1, col, extend_selection);
        } else {
            self.move_cursor(0, 0, extend_selection);
        }
    }

    pub fn move_down(&mut self, extend_selection: bool, to_doc_end: bool, by_paragraph: bool) {
        let last_row = self.lines.len().saturating_sub(1);
        if to_doc_end {
            let last_col = self.lines[last_row].chars().count();
            self.move_cursor(last_row, last_col, extend_selection);
            return;
        }

        let row = self.cursor_row;
        let col = self.cursor_col;

        if by_paragraph {
            let mut target_row = last_row;
            for r in (row + 1)..=last_row {
                if self.lines[r].trim().is_empty() {
                    target_row = r;
                    break;
                }
            }
            self.move_cursor(target_row, col, extend_selection);
        } else if row + 1 < self.lines.len() {
            self.move_cursor(row + 1, col, extend_selection);
        } else {
            let last_col = self.lines[last_row].chars().count();
            self.move_cursor(last_row, last_col, extend_selection);
        }
    }

    pub fn move_up_visual(&mut self, extend_selection: bool, to_doc_start: bool, by_paragraph: bool, wrap_cols: usize) {
        if to_doc_start {
            self.move_cursor(0, 0, extend_selection);
            return;
        }
        if by_paragraph {
            self.move_up(extend_selection, false, true);
            return;
        }

        let visual_lines = self.compute_visual_lines(wrap_cols);
        let current_v_idx = Self::find_visual_line_index(&visual_lines, self.cursor_row, self.cursor_col);
        if current_v_idx > 0 {
            let target_vl = visual_lines[current_v_idx - 1];
            let current_vl = visual_lines[current_v_idx];
            let col_in_seg = self.cursor_col.saturating_sub(current_vl.start_col);
            let target_seg_len = target_vl.end_col - target_vl.start_col;
            let new_col = target_vl.start_col + col_in_seg.min(target_seg_len);
            self.move_cursor(target_vl.buffer_row, new_col, extend_selection);
        } else {
            self.move_cursor(0, 0, extend_selection);
        }
    }

    pub fn move_down_visual(&mut self, extend_selection: bool, to_doc_end: bool, by_paragraph: bool, wrap_cols: usize) {
        let last_row = self.lines.len().saturating_sub(1);
        if to_doc_end {
            let last_col = self.lines[last_row].chars().count();
            self.move_cursor(last_row, last_col, extend_selection);
            return;
        }
        if by_paragraph {
            self.move_down(extend_selection, false, true);
            return;
        }

        let visual_lines = self.compute_visual_lines(wrap_cols);
        let current_v_idx = Self::find_visual_line_index(&visual_lines, self.cursor_row, self.cursor_col);
        if current_v_idx + 1 < visual_lines.len() {
            let target_vl = visual_lines[current_v_idx + 1];
            let current_vl = visual_lines[current_v_idx];
            let col_in_seg = self.cursor_col.saturating_sub(current_vl.start_col);
            let target_seg_len = target_vl.end_col - target_vl.start_col;
            let new_col = target_vl.start_col + col_in_seg.min(target_seg_len);
            self.move_cursor(target_vl.buffer_row, new_col, extend_selection);
        } else {
            let last_col = self.lines[last_row].chars().count();
            self.move_cursor(last_row, last_col, extend_selection);
        }
    }

    pub fn delete_to_line_start(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }
        self.push_undo();
        let row = self.cursor_row;
        let col = self.cursor_col;
        if col > 0 {
            let line = &mut self.lines[row];
            let mut chars: Vec<char> = line.chars().collect();
            chars.drain(0..col);
            *line = chars.into_iter().collect();
            self.cursor_col = 0;
        } else if row > 0 {
            let current = self.lines.remove(row);
            let prev_len = self.lines[row - 1].chars().count();
            self.lines[row - 1].push_str(&current);
            self.cursor_row -= 1;
            self.cursor_col = prev_len;
        }
        self.ensure_cursor_visible();
    }

    pub fn delete_prev_word(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }
        let col = self.cursor_col;
        let row = self.cursor_row;
        if col == 0 {
            self.delete_backwards();
            return;
        }
        let line = self.current_line();
        let target_col = Self::prev_word_boundary(line, col);
        self.push_undo();
        let line = &mut self.lines[row];
        let mut chars: Vec<char> = line.chars().collect();
        chars.drain(target_col..col);
        *line = chars.into_iter().collect();
        self.cursor_col = target_col;
        self.ensure_cursor_visible();
    }

    pub fn delete_next_word(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }
        let col = self.cursor_col;
        let row = self.cursor_row;
        let line_len = self.current_line_len();
        if col >= line_len {
            self.delete_forward();
            return;
        }
        let line = self.current_line();
        let target_col = Self::next_word_boundary(line, col);
        self.push_undo();
        let line = &mut self.lines[row];
        let mut chars: Vec<char> = line.chars().collect();
        chars.drain(col..target_col);
        *line = chars.into_iter().collect();
        self.ensure_cursor_visible();
    }

    pub fn delete_forward(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }
        self.push_undo();
        let row = self.cursor_row;
        let col = self.cursor_col;
        let line_len = self.current_line_len();

        if col < line_len {
            let line = &mut self.lines[row];
            let mut chars: Vec<char> = line.chars().collect();
            chars.remove(col);
            *line = chars.into_iter().collect();
        } else if row + 1 < self.lines.len() {
            let next_line = self.lines.remove(row + 1);
            self.lines[row].push_str(&next_line);
        }
        self.ensure_cursor_visible();
    }

    pub fn indent_lines(&mut self, unindent: bool) {
        self.push_undo();
        let (start_r, end_r) = if let Some(((s_r, _), (e_r, e_c))) = self.normalized_selection() {
            let effective_end = if e_r > s_r && e_c == 0 { e_r - 1 } else { e_r };
            (s_r, effective_end)
        } else {
            (self.cursor_row, self.cursor_row)
        };

        for r in start_r..=end_r {
            if r < self.lines.len() {
                if unindent {
                    if self.lines[r].starts_with("  ") {
                        self.lines[r] = self.lines[r][2..].to_string();
                    } else if self.lines[r].starts_with(' ') {
                        self.lines[r] = self.lines[r][1..].to_string();
                    } else if self.lines[r].starts_with('\t') {
                        self.lines[r] = self.lines[r][1..].to_string();
                    }
                } else {
                    self.lines[r] = format!("  {}", self.lines[r]);
                }
            }
        }

        if let Some(((s_r, s_c), (e_r, e_c))) = self.normalized_selection() {
            let delta: isize = if unindent { -2 } else { 2 };
            let new_s_c = (s_c as isize + delta).max(0) as usize;
            let new_e_c = (e_c as isize + delta).max(0) as usize;
            self.selection = Some(((s_r, new_s_c), (e_r, new_e_c)));
            self.cursor_col = (self.cursor_col as isize + delta).max(0) as usize;
        } else {
            let delta: isize = if unindent { -2 } else { 2 };
            self.cursor_col = (self.cursor_col as isize + delta).max(0) as usize;
        }
    }

    pub fn toggle_comment(&mut self) {
        self.push_undo();
        let (start_r, end_r) = if let Some(((s_r, _), (e_r, e_c))) = self.normalized_selection() {
            let effective_end = if e_r > s_r && e_c == 0 { e_r - 1 } else { e_r };
            (s_r, effective_end)
        } else {
            (self.cursor_row, self.cursor_row)
        };

        let all_commented = (start_r..=end_r).all(|r| {
            self.lines.get(r).map(|l| l.trim_start().starts_with('%')).unwrap_or(false)
        });

        for r in start_r..=end_r {
            if let Some(line) = self.lines.get_mut(r) {
                if all_commented {
                    if let Some(pos) = line.find('%') {
                        let mut chars: Vec<char> = line.chars().collect();
                        chars.remove(pos);
                        if pos < chars.len() && chars[pos] == ' ' {
                            chars.remove(pos);
                        }
                        *line = chars.into_iter().collect();
                    }
                } else {
                    *line = format!("% {}", line);
                }
            }
        }
    }

    pub fn duplicate_line_or_selection(&mut self) {
        self.push_undo();
        if let Some(selected_text) = self.get_selected_text() {
            self.paste_text(&selected_text);
        } else {
            let row = self.cursor_row;
            let line = self.lines[row].clone();
            self.lines.insert(row + 1, line);
            self.cursor_row += 1;
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        self.push_undo();

        // 1. Math auto-replacement for unicode symbols like → -> \rightarrow
        let mut processed_text = text.to_string();
        if text == "→" {
            processed_text = "\\rightarrow ".into();
        } else if text == "⇒" {
            processed_text = "\\Rightarrow ".into();
        } else if text == "α" {
            processed_text = "\\alpha ".into();
        } else if text == "β" {
            processed_text = "\\beta ".into();
        } else if text == "θ" {
            processed_text = "\\theta ".into();
        } else if text == "λ" {
            processed_text = "\\lambda ".into();
        } else if text == "π" {
            processed_text = "\\pi ".into();
        } else if text == "∑" {
            processed_text = "\\sum ".into();
        } else if text == "∫" {
            processed_text = "\\int ".into();
        } else if text == "≤" {
            processed_text = "\\le ".into();
        } else if text == "≥" {
            processed_text = "\\ge ".into();
        } else if text == "≠" {
            processed_text = "\\neq ".into();
        } else if text == "±" {
            processed_text = "\\pm ".into();
        } else if text == "∞" {
            processed_text = "\\infty ".into();
        }

        // 2. Delimiter Skipping: if typing closing delimiter that is already right in front of cursor
        if processed_text == "}" || processed_text == ")" || processed_text == "]" || processed_text == "$" || processed_text == "\"" {
            let line = self.current_line();
            let char_at_cursor = line.chars().nth(self.cursor_col);
            if char_at_cursor == processed_text.chars().next() {
                self.cursor_col += 1;
                self.ensure_cursor_visible();
                self.trigger_autocomplete();
                return;
            }
        }

        // 3. Delimiter Auto-Closing: $, (, {, [
        let mut auto_close = "";
        if processed_text == "$" {
            auto_close = "$";
        } else if processed_text == "{" {
            auto_close = "}";
        } else if processed_text == "(" {
            auto_close = ")";
        } else if processed_text == "[" {
            auto_close = "]";
        }

        let row = self.cursor_row;
        let col = self.cursor_col;
        let line = &mut self.lines[row];

        let mut chars: Vec<char> = line.chars().collect();
        let insert_chars: Vec<char> = format!("{}{}", processed_text, auto_close).chars().collect();

        if col >= chars.len() {
            chars.extend(insert_chars);
        } else {
            for (idx, ch) in insert_chars.into_iter().enumerate() {
                chars.insert(col + idx, ch);
            }
        }

        self.lines[row] = chars.into_iter().collect();
        self.cursor_col += processed_text.chars().count();
        self.ensure_cursor_visible();

        self.trigger_autocomplete();
    }

    pub fn insert_newline(&mut self) {
        self.push_undo();

        let row = self.cursor_row;
        let col = self.cursor_col;
        let line = self.lines[row].clone();
        let chars: Vec<char> = line.chars().collect();

        let before: String = chars[..col.min(chars.len())].iter().collect();
        let after: String = if col < chars.len() {
            chars[col..].iter().collect()
        } else {
            String::new()
        };

        let mut auto_prefix = String::new();
        let trimmed_before = before.trim();
        let is_item_line = trimmed_before.starts_with("\\item");

        if is_item_line {
            if trimmed_before == "\\item" {
                self.lines[row] = String::new();
                self.lines.insert(row + 1, after);
                self.cursor_row += 1;
                self.cursor_col = 0;
                self.ensure_cursor_visible();
                self.completion.close();
                return;
            } else {
                let indent_len = before.chars().take_while(|c| c.is_whitespace()).count();
                let indent: String = " ".repeat(indent_len);
                auto_prefix = format!("{}\\item ", indent);
            }
        } else {
            let indent_len = before.chars().take_while(|c| c.is_whitespace()).count();
            if indent_len > 0 {
                auto_prefix = " ".repeat(indent_len);
            }
        }

        self.lines[row] = before;
        self.lines.insert(row + 1, format!("{}{}", auto_prefix, after));

        self.cursor_row += 1;
        self.cursor_col = auto_prefix.chars().count();
        self.ensure_cursor_visible();
        self.completion.close();
    }

    pub fn delete_backwards(&mut self) {
        self.push_undo();

        let row = self.cursor_row;
        let col = self.cursor_col;

        if col > 0 {
            let line = &mut self.lines[row];
            let mut chars: Vec<char> = line.chars().collect();

            if col < chars.len() {
                let prev_ch = chars[col - 1];
                let next_ch = chars[col];
                if (prev_ch == '{' && next_ch == '}')
                    || (prev_ch == '(' && next_ch == ')')
                    || (prev_ch == '[' && next_ch == ']')
                    || (prev_ch == '$' && next_ch == '$')
                {
                    chars.remove(col);
                }
            }

            chars.remove(col - 1);
            self.lines[row] = chars.into_iter().collect();
            self.cursor_col -= 1;
        } else if row > 0 {
            let current = self.lines.remove(row);
            let prev_len = self.lines[row - 1].chars().count();
            self.lines[row - 1].push_str(&current);
            self.cursor_row -= 1;
            self.cursor_col = prev_len;
        }

        self.ensure_cursor_visible();
        self.trigger_autocomplete();
    }

    fn trigger_autocomplete(&mut self) {
        let line = self.current_line();
        let col = self.cursor_col;
        let prefix: String = line.chars().take(col).collect();

        // 1. Check if we are inside an unclosed '{'
        if let Some(open_brace_idx) = prefix.rfind('{') {
            let inside = &prefix[open_brace_idx + 1..];
            if !inside.contains('}') {
                let before_brace = prefix[..open_brace_idx].trim_end();

                // Strip any optional arguments like `[see][p.~10]` before the `{`
                let mut cmd_end = before_brace;
                while cmd_end.ends_with(']') {
                    if let Some(open_bracket) = cmd_end.rfind('[') {
                        cmd_end = cmd_end[..open_bracket].trim_end();
                    } else {
                        break;
                    }
                }

                if let Some(slash_idx) = cmd_end.rfind('\\') {
                    let cmd = &cmd_end[slash_idx..];
                    let cmd_clean = cmd.strip_suffix('*').unwrap_or(cmd);

                    match cmd_clean {
                        "\\cite"
                        | "\\citep"
                        | "\\citet"
                        | "\\citealt"
                        | "\\citealp"
                        | "\\citeauthor"
                        | "\\citeyear"
                        | "\\citeyearpar"
                        | "\\citetext"
                        | "\\parencite"
                        | "\\textcite"
                        | "\\autocite"
                        | "\\footcite"
                        | "\\nocite"
                        | "\\fullcite"
                        | "\\cites"
                        | "\\citeurl" => {
                            let query = if let Some(last_comma) = inside.rfind(',') {
                                inside[last_comma + 1..].trim_start()
                            } else {
                                inside.trim_start()
                            };
                            self.open_cite_completions(query);
                            return;
                        }
                        "\\ref" | "\\eqref" | "\\pageref" | "\\autoref" | "\\cref" | "\\Cref" => {
                            let query = if let Some(last_comma) = inside.rfind(',') {
                                inside[last_comma + 1..].trim_start()
                            } else {
                                inside.trim_start()
                            };
                            self.open_ref_completions(query);
                            return;
                        }
                        "\\begin" => {
                            self.open_env_completions(inside.trim());
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }

        // 2. Check for LaTeX command prefix: \alpha, \sub, etc.
        if let Some(pos) = prefix.rfind('\\') {
            let cmd_prefix = &prefix[pos + 1..];
            if !cmd_prefix.is_empty() && cmd_prefix.chars().all(|c| c.is_alphabetic()) {
                self.open_command_completions(cmd_prefix);
                return;
            }
        }

        self.completion.close();
    }

    fn open_ref_completions(&mut self, query: &str) {
        let current_file = self.current_file_path.as_deref();
        let matches = self.backend.fuzzy_search_labels(query, current_file).unwrap_or_default();

        let items: Vec<CompletionItem> = matches
            .into_iter()
            .map(|m| {
                let name = m.obj.get("key").and_then(|n| n.as_str())
                    .or_else(|| m.obj.get("name").and_then(|n| n.as_str()))
                    .unwrap_or("").to_string();
                let detail = m.obj.get("file").and_then(|f| f.as_str()).map(|s| {
                    let base = std::path::Path::new(s).file_name().and_then(|n| n.to_str()).unwrap_or(s);
                    let line = m.obj.get("line").and_then(|l| l.as_u64()).unwrap_or(0);
                    format!("{}:{}", base, line)
                });

                CompletionItem {
                    label: name.clone(),
                    kind: CompletionKind::Reference,
                    detail,
                    documentation: None,
                    insert_text: format!("{}}}", name),
                }
            })
            .collect();

        if !items.is_empty() {
            self.completion.open(items, self.cursor_row, self.cursor_col, query.to_string());
        } else {
            self.completion.close();
        }
    }

    fn open_cite_completions(&mut self, query: &str) {
        let current_file = self.current_file_path.as_deref();
        let matches = self.backend.fuzzy_search_citations(query, current_file).unwrap_or_default();

        let items: Vec<CompletionItem> = matches
            .into_iter()
            .map(|m| {
                let key = m.obj.get("key").and_then(|k| k.as_str()).unwrap_or("").to_string();
                let fields = m.obj.get("fields");
                let title = fields.and_then(|f| f.get("title")).and_then(|t| t.as_str());
                let author = fields.and_then(|f| f.get("author")).and_then(|a| a.as_str());
                let year = fields.and_then(|f| f.get("year")).and_then(|y| y.as_str());
                let journal = fields.and_then(|f| f.get("journal")).and_then(|j| j.as_str())
                    .or_else(|| fields.and_then(|f| f.get("booktitle")).and_then(|b| b.as_str()));

                let detail = match (author, year) {
                    (Some(a), Some(y)) => {
                        let first_author = a.split(" and ").next().unwrap_or(a);
                        let a_short = if a.contains(" and ") {
                            format!("{} et al.", first_author)
                        } else {
                            first_author.to_string()
                        };
                        Some(format!("{}, {}", a_short, y))
                    }
                    (Some(a), None) => Some(a.to_string()),
                    _ => title.map(|t| t.to_string()),
                };

                let doc = match (title, journal, year) {
                    (Some(t), Some(j), Some(y)) => Some(format!("{}\n{} ({})", t, j, y)),
                    (Some(t), Some(j), None) => Some(format!("{}\n{}", t, j)),
                    (Some(t), None, Some(y)) => Some(format!("{} ({})", t, y)),
                    (Some(t), None, None) => Some(t.to_string()),
                    _ => None,
                };

                CompletionItem {
                    label: key.clone(),
                    kind: CompletionKind::Citation,
                    detail,
                    documentation: doc,
                    insert_text: format!("{}}}", key),
                }
            })
            .collect();

        if !items.is_empty() {
            self.completion.open(items, self.cursor_row, self.cursor_col, query.to_string());
        } else {
            self.completion.close();
        }
    }


    fn open_env_completions(&mut self, query: &str) {
        let common_envs = [
            ("figure", "Figure environment with caption and label"),
            ("table", "Table environment"),
            ("tabular", "Tabular data grid"),
            ("equation", "Numbered single equation"),
            ("align", "Multi-line aligned equations"),
            ("itemize", "Bullet point list"),
            ("enumerate", "Numbered list"),
            ("matrix", "Matrix environment"),
            ("pmatrix", "Matrix with parentheses"),
            ("bmatrix", "Matrix with brackets"),
            ("proof", "Mathematical proof"),
            ("theorem", "Theorem environment"),
            ("lemma", "Lemma environment"),
            ("definition", "Definition environment"),
        ];

        let query_lower = query.to_lowercase();
        let items: Vec<CompletionItem> = common_envs
            .iter()
            .filter(|(env, _)| env.to_lowercase().contains(&query_lower))
            .map(|(env, desc)| CompletionItem {
                label: env.to_string(),
                kind: CompletionKind::Environment,
                detail: Some(desc.to_string()),
                documentation: None,
                insert_text: format!("{}}}\n  \n\\end{{{}}}", env, env),
            })
            .collect();

        if !items.is_empty() {
            self.completion.open(items, self.cursor_row, self.cursor_col, query.to_string());
        } else {
            self.completion.close();
        }
    }

    fn open_command_completions(&mut self, query: &str) {
        let common_cmds = [
            ("section", "\\section{title}", "section{}"),
            ("subsection", "\\subsection{title}", "subsection{}"),
            ("subsubsection", "\\subsubsection{title}", "subsubsection{}"),
            ("textbf", "\\textbf{bold text}", "textbf{}"),
            ("textit", "\\textit{italic text}", "textit{}"),
            ("texttt", "\\texttt{monospace text}", "texttt{}"),
            ("underline", "\\underline{text}", "underline{}"),
            ("label", "\\label{marker}", "label{}"),
            ("ref", "\\ref{marker}", "ref{}"),
            ("cite", "\\cite{key}", "cite{}"),
            ("frac", "\\frac{num}{den}", "frac{}{}"),
            ("sqrt", "\\sqrt{x}", "sqrt{}"),
            ("int", "\\int_{a}^{b}", "int_{}^{}"),
            ("sum", "\\sum_{i=1}^{n}", "sum_{}^{}"),
            ("includegraphics", "\\includegraphics[width=\\linewidth]{file}", "includegraphics[]{}"),
        ];

        let query_lower = query.to_lowercase();
        let items: Vec<CompletionItem> = common_cmds
            .iter()
            .filter(|(cmd, _, _)| cmd.to_lowercase().starts_with(&query_lower))
            .map(|(cmd, desc, ins)| CompletionItem {
                label: format!("\\{}", cmd),
                kind: CompletionKind::Command,
                detail: Some(desc.to_string()),
                documentation: None,
                insert_text: ins.to_string(),
            })
            .collect();

        if !items.is_empty() {
            self.completion.open(items, self.cursor_row, self.cursor_col, query.to_string());
        } else {
            self.completion.close();
        }
    }

    pub fn accept_completion(&mut self, item: &CompletionItem) {
        self.push_undo();

        let row = self.cursor_row;
        let col = self.cursor_col;
        let line = self.lines[row].clone();

        let query_len = self.completion.query.chars().count();
        let start_col = col.saturating_sub(query_len);

        let chars: Vec<char> = line.chars().collect();
        let before: String = chars[..start_col.min(chars.len())].iter().collect();
        let mut after_str: String = if col < chars.len() {
            chars[col..].iter().collect()
        } else {
            String::new()
        };

        if (item.insert_text.contains('}') || item.kind == CompletionKind::Environment) && after_str.starts_with('}') {
            after_str = after_str[1..].to_string();
        }

        if item.insert_text.contains('\n') {
            let indent: String = before.chars().take_while(|c| c.is_whitespace()).collect();

            let parts: Vec<&str> = item.insert_text.split('\n').collect();
            let first_line = format!("{}{}", before, parts[0]);

            self.lines[row] = first_line;

            for (i, middle_part) in parts[1..parts.len() - 1].iter().enumerate() {
                self.lines.insert(row + 1 + i, format!("{}{}", indent, middle_part));
            }

            let last_line = format!("{}{}{}", indent, parts[parts.len() - 1], after_str);
            self.lines.insert(row + parts.len() - 1, last_line);

            if parts.len() >= 3 {
                self.cursor_row = row + 1;
                self.cursor_col = self.lines[row + 1].chars().count();
            } else {
                self.cursor_row = row + parts.len() - 1;
                self.cursor_col = self.lines[self.cursor_row].chars().count();
            }
        } else {
            let new_line = format!("{}{}{}", before, item.insert_text, after_str);
            self.lines[row] = new_line;

            if item.kind == CompletionKind::Command && item.insert_text.ends_with("{}") {
                self.cursor_col = start_col + item.insert_text.len() - 1;
            } else if item.kind == CompletionKind::Command && item.insert_text.ends_with("[]{}") {
                self.cursor_col = start_col + item.insert_text.find('[').unwrap_or(0) + 1;
            } else if item.kind == CompletionKind::Command && item.insert_text.ends_with("{}{}") {
                self.cursor_col = start_col + item.insert_text.find('{').unwrap_or(0) + 1;
            } else {
                self.cursor_col = start_col + item.insert_text.chars().count();
            }
        }

        self.clear_selection();
        self.completion.close();
        self.ensure_cursor_visible();
    }

    fn highlight_line(&self, text: &str) -> Vec<HighlightSpan> {
        let mut spans = Vec::new();
        if text.is_empty() {
            return spans;
        }

        let mut chars = text.chars().peekable();
        let mut current_token = String::new();
        let mut in_comment = false;
        let mut in_math = false;

        while let Some(ch) = chars.next() {
            if in_comment {
                current_token.push(ch);
                continue;
            }

            if ch == '%' {
                if !current_token.is_empty() {
                    spans.push(HighlightSpan {
                        text: current_token.clone(),
                        color: if in_math { Theme::syn_math() } else { Theme::text_primary() },
                        is_bold: false,
                        is_italic: false,
                    });
                    current_token.clear();
                }
                in_comment = true;
                current_token.push('%');
                continue;
            }

            if ch == '$' {
                if !current_token.is_empty() {
                    spans.push(HighlightSpan {
                        text: current_token.clone(),
                        color: if in_math { Theme::syn_math() } else { Theme::text_primary() },
                        is_bold: false,
                        is_italic: false,
                    });
                    current_token.clear();
                }
                spans.push(HighlightSpan {
                    text: "$".into(),
                    color: Theme::syn_math(),
                    is_bold: true,
                    is_italic: false,
                });
                in_math = !in_math;
                continue;
            }

            if ch == '\\' {
                if !current_token.is_empty() {
                    spans.push(HighlightSpan {
                        text: current_token.clone(),
                        color: if in_math { Theme::syn_math() } else { Theme::text_primary() },
                        is_bold: false,
                        is_italic: false,
                    });
                    current_token.clear();
                }

                let mut cmd = String::from("\\");
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphabetic() {
                        cmd.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                let is_env = cmd == "\\begin" || cmd == "\\end";
                let is_sec = cmd == "\\section" || cmd == "\\subsection" || cmd == "\\subsubsection";

                spans.push(HighlightSpan {
                    text: cmd,
                    color: if is_env {
                        Theme::syn_environment()
                    } else if is_sec {
                        Theme::accent_yellow()
                    } else {
                        Theme::syn_command()
                    },
                    is_bold: is_sec || is_env,
                    is_italic: false,
                });
                continue;
            }

            if ch == '{' || ch == '}' {
                if !current_token.is_empty() {
                    spans.push(HighlightSpan {
                        text: current_token.clone(),
                        color: if in_math { Theme::syn_math() } else { Theme::text_primary() },
                        is_bold: false,
                        is_italic: false,
                    });
                    current_token.clear();
                }

                spans.push(HighlightSpan {
                    text: ch.to_string(),
                    color: Theme::syn_bracket(),
                    is_bold: true,
                    is_italic: false,
                });
                continue;
            }

            if ch == '[' || ch == ']' {
                if !current_token.is_empty() {
                    spans.push(HighlightSpan {
                        text: current_token.clone(),
                        color: if in_math { Theme::syn_math() } else { Theme::text_primary() },
                        is_bold: false,
                        is_italic: false,
                    });
                    current_token.clear();
                }

                spans.push(HighlightSpan {
                    text: ch.to_string(),
                    color: Theme::syn_optional_arg(),
                    is_bold: false,
                    is_italic: false,
                });
                continue;
            }

            current_token.push(ch);
        }

        if !current_token.is_empty() {
            spans.push(HighlightSpan {
                text: current_token,
                color: if in_comment {
                    Theme::syn_comment()
                } else if in_math {
                    Theme::syn_math()
                } else {
                    Theme::text_primary()
                },
                is_bold: false,
                is_italic: in_comment,
            });
        }

        spans
    }
}

impl Render for LatexEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let win_w = f32::from(window.viewport_size().width);
        let wrap_cols = self.calculate_wrap_cols(win_w);
        self.last_wrap_cols = wrap_cols;

        let visual_lines = self.compute_visual_lines(wrap_cols);
        let total_v_lines = visual_lines.len();

        let cursor_r = self.cursor_row;
        let cursor_c = self.cursor_col;

        let cursor_v_idx = Self::find_visual_line_index(&visual_lines, cursor_r, cursor_c);
        let current_vl = visual_lines[cursor_v_idx];
        let cursor_col_in_seg = cursor_c.saturating_sub(current_vl.start_col);

        let gutter_width = px(48.0);
        let cursor_x = gutter_width + px(12.0) + px(cursor_col_in_seg as f32 * CHAR_WIDTH);
        let cursor_screen_y = px((cursor_v_idx as f32 * LINE_HEIGHT) - self.scroll_top_px);

        let scroll_y = self.scroll_top_px;
        let first_visible_v_row = (scroll_y / LINE_HEIGHT).floor() as usize;
        let subpixel_y = scroll_y - (first_visible_v_row as f32 * LINE_HEIGHT);

        let start_v_row = first_visible_v_row.min(total_v_lines.saturating_sub(1));
        let end_v_row = (first_visible_v_row + 55).min(total_v_lines);

        let completion_open = self.completion.is_open;
        let completion_state = self.completion.clone();
        let view_handle = cx.entity().clone();
        let norm_sel = self.normalized_selection();
        let has_sel = norm_sel.is_some();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(Theme::bg_editor())
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| {
                window.focus(&this.focus_handle);
                let mouse_x = f32::from(event.position.x);
                let mouse_y = f32::from(event.position.y);
                let sidebar_w = if this.sidebar_visible { 240.0 } else { 0.0 };
                let win_w = f32::from(window.viewport_size().width);
                let pane_w = if this.has_right_pane { (win_w - sidebar_w).max(200.0) / 2.0 } else { (win_w - sidebar_w).max(200.0) };
                let ed_x = if this.is_right_pane { sidebar_w + pane_w } else { sidebar_w };
                let t_start_x = ed_x + 48.0 + 12.0;
                let ed_y = 38.0 + 32.0 + 8.0;

                let wrap_cols = this.calculate_wrap_cols(win_w);
                this.last_wrap_cols = wrap_cols;
                let visual_lines = this.compute_visual_lines(wrap_cols);

                let line_y = mouse_y - ed_y + this.scroll_top_px;
                let max_v_row = visual_lines.len().saturating_sub(1);
                let click_v_row = ((line_y / LINE_HEIGHT).floor() as isize).clamp(0, max_v_row as isize) as usize;
                let vl = visual_lines[click_v_row];
                let seg_len = vl.end_col - vl.start_col;
                let click_col_in_seg = (((mouse_x - t_start_x) / CHAR_WIDTH).max(0.0).round() as usize).min(seg_len);
                let click_row = vl.buffer_row;
                let click_col = vl.start_col + click_col_in_seg;

                if event.click_count == 2 {
                    this.select_word_at(click_row, click_col);
                    this.is_mouse_dragging = false;
                } else if event.click_count >= 3 {
                    this.select_line_at(click_row);
                    this.is_mouse_dragging = false;
                } else if event.modifiers.shift {
                    let anchor = this.selection_anchor.unwrap_or((this.cursor_row, this.cursor_col));
                    this.set_selection(anchor, (click_row, click_col));
                    this.is_mouse_dragging = true;
                } else {
                    this.cursor_row = click_row;
                    this.cursor_col = click_col;
                    this.selection_anchor = Some((click_row, click_col));
                    this.selection = None;
                    this.is_mouse_dragging = true;
                    this.ensure_cursor_visible();
                }

                this.completion.close();
                cx.notify();
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                if !this.is_mouse_dragging || event.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                let mouse_x = f32::from(event.position.x);
                let mouse_y = f32::from(event.position.y);
                let sidebar_w = if this.sidebar_visible { 240.0 } else { 0.0 };
                let win_w = f32::from(window.viewport_size().width);
                let pane_w = if this.has_right_pane { (win_w - sidebar_w).max(200.0) / 2.0 } else { (win_w - sidebar_w).max(200.0) };
                let ed_x = if this.is_right_pane { sidebar_w + pane_w } else { sidebar_w };
                let t_start_x = ed_x + 48.0 + 12.0;
                let ed_y = 38.0 + 32.0 + 8.0;

                let wrap_cols = this.calculate_wrap_cols(win_w);
                this.last_wrap_cols = wrap_cols;
                let visual_lines = this.compute_visual_lines(wrap_cols);

                let line_y = mouse_y - ed_y + this.scroll_top_px;
                let max_v_row = visual_lines.len().saturating_sub(1);
                let hover_v_row = ((line_y / LINE_HEIGHT).floor() as isize).clamp(0, max_v_row as isize) as usize;
                let vl = visual_lines[hover_v_row];
                let seg_len = vl.end_col - vl.start_col;
                let hover_col_in_seg = (((mouse_x - t_start_x) / CHAR_WIDTH).max(0.0).round() as usize).min(seg_len);
                let hover_row = vl.buffer_row;
                let hover_col = vl.start_col + hover_col_in_seg;

                let anchor = this.selection_anchor.unwrap_or((this.cursor_row, this.cursor_col));
                this.set_selection(anchor, (hover_row, hover_col));
                this.ensure_cursor_visible();
                cx.notify();
            }))

            .on_mouse_up(MouseButton::Left, cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                this.is_mouse_dragging = false;
                cx.notify();
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                let delta_y: f32 = match event.delta {
                    ScrollDelta::Pixels(p) => f32::from(p.y),
                    ScrollDelta::Lines(l) => l.y * LINE_HEIGHT,
                };

                let win_w = f32::from(window.viewport_size().width);
                let wrap_cols = this.calculate_wrap_cols(win_w);
                this.last_wrap_cols = wrap_cols;
                let visual_lines = this.compute_visual_lines(wrap_cols);
                let total_v_lines = visual_lines.len();
                let max_scroll = (total_v_lines.saturating_sub(5) as f32 * LINE_HEIGHT).max(0.0);

                this.scroll_top_px = (this.scroll_top_px - delta_y).clamp(0.0, max_scroll);
                this.completion.close();
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                let is_shift = event.keystroke.modifiers.shift;
                let is_ctrl = event.keystroke.modifiers.control;
                let is_alt = event.keystroke.modifiers.alt; // Option on macOS, Alt on Windows/Linux
                let is_platform = event.keystroke.modifiers.platform; // Cmd on macOS, Super/Win on Linux/Windows
                let is_cmd_or_ctrl = is_platform || is_ctrl;
                let is_macos = cfg!(target_os = "macos");

                // 1. Completion popup navigation
                if this.completion.is_open {
                    match key {
                        "up" => {
                            this.completion.select_prev();
                            cx.notify();
                            return;
                        }
                        "down" => {
                            this.completion.select_next();
                            cx.notify();
                            return;
                        }
                        "enter" | "tab" => {
                            if let Some(item) = this.completion.current_item().cloned() {
                                this.accept_completion(&item);
                                cx.notify();
                                return;
                            }
                        }
                        "escape" => {
                            this.completion.close();
                            cx.notify();
                            return;
                        }
                        _ => {}
                    }
                }

                // 2. Command / Control Key Shortcuts (Ctrl on Windows/Linux, Cmd on macOS)
                if is_cmd_or_ctrl && !is_alt {
                    match key {
                        "a" => {
                            this.select_all();
                            cx.notify();
                            return;
                        }
                        "c" => {
                            if let Some(text) = this.get_selected_text() {
                                cx.write_to_clipboard(ClipboardItem::new_string(text));
                            } else {
                                let line = format!("{}\n", this.current_line());
                                cx.write_to_clipboard(ClipboardItem::new_string(line));
                            }
                            cx.notify();
                            return;
                        }
                        "x" => {
                            if let Some(text) = this.get_selected_text() {
                                cx.write_to_clipboard(ClipboardItem::new_string(text));
                                this.delete_selection();
                            } else {
                                let line = format!("{}\n", this.current_line());
                                cx.write_to_clipboard(ClipboardItem::new_string(line));
                                this.push_undo();
                                if this.lines.len() > 1 {
                                    this.lines.remove(this.cursor_row);
                                    if this.cursor_row >= this.lines.len() {
                                        this.cursor_row = this.lines.len() - 1;
                                    }
                                } else {
                                    this.lines[0].clear();
                                }
                                this.cursor_col = 0;
                            }
                            cx.notify();
                            return;
                        }
                        "v" => {
                            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                                this.paste_text(&text);
                            }
                            cx.notify();
                            return;
                        }
                        "z" => {
                            if is_shift {
                                this.redo();
                            } else {
                                this.undo();
                            }
                            cx.notify();
                            return;
                        }
                        "y" => {
                            this.redo();
                            cx.notify();
                            return;
                        }
                        "/" => {
                            this.toggle_comment();
                            cx.notify();
                            return;
                        }
                        "d" => {
                            this.duplicate_line_or_selection();
                            cx.notify();
                            return;
                        }
                        _ => {}
                    }
                }

                // 3. Navigation & Editing with Left, Right, Up, Down, Home, End, PageUp, PageDown
                match key {
                    "left" => {
                        let to_line_start = if is_macos { is_platform } else { false };
                        let by_word = if is_macos { is_alt || is_ctrl } else { is_ctrl || is_alt };
                        this.move_left(is_shift, by_word, to_line_start);
                        this.completion.close();
                    }
                    "right" => {
                        let to_line_end = if is_macos { is_platform } else { false };
                        let by_word = if is_macos { is_alt || is_ctrl } else { is_ctrl || is_alt };
                        this.move_right(is_shift, by_word, to_line_end);
                        this.completion.close();
                    }
                    "up" => {
                        let to_doc_start = if is_macos { is_platform } else { false };
                        let by_paragraph = if is_macos { is_alt } else { is_ctrl || is_alt };
                        let win_w = f32::from(window.viewport_size().width);
                        let wrap_cols = this.calculate_wrap_cols(win_w);
                        this.last_wrap_cols = wrap_cols;
                        this.move_up_visual(is_shift, to_doc_start, by_paragraph, wrap_cols);
                        this.completion.close();
                    }
                    "down" => {
                        let to_doc_end = if is_macos { is_platform } else { false };
                        let by_paragraph = if is_macos { is_alt } else { is_ctrl || is_alt };
                        let win_w = f32::from(window.viewport_size().width);
                        let wrap_cols = this.calculate_wrap_cols(win_w);
                        this.last_wrap_cols = wrap_cols;
                        this.move_down_visual(is_shift, to_doc_end, by_paragraph, wrap_cols);
                        this.completion.close();
                    }
                    "home" => {
                        let to_doc_start = is_platform || is_ctrl;
                        if to_doc_start {
                            this.move_cursor(0, 0, is_shift);
                        } else {
                            this.move_left(is_shift, false, true);
                        }
                        this.completion.close();
                    }
                    "end" => {
                        let to_doc_end = is_platform || is_ctrl;
                        if to_doc_end {
                            let last_row = this.lines.len().saturating_sub(1);
                            let last_col = this.lines[last_row].chars().count();
                            this.move_cursor(last_row, last_col, is_shift);
                        } else {
                            this.move_right(is_shift, false, true);
                        }
                        this.completion.close();
                    }
                    "pageup" => {
                        let win_w = f32::from(window.viewport_size().width);
                        let wrap_cols = this.calculate_wrap_cols(win_w);
                        this.last_wrap_cols = wrap_cols;
                        let visual_lines = this.compute_visual_lines(wrap_cols);
                        let current_v = Self::find_visual_line_index(&visual_lines, this.cursor_row, this.cursor_col);
                        let target_v = current_v.saturating_sub(25);
                        let vl = visual_lines[target_v];
                        let col = vl.start_col;
                        this.move_cursor(vl.buffer_row, col, is_shift);
                        this.completion.close();
                    }
                    "pagedown" => {
                        let win_w = f32::from(window.viewport_size().width);
                        let wrap_cols = this.calculate_wrap_cols(win_w);
                        this.last_wrap_cols = wrap_cols;
                        let visual_lines = this.compute_visual_lines(wrap_cols);
                        let current_v = Self::find_visual_line_index(&visual_lines, this.cursor_row, this.cursor_col);
                        let target_v = (current_v + 25).min(visual_lines.len().saturating_sub(1));
                        let vl = visual_lines[target_v];
                        let col = vl.start_col;
                        this.move_cursor(vl.buffer_row, col, is_shift);
                        this.completion.close();
                    }
                    "enter" => {
                        if this.has_selection() {
                            this.delete_selection();
                        }
                        this.insert_newline();
                    }
                    "backspace" => {
                        if this.has_selection() {
                            this.delete_selection();
                        } else if is_macos && is_platform {
                            this.delete_to_line_start();
                        } else if is_ctrl || is_alt {
                            this.delete_prev_word();
                        } else {
                            this.delete_backwards();
                        }
                    }
                    "delete" => {
                        if this.has_selection() {
                            this.delete_selection();
                        } else if is_ctrl || is_alt || (is_macos && is_platform) {
                            this.delete_next_word();
                        } else {
                            this.delete_forward();
                        }
                    }
                    "tab" => {
                        if is_shift {
                            this.indent_lines(true);
                        } else if this.has_selection() {
                            let (s, e) = this.normalized_selection().unwrap();
                            if s.0 != e.0 {
                                this.indent_lines(false);
                            } else {
                                this.delete_selection();
                                this.insert_text("  ");
                            }
                        } else {
                            this.insert_text("  ");
                        }
                    }
                    _ => {
                        if key.chars().count() == 1 && !is_cmd_or_ctrl && !is_alt {
                            if this.has_selection() {
                                let (open, close) = match key {
                                    "$" => ("$", "$"),
                                    "{" | "}" => ("{", "}"),
                                    "(" | ")" => ("(", ")"),
                                    "[" | "]" => ("[", "]"),
                                    "\"" => ("\"", "\""),
                                    _ => ("", ""),
                                };
                                if !open.is_empty() {
                                    if let Some(selected) = this.get_selected_text() {
                                        this.delete_selection();
                                        this.insert_text(&format!("{}{}{}", open, selected, close));
                                        cx.notify();
                                        return;
                                    }
                                }
                                this.delete_selection();
                            }
                            this.insert_text(key);
                        }
                    }
                }
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .relative()
                    .overflow_hidden()
                    .child(
                        div()
                            .absolute()
                            .top(px(-subpixel_y))
                            .left_0()
                            .right_0()
                            .flex()
                            .child(
                                // Gutter (line numbers)
                                div()
                                    .w(gutter_width)
                                    .bg(Theme::bg_gutter())
                                    .border_r_1()
                                    .border_color(Theme::border_subtle())
                                    .flex()
                                    .flex_col()
                                    .py_2()
                                    .children((start_v_row..end_v_row).map(|v_idx| {
                                        let vl = &visual_lines[v_idx];
                                        let is_active = vl.buffer_row == cursor_r;
                                        let line_num_str = if vl.wrap_idx == 0 {
                                            format!("{}", vl.buffer_row + 1)
                                        } else {
                                            String::new()
                                        };
                                        div()
                                            .h(px(LINE_HEIGHT))
                                            .px_2()
                                            .flex()
                                            .justify_end()
                                            .items_center()
                                            .text_xs()
                                            .font_family(".AppleSystemUIFontMonospaced")
                                            .text_color(if is_active { Theme::line_num_active() } else { Theme::line_num_inactive() })
                                            .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
                                            .child(line_num_str)
                                    })),
                            )
                            .child(
                                // Code editor area
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .py_2()
                                    .px_3()
                                    .children((start_v_row..end_v_row).map(|v_idx| {
                                        let vl = &visual_lines[v_idx];
                                        let is_active_row = vl.buffer_row == cursor_r;
                                        let is_active_v_line = v_idx == cursor_v_idx;
                                        let line_text = &self.lines[vl.buffer_row];
                                        let all_spans = self.highlight_line(line_text);
                                        let spans = Self::slice_spans(&all_spans, vl.start_col, vl.end_col);

                                        let sel_span: Option<(Pixels, Pixels)> = if let Some(((s_r, s_c), (e_r, e_c))) = norm_sel {
                                            if vl.buffer_row < s_r || vl.buffer_row > e_r {
                                                None
                                            } else if s_r == e_r {
                                                let seg_s = s_c.clamp(vl.start_col, vl.end_col);
                                                let seg_e = e_c.clamp(vl.start_col, vl.end_col);
                                                if seg_s < seg_e {
                                                    let left = px((seg_s - vl.start_col) as f32 * CHAR_WIDTH);
                                                    let width = px((seg_e - seg_s) as f32 * CHAR_WIDTH);
                                                    Some((left, width))
                                                } else {
                                                    None
                                                }
                                            } else if vl.buffer_row == s_r {
                                                let seg_s = s_c.clamp(vl.start_col, vl.end_col);
                                                let seg_e = vl.end_col;
                                                if seg_s < seg_e {
                                                    let left = px((seg_s - vl.start_col) as f32 * CHAR_WIDTH);
                                                    let width = px((seg_e - seg_s) as f32 * CHAR_WIDTH);
                                                    Some((left, width))
                                                } else {
                                                    None
                                                }
                                            } else if vl.buffer_row == e_r {
                                                let seg_s = vl.start_col;
                                                let seg_e = e_c.clamp(vl.start_col, vl.end_col);
                                                if seg_s < seg_e {
                                                    let left = px(0.0);
                                                    let width = px((seg_e - seg_s) as f32 * CHAR_WIDTH);
                                                    Some((left, width))
                                                } else {
                                                    None
                                                }
                                            } else {
                                                let left = px(0.0);
                                                let width = px(((vl.end_col - vl.start_col) as f32 + 1.0) * CHAR_WIDTH).max(px(14.0));
                                                Some((left, width))
                                            }
                                        } else {
                                            None
                                        };

                                        div()
                                            .h(px(LINE_HEIGHT))
                                            .relative()
                                            .flex()
                                            .items_center()
                                            .cursor_text()
                                            .when(is_active_row && !has_sel, |d| d.bg(Theme::bg_current_line()))
                                            .when_some(sel_span, |d, (left, width)| {
                                                d.child(
                                                    div()
                                                        .absolute()
                                                        .left(left)
                                                        .top_0()
                                                        .bottom_0()
                                                        .w(width)
                                                        .bg(Theme::selection())
                                                        .rounded_xs(),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .font_family(".AppleSystemUIFontMonospaced")
                                                    .text_size(px(FONT_SIZE))
                                                    .children(spans.into_iter().map(|span| {
                                                        div()
                                                            .text_color(span.color)
                                                            .font_weight(if span.is_bold { FontWeight::BOLD } else { FontWeight::NORMAL })
                                                            .child(span.text)
                                                    })),
                                            )
                                            .when(is_active_v_line, |d| {
                                                // Blinking cursor
                                                let col_offset = px((cursor_c.saturating_sub(vl.start_col)) as f32 * CHAR_WIDTH);
                                                d.child(
                                                    div()
                                                        .absolute()
                                                        .left(col_offset)
                                                        .w(px(2.0))
                                                        .h(px(18.0))
                                                        .bg(Theme::caret()),
                                                )
                                            })
                                    })),

                            ),
                    )
                    .when(completion_open, move |d| {
                        let view = view_handle.clone();
                        d.child(render_completion_popup(
                            &completion_state,
                            cursor_x,
                            cursor_screen_y,
                            move |item, _window, cx| {
                                let item_clone = item.clone();
                                view.update(cx, |this, cx| {
                                    this.accept_completion(&item_clone);
                                    cx.notify();
                                });
                            },
                        ))
                    }),
            )
    }
}
