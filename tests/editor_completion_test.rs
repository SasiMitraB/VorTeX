use vortex::views::editor::completion::{CompletionItem, CompletionKind, CompletionState};

#[test]
fn test_environment_template_structure() {
    let env = "figure";
    let insert_text = format!("{}}}\n  \n\\end{{{}}}", env, env);
    let parts: Vec<&str> = insert_text.split('\n').collect();

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "figure}");
    assert_eq!(parts[1], "  ");
    assert_eq!(parts[2], "\\end{figure}");
}

#[test]
fn test_brace_deduplication_rule() {
    let insert_text = "figure}\n  \n\\end{figure}";
    let mut after = "}".to_string();

    if (insert_text.contains('}') || insert_text.starts_with("figure")) && after.starts_with('}') {
        after = after[1..].to_string();
    }

    assert_eq!(after, "");
}

#[test]
fn test_completion_auto_scroll_down_and_up() {
    let mut state = CompletionState::default();

    // 14 environments (like \begin{)
    let envs = vec![
        "figure", "table", "tabular", "equation", "align", "itemize", "enumerate", "matrix",
        "pmatrix", "bmatrix", "proof", "theorem", "lemma", "definition",
    ];

    let items: Vec<CompletionItem> = envs
        .into_iter()
        .map(|name| CompletionItem {
            label: name.to_string(),
            kind: CompletionKind::Environment,
            detail: None,
            documentation: None,
            insert_text: name.to_string(),
        })
        .collect();

    // Open completion
    state.open(items, 0, 0, "".to_string());
    assert_eq!(state.selected_index, 0);
    assert_eq!(f32::from(state.scroll_handle.offset().y), 0.0);

    // Navigate down through visible items (indices 0..6)
    for i in 1..=6 {
        state.select_next();
        assert_eq!(state.selected_index, i);
        // Still within first viewport page (7 items * 28.0 = 196.0)
        assert_eq!(f32::from(state.scroll_handle.offset().y), 0.0);
    }

    // Item 7 is below the 196px viewport, should auto-scroll down to show item 7
    state.select_next();
    assert_eq!(state.selected_index, 7);
    // item 7 bottom is 8 * 28.0 = 224.0; scroll_y = 224.0 - 196.0 = 28.0; offset = -28.0
    assert_eq!(f32::from(state.scroll_handle.offset().y), -28.0);

    // Item 8
    state.select_next();
    assert_eq!(state.selected_index, 8);
    assert_eq!(f32::from(state.scroll_handle.offset().y), -56.0);

    // Continue to last item (index 13)
    while state.selected_index < 13 {
        state.select_next();
    }
    assert_eq!(state.selected_index, 13);
    // Last item bottom is 14 * 28.0 = 392.0; max_scroll = 392.0 - 196.0 = 196.0; offset = -196.0
    assert_eq!(f32::from(state.scroll_handle.offset().y), -196.0);

    // Wrap around to index 0 on next
    state.select_next();
    assert_eq!(state.selected_index, 0);
    assert_eq!(f32::from(state.scroll_handle.offset().y), 0.0);

    // Wrap around backwards to index 13 on prev
    state.select_prev();
    assert_eq!(state.selected_index, 13);
    assert_eq!(f32::from(state.scroll_handle.offset().y), -196.0);

    // Move up until index 6 (which is above current viewport [196.0, 392.0])
    while state.selected_index > 6 {
        state.select_prev();
    }
    assert_eq!(state.selected_index, 6);
    // item 6 top is 6 * 28.0 = 168.0; offset = -168.0
    assert_eq!(f32::from(state.scroll_handle.offset().y), -168.0);
}

#[test]
fn test_citation_trigger_prefixes() {
    let test_cases = vec![
        ("\\cite{", Some("")),
        ("\\citep{", Some("")),
        ("\\citet{", Some("")),
        ("\\parencite{", Some("")),
        ("\\autocite{", Some("")),
        ("\\citep[see][p.~10]{vasw", Some("vasw")),
        ("\\cite{ref1, ref2", Some("ref2")),
        ("\\citep{vaswani2017, ", Some("")),
        ("\\cite{already_closed}", None),
    ];

    for (prefix, expected_query) in test_cases {
        let detected = if let Some(open_brace_idx) = prefix.rfind('{') {
            let inside = &prefix[open_brace_idx + 1..];
            if !inside.contains('}') {
                let before_brace = prefix[..open_brace_idx].trim_end();
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
                        "\\cite" | "\\citep" | "\\citet" | "\\parencite" | "\\autocite" => {
                            let query = if let Some(last_comma) = inside.rfind(',') {
                                inside[last_comma + 1..].trim_start()
                            } else {
                                inside.trim_start()
                            };
                            Some(query)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        assert_eq!(detected, expected_query, "Failed for prefix: {}", prefix);
    }
}

