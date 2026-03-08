use bl_core::value::Value;
use bl_runtime::json::{call_json_builtin, json_to_value, value_to_json};
use std::collections::HashMap;

// ── json_parse ───────────────────────────────────────────────────

#[test]
fn test_json_parse_object() {
    let result =
        call_json_builtin("json_parse", vec![Value::Str(r#"{"a": 1, "b": "hi"}"#.into())])
            .unwrap();
    if let Value::Record(map) = result {
        assert_eq!(map.get("a"), Some(&Value::Int(1)));
        assert_eq!(map.get("b"), Some(&Value::Str("hi".into())));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_json_parse_array() {
    let result =
        call_json_builtin("json_parse", vec![Value::Str("[1, 2, 3]".into())]).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_json_parse_null() {
    let result = call_json_builtin("json_parse", vec![Value::Str("null".into())]).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_json_parse_boolean_true() {
    let result = call_json_builtin("json_parse", vec![Value::Str("true".into())]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_json_parse_boolean_false() {
    let result = call_json_builtin("json_parse", vec![Value::Str("false".into())]).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_json_parse_nested_object() {
    let input = r#"{"outer": {"inner": 42}}"#;
    let result = call_json_builtin("json_parse", vec![Value::Str(input.into())]).unwrap();
    if let Value::Record(outer) = &result {
        if let Some(Value::Record(inner)) = outer.get("outer") {
            assert_eq!(inner.get("inner"), Some(&Value::Int(42)));
        } else {
            panic!("expected nested Record");
        }
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_json_parse_nested_array() {
    let input = "[[1, 2], [3, 4]]";
    let result = call_json_builtin("json_parse", vec![Value::Str(input.into())]).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::List(vec![Value::Int(3), Value::Int(4)]),
        ])
    );
}

#[test]
fn test_json_parse_mixed_nested() {
    let input = r#"{"data": [1, "two", null, true], "meta": {"count": 4}}"#;
    let result = call_json_builtin("json_parse", vec![Value::Str(input.into())]).unwrap();
    if let Value::Record(map) = &result {
        assert_eq!(
            map.get("data"),
            Some(&Value::List(vec![
                Value::Int(1),
                Value::Str("two".into()),
                Value::Nil,
                Value::Bool(true),
            ]))
        );
        if let Some(Value::Record(meta)) = map.get("meta") {
            assert_eq!(meta.get("count"), Some(&Value::Int(4)));
        } else {
            panic!("expected meta Record");
        }
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_json_parse_empty_object() {
    let result = call_json_builtin("json_parse", vec![Value::Str("{}".into())]).unwrap();
    if let Value::Record(map) = &result {
        assert!(map.is_empty());
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_json_parse_empty_array() {
    let result = call_json_builtin("json_parse", vec![Value::Str("[]".into())]).unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_json_parse_integer() {
    let result = call_json_builtin("json_parse", vec![Value::Str("42".into())]).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_json_parse_float() {
    let result = call_json_builtin("json_parse", vec![Value::Str("3.14".into())]).unwrap();
    assert_eq!(result, Value::Float(3.14));
}

#[test]
fn test_json_parse_negative_number() {
    let result = call_json_builtin("json_parse", vec![Value::Str("-99".into())]).unwrap();
    assert_eq!(result, Value::Int(-99));
}

#[test]
fn test_json_parse_scientific_notation() {
    let result = call_json_builtin("json_parse", vec![Value::Str("1.5e2".into())]).unwrap();
    // 1.5e2 = 150.0 — serde_json parses as f64
    assert_eq!(result, Value::Float(150.0));
}

#[test]
fn test_json_parse_string_with_escapes() {
    let input = r#""hello\nworld\t\"quoted\"""#;
    let result = call_json_builtin("json_parse", vec![Value::Str(input.into())]).unwrap();
    assert_eq!(result, Value::Str("hello\nworld\t\"quoted\"".into()));
}

// ── json_parse: invalid inputs ──────────────────────────────────

#[test]
fn test_json_parse_invalid() {
    assert!(call_json_builtin("json_parse", vec![Value::Str("{bad}".into())]).is_err());
}

#[test]
fn test_json_parse_trailing_comma() {
    assert!(
        call_json_builtin("json_parse", vec![Value::Str(r#"{"a": 1,}"#.into())]).is_err()
    );
}

#[test]
fn test_json_parse_unquoted_key() {
    assert!(
        call_json_builtin("json_parse", vec![Value::Str(r#"{a: 1}"#.into())]).is_err()
    );
}

#[test]
fn test_json_parse_single_quotes() {
    assert!(
        call_json_builtin("json_parse", vec![Value::Str("{'a': 1}".into())]).is_err()
    );
}

#[test]
fn test_json_parse_truncated() {
    assert!(call_json_builtin("json_parse", vec![Value::Str(r#"{"a":"#.into())]).is_err());
}

#[test]
fn test_json_parse_empty_string() {
    assert!(call_json_builtin("json_parse", vec![Value::Str("".into())]).is_err());
}

// ── json_stringify ──────────────────────────────────────────────

#[test]
fn test_json_stringify_record() {
    let mut map = HashMap::new();
    map.insert("x".to_string(), Value::Int(42));
    let result = call_json_builtin("json_stringify", vec![Value::Record(map)]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("\"x\""));
        assert!(s.contains("42"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_json_stringify_nil() {
    let result = call_json_builtin("json_stringify", vec![Value::Nil]).unwrap();
    if let Value::Str(s) = &result {
        assert_eq!(s.trim(), "null");
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_json_stringify_bool() {
    let result = call_json_builtin("json_stringify", vec![Value::Bool(true)]).unwrap();
    if let Value::Str(s) = &result {
        assert_eq!(s.trim(), "true");
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_json_stringify_int() {
    let result = call_json_builtin("json_stringify", vec![Value::Int(123)]).unwrap();
    if let Value::Str(s) = &result {
        assert_eq!(s.trim(), "123");
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_json_stringify_float() {
    let result = call_json_builtin("json_stringify", vec![Value::Float(2.5)]).unwrap();
    if let Value::Str(s) = &result {
        assert_eq!(s.trim(), "2.5");
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_json_stringify_list_mixed_types() {
    let list = Value::List(vec![
        Value::Int(1),
        Value::Str("two".into()),
        Value::Bool(false),
        Value::Nil,
        Value::Float(3.5),
    ]);
    let result = call_json_builtin("json_stringify", vec![list]).unwrap();
    if let Value::Str(s) = &result {
        // Parse it back to verify structure
        let parsed: serde_json::Value = serde_json::from_str(s).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 5);
        assert_eq!(arr[0], serde_json::json!(1));
        assert_eq!(arr[1], serde_json::json!("two"));
        assert_eq!(arr[2], serde_json::json!(false));
        assert!(arr[3].is_null());
        assert_eq!(arr[4], serde_json::json!(3.5));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_json_stringify_deeply_nested_record() {
    let mut inner = HashMap::new();
    inner.insert("z".to_string(), Value::Int(99));

    let mut middle = HashMap::new();
    middle.insert("b".to_string(), Value::Record(inner));

    let mut outer = HashMap::new();
    outer.insert("a".to_string(), Value::Record(middle));

    let result = call_json_builtin("json_stringify", vec![Value::Record(outer)]).unwrap();
    if let Value::Str(s) = &result {
        let parsed: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(parsed["a"]["b"]["z"], serde_json::json!(99));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_json_stringify_empty_list() {
    let result = call_json_builtin("json_stringify", vec![Value::List(vec![])]).unwrap();
    if let Value::Str(s) = &result {
        assert_eq!(s.trim(), "[]");
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_json_stringify_empty_record() {
    let result =
        call_json_builtin("json_stringify", vec![Value::Record(HashMap::new())]).unwrap();
    if let Value::Str(s) = &result {
        assert_eq!(s.trim(), "{}");
    } else {
        panic!("expected Str");
    }
}

// ── roundtrip ────────────────────────────────────────────────────

#[test]
fn test_json_roundtrip_list() {
    let original = Value::List(vec![
        Value::Int(1),
        Value::Str("two".into()),
        Value::Bool(true),
    ]);
    let json_str = call_json_builtin("json_stringify", vec![original.clone()]).unwrap();
    let parsed = call_json_builtin("json_parse", vec![json_str]).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_json_roundtrip_record() {
    let mut map = HashMap::new();
    map.insert("name".to_string(), Value::Str("test".into()));
    map.insert("value".to_string(), Value::Int(42));
    map.insert("flag".to_string(), Value::Bool(false));

    let json_str =
        call_json_builtin("json_stringify", vec![Value::Record(map)]).unwrap();
    let parsed = call_json_builtin("json_parse", vec![json_str]).unwrap();
    // Compare field-by-field since Record PartialEq doesn't compare HashMap contents
    if let Value::Record(rec) = &parsed {
        assert_eq!(rec.get("name"), Some(&Value::Str("test".into())));
        assert_eq!(rec.get("value"), Some(&Value::Int(42)));
        assert_eq!(rec.get("flag"), Some(&Value::Bool(false)));
        assert_eq!(rec.len(), 3);
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_json_roundtrip_nested() {
    let mut inner = HashMap::new();
    inner.insert("items".to_string(), Value::List(vec![Value::Int(1), Value::Int(2)]));

    let mut outer = HashMap::new();
    outer.insert("data".to_string(), Value::Record(inner));
    outer.insert("count".to_string(), Value::Int(2));

    let json_str =
        call_json_builtin("json_stringify", vec![Value::Record(outer)]).unwrap();
    let parsed = call_json_builtin("json_parse", vec![json_str]).unwrap();
    if let Value::Record(rec) = &parsed {
        assert_eq!(rec.get("count"), Some(&Value::Int(2)));
        if let Some(Value::Record(data)) = rec.get("data") {
            assert_eq!(
                data.get("items"),
                Some(&Value::List(vec![Value::Int(1), Value::Int(2)]))
            );
        } else {
            panic!("expected nested Record for 'data'");
        }
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_json_double_roundtrip() {
    // parse -> stringify -> parse -> stringify, confirm stability
    let input = r#"{"key": [1, "two", null, true]}"#;
    let parsed1 = call_json_builtin("json_parse", vec![Value::Str(input.into())]).unwrap();
    let str1 = call_json_builtin("json_stringify", vec![parsed1]).unwrap();
    let parsed2 = call_json_builtin("json_parse", vec![str1]).unwrap();
    let str2 = call_json_builtin("json_stringify", vec![parsed2]).unwrap();

    // Both stringified forms should parse to the same structure
    let final_val = call_json_builtin("json_parse", vec![str2]).unwrap();
    if let Value::Record(rec) = &final_val {
        assert_eq!(
            rec.get("key"),
            Some(&Value::List(vec![
                Value::Int(1),
                Value::Str("two".into()),
                Value::Nil,
                Value::Bool(true),
            ]))
        );
    } else {
        panic!("expected Record");
    }
}

// ── json_to_value / value_to_json direct ─────────────────────────

#[test]
fn test_json_to_value_null() {
    assert_eq!(json_to_value(serde_json::Value::Null), Value::Nil);
}

#[test]
fn test_json_to_value_bool() {
    assert_eq!(
        json_to_value(serde_json::Value::Bool(true)),
        Value::Bool(true)
    );
}

#[test]
fn test_json_to_value_number_int() {
    assert_eq!(json_to_value(serde_json::json!(42)), Value::Int(42));
}

#[test]
fn test_json_to_value_number_float() {
    assert_eq!(json_to_value(serde_json::json!(3.14)), Value::Float(3.14));
}

#[test]
fn test_value_to_json_nil() {
    assert_eq!(value_to_json(&Value::Nil), serde_json::Value::Null);
}

#[test]
fn test_value_to_json_string() {
    assert_eq!(
        value_to_json(&Value::Str("hello".into())),
        serde_json::json!("hello")
    );
}

#[test]
fn test_value_to_json_list() {
    let val = Value::List(vec![Value::Int(1), Value::Int(2)]);
    assert_eq!(value_to_json(&val), serde_json::json!([1, 2]));
}

// ── error cases ──────────────────────────────────────────────────

#[test]
fn test_json_parse_type_error() {
    // Passing a non-string to json_parse should error
    assert!(call_json_builtin("json_parse", vec![Value::Int(42)]).is_err());
}

#[test]
fn test_json_unknown_builtin() {
    assert!(call_json_builtin("json_unknown", vec![]).is_err());
}
