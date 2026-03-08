//! BioLang notebook (.bln) literate format.
//!
//! A `.bln` file interleaves Markdown prose with BioLang code blocks.
//!
//! Format rules:
//! - Lines starting with `##` (or more `#`) are Markdown headings — printed as-is.
//! - Lines between `---` separators are BioLang code — evaluated in order.
//! - Everything else outside code blocks is Markdown prose — printed as-is.
//!
//! Example:
//! ```text
//! ## Load Data
//! Read the FASTQ file and compute basic stats.
//! ---
//! let reads = read_fastq("sample.fq")
//! let stats = fastq_stats(reads)
//! print(stats)
//! ---
//! ## Results
//! The statistics above show the quality distribution.
//! ```

use std::path::PathBuf;

/// A parsed block from a .bln notebook.
#[derive(Debug)]
enum Block {
    /// Markdown prose — printed to stdout.
    Prose(String),
    /// BioLang code — evaluated by interpreter.
    Code(String),
}

/// Parse a .bln file into alternating prose/code blocks.
fn parse_notebook(source: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut in_code = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            // Flush current block
            let text = std::mem::take(&mut current);
            if !text.trim().is_empty() {
                if in_code {
                    blocks.push(Block::Code(text));
                } else {
                    blocks.push(Block::Prose(text));
                }
            }
            in_code = !in_code;
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }

    // Flush remaining
    let text = current;
    if !text.trim().is_empty() {
        if in_code {
            blocks.push(Block::Code(text));
        } else {
            blocks.push(Block::Prose(text));
        }
    }

    blocks
}

/// Run a .bln notebook file: print prose, evaluate code, interleave output.
pub fn run_notebook(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{path}': {e}");
            std::process::exit(1);
        }
    };

    let blocks = parse_notebook(&source);
    let mut interpreter = bl_runtime::Interpreter::new();

    if let Ok(canonical) = std::fs::canonicalize(path) {
        interpreter.set_current_file(Some(canonical));
    } else {
        interpreter.set_current_file(Some(PathBuf::from(path)));
    }

    for block in blocks {
        match block {
            Block::Prose(text) => {
                println!("{text}");
            }
            Block::Code(code) => {
                let tokens = match bl_lexer::Lexer::new(&code).tokenize() {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("{}", e.format_with_source(&code));
                        std::process::exit(1);
                    }
                };

                let parse_result = match bl_parser::Parser::new(tokens).parse() {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("{}", e.format_with_source(&code));
                        std::process::exit(1);
                    }
                };
                if parse_result.has_errors() {
                    for e in &parse_result.errors {
                        eprintln!("{}", e.format_with_source(&code));
                    }
                    std::process::exit(1);
                }

                match interpreter.run(&parse_result.program) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("{}", e.format_with_source(&code));
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let blocks = parse_notebook("");
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_parse_prose_only() {
        let blocks = parse_notebook("## Hello\nSome text here.");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Prose(t) => assert!(t.contains("Hello")),
            _ => panic!("expected prose"),
        }
    }

    #[test]
    fn test_parse_code_block() {
        let src = "## Intro\n---\nlet x = 1\n---\n## End";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 3);
        match &blocks[0] {
            Block::Prose(t) => assert!(t.contains("Intro")),
            _ => panic!("expected prose"),
        }
        match &blocks[1] {
            Block::Code(c) => assert!(c.contains("let x = 1")),
            _ => panic!("expected code"),
        }
        match &blocks[2] {
            Block::Prose(t) => assert!(t.contains("End")),
            _ => panic!("expected prose"),
        }
    }

    #[test]
    fn test_parse_multiple_code_blocks() {
        let src = "---\nlet a = 1\n---\nMiddle\n---\nlet b = 2\n---";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 3);
        match &blocks[0] {
            Block::Code(c) => assert!(c.contains("let a")),
            _ => panic!("expected code"),
        }
        match &blocks[1] {
            Block::Prose(t) => assert!(t.contains("Middle")),
            _ => panic!("expected prose"),
        }
        match &blocks[2] {
            Block::Code(c) => assert!(c.contains("let b")),
            _ => panic!("expected code"),
        }
    }
}
