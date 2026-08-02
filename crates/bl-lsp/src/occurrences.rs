//! Identifier occurrence scanning for references and rename.
//!
//! The analyser records where each symbol is *defined* but not where it is
//! used, so references are found by scanning the text. Scanning is done here
//! rather than with the lexer because the lexer discards `#` comments, and a
//! rename that silently edits a comment — or refuses to leave one alone — is
//! worse than one that skips comments entirely.
//!
//! This is deliberately document-scoped. Renaming across a whole workspace
//! needs real scope resolution, and a rename that guesses is a rename that
//! corrupts files.

/// A half-open byte range within the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occurrence {
    pub start: usize,
    pub end: usize,
}

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// Every standalone occurrence of `name` in `source`, skipping strings,
/// comments, and member positions such as the `end` in `record.end`.
pub fn identifier_occurrences(source: &str, name: &str) -> Vec<Occurrence> {
    if name.is_empty() || !name.starts_with(is_identifier_start) {
        return Vec::new();
    }

    let bytes: Vec<(usize, char)> = source.char_indices().collect();
    let mut found = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        let (offset, character) = bytes[index];

        // String literal: skip to the closing quote, honouring escapes.
        if character == '"' {
            index += 1;
            while index < bytes.len() {
                let (_, inner) = bytes[index];
                if inner == '\\' {
                    index += 2;
                    continue;
                }
                index += 1;
                if inner == '"' {
                    break;
                }
            }
            continue;
        }

        // `#{` is string interpolation, not a comment; everything else starting
        // with `#` runs to end of line.
        if character == '#' {
            let interpolation = bytes.get(index + 1).is_some_and(|&(_, next)| next == '{');
            if interpolation {
                index += 2;
                continue;
            }
            while index < bytes.len() && bytes[index].1 != '\n' {
                index += 1;
            }
            continue;
        }

        if !is_identifier_start(character) {
            index += 1;
            continue;
        }

        let start = index;
        while index < bytes.len() && is_identifier_continue(bytes[index].1) {
            index += 1;
        }
        let end_offset = bytes
            .get(index)
            .map(|&(next_offset, _)| next_offset)
            .unwrap_or(source.len());
        let word = &source[offset..end_offset];
        if word != name {
            continue;
        }

        // A member access is a different thing that happens to share a name.
        let preceded_by_dot = bytes[..start]
            .iter()
            .rev()
            .find(|&&(_, previous)| !previous.is_whitespace())
            .is_some_and(|&(_, previous)| previous == '.');
        if preceded_by_dot {
            continue;
        }

        found.push(Occurrence {
            start: offset,
            end: end_offset,
        });
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts<'a>(source: &'a str, name: &str) -> Vec<&'a str> {
        identifier_occurrences(source, name)
            .into_iter()
            .map(|found| &source[found.start..found.end])
            .collect()
    }

    #[test]
    fn finds_every_standalone_use() {
        let source = "let count = 1\nprintln(count)\nlet total = count + count\n";
        assert_eq!(texts(source, "count").len(), 4);
    }

    #[test]
    fn ignores_substrings_of_longer_identifiers() {
        let source = "let count = 1\nlet counter = 2\nlet recount = 3\n";
        assert_eq!(texts(source, "count").len(), 1);
    }

    #[test]
    fn ignores_occurrences_inside_strings() {
        let source = "let count = 1\nprintln(\"count is count\")\n";
        assert_eq!(texts(source, "count").len(), 1);
    }

    #[test]
    fn ignores_occurrences_inside_comments() {
        let source = "let count = 1  # count again\n# count\n";
        assert_eq!(texts(source, "count").len(), 1);
    }

    #[test]
    fn ignores_escaped_quotes_when_skipping_strings() {
        let source = "let s = \"a \\\" count\"\nlet count = 1\n";
        assert_eq!(texts(source, "count").len(), 1);
    }

    #[test]
    fn ignores_member_access() {
        let source = "let end = 1\nlet x = region.end\n";
        assert_eq!(texts(source, "end").len(), 1);
    }

    #[test]
    fn reports_byte_ranges_that_slice_the_source() {
        let source = "let count = 1\n";
        let found = identifier_occurrences(source, "count");
        assert_eq!(found.len(), 1);
        assert_eq!(&source[found[0].start..found[0].end], "count");
    }

    #[test]
    fn empty_and_non_identifier_queries_find_nothing() {
        assert!(identifier_occurrences("let a = 1", "").is_empty());
        assert!(identifier_occurrences("let a = 1", "1a").is_empty());
    }
}
