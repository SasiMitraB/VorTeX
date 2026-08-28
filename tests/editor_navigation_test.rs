use vortex::views::editor::LatexEditor;

#[test]
fn test_word_boundaries() {
    let line = "\\section{Introduction to LaTeX}";
    // Indices:
    // 0: '\'
    // 1..8: "section"
    // 8: '{'
    // 9..21: "Introduction"
    // 21: ' '
    // 22..24: "to"
    // 24: ' '
    // 25..30: "LaTeX"
    // 30: '}'

    // Forward word navigation (Option + Right)
    assert_eq!(LatexEditor::next_word_boundary(line, 0), 1); // after '\'
    assert_eq!(LatexEditor::next_word_boundary(line, 1), 8); // after "section"
    assert_eq!(LatexEditor::next_word_boundary(line, 8), 9); // after '{'
    assert_eq!(LatexEditor::next_word_boundary(line, 9), 21); // after "Introduction"
    assert_eq!(LatexEditor::next_word_boundary(line, 21), 24); // skips space, after "to"
    assert_eq!(LatexEditor::next_word_boundary(line, 24), 30); // skips space, after "LaTeX"
    assert_eq!(LatexEditor::next_word_boundary(line, 30), 31); // after '}'

    // Backward word navigation (Option + Left)
    assert_eq!(LatexEditor::prev_word_boundary(line, 31), 30); // before '}'
    assert_eq!(LatexEditor::prev_word_boundary(line, 30), 25); // before "LaTeX"
    assert_eq!(LatexEditor::prev_word_boundary(line, 25), 22); // skips space, before "to"
    assert_eq!(LatexEditor::prev_word_boundary(line, 22), 9); // skips space, before "Introduction"
    assert_eq!(LatexEditor::prev_word_boundary(line, 9), 8); // before '{'
    assert_eq!(LatexEditor::prev_word_boundary(line, 8), 1); // before "section"
    assert_eq!(LatexEditor::prev_word_boundary(line, 1), 0); // before '\'
}

#[test]
fn test_find_word_range_at_double_click() {
    let line = "\\textbf{VorTeX Editor}";

    // Inside "textbf" (col 3)
    let (s, e) = LatexEditor::find_word_range_at(line, 3);
    assert_eq!(&line[s..e], "textbf");

    // Inside "VorTeX" (col 10)
    let (s, e) = LatexEditor::find_word_range_at(line, 10);
    assert_eq!(&line[s..e], "VorTeX");

    // Inside "Editor" (col 18)
    let (s, e) = LatexEditor::find_word_range_at(line, 18);
    assert_eq!(&line[s..e], "Editor");
}

#[test]
fn test_math_formula_word_boundaries() {
    let line = "$E = mc^2 + \\alpha$";
    // 0: '$'
    // 1: 'E'
    // 2: ' '
    // 3: '='
    // 4: ' '
    // 5..7: "mc"
    // 7: '^'
    // 8: '2'
    // 9: ' '
    // 10: '+'
    // 11: ' '
    // 12: '\'
    // 13..18: "alpha"
    // 18: '$'

    assert_eq!(LatexEditor::next_word_boundary(line, 0), 1); // after '$'
    assert_eq!(LatexEditor::next_word_boundary(line, 1), 2); // after 'E'
    assert_eq!(LatexEditor::next_word_boundary(line, 2), 4); // after '='
    assert_eq!(LatexEditor::next_word_boundary(line, 4), 7); // after "mc"
    assert_eq!(LatexEditor::next_word_boundary(line, 7), 8); // after '^'
    assert_eq!(LatexEditor::next_word_boundary(line, 8), 9); // after '2'
    assert_eq!(LatexEditor::next_word_boundary(line, 9), 11); // after '+'
    assert_eq!(LatexEditor::next_word_boundary(line, 11), 13); // after '\'
    assert_eq!(LatexEditor::next_word_boundary(line, 13), 18); // after "alpha"
    assert_eq!(LatexEditor::next_word_boundary(line, 18), 19); // after '$'
}

#[test]
fn test_empty_line_and_single_char_boundaries() {
    assert_eq!(LatexEditor::prev_word_boundary("", 0), 0);
    assert_eq!(LatexEditor::next_word_boundary("", 0), 0);

    assert_eq!(LatexEditor::prev_word_boundary("a", 1), 0);
    assert_eq!(LatexEditor::next_word_boundary("a", 0), 1);
}

#[test]
fn test_last_character_interactions_and_selection() {
    let line = "\\section{Hello}";
    let line_len = line.chars().count(); // 15 chars: indices 0..14. Last char is '}' at index 14.
    assert_eq!(line_len, 15);

    // 1. Moving right to the last character (index 14) and past the last character (index 15)
    let col_before_last = 14;
    let col_after_last = 15;

    assert_eq!(line.chars().nth(col_before_last), Some('}'));
    assert_eq!(line.chars().nth(col_after_last), None);

    // 2. Selecting the last character
    let (s_col, e_col) = (col_before_last, col_after_last);
    let chars: Vec<char> = line.chars().collect();
    let selected: String = chars[s_col..e_col].iter().collect();
    assert_eq!(selected, "}");

    // 3. Double clicking on the last character '}'
    let (w_start, w_end) = LatexEditor::find_word_range_at(line, 14);
    assert_eq!(&line[w_start..w_end], "}");

    // Double clicking past the end of the line (col 15)
    let (w_start_past, w_end_past) = LatexEditor::find_word_range_at(line, 15);
    assert_eq!(&line[w_start_past..w_end_past], "}");

    // 4. Deleting the last character with Backspace from col 15
    let mut chars_del: Vec<char> = line.chars().collect();
    chars_del.remove(col_after_last - 1);
    let result: String = chars_del.into_iter().collect();
    assert_eq!(result, "\\section{Hello");

    // 5. Deleting the last character with Delete (forward) from col 14
    let mut chars_del_fwd: Vec<char> = line.chars().collect();
    chars_del_fwd.remove(col_before_last);
    let result_fwd: String = chars_del_fwd.into_iter().collect();
    assert_eq!(result_fwd, "\\section{Hello");

    // 6. Appending text after the last character at col 15
    let mut chars_app: Vec<char> = line.chars().collect();
    chars_app.extend("!".chars());
    let result_app: String = chars_app.into_iter().collect();
    assert_eq!(result_app, "\\section{Hello}!");
}

