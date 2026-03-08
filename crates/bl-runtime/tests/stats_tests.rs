use bl_core::value::{Table, Value};
use bl_runtime::stats::call_stats_builtin;

fn int_list(vals: &[i64]) -> Value {
    Value::List(vals.iter().map(|v| Value::Int(*v)).collect())
}
fn float_list(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|v| Value::Float(*v)).collect())
}

fn get_record_float(val: &Value, key: &str) -> f64 {
    match val {
        Value::Record(map) => match map.get(key).unwrap() {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => panic!("expected numeric for key {key}"),
        },
        _ => panic!("expected Record"),
    }
}

// ════════════════════════════════════════════════════════════════
// Original 58 tests (migrated from inline mod tests)
// ════════════════════════════════════════════════════════════════

#[test]
fn test_mean() {
    let result = call_stats_builtin("mean", vec![int_list(&[10, 20, 30])]).unwrap();
    assert_eq!(result, Value::Float(20.0));
}

#[test]
fn test_median_odd() {
    let result = call_stats_builtin("median", vec![int_list(&[3, 1, 2])]).unwrap();
    assert_eq!(result, Value::Float(2.0));
}

#[test]
fn test_median_even() {
    let result = call_stats_builtin("median", vec![int_list(&[1, 2, 3, 4])]).unwrap();
    assert_eq!(result, Value::Float(2.5));
}

