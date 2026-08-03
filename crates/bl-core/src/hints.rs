//! Hints attached to errors.
//!
//! `BioLangError` has carried a `suggestions` list and printed it as `hint:`
//! lines since the beginning, but only four places in the runtime ever filled
//! it in — all of them "did you mean". Error text is the most-read
//! documentation any language has, and for someone learning it *is* the
//! teaching, so the gap mattered more than its size suggested.
//!
//! Hints are derived centrally from the kind and message rather than added at
//! every raise site. One place is testable, covers errors raised inside
//! packages as well as the core, and cannot drift out of sync with itself.

use crate::error::ErrorKind;

/// A hint plus the Help Center topic that explains the underlying idea.
pub struct Hint {
    pub text: String,
    /// Help entry id, so a front end can offer "learn more" rather than
    /// leaving the reader to guess what to search for.
    pub topic: Option<&'static str>,
}

impl Hint {
    fn new(text: impl Into<String>, topic: Option<&'static str>) -> Self {
        Self {
            text: text.into(),
            topic,
        }
    }
}

/// Pull `name` out of a message of the form `... 'name' ...`.
fn quoted(message: &str) -> Option<&str> {
    let start = message.find('\'')? + 1;
    let rest = &message[start..];
    let end = rest.find('\'')?;
    Some(&rest[..end])
}

/// Parse the two integers out of "index N out of bounds (len L)".
fn index_and_length(message: &str) -> Option<(i64, i64)> {
    let index = message
        .split_whitespace()
        .nth(1)?
        .trim_matches(|c: char| !c.is_ascii_digit() && c != '-')
        .parse()
        .ok()?;
    let length = message
        .rsplit_once("len ")?
        .1
        .trim_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()?;
    Some((index, length))
}

