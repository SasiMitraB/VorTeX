//! Times grammar checks on a real `.tex` file: the first check, an unchanged re-check, and a
//! re-check after typing into one paragraph (what happens while editing).
//! usage: cargo run --release -p vortex-core --example grammar_timing -- path/to/main.tex
use std::collections::HashMap;
use std::time::Instant;
use vortex_core::grammar_checker::{run_grammar_check, GrammarDialect};

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    let path = std::env::args().nth(1).expect("path");
    let tex = std::fs::read_to_string(&path).unwrap();
    let bib = HashMap::new();
    let check = |text: &str| run_grammar_check(text, &bib, GrammarDialect::British);

    let t = Instant::now();
    check(&tex);
    println!("first check (cold)        {:>8.1} ms", ms(t));

    let t = Instant::now();
    check(&tex);
    println!("unchanged re-check        {:>8.1} ms", ms(t));

    // Type a word at a time into the Introduction, checking after each word.
    let at = tex.find("The interstellar medium").expect("anchor");
    let words = "The observed rotation curve flattens beyond the optical radius ".split(' ');
    let (mut typed, mut total, mut n) = (String::new(), 0.0, 0);
    for w in words {
        typed.push_str(w);
        typed.push(' ');
        let edited = format!("{}{}{}", &tex[..at], typed, &tex[at..]);
        let t = Instant::now();
        check(&edited);
        total += ms(t);
        n += 1;
    }
    println!("re-check after typing     {:>8.1} ms (mean of {n})", total / n as f64);
}