#[test]
fn test_sum_int() {
    let result = call_stats_builtin("sum", vec![int_list(&[1, 2, 3, 4, 5])]).unwrap();
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_sum_float() {
    let list = Value::List(vec![Value::Int(1), Value::Float(2.5), Value::Int(3)]);
    let result = call_stats_builtin("sum", vec![list]).unwrap();
    assert_eq!(result, Value::Float(6.5));
}

#[test]
fn test_variance() {
    let result =
        call_stats_builtin("variance", vec![int_list(&[2, 4, 4, 4, 5, 5, 7, 9])]).unwrap();
    if let Value::Float(v) = result {
        assert!((v - 4.571).abs() < 0.01);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_stdev() {
    let result =
        call_stats_builtin("stdev", vec![int_list(&[2, 4, 4, 4, 5, 5, 7, 9])]).unwrap();
    if let Value::Float(v) = result {
        assert!((v - 2.138).abs() < 0.01);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_quantile() {
    let result = call_stats_builtin(
        "quantile",
        vec![int_list(&[1, 2, 3, 4, 5]), Value::Float(0.5)],
    )
    .unwrap();
    assert_eq!(result, Value::Float(3.0));
}

#[test]
fn test_cor_perfect() {
    let result = call_stats_builtin(
        "cor",
        vec![int_list(&[1, 2, 3, 4, 5]), int_list(&[2, 4, 6, 8, 10])],
    )
    .unwrap();
    if let Value::Float(r) = result {
        assert!((r - 1.0).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_unique() {
    let result =
        call_stats_builtin("unique", vec![int_list(&[1, 2, 2, 3, 1, 3])]).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_sample_count() {
    let result =
        call_stats_builtin("sample", vec![int_list(&[1, 2, 3, 4, 5]), Value::Int(3)])
            .unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_cumsum() {
    let result = call_stats_builtin("cumsum", vec![int_list(&[1, 2, 3, 4])]).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Int(1),
            Value::Int(3),
            Value::Int(6),
            Value::Int(10)
        ])
    );
}

#[test]
fn test_sqrt() {
    assert_eq!(
        call_stats_builtin("sqrt", vec![Value::Int(16)]).unwrap(),
        Value::Float(4.0)
    );
}

#[test]
fn test_pow() {
    assert_eq!(
        call_stats_builtin("pow", vec![Value::Int(2), Value::Int(10)]).unwrap(),
        Value::Float(1024.0)
    );
}

#[test]
fn test_log_exp_roundtrip() {
    let e = call_stats_builtin("exp", vec![Value::Int(1)]).unwrap();
    if let Value::Float(e_val) = e {
        let ln = call_stats_builtin("log", vec![Value::Float(e_val)]).unwrap();
        if let Value::Float(v) = ln {
            assert!((v - 1.0).abs() < 1e-10);
        }
    }
}

#[test]
fn test_ceil_floor_round() {
    assert_eq!(
        call_stats_builtin("ceil", vec![Value::Float(3.2)]).unwrap(),
        Value::Int(4)
    );
    assert_eq!(
        call_stats_builtin("floor", vec![Value::Float(3.8)]).unwrap(),
        Value::Int(3)
    );
    assert_eq!(
        call_stats_builtin("round", vec![Value::Float(3.5)]).unwrap(),
        Value::Int(4)
    );
    assert_eq!(
        call_stats_builtin("round", vec![Value::Float(3.14159), Value::Int(2)]).unwrap(),
        Value::Float(3.14)
    );
}

#[test]
fn test_upper_lower_trim() {
    assert_eq!(
        call_stats_builtin("upper", vec![Value::Str("hello".into())]).unwrap(),
        Value::Str("HELLO".into())
    );
    assert_eq!(
        call_stats_builtin("lower", vec![Value::Str("HELLO".into())]).unwrap(),
        Value::Str("hello".into())
    );
    assert_eq!(
        call_stats_builtin("trim", vec![Value::Str("  hi  ".into())]).unwrap(),
        Value::Str("hi".into())
    );
}

#[test]
fn test_starts_ends_with() {
    assert_eq!(
        call_stats_builtin(
            "starts_with",
            vec![Value::Str("hello".into()), Value::Str("hel".into())]
        )
        .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        call_stats_builtin(
            "ends_with",
            vec![Value::Str("hello".into()), Value::Str("llo".into())]
        )
        .unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn test_str_replace() {
    assert_eq!(
        call_stats_builtin(
            "str_replace",
            vec![
                Value::Str("hello world".into()),
                Value::Str("world".into()),
                Value::Str("there".into()),
            ]
        )
        .unwrap(),
        Value::Str("hello there".into())
    );
}

#[test]
fn test_substr() {
    assert_eq!(
        call_stats_builtin(
            "substr",
            vec![Value::Str("hello world".into()), Value::Int(6), Value::Int(5),]
        )
        .unwrap(),
        Value::Str("world".into())
    );
}

#[test]
fn test_summary() {
    let table = Table::new(
        vec!["name".into(), "score".into()],
        vec![
            vec![Value::Str("Alice".into()), Value::Int(90)],
            vec![Value::Str("Bob".into()), Value::Int(80)],
            vec![Value::Str("Carol".into()), Value::Int(70)],
        ],
    );
    let result = call_stats_builtin("summary", vec![Value::Table(table)]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 2);
        assert_eq!(
            t.columns,
            vec!["column", "type", "count", "min", "max", "mean"]
        );
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_log2_log10() {
    let r = call_stats_builtin("log2", vec![Value::Int(8)]).unwrap();
    if let Value::Float(v) = r {
        assert!((v - 3.0).abs() < 1e-10);
    }
    let r = call_stats_builtin("log10", vec![Value::Int(1000)]).unwrap();
    if let Value::Float(v) = r {
        assert!((v - 3.0).abs() < 1e-10);
    }
}

#[test]
fn test_ttest_significant() {
    let a = float_list(&[2.0, 3.0, 4.0, 5.0, 6.0]);
    let b = float_list(&[5.0, 6.0, 7.0, 8.0, 9.0]);
    let result = call_stats_builtin("ttest", vec![a, b]).unwrap();
    let p = get_record_float(&result, "p_value");
    assert!(p < 0.05, "p={p} should be < 0.05");
}

#[test]
fn test_ttest_one_known() {
    let data = float_list(&[10.0, 10.1, 9.9, 10.0, 10.05]);
    let result = call_stats_builtin("ttest_one", vec![data, Value::Float(10.0)]).unwrap();
    let p = get_record_float(&result, "p_value");
    assert!(p > 0.05, "p={p} should be > 0.05 for data centered at mu");
}

#[test]
fn test_anova_same_groups() {
    let groups = Value::List(vec![
        float_list(&[5.0, 5.1, 4.9]),
        float_list(&[5.0, 5.1, 4.9]),
        float_list(&[5.0, 5.1, 4.9]),
    ]);
    let result = call_stats_builtin("anova", vec![groups]).unwrap();
    let p = get_record_float(&result, "p_value");
    assert!(p > 0.5, "p={p} should be high for identical groups");
}

#[test]
fn test_chi_square_uniform() {
    let obs = float_list(&[25.0, 25.0, 25.0, 25.0]);
    let exp = float_list(&[25.0, 25.0, 25.0, 25.0]);
    let result = call_stats_builtin("chi_square", vec![obs, exp]).unwrap();
    let chi2 = get_record_float(&result, "chi2");
    assert!(chi2 < 0.01, "chi2={chi2} should be ~0 for uniform");
}

#[test]
fn test_fisher_exact_2x2() {
    let result = call_stats_builtin(
        "fisher_exact",
        vec![Value::Int(10), Value::Int(2), Value::Int(3), Value::Int(15)],
    )
    .unwrap();
    let p = get_record_float(&result, "p_value");
    assert!(p < 0.05, "p={p} should be significant");
}

#[test]
fn test_p_adjust_bh() {
    let pvals = float_list(&[0.01, 0.04, 0.03, 0.005]);
    let result = call_stats_builtin("p_adjust", vec![pvals, Value::Str("bh".into())]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 4);
        for item in &items {
            if let Value::Float(p) = item {
                assert!(*p >= 0.0 && *p <= 1.0);
            }
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_normalize_zscore() {
    let data = float_list(&[10.0, 20.0, 30.0, 40.0, 50.0]);
    let result =
        call_stats_builtin("normalize", vec![data, Value::Str("zscore".into())]).unwrap();
    if let Value::List(items) = result {
        let vals: Vec<f64> = items
            .iter()
            .map(|v| match v {
                Value::Float(f) => *f,
                _ => panic!("expected Float"),
            })
            .collect();
        let mean: f64 = vals.iter().sum::<f64>() / vals.len() as f64;
        assert!(mean.abs() < 1e-10, "mean should be ~0, got {mean}");
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_lm_perfect_linear() {
    let x = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let y = float_list(&[3.0, 5.0, 7.0, 9.0, 11.0]);
    let result = call_stats_builtin("lm", vec![x, y]).unwrap();
    let slope = get_record_float(&result, "slope");
    let intercept = get_record_float(&result, "intercept");
    let r2 = get_record_float(&result, "r_squared");
    assert!((slope - 2.0).abs() < 1e-10, "slope={slope}");
    assert!((intercept - 1.0).abs() < 1e-10, "intercept={intercept}");
    assert!((r2 - 1.0).abs() < 1e-10, "r2={r2}");
}

#[test]
fn test_char_at() {
    let result =
        call_stats_builtin("char_at", vec![Value::Str("hello".into()), Value::Int(1)]).unwrap();
    assert_eq!(result, Value::Str("e".into()));
}

#[test]
fn test_index_of() {
    let result = call_stats_builtin(
        "index_of",
        vec![Value::Str("hello world".into()), Value::Str("world".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_str_repeat() {
    let result =
        call_stats_builtin("str_repeat", vec![Value::Str("ab".into()), Value::Int(3)]).unwrap();
    assert_eq!(result, Value::Str("ababab".into()));
}

#[test]
fn test_pad_left() {
    let result = call_stats_builtin(
        "pad_left",
        vec![Value::Str("42".into()), Value::Int(5), Value::Str("0".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Str("00042".into()));
}

#[test]
fn test_pad_right() {
    let result = call_stats_builtin(
        "pad_right",
        vec![Value::Str("hi".into()), Value::Int(5), Value::Str(".".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Str("hi...".into()));
}

#[test]
fn test_trim_left_right() {
    assert_eq!(
        call_stats_builtin("trim_left", vec![Value::Str("  hi  ".into())]).unwrap(),
        Value::Str("hi  ".into())
    );
    assert_eq!(
        call_stats_builtin("trim_right", vec![Value::Str("  hi  ".into())]).unwrap(),
        Value::Str("  hi".into())
    );
}

#[test]
fn test_format() {
    let result = call_stats_builtin(
        "format",
        vec![
            Value::Str("Hello {0}, you are {1}".into()),
            Value::Str("Alice".into()),
            Value::Int(30),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("Hello Alice, you are 30".into()));
}

#[test]
fn test_sign() {
    assert_eq!(
        call_stats_builtin("sign", vec![Value::Int(-5)]).unwrap(),
        Value::Int(-1)
    );
    assert_eq!(
        call_stats_builtin("sign", vec![Value::Int(0)]).unwrap(),
        Value::Int(0)
    );
    assert_eq!(
        call_stats_builtin("sign", vec![Value::Float(3.14)]).unwrap(),
        Value::Float(1.0)
    );
}

#[test]
fn test_clamp() {
    let result = call_stats_builtin(
        "clamp",
        vec![Value::Float(15.0), Value::Float(0.0), Value::Float(10.0)],
    )
    .unwrap();
    assert_eq!(result, Value::Float(10.0));
}

#[test]
fn test_trig_sin_cos() {
    let result = call_stats_builtin("sin", vec![Value::Float(0.0)]).unwrap();
    assert_eq!(result, Value::Float(0.0));
    if let Value::Float(c) = call_stats_builtin("cos", vec![Value::Float(0.0)]).unwrap() {
        assert!((c - 1.0).abs() < 1e-10);
    }
}

#[test]
fn test_pi_euler() {
    assert_eq!(
        call_stats_builtin("pi", vec![]).unwrap(),
        Value::Float(std::f64::consts::PI)
    );
    assert_eq!(
        call_stats_builtin("euler", vec![]).unwrap(),
        Value::Float(std::f64::consts::E)
    );
}

#[test]
fn test_is_nan_is_finite() {
    assert_eq!(
        call_stats_builtin("is_nan", vec![Value::Float(f64::NAN)]).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        call_stats_builtin("is_nan", vec![Value::Float(1.0)]).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        call_stats_builtin("is_finite", vec![Value::Float(f64::INFINITY)]).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        call_stats_builtin("is_finite", vec![Value::Int(42)]).unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn test_random() {
    if let Value::Float(r) = call_stats_builtin("random", vec![]).unwrap() {
        assert!(r >= 0.0 && r <= 1.0, "random() out of range: {r}");
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_random_int() {
    if let Value::Int(r) =
        call_stats_builtin("random_int", vec![Value::Int(1), Value::Int(10)]).unwrap()
    {
        assert!(r >= 1 && r < 10, "random_int() out of range: {r}");
    } else {
        panic!("expected Int");
    }
}

// ════════════════════════════════════════════════════════════════
// New edge-case and coverage tests
// ════════════════════════════════════════════════════════════════

// ── mean edge cases ─────────────────────────────────────────────

#[test]
fn test_mean_empty_list_error() {
    let result = call_stats_builtin("mean", vec![Value::List(vec![])]);
    assert!(result.is_err(), "mean of empty list should error");
}

#[test]
fn test_mean_single_element() {
    let result = call_stats_builtin("mean", vec![int_list(&[42])]).unwrap();
    assert_eq!(result, Value::Float(42.0));
}

#[test]
fn test_mean_mixed_int_float() {
    let list = Value::List(vec![Value::Int(1), Value::Float(2.0), Value::Int(3)]);
    let result = call_stats_builtin("mean", vec![list]).unwrap();
    assert_eq!(result, Value::Float(2.0));
}

// ── median edge cases ───────────────────────────────────────────

#[test]
fn test_median_single_element() {
    let result = call_stats_builtin("median", vec![int_list(&[7])]).unwrap();
    assert_eq!(result, Value::Float(7.0));
}

#[test]
fn test_median_two_elements() {
    let result = call_stats_builtin("median", vec![int_list(&[10, 20])]).unwrap();
    assert_eq!(result, Value::Float(15.0));
}

// ── sum edge cases ──────────────────────────────────────────────

#[test]
fn test_sum_empty_list() {
    let result = call_stats_builtin("sum", vec![Value::List(vec![])]).unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_sum_single_int() {
    let result = call_stats_builtin("sum", vec![int_list(&[99])]).unwrap();
    assert_eq!(result, Value::Int(99));
}

// ── variance / stdev edge cases ─────────────────────────────────

#[test]
fn test_variance_single_element_error() {
    let result = call_stats_builtin("variance", vec![int_list(&[5])]);
    assert!(result.is_err(), "variance of single element should error (need >= 2)");
}

#[test]
fn test_stdev_single_element_error() {
    let result = call_stats_builtin("stdev", vec![int_list(&[5])]);
    assert!(result.is_err(), "stdev of single element should error");
}

#[test]
fn test_variance_identical_values() {
    let result = call_stats_builtin("variance", vec![int_list(&[5, 5, 5, 5])]).unwrap();
    assert_eq!(result, Value::Float(0.0));
}

// ── quantile edge cases ─────────────────────────────────────────

#[test]
fn test_quantile_at_zero() {
    let result = call_stats_builtin(
        "quantile",
        vec![int_list(&[1, 2, 3, 4, 5]), Value::Float(0.0)],
    )
    .unwrap();
    assert_eq!(result, Value::Float(1.0));
}

#[test]
fn test_quantile_at_one() {
    let result = call_stats_builtin(
        "quantile",
        vec![int_list(&[1, 2, 3, 4, 5]), Value::Float(1.0)],
    )
    .unwrap();
    assert_eq!(result, Value::Float(5.0));
}

#[test]
fn test_quantile_at_half_equals_median() {
    let data = int_list(&[1, 2, 3, 4, 5]);
    let q = call_stats_builtin("quantile", vec![data.clone(), Value::Float(0.5)]).unwrap();
    let m = call_stats_builtin("median", vec![data]).unwrap();
    assert_eq!(q, m, "quantile(0.5) should equal median");
}

#[test]
fn test_quantile_invalid_too_high() {
    let result = call_stats_builtin(
        "quantile",
        vec![int_list(&[1, 2, 3]), Value::Float(1.5)],
    );
    assert!(result.is_err(), "quantile > 1 should error");
}

#[test]
fn test_quantile_invalid_negative() {
    let result = call_stats_builtin(
        "quantile",
        vec![int_list(&[1, 2, 3]), Value::Float(-0.1)],
    );
    assert!(result.is_err(), "quantile < 0 should error");
}

// ── cor edge cases ──────────────────────────────────────────────

#[test]
fn test_cor_different_lengths_error() {
    let result = call_stats_builtin(
        "cor",
        vec![int_list(&[1, 2, 3]), int_list(&[1, 2])],
    );
    assert!(result.is_err(), "cor of unequal length lists should error");
}

#[test]
fn test_cor_constant_list_nan() {
    let result = call_stats_builtin(
        "cor",
        vec![int_list(&[5, 5, 5, 5]), int_list(&[1, 2, 3, 4])],
    )
    .unwrap();
    if let Value::Float(r) = result {
        assert!(r.is_nan(), "cor of constant list should be NaN");
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_cor_negative_perfect() {
    let result = call_stats_builtin(
        "cor",
        vec![int_list(&[1, 2, 3, 4, 5]), int_list(&[10, 8, 6, 4, 2])],
    )
    .unwrap();
    if let Value::Float(r) = result {
        assert!((r - (-1.0)).abs() < 1e-10, "perfect negative correlation expected, got {r}");
    } else {
        panic!("expected Float");
    }
}

// ── unique edge cases ───────────────────────────────────────────

#[test]
fn test_unique_empty_list() {
    let result = call_stats_builtin("unique", vec![Value::List(vec![])]).unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_unique_preserves_order() {
    let result = call_stats_builtin("unique", vec![int_list(&[3, 1, 2, 1, 3])]).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(3), Value::Int(1), Value::Int(2)])
    );
}

#[test]
fn test_unique_all_same() {
    let result = call_stats_builtin("unique", vec![int_list(&[7, 7, 7])]).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(7)]));
}

// ── cumsum edge cases ───────────────────────────────────────────

#[test]
fn test_cumsum_empty_list() {
    let result = call_stats_builtin("cumsum", vec![Value::List(vec![])]).unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_cumsum_single_element() {
    let result = call_stats_builtin("cumsum", vec![int_list(&[42])]).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(42)]));
}

#[test]
fn test_cumsum_mixed_int_float() {
    let list = Value::List(vec![Value::Int(1), Value::Float(2.5), Value::Int(3)]);
    let result = call_stats_builtin("cumsum", vec![list]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
        // After encountering a float, all become floats
        for item in &items {
            assert!(matches!(item, Value::Float(_)), "expected all Float after mixed list");
        }
    } else {
        panic!("expected List");
    }
}

// ── sqrt edge cases ─────────────────────────────────────────────

#[test]
fn test_sqrt_negative_nan() {
    if let Value::Float(v) = call_stats_builtin("sqrt", vec![Value::Float(-1.0)]).unwrap() {
        assert!(v.is_nan(), "sqrt of negative should be NaN");
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_sqrt_zero() {
    assert_eq!(
        call_stats_builtin("sqrt", vec![Value::Float(0.0)]).unwrap(),
        Value::Float(0.0)
    );
}

// ── pow edge cases ──────────────────────────────────────────────

#[test]
fn test_pow_zero_exponent() {
    assert_eq!(
        call_stats_builtin("pow", vec![Value::Int(5), Value::Int(0)]).unwrap(),
        Value::Float(1.0)
    );
}

#[test]
fn test_pow_negative_exponent() {
    if let Value::Float(v) =
        call_stats_builtin("pow", vec![Value::Int(2), Value::Int(-1)]).unwrap()
    {
        assert!((v - 0.5).abs() < 1e-10, "2^-1 should be 0.5, got {v}");
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_pow_zero_base() {
    assert_eq!(
        call_stats_builtin("pow", vec![Value::Int(0), Value::Int(5)]).unwrap(),
        Value::Float(0.0)
    );
}

// ── log edge cases ──────────────────────────────────────────────

#[test]
fn test_log_of_zero_neg_inf() {
    if let Value::Float(v) = call_stats_builtin("log", vec![Value::Float(0.0)]).unwrap() {
        assert!(v.is_infinite() && v < 0.0, "log(0) should be -inf, got {v}");
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_log_of_negative_nan() {
    if let Value::Float(v) = call_stats_builtin("log", vec![Value::Float(-1.0)]).unwrap() {
        assert!(v.is_nan(), "log(negative) should be NaN, got {v}");
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_log_of_one_is_zero() {
    if let Value::Float(v) = call_stats_builtin("log", vec![Value::Float(1.0)]).unwrap() {
        assert!((v - 0.0).abs() < 1e-15, "log(1) should be 0, got {v}");
    }
}

// ── log2 / log10 ────────────────────────────────────────────────

#[test]
fn test_log2_basic() {
    if let Value::Float(v) = call_stats_builtin("log2", vec![Value::Int(16)]).unwrap() {
        assert!((v - 4.0).abs() < 1e-10);
    }
}

#[test]
fn test_log10_basic() {
    if let Value::Float(v) = call_stats_builtin("log10", vec![Value::Int(100)]).unwrap() {
        assert!((v - 2.0).abs() < 1e-10);
    }
}

// ── ceil / floor / round of exact integers ──────────────────────

#[test]
fn test_ceil_of_integer() {
    assert_eq!(
        call_stats_builtin("ceil", vec![Value::Int(5)]).unwrap(),
        Value::Int(5)
    );
}

#[test]
fn test_floor_of_integer() {
    assert_eq!(
        call_stats_builtin("floor", vec![Value::Int(5)]).unwrap(),
        Value::Int(5)
    );
}

#[test]
fn test_round_of_integer() {
    assert_eq!(
        call_stats_builtin("round", vec![Value::Int(5)]).unwrap(),
        Value::Int(5)
    );
}

// ── string edge cases ───────────────────────────────────────────

#[test]
fn test_upper_empty_string() {
    assert_eq!(
        call_stats_builtin("upper", vec![Value::Str("".into())]).unwrap(),
        Value::Str("".into())
    );
}

#[test]
fn test_lower_empty_string() {
    assert_eq!(
        call_stats_builtin("lower", vec![Value::Str("".into())]).unwrap(),
        Value::Str("".into())
    );
}

#[test]
fn test_trim_already_trimmed() {
    assert_eq!(
        call_stats_builtin("trim", vec![Value::Str("hello".into())]).unwrap(),
        Value::Str("hello".into())
    );
}

#[test]
fn test_starts_with_empty_prefix() {
    assert_eq!(
        call_stats_builtin(
            "starts_with",
            vec![Value::Str("hello".into()), Value::Str("".into())]
        )
        .unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn test_ends_with_empty_suffix() {
    assert_eq!(
        call_stats_builtin(
            "ends_with",
            vec![Value::Str("hello".into()), Value::Str("".into())]
        )
        .unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn test_starts_with_false() {
    assert_eq!(
        call_stats_builtin(
            "starts_with",
            vec![Value::Str("hello".into()), Value::Str("xyz".into())]
        )
        .unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn test_ends_with_false() {
    assert_eq!(
        call_stats_builtin(
            "ends_with",
            vec![Value::Str("hello".into()), Value::Str("xyz".into())]
        )
        .unwrap(),
        Value::Bool(false)
    );
}

// ── substr edge cases ───────────────────────────────────────────

#[test]
fn test_substr_out_of_bounds() {
    // start beyond string length should return empty
    assert_eq!(
        call_stats_builtin(
            "substr",
            vec![Value::Str("hi".into()), Value::Int(100), Value::Int(5)]
        )
        .unwrap(),
        Value::Str("".into())
    );
}

#[test]
fn test_substr_length_beyond_end() {
    // length extends past end, should clamp
    assert_eq!(
        call_stats_builtin(
            "substr",
            vec![Value::Str("hello".into()), Value::Int(3), Value::Int(100)]
        )
        .unwrap(),
        Value::Str("lo".into())
    );
}

#[test]
fn test_substr_zero_length() {
    assert_eq!(
        call_stats_builtin(
            "substr",
            vec![Value::Str("hello".into()), Value::Int(0), Value::Int(0)]
        )
        .unwrap(),
        Value::Str("".into())
    );
}

// ── str_replace edge cases ──────────────────────────────────────

#[test]
fn test_str_replace_no_match() {
    assert_eq!(
        call_stats_builtin(
            "str_replace",
            vec![
                Value::Str("hello".into()),
                Value::Str("xyz".into()),
                Value::Str("abc".into()),
            ]
        )
        .unwrap(),
        Value::Str("hello".into())
    );
}

#[test]
fn test_str_replace_multiple_occurrences() {
    assert_eq!(
        call_stats_builtin(
            "str_replace",
            vec![
                Value::Str("aabaa".into()),
                Value::Str("a".into()),
                Value::Str("x".into()),
            ]
        )
        .unwrap(),
        Value::Str("xxbxx".into())
    );
}

// ── ttest edge cases ────────────────────────────────────────────

#[test]
fn test_ttest_equal_groups_high_p() {
    let a = float_list(&[5.0, 5.1, 4.9, 5.0, 5.05]);
    let b = float_list(&[5.0, 5.1, 4.9, 5.0, 4.95]);
    let result = call_stats_builtin("ttest", vec![a, b]).unwrap();
    let p = get_record_float(&result, "p_value");
    assert!(p > 0.3, "p={p} should be high for nearly equal groups");
}

#[test]
fn test_ttest_paired_basic() {
    let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = float_list(&[1.1, 2.1, 3.1, 4.1, 5.1]);
    let result = call_stats_builtin("ttest_paired", vec![a, b]).unwrap();
    let _p = get_record_float(&result, "p_value");
    let _t = get_record_float(&result, "t_statistic");
    // Just ensure it returns a valid result
}

// ── anova edge cases ────────────────────────────────────────────

#[test]
fn test_anova_different_groups() {
    let groups = Value::List(vec![
        float_list(&[1.0, 2.0, 3.0]),
        float_list(&[10.0, 11.0, 12.0]),
        float_list(&[20.0, 21.0, 22.0]),
    ]);
    let result = call_stats_builtin("anova", vec![groups]).unwrap();
    let p = get_record_float(&result, "p_value");
    assert!(p < 0.01, "p={p} should be very small for distinct groups");
}

// ── chi_square edge cases ───────────────────────────────────────

#[test]
fn test_chi_square_different_lengths_error() {
    let obs = float_list(&[10.0, 20.0]);
    let exp = float_list(&[15.0]);
    let result = call_stats_builtin("chi_square", vec![obs, exp]);
    assert!(result.is_err(), "chi_square with different lengths should error");
}

// ── fisher_exact edge cases ─────────────────────────────────────

#[test]
fn test_fisher_exact_non_significant() {
    // Balanced table
    let result = call_stats_builtin(
        "fisher_exact",
        vec![Value::Int(10), Value::Int(10), Value::Int(10), Value::Int(10)],
    )
    .unwrap();
    let p = get_record_float(&result, "p_value");
    assert!(p > 0.5, "p={p} should be high for balanced 2x2");
}

// ── p_adjust edge cases ─────────────────────────────────────────

#[test]
fn test_p_adjust_single_pvalue() {
    let pvals = float_list(&[0.05]);
    let result =
        call_stats_builtin("p_adjust", vec![pvals, Value::Str("bonferroni".into())]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 1);
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_p_adjust_holm() {
    let pvals = float_list(&[0.01, 0.04, 0.03]);
    let result =
        call_stats_builtin("p_adjust", vec![pvals, Value::Str("holm".into())]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_p_adjust_unknown_method_error() {
    let pvals = float_list(&[0.05]);
    let result = call_stats_builtin("p_adjust", vec![pvals, Value::Str("unknown".into())]);
    assert!(result.is_err(), "unknown p_adjust method should error");
}

// ── normalize edge cases ────────────────────────────────────────

#[test]
fn test_normalize_constant_values() {
    let data = float_list(&[5.0, 5.0, 5.0, 5.0]);
    let result =
        call_stats_builtin("normalize", vec![data, Value::Str("zscore".into())]).unwrap();
    if let Value::List(items) = result {
        for item in &items {
            if let Value::Float(v) = item {
                assert_eq!(*v, 0.0, "zscore of constant should be 0");
            }
        }
    }
}

#[test]
fn test_normalize_minmax() {
    let data = float_list(&[10.0, 20.0, 30.0]);
    let result =
        call_stats_builtin("normalize", vec![data, Value::Str("minmax".into())]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
        // min should map to 0, max to 1
        if let (Value::Float(first), Value::Float(last)) = (&items[0], &items[2]) {
            assert!((first - 0.0).abs() < 1e-10);
            assert!((last - 1.0).abs() < 1e-10);
        }
    }
}

#[test]
fn test_normalize_minmax_constant() {
    let data = float_list(&[5.0, 5.0, 5.0]);
    let result =
        call_stats_builtin("normalize", vec![data, Value::Str("minmax".into())]).unwrap();
    if let Value::List(items) = result {
        for item in &items {
            if let Value::Float(v) = item {
                assert_eq!(*v, 0.0, "minmax of constant should be 0");
            }
        }
    }
}

#[test]
fn test_normalize_quantile() {
    let data = float_list(&[10.0, 30.0, 20.0]);
    let result =
        call_stats_builtin("normalize", vec![data, Value::Str("quantile".into())]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
    }
}

#[test]
fn test_normalize_unknown_method_error() {
    let data = float_list(&[1.0, 2.0]);
    let result = call_stats_builtin("normalize", vec![data, Value::Str("magic".into())]);
    assert!(result.is_err(), "unknown normalize method should error");
}

// ── lm edge cases ───────────────────────────────────────────────

#[test]
fn test_lm_no_correlation() {
    // Random-ish data with essentially zero R^2
    let x = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let y = float_list(&[5.0, 1.0, 4.0, 2.0, 3.0]);
    let result = call_stats_builtin("lm", vec![x, y]).unwrap();
    let r2 = get_record_float(&result, "r_squared");
    assert!(r2 < 0.2, "r2={r2} should be low for uncorrelated data");
}

// ── sign edge cases ─────────────────────────────────────────────

#[test]
fn test_sign_negative_float() {
    assert_eq!(
        call_stats_builtin("sign", vec![Value::Float(-3.14)]).unwrap(),
        Value::Float(-1.0)
    );
}

#[test]
fn test_sign_zero_float() {
    // Note: f64 0.0.signum() = 1.0 in Rust (IEEE)
    if let Value::Float(v) = call_stats_builtin("sign", vec![Value::Float(0.0)]).unwrap() {
        // IEEE says signum(0.0) = 1.0 in Rust
        assert!((v - 1.0).abs() < 1e-10 || v == 0.0, "sign(0.0) got {v}");
    }
}

#[test]
fn test_sign_nan() {
    if let Value::Float(v) = call_stats_builtin("sign", vec![Value::Float(f64::NAN)]).unwrap() {
        assert!(v.is_nan(), "sign(NaN) should be NaN");
    }
}

// ── clamp edge cases ────────────────────────────────────────────

#[test]
fn test_clamp_value_in_range() {
    let result = call_stats_builtin(
        "clamp",
        vec![Value::Float(5.0), Value::Float(0.0), Value::Float(10.0)],
    )
    .unwrap();
    assert_eq!(result, Value::Float(5.0));
}

#[test]
fn test_clamp_value_below_min() {
    let result = call_stats_builtin(
        "clamp",
        vec![Value::Float(-5.0), Value::Float(0.0), Value::Float(10.0)],
    )
    .unwrap();
    assert_eq!(result, Value::Float(0.0));
}

#[test]
fn test_clamp_int_args() {
    let result = call_stats_builtin(
        "clamp",
        vec![Value::Int(15), Value::Int(0), Value::Int(10)],
    )
    .unwrap();
    assert_eq!(result, Value::Float(10.0));
}

// ── is_nan / is_finite edge cases ───────────────────────────────

#[test]
fn test_is_nan_regular_number_false() {
    assert_eq!(
        call_stats_builtin("is_nan", vec![Value::Float(3.14)]).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn test_is_nan_int_false() {
    assert_eq!(
        call_stats_builtin("is_nan", vec![Value::Int(0)]).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn test_is_finite_neg_inf_false() {
    assert_eq!(
        call_stats_builtin("is_finite", vec![Value::Float(f64::NEG_INFINITY)]).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn test_is_finite_nan_false() {
    assert_eq!(
        call_stats_builtin("is_finite", vec![Value::Float(f64::NAN)]).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn test_is_finite_regular_float_true() {
    assert_eq!(
        call_stats_builtin("is_finite", vec![Value::Float(3.14)]).unwrap(),
        Value::Bool(true)
    );
}

// ── trig edge cases ─────────────────────────────────────────────

#[test]
fn test_tan_zero() {
    assert_eq!(
        call_stats_builtin("tan", vec![Value::Float(0.0)]).unwrap(),
        Value::Float(0.0)
    );
}

#[test]
fn test_asin_acos_atan() {
    if let Value::Float(v) = call_stats_builtin("asin", vec![Value::Float(0.0)]).unwrap() {
        assert!((v - 0.0).abs() < 1e-10);
    }
    if let Value::Float(v) = call_stats_builtin("acos", vec![Value::Float(1.0)]).unwrap() {
        assert!((v - 0.0).abs() < 1e-10);
    }
    if let Value::Float(v) = call_stats_builtin("atan", vec![Value::Float(0.0)]).unwrap() {
        assert!((v - 0.0).abs() < 1e-10);
    }
}

#[test]
fn test_atan2() {
    if let Value::Float(v) =
        call_stats_builtin("atan2", vec![Value::Float(1.0), Value::Float(1.0)]).unwrap()
    {
        assert!(
            (v - std::f64::consts::FRAC_PI_4).abs() < 1e-10,
            "atan2(1,1) should be pi/4, got {v}"
        );
    }
}

// ── exp edge cases ──────────────────────────────────────────────

#[test]
fn test_exp_zero() {
    assert_eq!(
        call_stats_builtin("exp", vec![Value::Float(0.0)]).unwrap(),
        Value::Float(1.0)
    );
}

// ── random_int edge cases ───────────────────────────────────────

#[test]
fn test_random_int_lo_ge_hi_error() {
    let result = call_stats_builtin("random_int", vec![Value::Int(10), Value::Int(10)]);
    assert!(result.is_err(), "random_int with lo >= hi should error");
}

#[test]
fn test_random_int_lo_gt_hi_error() {
    let result = call_stats_builtin("random_int", vec![Value::Int(10), Value::Int(5)]);
    assert!(result.is_err(), "random_int with lo > hi should error");
}

// ── extended string builtins ────────────────────────────────────

#[test]
fn test_char_at_out_of_bounds() {
    let result =
        call_stats_builtin("char_at", vec![Value::Str("hi".into()), Value::Int(100)]).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_index_of_not_found() {
    let result = call_stats_builtin(
        "index_of",
        vec![Value::Str("hello".into()), Value::Str("xyz".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Int(-1));
}

#[test]
fn test_str_repeat_zero() {
    let result =
        call_stats_builtin("str_repeat", vec![Value::Str("abc".into()), Value::Int(0)]).unwrap();
    assert_eq!(result, Value::Str("".into()));
}

#[test]
fn test_str_len() {
    assert_eq!(
        call_stats_builtin("str_len", vec![Value::Str("hello".into())]).unwrap(),
        Value::Int(5)
    );
}

#[test]
fn test_str_len_empty() {
    assert_eq!(
        call_stats_builtin("str_len", vec![Value::Str("".into())]).unwrap(),
        Value::Int(0)
    );
}

#[test]
fn test_pad_left_already_wide_enough() {
    let result = call_stats_builtin(
        "pad_left",
        vec![Value::Str("hello".into()), Value::Int(3), Value::Str("0".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Str("hello".into()));
}

#[test]
fn test_pad_right_already_wide_enough() {
    let result = call_stats_builtin(
        "pad_right",
        vec![Value::Str("hello".into()), Value::Int(3), Value::Str("0".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Str("hello".into()));
}

#[test]
fn test_trim_left_nothing_to_trim() {
    assert_eq!(
        call_stats_builtin("trim_left", vec![Value::Str("hello".into())]).unwrap(),
        Value::Str("hello".into())
    );
}

#[test]
fn test_trim_right_nothing_to_trim() {
    assert_eq!(
        call_stats_builtin("trim_right", vec![Value::Str("hello".into())]).unwrap(),
        Value::Str("hello".into())
    );
}

// ── sample edge cases ───────────────────────────────────────────

#[test]
fn test_sample_n_exceeds_length_error() {
    let result = call_stats_builtin("sample", vec![int_list(&[1, 2, 3]), Value::Int(5)]);
    assert!(result.is_err(), "sample n > list length should error");
}

#[test]
fn test_sample_zero() {
    let result =
        call_stats_builtin("sample", vec![int_list(&[1, 2, 3]), Value::Int(0)]).unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_sample_all() {
    let result =
        call_stats_builtin("sample", vec![int_list(&[1, 2, 3]), Value::Int(3)]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
    }
}

// ── wilcoxon ────────────────────────────────────────────────────

#[test]
fn test_wilcoxon_basic() {
    let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = float_list(&[6.0, 7.0, 8.0, 9.0, 10.0]);
    let result = call_stats_builtin("wilcoxon", vec![a, b]).unwrap();
    let p = get_record_float(&result, "p_value");
    assert!(p < 0.05, "p={p} should be significant for separated groups");
}

// ── ks_test ─────────────────────────────────────────────────────

#[test]
fn test_ks_test_same_distribution() {
    let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let result = call_stats_builtin("ks_test", vec![a, b]).unwrap();
    let stat = get_record_float(&result, "statistic");
    assert!(stat < 0.01, "KS stat={stat} should be ~0 for same data");
}

#[test]
fn test_ks_test_different_distributions() {
    let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = float_list(&[10.0, 20.0, 30.0, 40.0, 50.0]);
    let result = call_stats_builtin("ks_test", vec![a, b]).unwrap();
    let stat = get_record_float(&result, "statistic");
    assert!(stat > 0.5, "KS stat={stat} should be high for very different distributions");
}

// ── spearman ────────────────────────────────────────────────────

#[test]
fn test_spearman_perfect_monotone() {
    let x = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let y = float_list(&[2.0, 4.0, 6.0, 8.0, 10.0]);
    let result = call_stats_builtin("spearman", vec![x, y]).unwrap();
    let rho = get_record_float(&result, "coefficient");
    assert!((rho - 1.0).abs() < 1e-10, "spearman rho={rho} should be 1.0");
}

// ── kendall ─────────────────────────────────────────────────────

#[test]
fn test_kendall_perfect_concordance() {
    let x = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let y = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let result = call_stats_builtin("kendall", vec![x, y]).unwrap();
    let tau = get_record_float(&result, "coefficient");
    assert!((tau - 1.0).abs() < 1e-10, "kendall tau={tau} should be 1.0");
}

// ── kaplan_meier ────────────────────────────────────────────────

#[test]
fn test_kaplan_meier_basic() {
    let times = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let events = Value::List(vec![
        Value::Bool(true),
        Value::Bool(true),
        Value::Bool(false),
        Value::Bool(true),
        Value::Bool(false),
    ]);
    let result = call_stats_builtin("kaplan_meier", vec![times, events]).unwrap();
    if let Value::Record(map) = &result {
        assert!(map.contains_key("times"));
        assert!(map.contains_key("survival"));
        assert!(map.contains_key("at_risk"));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_kaplan_meier_int_events() {
    let times = float_list(&[1.0, 2.0, 3.0]);
    let events = Value::List(vec![Value::Int(1), Value::Int(0), Value::Int(1)]);
    let result = call_stats_builtin("kaplan_meier", vec![times, events]).unwrap();
    assert!(matches!(result, Value::Record(_)));
}

// ── cox_ph ──────────────────────────────────────────────────────

#[test]
fn test_cox_ph_basic() {
    let times = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let events = Value::List(vec![
        Value::Bool(true),
        Value::Bool(true),
        Value::Bool(false),
        Value::Bool(true),
        Value::Bool(true),
        Value::Bool(false),
        Value::Bool(true),
        Value::Bool(true),
    ]);
    let covariates = Value::List(vec![
        float_list(&[1.0]),
        float_list(&[2.0]),
        float_list(&[1.5]),
        float_list(&[3.0]),
        float_list(&[2.5]),
        float_list(&[1.0]),
        float_list(&[3.5]),
        float_list(&[2.0]),
    ]);
    let result = call_stats_builtin("cox_ph", vec![times, events, covariates]).unwrap();
    if let Value::Record(map) = &result {
        assert!(map.contains_key("coefficients"));
        assert!(map.contains_key("hazard_ratios"));
        assert!(map.contains_key("concordance"));
    } else {
        panic!("expected Record");
    }
}

// ── hist / scatter (just ensure they don't panic) ───────────────

#[test]
fn test_hist_basic() {
    let result = call_stats_builtin("hist", vec![float_list(&[1.0, 2.0, 3.0, 4.0, 5.0])]);
    assert!(result.is_ok());
}

#[test]
fn test_hist_custom_bins() {
    let result = call_stats_builtin(
        "hist",
        vec![float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]), Value::Int(3)],
    );
    assert!(result.is_ok());
}

#[test]
fn test_hist_zero_bins_error() {
    let result = call_stats_builtin(
        "hist",
        vec![float_list(&[1.0, 2.0, 3.0]), Value::Int(0)],
    );
    assert!(result.is_err());
}

#[test]
fn test_scatter_basic() {
    let x = float_list(&[1.0, 2.0, 3.0]);
    let y = float_list(&[4.0, 5.0, 6.0]);
    let result = call_stats_builtin("scatter", vec![x, y]);
    assert!(result.is_ok());
}

#[test]
fn test_scatter_unequal_lengths_error() {
    let x = float_list(&[1.0, 2.0]);
    let y = float_list(&[1.0]);
    let result = call_stats_builtin("scatter", vec![x, y]);
    assert!(result.is_err());
}

// ── format edge cases ───────────────────────────────────────────

#[test]
fn test_format_no_placeholders() {
    let result = call_stats_builtin("format", vec![Value::Str("plain text".into())]).unwrap();
    assert_eq!(result, Value::Str("plain text".into()));
}

#[test]
fn test_format_sequential_placeholders() {
    let result = call_stats_builtin(
        "format",
        vec![
            Value::Str("{} and {}".into()),
            Value::Int(1),
            Value::Int(2),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("1 and 2".into()));
}

// ── mixed int/float lists for stats ─────────────────────────────

#[test]
fn test_variance_mixed_types() {
    let list = Value::List(vec![
        Value::Int(2),
        Value::Float(4.0),
        Value::Int(4),
        Value::Float(4.0),
        Value::Int(5),
        Value::Int(5),
        Value::Int(7),
        Value::Int(9),
    ]);
    let result = call_stats_builtin("variance", vec![list]).unwrap();
    if let Value::Float(v) = result {
        assert!((v - 4.571).abs() < 0.01, "variance={v}");
    }
}

#[test]
fn test_cor_mixed_int_float() {
    let x = Value::List(vec![Value::Int(1), Value::Float(2.0), Value::Int(3)]);
    let y = Value::List(vec![Value::Float(2.0), Value::Int(4), Value::Float(6.0)]);
    let result = call_stats_builtin("cor", vec![x, y]).unwrap();
    if let Value::Float(r) = result {
        assert!((r - 1.0).abs() < 1e-10, "perfect correlation expected, got {r}");
    }
}

// ── large list for performance ──────────────────────────────────

#[test]
fn test_mean_large_list() {
    let data: Vec<i64> = (1..=1000).collect();
    let result = call_stats_builtin("mean", vec![int_list(&data)]).unwrap();
    if let Value::Float(v) = result {
        assert!((v - 500.5).abs() < 1e-10, "mean of 1..1000 should be 500.5, got {v}");
    }
}

#[test]
fn test_sum_large_list() {
    let data: Vec<i64> = (1..=1000).collect();
    let result = call_stats_builtin("sum", vec![int_list(&data)]).unwrap();
    assert_eq!(result, Value::Int(500500));
}

#[test]
fn test_median_large_list() {
    let data: Vec<i64> = (1..=1000).collect();
    let result = call_stats_builtin("median", vec![int_list(&data)]).unwrap();
    if let Value::Float(v) = result {
        assert!((v - 500.5).abs() < 1e-10, "median of 1..1000 should be 500.5, got {v}");
    }
}

#[test]
fn test_unique_large_list() {
    let data: Vec<i64> = (1..=500).chain(1..=500).collect();
    let result = call_stats_builtin("unique", vec![int_list(&data)]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 500);
    }
}

// ── random returns value in expected range ──────────────────────

#[test]
fn test_random_range_multiple_calls() {
    for _ in 0..10 {
        if let Value::Float(r) = call_stats_builtin("random", vec![]).unwrap() {
            assert!(r >= 0.0 && r <= 1.0, "random() = {r} out of [0,1]");
        }
    }
}

// ── unknown builtin ─────────────────────────────────────────────

#[test]
fn test_unknown_builtin_error() {
    let result = call_stats_builtin("nonexistent_func", vec![]);
    assert!(result.is_err());
}

// ── type error cases ────────────────────────────────────────────

#[test]
fn test_mean_not_list_error() {
    let result = call_stats_builtin("mean", vec![Value::Int(5)]);
    assert!(result.is_err(), "mean of non-list should error");
}

#[test]
fn test_mean_non_numeric_list_error() {
    let list = Value::List(vec![Value::Str("hello".into())]);
    let result = call_stats_builtin("mean", vec![list]);
    assert!(result.is_err(), "mean of non-numeric list should error");
}

#[test]
fn test_upper_non_string_error() {
    let result = call_stats_builtin("upper", vec![Value::Int(5)]);
    assert!(result.is_err(), "upper of non-string should error");
}

#[test]
fn test_sqrt_non_number_error() {
    let result = call_stats_builtin("sqrt", vec![Value::Str("hello".into())]);
    assert!(result.is_err(), "sqrt of non-number should error");
}

// ── is_stats_builtin ────────────────────────────────────────────

#[test]
fn test_is_stats_builtin_known() {
    use bl_runtime::stats::is_stats_builtin;
    assert!(is_stats_builtin("mean"));
    assert!(is_stats_builtin("median"));
    assert!(is_stats_builtin("ttest"));
    assert!(is_stats_builtin("fisher_exact"));
    assert!(is_stats_builtin("ks_test"));
    assert!(is_stats_builtin("spearman"));
    assert!(is_stats_builtin("kendall"));
    assert!(is_stats_builtin("kaplan_meier"));
    assert!(is_stats_builtin("cox_ph"));
    assert!(is_stats_builtin("mean_phred"));
    assert!(is_stats_builtin("trim_quality"));
}

#[test]
fn test_is_stats_builtin_unknown() {
    use bl_runtime::stats::is_stats_builtin;
    assert!(!is_stats_builtin("nonexistent"));
    assert!(!is_stats_builtin(""));
}

// ── stats_builtin_list ──────────────────────────────────────────

#[test]
fn test_stats_builtin_list_not_empty() {
    use bl_runtime::stats::stats_builtin_list;
    let list = stats_builtin_list();
    assert!(list.len() >= 70, "should have 70+ builtins, got {}", list.len());
}

// ── round with decimal places ───────────────────────────────────

#[test]
fn test_round_negative() {
    // round(-2.5) -- Rust rounds half-to-even, so this should be -2 or -3
    if let Value::Int(v) = call_stats_builtin("round", vec![Value::Float(-2.5)]).unwrap() {
        // Rust's f64::round rounds away from zero: -2.5 -> -3
        assert_eq!(v, -3);
    }
}

#[test]
fn test_round_with_zero_places() {
    let result =
        call_stats_builtin("round", vec![Value::Float(3.7), Value::Int(0)]).unwrap();
    assert_eq!(result, Value::Float(4.0));
}