/// Suggestions for an error, or an empty list when nothing useful can be said.
///
/// Deliberately conservative: a wrong hint sends someone down a blind alley and
/// costs more than the silence it replaced.
pub fn suggest(kind: &ErrorKind, message: &str) -> Vec<Hint> {
    let lower = message.to_lowercase();

    match kind {
        ErrorKind::IndexOutOfBounds => {
            let mut hints = Vec::new();
            if let Some((index, length)) = index_and_length(message) {
                if length == 0 {
                    hints.push(Hint::new(
                        "the collection is empty, so no index is valid — check whether the step before it returned anything",
                        Some("collections"),
                    ));
                } else if index == length {
                    // The classic off-by-one, and worth naming as such.
                    hints.push(Hint::new(
                        format!(
                            "indices start at 0, so the last item of a {length}-item collection is at index {}",
                            length - 1
                        ),
                        Some("collections"),
                    ));
                } else {
                    hints.push(Hint::new(
                        format!("valid indices are 0 to {}", length - 1),
                        Some("collections"),
                    ));
                }
            }
            hints
        }

        ErrorKind::ArityError => {
            let mut hints = Vec::new();
            if let Some(name) = quoted(message).or_else(|| message.split('(').next()) {
                let name = name.trim();
                if !name.is_empty() {
                    hints.push(Hint::new(
                        format!("check the signature with `help {name}`, or hover the name in the editor"),
                        Some("functions"),
                    ));
                }
            }
            if lower.contains("got 0") {
                hints.push(Hint::new(
                    "a function called with no arguments still needs its required ones",
                    Some("functions"),
                ));
            }
            hints
        }

        ErrorKind::NameError => {
            let mut hints = Vec::new();
            // A "did you mean" is already attached where one exists; this is
            // for the case where nothing was close enough.
            if lower.contains("undefined variable") {
                hints.push(Hint::new(
                    "declare it with `let` before use, or check the spelling",
                    Some("variables"),
                ));
                if let Some(name) = quoted(message) {
                    if name.contains('.') {
                        hints.push(Hint::new(
                            "for a package function, import the package first: `import \"name\" as alias`",
                            Some("packages"),
                        ));
                    }
                }
            }
            hints
        }

        ErrorKind::IOError => {
            let mut hints = Vec::new();
            if lower.contains("cannot open")
                || lower.contains("not found")
                || lower.contains("no such file")
            {
                hints.push(Hint::new(
                    "relative paths resolve from the folder you ran from, not from the file's own folder",
                    Some("file-io"),
                ));
                if let Some(path) = quoted(message) {
                    if !path.contains('/') && !path.contains('\\') {
                        hints.push(Hint::new(
                            format!("if {path} lives in a subfolder, include it: \"data/{path}\""),
                            Some("file-io"),
                        ));
                    }
                }
            }
            hints
        }

        ErrorKind::TypeError => {
            let mut hints = Vec::new();
            if lower.contains("stream") {
                // The single most common surprise for anyone new to the
                // pipe-first style.
                hints.push(Hint::new(
                    "a stream is consumed lazily — add `|> collect()` to turn it into a list first",
                    Some("streams"),
                ));
            }
            if lower.contains("requires table") {
                hints.push(Hint::new(
                    "build a table from records with `|> table()` before using table verbs",
                    Some("tables"),
                ));
            }
            hints
        }

        ErrorKind::DivisionByZero => vec![Hint::new(
            "guard the denominator, or use a conditional to handle the empty case",
            None,
        )],

        ErrorKind::UnterminatedString => vec![Hint::new(
            "add the closing quote; a `\"` inside a string needs to be written as `\\\"`",
            Some("strings"),
        )],

        ErrorKind::AssertionFailed => vec![Hint::new(
            "add a message to say what was expected: `assert condition, \"why\"`",
            Some("testing"),
        )],

        ErrorKind::ImportError => vec![Hint::new(
            "install dependencies with `bl install`, and check the name against biolang.toml",
            Some("packages"),
        )],

        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(kind: ErrorKind, message: &str) -> Vec<String> {
        suggest(&kind, message)
            .into_iter()
            .map(|hint| hint.text)
            .collect()
    }

    #[test]
    fn an_off_by_one_index_names_the_last_valid_index() {
        let hints = texts(ErrorKind::IndexOutOfBounds, "index 3 out of bounds (len 3)");
        assert!(hints[0].contains("indices start at 0"), "{hints:?}");
        assert!(hints[0].contains("index 2"), "{hints:?}");
    }

    #[test]
    fn an_index_far_past_the_end_states_the_valid_range() {
        let hints = texts(ErrorKind::IndexOutOfBounds, "index 9 out of bounds (len 3)");
        assert!(hints[0].contains("0 to 2"), "{hints:?}");
    }

    #[test]
    fn an_empty_collection_points_at_the_step_before() {
        let hints = texts(ErrorKind::IndexOutOfBounds, "index 0 out of bounds (len 0)");
        assert!(hints[0].contains("empty"), "{hints:?}");
    }

    #[test]
    fn an_arity_error_points_at_the_signature() {
        let hints = texts(
            ErrorKind::ArityError,
            "reverse_complement() expected 1 arguments, got 0",
        );
        assert!(
            hints
                .iter()
                .any(|hint| hint.contains("help reverse_complement")),
            "{hints:?}"
        );
    }

    #[test]
    fn a_missing_file_explains_relative_paths() {
        let hints = texts(
            ErrorKind::IOError,
            "csv: cannot open 'counts.csv': The system cannot find the file specified.",
        );
        assert!(hints[0].contains("relative paths"), "{hints:?}");
        assert!(
            hints.iter().any(|hint| hint.contains("data/counts.csv")),
            "{hints:?}"
        );
    }

    #[test]
    fn a_path_that_already_has_a_folder_does_not_get_the_subfolder_hint() {
        let hints = texts(ErrorKind::IOError, "cannot open 'data/counts.csv': missing");
        assert_eq!(hints.len(), 1, "{hints:?}");
    }

    #[test]
    fn a_stream_type_error_recommends_collect() {
        let hints = texts(ErrorKind::TypeError, "expected List, got Stream");
        assert!(hints[0].contains("collect()"), "{hints:?}");
    }

    #[test]
    fn an_undefined_variable_says_how_to_declare_one() {
        let hints = texts(ErrorKind::NameError, "undefined variable 'counts'");
        assert!(hints[0].contains("let"), "{hints:?}");
    }

    #[test]
    fn a_qualified_undefined_name_mentions_the_import() {
        let hints = texts(ErrorKind::NameError, "undefined variable 'sc.summary'");
        assert!(
            hints.iter().any(|hint| hint.contains("import")),
            "{hints:?}"
        );
    }

    #[test]
    fn hints_carry_a_help_topic_where_one_applies() {
        let hints = suggest(
            &ErrorKind::IndexOutOfBounds,
            "index 9 out of bounds (len 3)",
        );
        assert_eq!(hints[0].topic, Some("collections"));
    }

    #[test]
    fn kinds_with_nothing_useful_to_add_stay_silent() {
        // A wrong hint costs more than the silence it replaces.
        assert!(texts(ErrorKind::Return, "return").is_empty());
        assert!(texts(ErrorKind::Break, "break").is_empty());
    }
}
