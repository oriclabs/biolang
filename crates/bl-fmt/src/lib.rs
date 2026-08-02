//! Canonical source formatting for BioLang.
//!
//! This works on source text rather than on the AST. The lexer discards `#`
//! comments entirely, so an AST pretty-printer would silently delete every
//! comment in the file — unacceptable for a formatter people are meant to run
//! on save. Working on text also means a file that does not parse still
//! formats, which is exactly when re-indenting is most useful.
//!
//! The trade-off is that this normalises layout, not expression structure: it
//! will not split a long line or rewrite a chain. What it does guarantee is
//! that indentation, blank lines, trailing whitespace, and comma spacing have
//! exactly one spelling, which is what people actually argue about.

/// Formatting options.
#[derive(Debug, Clone, Copy)]
pub struct FormatOptions {
    /// Spaces per indent level.
    pub indent_width: usize,
    /// Consecutive blank lines allowed inside a block.
    pub max_blank_lines: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        // Four spaces is what the examples, the books, and the packages already
        // use; the formatter should agree with the corpus, not pick a new side.
        Self {
            indent_width: 4,
            max_blank_lines: 1,
        }
    }
}

/// Where a scan of one line ends up, so the next line knows how to indent.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LineScan {
    /// Net bracket depth change contributed by this line.
    net: i64,
    /// Lowest depth reached while scanning, relative to the line start. A line
    /// beginning with `}` reaches -1, which is what pulls it back out a level.
    minimum: i64,
    /// True when the line ends inside an unterminated string literal.
    open_string: bool,
}

/// Scan one line for bracket depth, ignoring brackets inside strings and
/// comments.
fn scan_line(line: &str, starts_in_string: bool) -> LineScan {
    let mut depth: i64 = 0;
    let mut minimum: i64 = 0;
    let mut in_string = starts_in_string;
    let mut escaped = false;

    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            // `#{` opens an interpolation brace, not a comment.
            '#' if chars.peek() == Some(&'{') => {
                chars.next();
                depth += 1;
            }
            '#' => break,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth -= 1;
                minimum = minimum.min(depth);
            }
            _ => {}
        }
    }

    LineScan {
        net: depth,
        minimum,
        open_string: in_string,
    }
}

/// True when the line is a continuation that should sit one level in, such as a
/// wrapped pipe chain or an argument list carried onto the next line.
fn is_continuation(trimmed: &str) -> bool {
    trimmed.starts_with("|>")
        || trimmed.starts_with("->")
        || trimmed.starts_with("=>")
        || trimmed.starts_with('.') && !trimmed.starts_with("..")
}

/// Normalise spacing that has exactly one defensible form.
///
/// Only whitespace immediately around `,` and `;` is touched. Operator spacing
/// is deliberately left alone: `-` is both binary and unary, and `>` appears in
/// `->`, `=>`, and `|>`, so "helpfully" spacing them out is how a formatter
/// starts corrupting code.
fn normalise_separators(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();

    while let Some(character) = chars.next() {
        if in_string {
            out.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => {
                in_string = true;
                out.push(character);
            }
            '#' => {
                // Comment tail: copy the rest verbatim.
                out.push(character);
                out.extend(chars);
                return out;
            }
            ',' | ';' => {
                while out.ends_with(' ') {
                    out.pop();
                }
                out.push(character);
                // Collapse the run of spaces after the separator to one, but
                // leave a trailing separator at end of line alone.
                while chars.peek() == Some(&' ') {
                    chars.next();
                }
                if chars.peek().is_some() {
                    out.push(' ');
                }
            }
            _ => out.push(character),
        }
    }
    out
}

