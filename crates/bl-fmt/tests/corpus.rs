//! The formatter run against every BioLang file in the repository.
//!
//! Unit tests prove the rules in isolation; this proves the rules do not
//! destroy real code. A formatter that turns a working script into a parse
//! error is worse than no formatter, so parse status is the property under
//! test, not byte equality.

use std::path::{Path, PathBuf};

use bl_fmt::{format_source, FormatOptions};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two levels below the repository root")
        .to_path_buf()
}

fn biolang_sources() -> Vec<PathBuf> {
    let mut sources = Vec::new();
    let mut stack: Vec<PathBuf> = ["examples", "books", "packages", "tests"]
        .iter()
        .map(|directory| repository_root().join(directory))
        .filter(|path| path.is_dir())
        .collect();

    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "bl") {
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources
}

/// True when the source parses cleanly.
fn parses(source: &str) -> bool {
    let Ok(tokens) = bl_lexer::Lexer::new(source).tokenize() else {
        return false;
    };
    match bl_parser::Parser::new(tokens).parse() {
        Ok(program) => !program.has_errors(),
        Err(_) => false,
    }
}

#[test]
fn formatting_is_idempotent_across_the_repository() {
    let sources = biolang_sources();
    assert!(
        sources.len() > 50,
        "expected to find the BioLang corpus, found {} files",
        sources.len()
    );

    let mut unstable = Vec::new();
    for path in &sources {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let once = format_source(&source, FormatOptions::default());
        let twice = format_source(&once, FormatOptions::default());
        if once != twice {
            unstable.push(path.display().to_string());
        }
    }
    assert!(
        unstable.is_empty(),
        "formatting is not stable for: {unstable:#?}"
    );
}

#[test]
fn formatting_never_breaks_a_file_that_parsed() {
    let mut broken = Vec::new();
    for path in biolang_sources() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !parses(&source) {
            // Already broken before we touched it; not the formatter's problem.
            continue;
        }
        if !parses(&format_source(&source, FormatOptions::default())) {
            broken.push(path.display().to_string());
        }
    }
    assert!(broken.is_empty(), "formatting broke: {broken:#?}");
}

#[test]
fn formatting_preserves_every_comment() {
    let mut lost = Vec::new();
    for path in biolang_sources() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let formatted = format_source(&source, FormatOptions::default());
        let before = source.matches('#').count();
        let after = formatted.matches('#').count();
        if before != after {
            lost.push(format!("{} ({before} -> {after})", path.display()));
        }
    }
    assert!(lost.is_empty(), "comment characters changed in: {lost:#?}");
}
