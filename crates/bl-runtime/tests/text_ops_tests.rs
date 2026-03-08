use bl_core::value::{StreamValue, Value};
use bl_runtime::text_ops::call_text_builtin;
use std::collections::HashMap;

// ── lines ────────────────────────────────────────────────────────

#[test]
fn test_lines_basic() {
    let result = call_text_builtin("lines", vec![Value::Str("a\nb\nc".into())]).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("c".into()),
        ])
    );
}

#[test]
fn test_lines_crlf() {
    let result = call_text_builtin("lines", vec![Value::Str("a\r\nb\r\nc".into())]).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("c".into()),
        ])
    );
}

#[test]
fn test_lines_trailing_newline() {
    let result = call_text_builtin("lines", vec![Value::Str("a\nb\n".into())]).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Str("a".into()), Value::Str("b".into())])
    );
}

#[test]
fn test_lines_empty_string() {
    let result = call_text_builtin("lines", vec![Value::Str("".into())]).unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_lines_single_line_no_newline() {
    let result = call_text_builtin("lines", vec![Value::Str("hello".into())]).unwrap();
    assert_eq!(result, Value::List(vec![Value::Str("hello".into())]));
}

#[test]
fn test_lines_single_line_with_newline() {
    let result = call_text_builtin("lines", vec![Value::Str("hello\n".into())]).unwrap();
    assert_eq!(result, Value::List(vec![Value::Str("hello".into())]));
}

// ── grep ─────────────────────────────────────────────────────────

#[test]
fn test_grep_basic() {
    let result = call_text_builtin(
        "grep",
        vec![
            Value::Str("PASS\nFAIL\nPASS\nERROR".into()),
            Value::Str("PASS".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Str("PASS".into()), Value::Str("PASS".into())])
    );
}

#[test]
fn test_grep_no_match() {
    let result = call_text_builtin(
        "grep",
        vec![
            Value::Str("hello\nworld".into()),
            Value::Str("xyz".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_grep_every_line_matches() {
    let result = call_text_builtin(
        "grep",
        vec![
            Value::Str("abc\nabc\nabc".into()),
            Value::Str("abc".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("abc".into()),
            Value::Str("abc".into()),
            Value::Str("abc".into()),
        ])
    );
}

#[test]
fn test_grep_list_input() {
    let result = call_text_builtin(
        "grep",
        vec![
            Value::List(vec![
                Value::Str("foo".into()),
                Value::Str("bar".into()),
                Value::Str("baz".into()),
            ]),
            Value::Str("ba".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Str("bar".into()), Value::Str("baz".into())])
    );
}

#[test]
fn test_grep_invert() {
    let mut flags = HashMap::new();
    flags.insert("invert".to_string(), Value::Bool(true));
    let result = call_text_builtin(
        "grep",
        vec![
            Value::Str("PASS\nFAIL\nPASS".into()),
            Value::Str("PASS".into()),
            Value::Record(flags),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::List(vec![Value::Str("FAIL".into())]));
}

#[test]
fn test_grep_count() {
    let mut flags = HashMap::new();
    flags.insert("count".to_string(), Value::Bool(true));
    let result = call_text_builtin(
        "grep",
        vec![
            Value::Str("a\nb\na\nc\na".into()),
            Value::Str("a".into()),
            Value::Record(flags),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_grep_ignore_case() {
    let mut flags = HashMap::new();
    flags.insert("ignore_case".to_string(), Value::Bool(true));
    let result = call_text_builtin(
        "grep",
        vec![
            Value::Str("Hello\nhello\nHELLO".into()),
            Value::Str("hello".into()),
            Value::Record(flags),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("Hello".into()),
            Value::Str("hello".into()),
            Value::Str("HELLO".into()),
        ])
    );
}

#[test]
fn test_grep_line_numbers() {
    let mut flags = HashMap::new();
    flags.insert("line_numbers".to_string(), Value::Bool(true));
    let result = call_text_builtin(
        "grep",
        vec![
            Value::Str("a\nb\nc\nb".into()),
            Value::Str("b".into()),
            Value::Record(flags),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::List(vec![Value::Int(2), Value::Str("b".into())]),
            Value::List(vec![Value::Int(4), Value::Str("b".into())]),
        ])
    );
}

#[test]
fn test_grep_regex_pattern() {
    let result = call_text_builtin(
        "grep",
        vec![
            Value::Str("abc123\ndef456\nghi".into()),
            Value::Str(r"\d+".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("abc123".into()),
            Value::Str("def456".into()),
        ])
    );
}

#[test]
fn test_grep_empty_input() {
    let result = call_text_builtin(
        "grep",
        vec![Value::Str("".into()), Value::Str("anything".into())],
    )
    .unwrap();
    assert_eq!(result, Value::List(vec![]));
}

// ── grep_count ───────────────────────────────────────────────────

#[test]
fn test_grep_count_basic() {
    let result = call_text_builtin(
        "grep_count",
        vec![
            Value::Str("ERR\nOK\nERR\nOK\nOK".into()),
            Value::Str("ERR".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_grep_count_zero() {
    let result = call_text_builtin(
        "grep_count",
        vec![
            Value::Str("hello\nworld".into()),
            Value::Str("zzz".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Int(0));
}

// ── cut ──────────────────────────────────────────────────────────

#[test]
fn test_cut_single_field() {
    let result = call_text_builtin(
        "cut",
        vec![
            Value::Str("a\tb\tc\nd\te\tf".into()),
            Value::Str("\t".into()),
            Value::Int(1),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Str("b".into()), Value::Str("e".into())])
    );
}

#[test]
fn test_cut_multiple_fields() {
    let result = call_text_builtin(
        "cut",
        vec![
            Value::Str("a,b,c\nd,e,f".into()),
            Value::Str(",".into()),
            Value::List(vec![Value::Int(0), Value::Int(2)]),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::List(vec![Value::Str("a".into()), Value::Str("c".into())]),
            Value::List(vec![Value::Str("d".into()), Value::Str("f".into())]),
        ])
    );
}

#[test]
fn test_cut_field_out_of_bounds() {
    let result = call_text_builtin(
        "cut",
        vec![
            Value::Str("a,b".into()),
            Value::Str(",".into()),
            Value::Int(5),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::List(vec![Value::Str("".into())]));
}

#[test]
fn test_cut_single_column_data() {
    // Data with no delimiters: entire line is field 0
    let result = call_text_builtin(
        "cut",
        vec![
            Value::Str("alpha\nbeta\ngamma".into()),
            Value::Str(",".into()),
            Value::Int(0),
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

#[test]
fn test_cut_single_column_out_of_bounds() {
    // Data with no delimiter, requesting field 1 (out of bounds)
    let result = call_text_builtin(
        "cut",
        vec![
            Value::Str("alpha\nbeta".into()),
            Value::Str(",".into()),
            Value::Int(1),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Str("".into()), Value::Str("".into())])
    );
}

// ── paste ────────────────────────────────────────────────────────

#[test]
fn test_paste_default_tab() {
    let result = call_text_builtin(
        "paste",
        vec![
            Value::List(vec![Value::Str("a".into()), Value::Str("b".into())]),
            Value::List(vec![Value::Str("1".into()), Value::Str("2".into())]),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("a\t1".into()),
            Value::Str("b\t2".into()),
        ])
    );
}

#[test]
fn test_paste_custom_sep() {
    let result = call_text_builtin(
        "paste",
        vec![
            Value::List(vec![Value::Str("x".into()), Value::Str("y".into())]),
            Value::List(vec![Value::Str("1".into()), Value::Str("2".into())]),
            Value::Str(",".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("x,1".into()),
            Value::Str("y,2".into()),
        ])
    );
}

#[test]
fn test_paste_unequal_lengths() {
    let result = call_text_builtin(
        "paste",
        vec![
            Value::List(vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ]),
            Value::List(vec![Value::Str("1".into())]),
            Value::Str(",".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("a,1".into()),
            Value::Str("b,".into()),
            Value::Str("c,".into()),
        ])
    );
}

#[test]
fn test_paste_both_empty() {
    let result = call_text_builtin(
        "paste",
        vec![Value::List(vec![]), Value::List(vec![])],
    )
    .unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_paste_one_empty() {
    let result = call_text_builtin(
        "paste",
        vec![
            Value::List(vec![Value::Str("a".into()), Value::Str("b".into())]),
            Value::List(vec![]),
            Value::Str(",".into()),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("a,".into()),
            Value::Str("b,".into()),
        ])
    );
}

// ── uniq_count ───────────────────────────────────────────────────

#[test]
fn test_uniq_count_basic() {
    let result = call_text_builtin(
        "uniq_count",
        vec![Value::List(vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("a".into()),
            Value::Str("c".into()),
            Value::Str("a".into()),
            Value::Str("b".into()),
        ])],
    )
    .unwrap();

    if let Value::List(items) = &result {
        assert_eq!(items.len(), 3);
        // First should be "a" with count 3 (sorted desc by count)
        if let Value::Record(rec) = &items[0] {
            assert_eq!(rec.get("value"), Some(&Value::Str("a".into())));
            assert_eq!(rec.get("count"), Some(&Value::Int(3)));
        } else {
            panic!("expected Record");
        }
        // Second should be "b" with count 2
        if let Value::Record(rec) = &items[1] {
            assert_eq!(rec.get("value"), Some(&Value::Str("b".into())));
            assert_eq!(rec.get("count"), Some(&Value::Int(2)));
        } else {
            panic!("expected Record");
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_uniq_count_empty() {
    let result = call_text_builtin("uniq_count", vec![Value::List(vec![])]).unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_uniq_count_single() {
    let result =
        call_text_builtin("uniq_count", vec![Value::List(vec![Value::Int(42)])]).unwrap();
    if let Value::List(items) = &result {
        assert_eq!(items.len(), 1);
        if let Value::Record(rec) = &items[0] {
            assert_eq!(rec.get("value"), Some(&Value::Int(42)));
            assert_eq!(rec.get("count"), Some(&Value::Int(1)));
        }
    }
}

#[test]
fn test_uniq_count_all_unique() {
    let result = call_text_builtin(
        "uniq_count",
        vec![Value::List(vec![
            Value::Str("x".into()),
            Value::Str("y".into()),
            Value::Str("z".into()),
        ])],
    )
    .unwrap();
    if let Value::List(items) = &result {
        assert_eq!(items.len(), 3);
        // All have count 1, so order is by insertion
        for item in items {
            if let Value::Record(rec) = item {
                assert_eq!(rec.get("count"), Some(&Value::Int(1)));
            } else {
                panic!("expected Record");
            }
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_uniq_count_single_item_repeated() {
    let result = call_text_builtin(
        "uniq_count",
        vec![Value::List(vec![
            Value::Str("dup".into()),
            Value::Str("dup".into()),
            Value::Str("dup".into()),
            Value::Str("dup".into()),
        ])],
    )
    .unwrap();
    if let Value::List(items) = &result {
        assert_eq!(items.len(), 1);
        if let Value::Record(rec) = &items[0] {
            assert_eq!(rec.get("value"), Some(&Value::Str("dup".into())));
            assert_eq!(rec.get("count"), Some(&Value::Int(4)));
        } else {
            panic!("expected Record");
        }
    } else {
        panic!("expected List");
    }
}

// ── wc ───────────────────────────────────────────────────────────

#[test]
fn test_wc_basic() {
    let result =
        call_text_builtin("wc", vec![Value::Str("hello world\nfoo".into())]).unwrap();
    if let Value::Record(rec) = &result {
        assert_eq!(rec.get("lines"), Some(&Value::Int(2)));
        assert_eq!(rec.get("words"), Some(&Value::Int(3)));
        assert_eq!(rec.get("chars"), Some(&Value::Int(15)));
        assert_eq!(rec.get("bytes"), Some(&Value::Int(15)));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_wc_empty() {
    let result = call_text_builtin("wc", vec![Value::Str("".into())]).unwrap();
    if let Value::Record(rec) = &result {
        assert_eq!(rec.get("lines"), Some(&Value::Int(0)));
        assert_eq!(rec.get("words"), Some(&Value::Int(0)));
        assert_eq!(rec.get("chars"), Some(&Value::Int(0)));
        assert_eq!(rec.get("bytes"), Some(&Value::Int(0)));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_wc_unicode() {
    // Unicode: chars and bytes will differ
    // "cafe\u0301" = "cafe" + combining acute = 5 chars but 6 bytes
    // Or simpler: use multi-byte chars like emojis or CJK
    let result = call_text_builtin("wc", vec![Value::Str("hello\u{00e9}".into())]).unwrap();
    // "hello" (5 chars) + e-acute (1 char, 2 bytes) = 6 chars, 7 bytes
    if let Value::Record(rec) = &result {
        assert_eq!(rec.get("chars"), Some(&Value::Int(6)));
        assert_eq!(rec.get("bytes"), Some(&Value::Int(7)));
        assert_eq!(rec.get("words"), Some(&Value::Int(1)));
        assert_eq!(rec.get("lines"), Some(&Value::Int(1)));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_wc_only_whitespace() {
    let result = call_text_builtin("wc", vec![Value::Str("   \n  \n".into())]).unwrap();
    if let Value::Record(rec) = &result {
        assert_eq!(rec.get("words"), Some(&Value::Int(0)));
        assert_eq!(rec.get("lines"), Some(&Value::Int(2)));
    } else {
        panic!("expected Record");
    }
}

// ── stream_concat ───────────────────────────────────────────────

#[test]
fn test_stream_concat_two_lists() {
    let result = call_text_builtin(
        "stream_concat",
        vec![
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::List(vec![Value::Int(3), Value::Int(4)]),
        ],
    )
    .unwrap();
    assert!(matches!(result, Value::Stream(_)));
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(
            items,
            vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)]
        );
    }
}

#[test]
fn test_stream_concat_stream_and_list() {
    let stream = Value::Stream(StreamValue::from_list(
        "a",
        vec![Value::Str("x".into()), Value::Str("y".into())],
    ));
    let list = Value::List(vec![Value::Str("z".into())]);
    let result = call_text_builtin("stream_concat", vec![stream, list]).unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(
            items,
            vec![
                Value::Str("x".into()),
                Value::Str("y".into()),
                Value::Str("z".into()),
            ]
        );
    }
}

#[test]
fn test_stream_concat_both_empty() {
    let result = call_text_builtin(
        "stream_concat",
        vec![Value::List(vec![]), Value::List(vec![])],
    )
    .unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert!(items.is_empty());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_stream_concat_first_empty() {
    let result = call_text_builtin(
        "stream_concat",
        vec![
            Value::List(vec![]),
            Value::List(vec![Value::Int(1), Value::Int(2)]),
        ],
    )
    .unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items, vec![Value::Int(1), Value::Int(2)]);
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_stream_concat_second_empty() {
    let result = call_text_builtin(
        "stream_concat",
        vec![
            Value::List(vec![Value::Str("a".into())]),
            Value::List(vec![]),
        ],
    )
    .unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items, vec![Value::Str("a".into())]);
    } else {
        panic!("expected Stream");
    }
}

// ── error cases ──────────────────────────────────────────────────

#[test]
fn test_unknown_builtin() {
    assert!(call_text_builtin("nonexistent", vec![]).is_err());
}

#[test]
fn test_lines_type_error() {
    assert!(call_text_builtin("lines", vec![Value::Int(42)]).is_err());
}

#[test]
fn test_grep_invalid_regex() {
    let result = call_text_builtin(
        "grep",
        vec![
            Value::Str("test".into()),
            Value::Str("[invalid".into()),
        ],
    );
    assert!(result.is_err());
}

#[test]
fn test_cut_bad_field_type() {
    let result = call_text_builtin(
        "cut",
        vec![
            Value::Str("a,b".into()),
            Value::Str(",".into()),
            Value::Str("not a number".into()),
        ],
    );
    assert!(result.is_err());
}

#[test]
fn test_paste_type_error() {
    let result = call_text_builtin(
        "paste",
        vec![Value::Str("not a list".into()), Value::List(vec![])],
    );
    assert!(result.is_err());
}

#[test]
fn test_wc_type_error() {
    let result = call_text_builtin("wc", vec![Value::List(vec![])]);
    assert!(result.is_err());
}