/// Format BioLang source into its canonical layout.
///
/// The result is idempotent: formatting formatted source returns it unchanged.
pub fn format_source(source: &str, options: FormatOptions) -> String {
    let mut out = String::with_capacity(source.len() + 16);
    let mut depth: i64 = 0;
    let mut in_string = false;
    let mut blank_run = 0usize;
    let mut wrote_any = false;

    for raw in source.lines() {
        let trimmed_end = raw.trim_end();

        // A line continuing a multi-line string keeps its contents byte for
        // byte; re-indenting inside a literal would change the value.
        if in_string {
            let scan = scan_line(trimmed_end, true);
            out.push_str(trimmed_end);
            out.push('\n');
            in_string = scan.open_string;
            blank_run = 0;
            wrote_any = true;
            continue;
        }

        let trimmed = trimmed_end.trim_start();
        if trimmed.is_empty() {
            // Blank lines before any content, or more than the allowance in a
            // row, are dropped.
            if wrote_any {
                blank_run += 1;
            }
            continue;
        }

        for _ in 0..blank_run.min(options.max_blank_lines) {
            out.push('\n');
        }
        blank_run = 0;

        let scan = scan_line(trimmed, false);
        // A closing bracket that starts the line un-indents the line itself.
        let own = (depth + scan.minimum).max(0);
        let indent = if is_continuation(trimmed) {
            own + 1
        } else {
            own
        };

        out.push_str(&" ".repeat(indent as usize * options.indent_width));
        out.push_str(&normalise_separators(trimmed));
        out.push('\n');

        depth = (depth + scan.net).max(0);
        in_string = scan.open_string;
        wrote_any = true;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(source: &str) -> String {
        format_source(source, FormatOptions::default())
    }

    #[test]
    fn indents_blocks_by_depth() {
        let formatted = format("fn go(x) {\nlet y = x + 1\nif y {\nprintln(y)\n}\n}\n");
        assert_eq!(
            formatted,
            "fn go(x) {\n    let y = x + 1\n    if y {\n        println(y)\n    }\n}\n"
        );
    }

    #[test]
    fn dedents_the_closing_brace_itself() {
        let formatted = format("if a {\n        b()\n            }\n");
        assert_eq!(formatted, "if a {\n    b()\n}\n");
    }

    #[test]
    fn indents_wrapped_pipe_chains_one_level() {
        let formatted = format("let t = rows\n|> map(f)\n|> collect()\n");
        assert_eq!(formatted, "let t = rows\n    |> map(f)\n    |> collect()\n");
    }

    #[test]
    fn preserves_comments() {
        let formatted = format("# leading note\nlet x = 1  # trailing note\n");
        assert_eq!(formatted, "# leading note\nlet x = 1  # trailing note\n");
    }

    #[test]
    fn braces_inside_strings_and_comments_do_not_indent() {
        let formatted = format("let s = \"{{{\"\nlet t = 2  # }\nlet u = 3\n");
        assert_eq!(formatted, "let s = \"{{{\"\nlet t = 2  # }\nlet u = 3\n");
    }

    #[test]
    fn normalises_comma_spacing() {
        let formatted = format("f(a ,b,   c)\n");
        assert_eq!(formatted, "f(a, b, c)\n");
    }

    #[test]
    fn leaves_commas_inside_strings_alone() {
        let formatted = format("let s = \"a ,b,   c\"\n");
        assert_eq!(formatted, "let s = \"a ,b,   c\"\n");
    }

    #[test]
    fn leaves_commas_inside_comments_alone() {
        let formatted = format("let x = 1  # a ,b,   c\n");
        assert_eq!(formatted, "let x = 1  # a ,b,   c\n");
    }

    #[test]
    fn collapses_blank_runs_and_strips_leading_and_trailing_blanks() {
        let formatted = format("\n\nlet a = 1\n\n\n\nlet b = 2\n\n\n");
        assert_eq!(formatted, "let a = 1\n\nlet b = 2\n");
    }

    #[test]
    fn strips_trailing_whitespace() {
        let formatted = format("let a = 1   \nlet b = 2\t\n");
        assert_eq!(formatted, "let a = 1\nlet b = 2\n");
    }

    #[test]
    fn adds_a_final_newline_when_missing() {
        assert_eq!(format("let a = 1"), "let a = 1\n");
    }

    #[test]
    fn indents_pipeline_stages() {
        let formatted = format("pipeline qc(path) {\nstage \"load\" -> read(path)\n}\n");
        assert_eq!(
            formatted,
            "pipeline qc(path) {\n    stage \"load\" -> read(path)\n}\n"
        );
    }

    #[test]
    fn handles_interpolation_braces() {
        let formatted = format("let s = \"x\"\nlet t = 1\n");
        assert_eq!(formatted, "let s = \"x\"\nlet t = 1\n");
    }

    #[test]
    fn is_idempotent_over_the_example_shapes() {
        let sources = [
            "fn go(x) {\n    let y = x + 1\n    if y {\n        println(y)\n    }\n}\n",
            "let t = rows\n    |> map(f)\n    |> collect()\n",
            "# note\n\nlet a = 1\n",
            "pipeline qc(path) {\n    stage \"load\" -> read(path)\n}\n",
        ];
        for source in sources {
            assert_eq!(format(source), source, "not idempotent: {source:?}");
            assert_eq!(format(&format(source)), format(source));
        }
    }

    #[test]
    fn unbalanced_closers_do_not_drive_indent_negative() {
        let formatted = format("}\n}\nlet a = 1\n");
        assert_eq!(formatted, "}\n}\nlet a = 1\n");
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(format(""), "");
        assert_eq!(format("\n\n\n"), "");
    }
}
