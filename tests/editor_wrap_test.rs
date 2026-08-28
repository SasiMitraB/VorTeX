use gpui::hsla;
use vortex::views::editor::latex_editor::{HighlightSpan, LatexEditor, VisualLine};

#[test]
fn test_empty_and_short_line_wrapping() {
    // Empty line should yield exactly 1 visual segment (0, 0)
    let segments = LatexEditor::wrap_line_to_segments("", 50);
    assert_eq!(segments, vec![(0, 0)]);

    // Line shorter than max_cols should not wrap
    let short_line = "\\begin{document}";
    let segments = LatexEditor::wrap_line_to_segments(short_line, 50);
    assert_eq!(segments, vec![(0, short_line.chars().count())]);
}

#[test]
fn test_long_line_word_boundary_wrapping() {
    let line = "This is a long sentence in LaTeX that needs to wrap properly across multiple lines.";
    let max_cols = 25;
    let segments = LatexEditor::wrap_line_to_segments(line, max_cols);

    // Verify all segments are <= max_cols
    for &(start, end) in &segments {
        assert!(end - start <= max_cols, "Segment length {} exceeded max_cols {}", end - start, max_cols);
    }

    // Verify character count preservation (no lost or duplicated characters)
    let chars: Vec<char> = line.chars().collect();
    let mut reconstructed = String::new();
    for (start, end) in segments {
        reconstructed.push_str(&chars[start..end].iter().collect::<String>());
    }
    assert_eq!(reconstructed, line);
}

#[test]
fn test_long_continuous_token_hard_breaking() {
    let line = "https://example.com/very/long/unbroken/url/path/that/has/no/spaces/at/all/in/the/middle";
    let max_cols = 20;
    let segments = LatexEditor::wrap_line_to_segments(line, max_cols);

    for &(start, end) in &segments {
        assert!(end - start <= max_cols);
    }

    let chars: Vec<char> = line.chars().collect();
    let mut reconstructed = String::new();
    for (start, end) in segments {
        reconstructed.push_str(&chars[start..end].iter().collect::<String>());
    }
    assert_eq!(reconstructed, line);
}

#[test]
fn test_visual_line_index_lookup() {
    // Line 0 wraps into 2 segments: (0..20) and (20..35)
    // Line 1 has 1 segment: (0..10)
    let visual_lines = vec![
        VisualLine { buffer_row: 0, wrap_idx: 0, start_col: 0, end_col: 20 },
        VisualLine { buffer_row: 0, wrap_idx: 1, start_col: 20, end_col: 35 },
        VisualLine { buffer_row: 1, wrap_idx: 0, start_col: 0, end_col: 10 },
    ];

    // Col 0 on buffer line 0 -> visual line 0
    assert_eq!(LatexEditor::find_visual_line_index(&visual_lines, 0, 0), 0);
    // Col 15 on buffer line 0 -> visual line 0
    assert_eq!(LatexEditor::find_visual_line_index(&visual_lines, 0, 15), 0);
    // Col 20 (boundary) on buffer line 0 -> visual line 1 (start of wrapped continuation)
    assert_eq!(LatexEditor::find_visual_line_index(&visual_lines, 0, 20), 1);
    // Col 30 on buffer line 0 -> visual line 1
    assert_eq!(LatexEditor::find_visual_line_index(&visual_lines, 0, 30), 1);
    // Col 5 on buffer line 1 -> visual line 2
    assert_eq!(LatexEditor::find_visual_line_index(&visual_lines, 1, 5), 2);
}

#[test]
fn test_slice_spans_for_syntax_highlighting() {
    let spans = vec![
        HighlightSpan {
            text: "\\section{".to_string(),
            color: hsla(0.6, 0.8, 0.6, 1.0),
            is_bold: true,
            is_italic: false,
        },
        HighlightSpan {
            text: "Introduction".to_string(),
            color: hsla(0.3, 0.8, 0.6, 1.0),
            is_bold: true,
            is_italic: false,
        },
        HighlightSpan {
            text: "}".to_string(),
            color: hsla(0.6, 0.8, 0.6, 1.0),
            is_bold: true,
            is_italic: false,
        },
    ];

    // Slice from col 0 to 15: covers all of "\\section{" (9) + first 6 chars of "Introduction" ("Introd")
    let sliced = LatexEditor::slice_spans(&spans, 0, 15);
    assert_eq!(sliced.len(), 2);
    assert_eq!(sliced[0].text, "\\section{");
    assert_eq!(sliced[1].text, "Introd");

    // Slice from col 15 to 22: covers remaining 6 chars of "Introduction" ("uction") + "}" (1)
    let sliced2 = LatexEditor::slice_spans(&spans, 15, 22);
    assert_eq!(sliced2.len(), 2);
    assert_eq!(sliced2[0].text, "uction");
    assert_eq!(sliced2[1].text, "}");
}
