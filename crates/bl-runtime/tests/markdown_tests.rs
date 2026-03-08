// External tests for markdown PUBLIC API: call_markdown_builtin()
// Private helper tests (value_to_markdown, value_to_html) remain inline in markdown.rs.

use bl_core::value::{BioSequence, Table, Value};
use bl_runtime::markdown::call_markdown_builtin;
use std::collections::HashMap;

// ── to_markdown via call_markdown_builtin ─────────────────────

#[test]
fn to_markdown_empty_string() {
    let result = call_markdown_builtin("to_markdown", vec![Value::Str(String::new())]).unwrap();
    assert_eq!(result, Value::Str(String::new()));
}

#[test]
fn to_markdown_nil() {
    let result = call_markdown_builtin("to_markdown", vec![Value::Nil]).unwrap();
    assert_eq!(result, Value::Str(String::new()));
}

#[test]
fn to_markdown_int() {
    let result = call_markdown_builtin("to_markdown", vec![Value::Int(42)]).unwrap();
    assert_eq!(result, Value::Str("42".into()));
}

#[test]
fn to_markdown_float() {
    let result = call_markdown_builtin("to_markdown", vec![Value::Float(3.14)]).unwrap();
    if let Value::Str(s) = &result {
        assert!(s.contains("3.14"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_markdown_bool_true() {
    let result = call_markdown_builtin("to_markdown", vec![Value::Bool(true)]).unwrap();
    assert_eq!(result, Value::Str("true".into()));
}

#[test]
fn to_markdown_bool_false() {
    let result = call_markdown_builtin("to_markdown", vec![Value::Bool(false)]).unwrap();
    assert_eq!(result, Value::Str("false".into()));
}

#[test]
fn to_markdown_dna_sequence() {
    let seq = Value::DNA(BioSequence {
        data: "ATCGATCG".to_string(),
    });
    let result = call_markdown_builtin("to_markdown", vec![seq]).unwrap();
    assert_eq!(result, Value::Str("`ATCGATCG`".into()));
}

#[test]
fn to_markdown_rna_sequence() {
    let seq = Value::RNA(BioSequence {
        data: "AUCGAUCG".to_string(),
    });
    let result = call_markdown_builtin("to_markdown", vec![seq]).unwrap();
    assert_eq!(result, Value::Str("`AUCGAUCG`".into()));
}

#[test]
fn to_markdown_protein_sequence() {
    let seq = Value::Protein(BioSequence {
        data: "MVLSPADKTN".to_string(),
    });
    let result = call_markdown_builtin("to_markdown", vec![seq]).unwrap();
    assert_eq!(result, Value::Str("`MVLSPADKTN`".into()));
}

#[test]
fn to_markdown_special_characters() {
    let result =
        call_markdown_builtin("to_markdown", vec![Value::Str("**bold** & <tag>".into())]).unwrap();
    if let Value::Str(s) = &result {
        // to_markdown doesn't escape — it passes through as-is
        assert!(s.contains("**bold**"));
        assert!(s.contains("<tag>"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_markdown_empty_list() {
    let result = call_markdown_builtin("to_markdown", vec![Value::List(vec![])]).unwrap();
    assert_eq!(result, Value::Str(String::new()));
}

#[test]
fn to_markdown_simple_list() {
    let list = Value::List(vec![
        Value::Str("alpha".into()),
        Value::Str("beta".into()),
        Value::Str("gamma".into()),
    ]);
    let result = call_markdown_builtin("to_markdown", vec![list]).unwrap();
    if let Value::Str(s) = &result {
        assert!(s.contains("- alpha"));
        assert!(s.contains("- beta"));
        assert!(s.contains("- gamma"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_markdown_nested_list() {
    let inner = Value::List(vec![Value::Int(1), Value::Int(2)]);
    let outer = Value::List(vec![inner, Value::Int(3)]);
    let result = call_markdown_builtin("to_markdown", vec![outer]).unwrap();
    assert!(matches!(result, Value::Str(_)));
}

#[test]
fn to_markdown_table() {
    let table = Table {
        columns: vec!["gene".into(), "pval".into()],
        rows: vec![
            vec![Value::Str("BRCA1".into()), Value::Float(0.001)],
            vec![Value::Str("TP53".into()), Value::Float(0.05)],
        ],
        max_col_width: None,
    };
    let result = call_markdown_builtin("to_markdown", vec![Value::Table(table)]).unwrap();
    if let Value::Str(s) = &result {
        assert!(s.contains("| gene |"));
        assert!(s.contains("| BRCA1 |"));
        assert!(s.contains("| TP53 |"));
        // Should have separator line
        assert!(s.contains("---"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_markdown_empty_table() {
    let table = Table {
        columns: vec!["a".into(), "b".into()],
        rows: vec![],
        max_col_width: None,
    };
    let result = call_markdown_builtin("to_markdown", vec![Value::Table(table)]).unwrap();
    if let Value::Str(s) = &result {
        // Should still have header
        assert!(s.contains("| a |"));
        assert!(s.contains("| b |"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_markdown_record() {
    let mut map = HashMap::new();
    map.insert("sample".to_string(), Value::Str("S001".into()));
    map.insert("reads".to_string(), Value::Int(1000000));
    let result = call_markdown_builtin("to_markdown", vec![Value::Record(map)]).unwrap();
    if let Value::Str(s) = &result {
        assert!(s.contains("**sample**: S001"));
        assert!(s.contains("**reads**: 1000000"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_markdown_list_of_records_as_table() {
    let mut r1 = HashMap::new();
    r1.insert("name".to_string(), Value::Str("BRCA1".into()));
    r1.insert("chr".to_string(), Value::Str("17".into()));
    let mut r2 = HashMap::new();
    r2.insert("name".to_string(), Value::Str("TP53".into()));
    r2.insert("chr".to_string(), Value::Str("17".into()));

    let list = Value::List(vec![Value::Record(r1), Value::Record(r2)]);
    let result = call_markdown_builtin("to_markdown", vec![list]).unwrap();
    if let Value::Str(s) = &result {
        // Should render as table, not bullet list
        assert!(s.contains("|"), "expected table format");
        assert!(s.contains("BRCA1"));
        assert!(s.contains("TP53"));
    } else {
        panic!("expected Str");
    }
}

// ── to_html via call_markdown_builtin ─────────────────────────

#[test]
fn to_html_nil() {
    let result = call_markdown_builtin("to_html", vec![Value::Nil]).unwrap();
    if let Value::Str(s) = &result {
        assert!(s.contains("<!DOCTYPE html>"));
        assert!(s.contains("BioLang Report"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_html_int() {
    let result = call_markdown_builtin("to_html", vec![Value::Int(99)]).unwrap();
    if let Value::Str(s) = &result {
        assert!(s.contains("<p>99</p>"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_html_special_characters_escaped() {
    let result = call_markdown_builtin(
        "to_html",
        vec![Value::Str("<script>alert('xss')</script>".into())],
    )
    .unwrap();
    if let Value::Str(s) = &result {
        assert!(!s.contains("<script>"));
        assert!(s.contains("&lt;script&gt;"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_html_dna_sequence() {
    let seq = Value::DNA(BioSequence {
        data: "GATTACA".to_string(),
    });
    let result = call_markdown_builtin("to_html", vec![seq]).unwrap();
    if let Value::Str(s) = &result {
        assert!(s.contains("<code>GATTACA</code>"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_html_table() {
    let table = Table {
        columns: vec!["id".into(), "value".into()],
        rows: vec![vec![Value::Int(1), Value::Str("hello".into())]],
        max_col_width: None,
    };
    let result = call_markdown_builtin("to_html", vec![Value::Table(table)]).unwrap();
    if let Value::Str(s) = &result {
        assert!(s.contains("<table>"));
        assert!(s.contains("<th>id</th>"));
        assert!(s.contains("<td>hello</td>"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn to_html_empty_list() {
    let result = call_markdown_builtin("to_html", vec![Value::List(vec![])]).unwrap();
    if let Value::Str(s) = &result {
        assert!(s.contains("<!DOCTYPE html>"));
        assert!(s.contains("<ul>"));
    } else {
        panic!("expected Str");
    }
}

// ── Error cases ───────────────────────────────────────────────

#[test]
fn unknown_builtin_gives_error() {
    let result = call_markdown_builtin("nonexistent", vec![]);
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("unknown markdown builtin"));
}

#[test]
fn is_markdown_builtin_checks() {
    use bl_runtime::markdown::is_markdown_builtin;
    assert!(is_markdown_builtin("to_markdown"));
    assert!(is_markdown_builtin("to_html"));
    assert!(!is_markdown_builtin("to_pdf"));
    assert!(!is_markdown_builtin(""));
}

#[test]
fn markdown_builtin_list_contains_core() {
    use bl_runtime::markdown::markdown_builtin_list;
    let list = markdown_builtin_list();
    let names: Vec<&str> = list.iter().map(|(n, _)| *n).collect();
    assert!(names.contains(&"to_markdown"));
    assert!(names.contains(&"to_html"));
}
