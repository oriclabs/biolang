use bl_core::value::Value;
use bl_runtime::regex_ops::call_regex_builtin;

// ── regex_match ──────────────────────────────────────────────────

#[test]
fn test_regex_match_true() {
    let result = call_regex_builtin(
        "regex_match",
        vec![
            Value::Str("sample_001.fastq.gz".into()),
            Value::Str(r"\.fastq(\.gz)?$".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_regex_match_false() {
    let result = call_regex_builtin(
        "regex_match",
        vec![
            Value::Str("sample.bam".into()),
            Value::Str(r"\.fastq$".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_regex_match_anchored_full_match() {
    let result = call_regex_builtin(
        "regex_match",
        vec![
            Value::Str("abc".into()),
            Value::Str(r"^abc$".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_regex_match_anchored_no_match() {
    let result = call_regex_builtin(
        "regex_match",
        vec![
            Value::Str("xabcx".into()),
            Value::Str(r"^abc$".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_regex_match_case_sensitive_by_default() {
    let result = call_regex_builtin(
        "regex_match",
        vec![
            Value::Str("Hello".into()),
            Value::Str("hello".into()),
        ],
    )
    .unwrap();
    // is_match does partial match: "Hello" contains no "hello" (case-sensitive)
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_regex_match_case_insensitive_via_flag() {
    // Using inline flag (?i)
    let result = call_regex_builtin(
        "regex_match",
        vec![
            Value::Str("Hello".into()),
            Value::Str("(?i)hello".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_regex_match_with_regex_value() {
    // Test with Value::Regex variant using case-insensitive flag
    let result = call_regex_builtin(
        "regex_match",
        vec![
            Value::Str("Hello World".into()),
            Value::Regex {
                pattern: "hello".into(),
                flags: "i".into(),
            },
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_regex_match_empty_string() {
    let result = call_regex_builtin(
        "regex_match",
        vec![Value::Str("".into()), Value::Str(".*".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_regex_match_empty_pattern() {
    // Empty pattern matches everything
    let result = call_regex_builtin(
        "regex_match",
        vec![Value::Str("anything".into()), Value::Str("".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_regex_match_unicode_input() {
    let result = call_regex_builtin(
        "regex_match",
        vec![
            Value::Str("caf\u{00e9}".into()),
            Value::Str(r"caf\x{e9}".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

// ── regex_find ───────────────────────────────────────────────────

#[test]
fn test_regex_find_multiple() {
    let result = call_regex_builtin(
        "regex_find",
        vec![
            Value::Str("chr1:100-200, chr2:300-400".into()),
            Value::Str(r"chr\d+:\d+-\d+".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("chr1:100-200".into()),
            Value::Str("chr2:300-400".into()),
        ])
    );
}

#[test]
fn test_regex_find_no_match() {
    let result = call_regex_builtin(
        "regex_find",
        vec![Value::Str("hello world".into()), Value::Str(r"\d+".into())],
    )
    .unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_regex_find_with_groups_returns_full_match() {
    // regex_find uses find_iter which returns the full match, not capture groups
    let result = call_regex_builtin(
        "regex_find",
        vec![
            Value::Str("foo123bar456".into()),
            Value::Str(r"([a-z]+)(\d+)".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("foo123".into()),
            Value::Str("bar456".into()),
        ])
    );
}

#[test]
fn test_regex_find_empty_string() {
    let result = call_regex_builtin(
        "regex_find",
        vec![Value::Str("".into()), Value::Str(r"\w+".into())],
    )
    .unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_regex_find_empty_pattern() {
    // Empty pattern matches at every position: between each char and at start/end
    let result = call_regex_builtin(
        "regex_find",
        vec![Value::Str("ab".into()), Value::Str("".into())],
    )
    .unwrap();
    // Empty pattern matches empty string at each position
    if let Value::List(items) = &result {
        assert!(items.len() >= 3); // "", "", "" (before a, before b, after b)
        for item in items {
            assert_eq!(item, &Value::Str("".into()));
        }
    }
}

#[test]
fn test_regex_find_unicode_pattern() {
    let result = call_regex_builtin(
        "regex_find",
        vec![
            Value::Str("alpha-\u{03b1} beta-\u{03b2} gamma-\u{03b3}".into()),
            Value::Str(r"[\x{03b1}-\x{03b3}]".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("\u{03b1}".into()),
            Value::Str("\u{03b2}".into()),
            Value::Str("\u{03b3}".into()),
        ])
    );
}

#[test]
fn test_regex_find_adjacent_matches() {
    let result = call_regex_builtin(
        "regex_find",
        vec![
            Value::Str("aabbb".into()),
            Value::Str(r"a+|b+".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("aa".into()),
            Value::Str("bbb".into()),
        ])
    );
}

// ── regex_replace ────────────────────────────────────────────────

#[test]
fn test_regex_replace_basic() {
    let result = call_regex_builtin(
        "regex_replace",
        vec![
            Value::Str("sample_001_R1.fastq".into()),
            Value::Str(r"_R[12]".into()),
            Value::Str("".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("sample_001.fastq".into()));
}

#[test]
fn test_regex_replace_all_occurrences() {
    // replace_all replaces every occurrence
    let result = call_regex_builtin(
        "regex_replace",
        vec![
            Value::Str("aaa bbb aaa".into()),
            Value::Str("aaa".into()),
            Value::Str("XXX".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("XXX bbb XXX".into()));
}

#[test]
fn test_regex_replace_with_backreference() {
    let result = call_regex_builtin(
        "regex_replace",
        vec![
            Value::Str("John Smith".into()),
            Value::Str(r"(\w+)\s+(\w+)".into()),
            Value::Str("$2, $1".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("Smith, John".into()));
}

#[test]
fn test_regex_replace_no_match() {
    let result = call_regex_builtin(
        "regex_replace",
        vec![
            Value::Str("hello world".into()),
            Value::Str(r"\d+".into()),
            Value::Str("NUM".into()),
        ],
    )
    .unwrap();
    // No match means original string returned unchanged
    assert_eq!(result, Value::Str("hello world".into()));
}

#[test]
fn test_regex_replace_empty_string_input() {
    let result = call_regex_builtin(
        "regex_replace",
        vec![
            Value::Str("".into()),
            Value::Str("anything".into()),
            Value::Str("replaced".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("".into()));
}

#[test]
fn test_regex_replace_empty_pattern() {
    // Empty pattern matches at every position, inserts replacement between each char
    let result = call_regex_builtin(
        "regex_replace",
        vec![
            Value::Str("ab".into()),
            Value::Str("".into()),
            Value::Str("-".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("-a-b-".into()));
}

#[test]
fn test_regex_replace_unicode() {
    let result = call_regex_builtin(
        "regex_replace",
        vec![
            Value::Str("caf\u{00e9} na\u{00ef}ve".into()),
            Value::Str(r"[\x{e9}\x{ef}]".into()),
            Value::Str("_".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("caf_ na_ve".into()));
}

// ── regex_split ──────────────────────────────────────────────────

#[test]
fn test_regex_split_basic() {
    let result = call_regex_builtin(
        "regex_split",
        vec![
            Value::Str("gene1;gene2;;gene3".into()),
            Value::Str(r";+".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("gene1".into()),
            Value::Str("gene2".into()),
            Value::Str("gene3".into()),
        ])
    );
}

#[test]
fn test_regex_split_pattern_at_start() {
    let result = call_regex_builtin(
        "regex_split",
        vec![
            Value::Str(",a,b,c".into()),
            Value::Str(",".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("".into()),
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("c".into()),
        ])
    );
}

#[test]
fn test_regex_split_pattern_at_end() {
    let result = call_regex_builtin(
        "regex_split",
        vec![
            Value::Str("a,b,c,".into()),
            Value::Str(",".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("c".into()),
            Value::Str("".into()),
        ])
    );
}

#[test]
fn test_regex_split_no_match() {
    let result = call_regex_builtin(
        "regex_split",
        vec![
            Value::Str("hello world".into()),
            Value::Str(r"\d+".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Str("hello world".into())])
    );
}

#[test]
fn test_regex_split_empty_string() {
    let result = call_regex_builtin(
        "regex_split",
        vec![Value::Str("".into()), Value::Str(",".into())],
    )
    .unwrap();
    assert_eq!(result, Value::List(vec![Value::Str("".into())]));
}

#[test]
fn test_regex_split_whitespace_pattern() {
    let result = call_regex_builtin(
        "regex_split",
        vec![
            Value::Str("  hello   world  ".into()),
            Value::Str(r"\s+".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("".into()),
            Value::Str("hello".into()),
            Value::Str("world".into()),
            Value::Str("".into()),
        ])
    );
}

#[test]
fn test_regex_split_unicode() {
    let result = call_regex_builtin(
        "regex_split",
        vec![
            Value::Str("alpha\u{00b7}beta\u{00b7}gamma".into()),
            Value::Str(r"\x{b7}".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("alpha".into()),
            Value::Str("beta".into()),
            Value::Str("gamma".into()),
        ])
    );
}

// ── error cases ──────────────────────────────────────────────────

#[test]
fn test_regex_invalid_pattern() {
    assert!(call_regex_builtin(
        "regex_match",
        vec![Value::Str("test".into()), Value::Str(r"[invalid".into())]
    )
    .is_err());
}

#[test]
fn test_regex_invalid_pattern_find() {
    assert!(call_regex_builtin(
        "regex_find",
        vec![Value::Str("test".into()), Value::Str(r"(?P<".into())]
    )
    .is_err());
}

#[test]
fn test_regex_invalid_pattern_replace() {
    assert!(call_regex_builtin(
        "regex_replace",
        vec![
            Value::Str("test".into()),
            Value::Str(r"(unclosed".into()),
            Value::Str("x".into()),
        ]
    )
    .is_err());
}

#[test]
fn test_regex_invalid_pattern_split() {
    assert!(call_regex_builtin(
        "regex_split",
        vec![Value::Str("test".into()), Value::Str(r"[bad".into())]
    )
    .is_err());
}

#[test]
fn test_regex_unknown_builtin() {
    assert!(call_regex_builtin("regex_unknown", vec![]).is_err());
}

#[test]
fn test_regex_match_type_error() {
    assert!(call_regex_builtin(
        "regex_match",
        vec![Value::Int(42), Value::Str("pattern".into())]
    )
    .is_err());
}

#[test]
fn test_regex_replace_type_error_replacement() {
    assert!(call_regex_builtin(
        "regex_replace",
        vec![
            Value::Str("text".into()),
            Value::Str("pattern".into()),
            Value::Int(42),
        ]
    )
    .is_err());
}
