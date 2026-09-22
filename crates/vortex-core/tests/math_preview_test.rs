use vortex_core::math_preview::{
    build_preamble, extract_macros, extract_math_packages, find_math_spans, first_latex_error, math_span_at,
    MathKind,
};

fn lines(src: &str) -> Vec<String> {
    src.lines().map(String::from).collect()
}

#[test]
fn finds_inline_and_display_math() {
    let doc = lines("Energy $E = mc^2$ and \\(a+b\\).\n$$x^2$$ then \\[\\int f\\]");
    let spans = find_math_spans(&doc);
    assert_eq!(spans.len(), 4);
    assert_eq!(spans[0].body, "E = mc^2");
    assert_eq!(spans[0].kind, MathKind::Inline);
    assert_eq!(spans[0].start, (0, 7));
    assert_eq!(spans[0].end, (0, 16));
    assert_eq!(spans[1].body, "a+b");
    assert_eq!(spans[2].kind, MathKind::Display);
    assert_eq!(spans[2].body, "x^2");
    assert_eq!(spans[3].body, "\\int f");
    assert_eq!(spans[3].snippet(), "$\\displaystyle \\int f$");
}

#[test]
fn equation_with_split_becomes_aligned() {
    let doc = lines("\\begin{equation}\\label{eq:a}\n\\begin{split} a &= b \\notag \\\\ &= c \\end{split}\n\\end{equation}");
    let spans = find_math_spans(&doc);
    assert_eq!(spans.len(), 1);
    assert_eq!(
        spans[0].snippet(),
        "$\\displaystyle \\begin{aligned} a &= b  \\\\ &= c \\end{aligned}$"
    );
}

#[test]
fn finds_multiline_environments_and_stars_numbered_ones() {
    let doc = lines("\\begin{align}\n  a &= b \\\\\n  c &= d \\label{eq:x}\n\\end{align}\nafter");
    let spans = find_math_spans(&doc);
    assert_eq!(spans.len(), 1);
    let s = &spans[0];
    assert_eq!(s.kind, MathKind::Environment("align".into()));
    assert_eq!(s.start, (0, 0));
    assert_eq!(s.end, (3, 10));
    let snippet = s.snippet();
    assert!(snippet.starts_with("$\\displaystyle \\begin{aligned}"), "{snippet}");
    assert!(snippet.ends_with("\\end{aligned}$"), "{snippet}");
    assert!(!snippet.contains("\\label"), "{snippet}");
    assert!(math_span_at(&spans, 2, 4).is_some());
    assert!(math_span_at(&spans, 4, 0).is_none());
}

#[test]
fn ignores_escaped_dollars_comments_and_verbatim() {
    let doc = lines("costs \\$5 and \\$6\n% $not math$\n\\begin{verbatim}\n$x$\n\\end{verbatim}\nreal $y$");
    let spans = find_math_spans(&doc);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].body, "y");
    assert_eq!(spans[0].start, (5, 5));
}

#[test]
fn unterminated_inline_math_stops_at_paragraph_break() {
    let doc = lines("a $ stray\n\nlater $z$");
    let spans = find_math_spans(&doc);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].body, "z");
}

#[test]
fn extracts_macro_definitions() {
    let src = "\\newcommand{\\R}{\\mathbb{R}}\n\\newcommand\\vect[1]{\\mathbf{#1}}\n\\renewcommand{\\vec}[2][x]{#1_{#2}}\n\\DeclareMathOperator*{\\argmax}{arg\\,max}\n\\def\\half#1{\\frac{#1}{2}}\n\\let\\oldphi\\phi\n% \\newcommand{\\ignored}{x}\n\\newcommand{\\broken}";
    let macros = extract_macros(src);
    assert_eq!(
        macros,
        vec![
            "\\newcommand{\\R}{\\mathbb{R}}",
            "\\newcommand\\vect[1]{\\mathbf{#1}}",
            "\\renewcommand{\\vec}[2][x]{#1_{#2}}",
            "\\DeclareMathOperator*{\\argmax}{arg\\,max}",
            "\\def\\half#1{\\frac{#1}{2}}",
            "\\let\\oldphi\\phi",
        ]
    );
}

#[test]
fn keeps_only_whitelisted_packages() {
    let src = "\\usepackage{graphicx}\n\\usepackage{amsmath, tikz, bm}\n\\usepackage[T1]{fontenc}\n\\usepackage[italic]{mathastext}\n\\usepackage[varg]{newtxmath} % fonts";
    assert_eq!(
        extract_math_packages(src),
        vec!["\\usepackage{amsmath,bm}", "\\usepackage[varg]{newtxmath}"]
    );
}

#[test]
fn preamble_skips_amssymb_with_math_font_packages() {
    let with_font = build_preamble("\\usepackage{newtxmath}", None);
    assert!(!with_font.contains("amssymb"));
    let plain = build_preamble("\\newcommand{\\R}{\\mathbb{R}}", None);
    assert!(plain.contains("\\usepackage{amssymb}"));
    assert!(plain.contains("\\providecommand{\\R}{}\\renewcommand{\\R}{\\mathbb{R}}"));
    let starred = build_preamble("\\renewcommand*\\vect[1]{\\mathbf{#1}}", None);
    assert!(starred.contains("\\providecommand{\\vect}{}\\renewcommand*{\\vect}[1]{\\mathbf{#1}}"), "{starred}");
}

#[test]
fn parses_latex_log_errors() {
    let log = "(./snippet.tex\n! Undefined control sequence.\n<argument> \\foo\nl.6 $\\displaystyle \\foo\n";
    assert_eq!(
        first_latex_error(log).as_deref(),
        Some("Undefined control sequence. — $\\displaystyle \\foo")
    );
}

fn pdflatex_available() -> bool {
    std::process::Command::new("pdflatex")
        .arg("--version")
        .env("PATH", vortex_core::compiler::get_latex_path_env())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn renders_math_to_png_and_reports_errors() {
    use vortex_core::math_preview::{render_math, MathRenderRequest};
    if !pdflatex_available() {
        eprintln!("pdflatex not found; skipping");
        return;
    }
    let ok = render_math(&MathRenderRequest {
        snippet: "$\\displaystyle \\int_0^\\infty e^{-x^2}\\,dx = \\tfrac{\\sqrt{\\pi}}{2}$".into(),
        preamble: build_preamble("", None),
        color_hex: "1E1E2E".into(),
    })
    .expect("valid math should render");
    assert!(ok.png_path.exists());
    assert!(ok.width_pt > 20.0 && ok.height_pt > 5.0, "{:?}", ok);

    let err = render_math(&MathRenderRequest {
        snippet: "$\\thisIsNotAMacro$".into(),
        preamble: build_preamble("", None),
        color_hex: "1E1E2E".into(),
    })
    .expect_err("undefined macro should fail");
    assert!(err.contains("Undefined control sequence"), "{err}");
}
