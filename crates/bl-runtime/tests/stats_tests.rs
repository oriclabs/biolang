use bl_core::value::{Table, Value};
use bl_runtime::stats::call_stats_builtin;
use std::collections::HashMap;

fn int_list(vals: &[i64]) -> Value {
    Value::List(
        vals.iter()
            .map(|v| Value::Int(*v))
            .collect::<Vec<_>>()
            .into(),
    )
}
fn float_list(vals: &[f64]) -> Value {
    Value::List(
        vals.iter()
            .map(|v| Value::Float(*v))
            .collect::<Vec<_>>()
            .into(),
    )
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
    let list = Value::List((vec![Value::Int(1), Value::Float(2.5), Value::Int(3)]).into());
    let result = call_stats_builtin("sum", vec![list]).unwrap();
    assert_eq!(result, Value::Float(6.5));
}

#[test]
fn test_variance() {
    let result = call_stats_builtin("variance", vec![int_list(&[2, 4, 4, 4, 5, 5, 7, 9])]).unwrap();
    if let Value::Float(v) = result {
        assert!((v - 4.571).abs() < 0.01);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_stdev() {
    let result = call_stats_builtin("stdev", vec![int_list(&[2, 4, 4, 4, 5, 5, 7, 9])]).unwrap();
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
    let result = call_stats_builtin("unique", vec![int_list(&[1, 2, 2, 3, 1, 3])]).unwrap();
    assert_eq!(
        result,
        Value::List((vec![Value::Int(1), Value::Int(2), Value::Int(3)]).into())
    );
}

#[test]
fn test_sample_count() {
    let result =
        call_stats_builtin("sample", vec![int_list(&[1, 2, 3, 4, 5]), Value::Int(3)]).unwrap();
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
        Value::List((vec![Value::Int(1), Value::Int(3), Value::Int(6), Value::Int(10)]).into())
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
        call_stats_builtin("round", vec![Value::Float(2.75159), Value::Int(2)]).unwrap(),
        Value::Float(2.75)
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
            vec![
                Value::Str("hello world".into()),
                Value::Int(6),
                Value::Int(5),
            ]
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
    let groups = Value::List(
        (vec![
            float_list(&[5.0, 5.1, 4.9]),
            float_list(&[5.0, 5.1, 4.9]),
            float_list(&[5.0, 5.1, 4.9]),
        ])
        .into(),
    );
    let result = call_stats_builtin("anova", vec![groups]).unwrap();
    let p = get_record_float(&result, "p_value");
    assert!(p > 0.5, "p={p} should be high for identical groups");
}

fn unequal_anova_groups() -> Value {
    Value::List(
        vec![
            float_list(&[1.2, 2.4, 3.1, 4.8, 5.5]),
            float_list(&[2.0, 3.3, 4.1, 6.2, 7.9, 9.1, 10.3]),
            float_list(&[0.5, 1.1, 1.8, 2.2]),
        ]
        .into(),
    )
}

#[test]
fn test_anova_explicit_welch_matches_r_and_discloses_effect_sizes() {
    let options = option_record(&[("variance", Value::Str("welch".into()))]);
    let result = call_stats_builtin("anova", vec![unequal_anova_groups(), options]).unwrap();
    assert_eq!(get_record_str(&result, "method"), "welch_anova");
    assert!((get_record_float(&result, "f_statistic") - 8.238_003_24).abs() < 1e-8);
    assert!((get_record_float(&result, "df_within") - 7.986_031_25).abs() < 1e-8);
    assert!((get_record_float(&result, "p_value") - 0.011_448_51).abs() < 1e-8);
    assert!((get_record_float(&result, "eta_squared") - 0.454_421_76).abs() < 1e-8);
    assert!((get_record_float(&result, "omega_squared") - 0.355_564_48).abs() < 1e-8);
}

#[test]
fn test_kruskal_wallis_matches_r_and_returns_epsilon_squared() {
    let result = call_stats_builtin("kruskal_wallis", vec![unequal_anova_groups()]).unwrap();
    assert_eq!(get_record_str(&result, "method"), "kruskal_wallis_rank_sum");
    let h = get_record_float(&result, "h_statistic");
    let p = get_record_float(&result, "p_value");
    let epsilon = get_record_float(&result, "epsilon_squared");
    assert!((h - 8.074_474_79).abs() < 1e-8, "H={h}");
    assert!((p - 0.017_646_15).abs() < 1e-8, "p={p}");
    assert!((epsilon - 0.467_267_29).abs() < 1e-8, "epsilon={epsilon}");
}

#[test]
fn test_tukey_hsd_is_a_studentized_range_procedure_not_pairwise_ttests() {
    let result = call_stats_builtin("tukey_hsd", vec![unequal_anova_groups()]).unwrap();
    assert_eq!(get_record_str(&result, "method"), "tukey_kramer_hsd");
    let comparisons = get_record_list(&result, "comparisons");
    assert_eq!(comparisons.len(), 3);
    let third = &comparisons[2];
    assert!((get_record_float(third, "p_adjusted") - 0.018_042_4).abs() < 2e-5);
    assert!((get_record_float(third, "confidence_lower") - 0.819_324_1).abs() < 2e-5);
    assert!((get_record_float(third, "confidence_upper") - 8.637_818_8).abs() < 2e-5);
}

#[test]
fn test_pairwise_ttest_defaults_to_welch_with_holm_adjustment() {
    let result = call_stats_builtin("pairwise_ttest", vec![unequal_anova_groups()]).unwrap();
    assert_eq!(get_record_str(&result, "method"), "pairwise_welch_t");
    assert_eq!(get_record_str(&result, "adjustment"), "holm");
    let comparisons = get_record_list(&result, "comparisons");
    assert_eq!(comparisons.len(), 3);
    for comparison in comparisons {
        assert!(
            get_record_float(comparison, "p_adjusted") >= get_record_float(comparison, "p_value")
        );
    }
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
        for item in items.iter() {
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
    let result = call_stats_builtin("normalize", vec![data, Value::Str("zscore".into())]).unwrap();
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
        vec![
            Value::Str("42".into()),
            Value::Int(5),
            Value::Str("0".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("00042".into()));
}

#[test]
fn test_pad_right() {
    let result = call_stats_builtin(
        "pad_right",
        vec![
            Value::Str("hi".into()),
            Value::Int(5),
            Value::Str(".".into()),
        ],
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
        call_stats_builtin("sign", vec![Value::Float(2.75)]).unwrap(),
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
        assert!((0.0..=1.0).contains(&r), "random() out of range: {r}");
    } else {
        panic!("expected Float");
    }
}

#[test]
fn test_random_int() {
    if let Value::Int(r) =
        call_stats_builtin("random_int", vec![Value::Int(1), Value::Int(10)]).unwrap()
    {
        assert!((1..10).contains(&r), "random_int() out of range: {r}");
    } else {
        panic!("expected Int");
    }
}

#[test]
fn test_set_seed_reproduces_random_sequence() {
    call_stats_builtin("set_seed", vec![Value::Int(42)]).unwrap();
    let first = call_stats_builtin("random", vec![]).unwrap();
    let first_int = call_stats_builtin("random_int", vec![Value::Int(10), Value::Int(20)]).unwrap();
    let population =
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)].into());
    let first_sample =
        call_stats_builtin("sample", vec![population.clone(), Value::Int(2)]).unwrap();

    call_stats_builtin("set_seed", vec![Value::Int(42)]).unwrap();
    assert_eq!(call_stats_builtin("random", vec![]).unwrap(), first);
    assert_eq!(
        call_stats_builtin("random_int", vec![Value::Int(10), Value::Int(20)]).unwrap(),
        first_int
    );
    assert_eq!(
        call_stats_builtin("sample", vec![population, Value::Int(2)]).unwrap(),
        first_sample
    );
}

#[test]
fn test_hist_returns_rendered_text() {
    let values =
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)].into());
    let result = call_stats_builtin("hist", vec![values, Value::Int(2)]).unwrap();

    if let Value::Str(output) = result {
        assert!(output.contains("Histogram (n=4, bins=2):"));
        assert!(output.contains("[     1.0,      2.5)"));
        assert!(output.contains("[     2.5,      4.0]"));
    } else {
        panic!("expected histogram text");
    }
}

// ════════════════════════════════════════════════════════════════
// New edge-case and coverage tests
// ════════════════════════════════════════════════════════════════

// ── mean edge cases ─────────────────────────────────────────────

#[test]
fn test_mean_empty_list_error() {
    let result = call_stats_builtin("mean", vec![Value::List((vec![]).into())]);
    assert!(result.is_err(), "mean of empty list should error");
}

#[test]
fn test_mean_single_element() {
    let result = call_stats_builtin("mean", vec![int_list(&[42])]).unwrap();
    assert_eq!(result, Value::Float(42.0));
}

#[test]
fn test_mean_mixed_int_float() {
    let list = Value::List((vec![Value::Int(1), Value::Float(2.0), Value::Int(3)]).into());
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
    let result = call_stats_builtin("sum", vec![Value::List((vec![]).into())]).unwrap();
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
    assert!(
        result.is_err(),
        "variance of single element should error (need >= 2)"
    );
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
    let result = call_stats_builtin("quantile", vec![int_list(&[1, 2, 3]), Value::Float(1.5)]);
    assert!(result.is_err(), "quantile > 1 should error");
}

#[test]
fn test_quantile_invalid_negative() {
    let result = call_stats_builtin("quantile", vec![int_list(&[1, 2, 3]), Value::Float(-0.1)]);
    assert!(result.is_err(), "quantile < 0 should error");
}

// ── cor edge cases ──────────────────────────────────────────────

#[test]
fn test_cor_different_lengths_error() {
    let result = call_stats_builtin("cor", vec![int_list(&[1, 2, 3]), int_list(&[1, 2])]);
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
        assert!(
            (r - (-1.0)).abs() < 1e-10,
            "perfect negative correlation expected, got {r}"
        );
    } else {
        panic!("expected Float");
    }
}

// ── unique edge cases ───────────────────────────────────────────

#[test]
fn test_unique_empty_list() {
    let result = call_stats_builtin("unique", vec![Value::List((vec![]).into())]).unwrap();
    assert_eq!(result, Value::List((vec![]).into()));
}

#[test]
fn test_unique_preserves_order() {
    let result = call_stats_builtin("unique", vec![int_list(&[3, 1, 2, 1, 3])]).unwrap();
    assert_eq!(
        result,
        Value::List((vec![Value::Int(3), Value::Int(1), Value::Int(2)]).into())
    );
}

#[test]
fn test_unique_all_same() {
    let result = call_stats_builtin("unique", vec![int_list(&[7, 7, 7])]).unwrap();
    assert_eq!(result, Value::List((vec![Value::Int(7)]).into()));
}

// ── cumsum edge cases ───────────────────────────────────────────

#[test]
fn test_cumsum_empty_list() {
    let result = call_stats_builtin("cumsum", vec![Value::List((vec![]).into())]).unwrap();
    assert_eq!(result, Value::List((vec![]).into()));
}

#[test]
fn test_cumsum_single_element() {
    let result = call_stats_builtin("cumsum", vec![int_list(&[42])]).unwrap();
    assert_eq!(result, Value::List((vec![Value::Int(42)]).into()));
}

#[test]
fn test_cumsum_mixed_int_float() {
    let list = Value::List((vec![Value::Int(1), Value::Float(2.5), Value::Int(3)]).into());
    let result = call_stats_builtin("cumsum", vec![list]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
        // After encountering a float, all become floats
        for item in items.iter() {
            assert!(
                matches!(item, Value::Float(_)),
                "expected all Float after mixed list"
            );
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
    if let Value::Float(v) = call_stats_builtin("pow", vec![Value::Int(2), Value::Int(-1)]).unwrap()
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
    let groups = Value::List(
        (vec![
            float_list(&[1.0, 2.0, 3.0]),
            float_list(&[10.0, 11.0, 12.0]),
            float_list(&[20.0, 21.0, 22.0]),
        ])
        .into(),
    );
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
    assert!(
        result.is_err(),
        "chi_square with different lengths should error"
    );
}

// ── fisher_exact edge cases ─────────────────────────────────────

#[test]
fn test_fisher_exact_non_significant() {
    // Balanced table
    let result = call_stats_builtin(
        "fisher_exact",
        vec![
            Value::Int(10),
            Value::Int(10),
            Value::Int(10),
            Value::Int(10),
        ],
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
    let result = call_stats_builtin("p_adjust", vec![pvals, Value::Str("holm".into())]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
    } else {
        panic!("expected List");
    }
}

fn get_record_list<'a>(val: &'a Value, key: &str) -> &'a [Value] {
    match val {
        Value::Record(map) => match map.get(key).unwrap() {
            Value::List(values) => values,
            _ => panic!("expected list for key {key}"),
        },
        _ => panic!("expected Record"),
    }
}

#[test]
fn test_fisher_exact_labels_sample_odds_and_returns_wald_interval() {
    let result = call_stats_builtin(
        "fisher_exact",
        vec![Value::Int(8), Value::Int(2), Value::Int(1), Value::Int(5)],
    )
    .unwrap();
    assert_eq!(
        get_record_str(&result, "odds_ratio_estimator"),
        "sample_cross_product"
    );
    assert_eq!(
        get_record_str(&result, "confidence_interval_method"),
        "wald_log_odds"
    );
    assert!((get_record_float(&result, "confidence_lower") - 1.416185).abs() < 1e-6);
    assert!((get_record_float(&result, "confidence_upper") - 282.448946).abs() < 1e-5);
}

#[test]
fn test_fisher_exact_accepts_an_explicit_confidence_level() {
    let options = option_record(&[("confidence", Value::Float(0.90))]);
    let result = call_stats_builtin(
        "fisher_exact",
        vec![
            Value::Int(8),
            Value::Int(2),
            Value::Int(1),
            Value::Int(5),
            options,
        ],
    )
    .unwrap();
    assert!((get_record_float(&result, "confidence_level") - 0.90).abs() < 1e-12);
    assert!((get_record_float(&result, "confidence_lower") - 2.167680).abs() < 1e-6);
    assert!((get_record_float(&result, "confidence_upper") - 184.529097).abs() < 1e-5);
}

#[test]
fn test_ttest_default_remains_pooled_and_discloses_method() {
    let a = float_list(&[1.2, 2.4, 3.1, 4.8, 5.5]);
    let b = float_list(&[2.0, 3.3, 4.1, 6.2, 7.9]);
    let result = call_stats_builtin("ttest", vec![a, b]).unwrap();
    assert_eq!(get_record_str(&result, "method"), "student_pooled");
    assert!((get_record_float(&result, "df") - 8.0).abs() < 1e-12);
    assert!((get_record_float(&result, "confidence_lower") + 4.324296).abs() < 1e-6);
    assert!((get_record_float(&result, "confidence_upper") - 1.724296).abs() < 1e-6);
}

#[test]
fn test_ttest_welch_matches_r_default() {
    let a = float_list(&[1.2, 2.4, 3.1, 4.8, 5.5]);
    let b = float_list(&[2.0, 3.3, 4.1, 6.2, 7.9]);
    let options = option_record(&[("variance", Value::Str("welch".into()))]);
    let result = call_stats_builtin("ttest", vec![a, b, options]).unwrap();
    assert_eq!(get_record_str(&result, "method"), "welch");
    assert!((get_record_float(&result, "df") - 7.399468500859778).abs() < 1e-10);
    assert!((get_record_float(&result, "p_value") - 0.35286953859808035).abs() < 1e-10);
    assert!((get_record_float(&result, "confidence_lower") + 4.367557).abs() < 1e-6);
    assert!((get_record_float(&result, "confidence_upper") - 1.767557).abs() < 1e-6);
}

fn option_record(values: &[(&str, Value)]) -> Value {
    Value::Record(
        values
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn get_record_str<'a>(val: &'a Value, key: &str) -> &'a str {
    match val {
        Value::Record(map) => match map.get(key).unwrap() {
            Value::Str(value) => value,
            _ => panic!("expected string for key {key}"),
        },
        _ => panic!("expected Record"),
    }
}

#[test]
fn test_p_adjust_holm_is_monotone_with_tied_pvalues() {
    let pvals = float_list(&[0.0, 0.01, 0.01, 1.0]);
    let result = call_stats_builtin("p_adjust", vec![pvals, Value::Str("holm".into())]).unwrap();
    let Value::List(values) = result else {
        panic!("p_adjust should return a List");
    };
    let adjusted: Vec<f64> = values
        .iter()
        .map(|value| match value {
            Value::Float(number) => *number,
            other => panic!("expected Float, got {other:?}"),
        })
        .collect();
    assert_eq!(adjusted, vec![0.0, 0.03, 0.03, 1.0]);
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
    let result = call_stats_builtin("normalize", vec![data, Value::Str("zscore".into())]).unwrap();
    if let Value::List(items) = result {
        for item in items.iter() {
            if let Value::Float(v) = item {
                assert_eq!(*v, 0.0, "zscore of constant should be 0");
            }
        }
    }
}

#[test]
fn test_normalize_minmax() {
    let data = float_list(&[10.0, 20.0, 30.0]);
    let result = call_stats_builtin("normalize", vec![data, Value::Str("minmax".into())]).unwrap();
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
    let result = call_stats_builtin("normalize", vec![data, Value::Str("minmax".into())]).unwrap();
    if let Value::List(items) = result {
        for item in items.iter() {
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
        call_stats_builtin("sign", vec![Value::Float(-2.75)]).unwrap(),
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
    let result =
        call_stats_builtin("clamp", vec![Value::Int(15), Value::Int(0), Value::Int(10)]).unwrap();
    assert_eq!(result, Value::Float(10.0));
}

// ── is_nan / is_finite edge cases ───────────────────────────────

#[test]
fn test_is_nan_regular_number_false() {
    assert_eq!(
        call_stats_builtin("is_nan", vec![Value::Float(2.75)]).unwrap(),
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
        call_stats_builtin("is_finite", vec![Value::Float(2.75)]).unwrap(),
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
        vec![
            Value::Str("hello".into()),
            Value::Int(3),
            Value::Str("0".into()),
        ],
    )
    .unwrap();
    assert_eq!(result, Value::Str("hello".into()));
}

#[test]
fn test_pad_right_already_wide_enough() {
    let result = call_stats_builtin(
        "pad_right",
        vec![
            Value::Str("hello".into()),
            Value::Int(3),
            Value::Str("0".into()),
        ],
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
    let result = call_stats_builtin("sample", vec![int_list(&[1, 2, 3]), Value::Int(0)]).unwrap();
    assert_eq!(result, Value::List((vec![]).into()));
}

#[test]
fn test_sample_all() {
    let result = call_stats_builtin("sample", vec![int_list(&[1, 2, 3]), Value::Int(3)]).unwrap();
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

#[test]
fn test_wilcoxon_exact_matches_r_for_untied_small_samples() {
    let a = float_list(&[1.2, 2.4, 3.1, 4.8, 5.5]);
    let b = float_list(&[2.0, 3.3, 4.1, 6.2, 7.9]);
    let options = option_record(&[("method", Value::Str("exact".into()))]);
    let result = call_stats_builtin("wilcoxon", vec![a, b, options]).unwrap();
    assert_eq!(get_record_str(&result, "method"), "mann_whitney_exact");
    assert!((get_record_float(&result, "p_value") - 0.42063492063492064).abs() < 1e-12);
    assert!((get_record_float(&result, "rank_biserial") + 0.36).abs() < 1e-12);
}

#[test]
fn test_wilcoxon_normal_continuity_matches_r() {
    let a = float_list(&[1.2, 2.4, 3.1, 4.8, 5.5]);
    let b = float_list(&[2.0, 3.3, 4.1, 6.2, 7.9]);
    let options = option_record(&[
        ("method", Value::Str("normal".into())),
        ("continuity", Value::Bool(true)),
    ]);
    let result = call_stats_builtin("wilcoxon", vec![a, b, options]).unwrap();
    assert_eq!(get_record_str(&result, "method"), "mann_whitney_normal");
    let p_value = get_record_float(&result, "p_value");
    assert!(
        (p_value - 0.4033953048926283).abs() < 2e-7,
        "BioLang p={p_value}"
    );
}

#[test]
fn test_wilcoxon_exact_rejects_ties_instead_of_silently_approximating() {
    let a = float_list(&[1.0, 2.0, 2.0]);
    let b = float_list(&[2.0, 3.0, 4.0]);
    let options = option_record(&[("method", Value::Str("exact".into()))]);
    let error = call_stats_builtin("wilcoxon", vec![a, b, options]).unwrap_err();
    assert!(error.to_string().contains("untied"));
}

#[test]
fn test_paired_wilcoxon_exact_discloses_direction_and_effect() {
    let a = float_list(&[11.0, 8.0, 13.0, 6.0, 15.0, 4.0, 17.0, 2.0]);
    let b = float_list(&[10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0]);
    let options = option_record(&[("method", Value::Str("exact".into()))]);
    let result = call_stats_builtin("wilcoxon_paired", vec![a, b, options]).unwrap();
    assert_eq!(
        get_record_str(&result, "method"),
        "wilcoxon_signed_rank_exact"
    );
    assert_eq!(get_record_float(&result, "statistic"), 16.0);
    assert!((get_record_float(&result, "rank_biserial") + 1.0 / 9.0).abs() < 1e-12);
}

#[test]
fn test_paired_wilcoxon_normal_handles_tied_differences() {
    let before = float_list(&[12.1, 13.5, 11.8, 14.2, 15.0, 13.0]);
    let after = float_list(&[11.7, 12.9, 11.5, 13.1, 14.4, 12.8]);
    let result = call_stats_builtin("wilcoxon_paired", vec![before, after]).unwrap();
    assert_eq!(
        get_record_str(&result, "method"),
        "wilcoxon_signed_rank_normal"
    );
    assert_eq!(get_record_float(&result, "statistic"), 21.0);
    assert_eq!(get_record_float(&result, "rank_biserial"), 1.0);
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
    assert!(
        stat > 0.5,
        "KS stat={stat} should be high for very different distributions"
    );
}

// ── spearman ────────────────────────────────────────────────────

#[test]
fn test_spearman_perfect_monotone() {
    let x = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let y = float_list(&[2.0, 4.0, 6.0, 8.0, 10.0]);
    let result = call_stats_builtin("spearman", vec![x, y]).unwrap();
    let rho = get_record_float(&result, "coefficient");
    assert!(
        (rho - 1.0).abs() < 1e-10,
        "spearman rho={rho} should be 1.0"
    );
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
    let events = Value::List(
        (vec![
            Value::Bool(true),
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(false),
        ])
        .into(),
    );
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
    let events = Value::List((vec![Value::Int(1), Value::Int(0), Value::Int(1)]).into());
    let result = call_stats_builtin("kaplan_meier", vec![times, events]).unwrap();
    assert!(matches!(result, Value::Record(_)));
}

// ── cox_ph ──────────────────────────────────────────────────────

#[test]
fn test_cox_ph_basic() {
    let times = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let events = Value::List(
        (vec![
            Value::Bool(true),
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(true),
        ])
        .into(),
    );
    let covariates = Value::List(
        (vec![
            float_list(&[1.0]),
            float_list(&[2.0]),
            float_list(&[1.5]),
            float_list(&[3.0]),
            float_list(&[2.5]),
            float_list(&[1.0]),
            float_list(&[3.5]),
            float_list(&[2.0]),
        ])
        .into(),
    );
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
    let result = call_stats_builtin("hist", vec![float_list(&[1.0, 2.0, 3.0]), Value::Int(0)]);
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
        vec![Value::Str("{} and {}".into()), Value::Int(1), Value::Int(2)],
    )
    .unwrap();
    assert_eq!(result, Value::Str("1 and 2".into()));
}

// ── mixed int/float lists for stats ─────────────────────────────

#[test]
fn test_variance_mixed_types() {
    let list = Value::List(
        (vec![
            Value::Int(2),
            Value::Float(4.0),
            Value::Int(4),
            Value::Float(4.0),
            Value::Int(5),
            Value::Int(5),
            Value::Int(7),
            Value::Int(9),
        ])
        .into(),
    );
    let result = call_stats_builtin("variance", vec![list]).unwrap();
    if let Value::Float(v) = result {
        assert!((v - 4.571).abs() < 0.01, "variance={v}");
    }
}

#[test]
fn test_cor_mixed_int_float() {
    let x = Value::List((vec![Value::Int(1), Value::Float(2.0), Value::Int(3)]).into());
    let y = Value::List((vec![Value::Float(2.0), Value::Int(4), Value::Float(6.0)]).into());
    let result = call_stats_builtin("cor", vec![x, y]).unwrap();
    if let Value::Float(r) = result {
        assert!(
            (r - 1.0).abs() < 1e-10,
            "perfect correlation expected, got {r}"
        );
    }
}

// ── large list for performance ──────────────────────────────────

#[test]
fn test_mean_large_list() {
    let data: Vec<i64> = (1..=1000).collect();
    let result = call_stats_builtin("mean", vec![int_list(&data)]).unwrap();
    if let Value::Float(v) = result {
        assert!(
            (v - 500.5).abs() < 1e-10,
            "mean of 1..1000 should be 500.5, got {v}"
        );
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
        assert!(
            (v - 500.5).abs() < 1e-10,
            "median of 1..1000 should be 500.5, got {v}"
        );
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
            assert!((0.0..=1.0).contains(&r), "random() = {r} out of [0,1]");
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
    let list = Value::List((vec![Value::Str("hello".into())]).into());
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
    assert!(
        list.len() >= 70,
        "should have 70+ builtins, got {}",
        list.len()
    );
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
    let result = call_stats_builtin("round", vec![Value::Float(3.7), Value::Int(0)]).unwrap();
    assert_eq!(result, Value::Float(4.0));
}

// ── choose() beyond i128 ─────────────────────────────────────────────────────
//
// choose(300, 40) is 9.79e49. The implementation accumulated in i128, which tops
// out near 1.7e38, so the multiply wrapped: debug builds panicked, release
// builds returned 3.457e36 with no error. Exact answers are still exact; only
// the values that cannot be represented fall back to floating point.

fn choose(n: i64, k: i64) -> Value {
    call_stats_builtin("choose", vec![Value::Int(n), Value::Int(k)]).unwrap()
}

#[test]
fn choose_is_exact_while_it_fits() {
    assert_eq!(choose(15, 4), Value::Int(1365));
    assert_eq!(choose(52, 5), Value::Int(2_598_960));
    assert_eq!(choose(10, 0), Value::Int(1));
    assert_eq!(choose(10, 10), Value::Int(1));
    assert_eq!(choose(3, 5), Value::Int(0));
    // Largest exact central coefficient that still fits in i64.
    assert_eq!(choose(62, 31), Value::Int(465_428_353_255_261_088));
}

#[test]
fn choose_falls_back_to_float_past_i128() {
    // The case that used to wrap. Relative error must be float-level, not
    // thirteen orders of magnitude.
    let value = match choose(300, 40) {
        Value::Float(f) => f,
        other => panic!("expected Float, got {other:?}"),
    };
    let expected = 9.793_478_923_217_97e49_f64;
    assert!(
        ((value - expected) / expected).abs() < 1e-9,
        "choose(300,40) = {value}, expected about {expected}"
    );
}

#[test]
fn choose_handles_very_large_arguments() {
    let value = match choose(1000, 500) {
        Value::Float(f) => f,
        other => panic!("expected Float, got {other:?}"),
    };
    let expected = 2.702_882_409_454_365e299_f64;
    assert!(
        ((value - expected) / expected).abs() < 1e-9,
        "choose(1000,500) = {value}, expected about {expected}"
    );
    assert!(value.is_finite(), "choose(1000,500) overflowed to infinity");
}

#[test]
fn choose_is_symmetric() {
    for (n, k) in [(300i64, 40i64), (100, 7), (52, 5), (1000, 3)] {
        let a = choose(n, k);
        let b = choose(n, n - k);
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => assert_eq!(x, y, "choose({n},{k}) asymmetric"),
            (Value::Float(x), Value::Float(y)) => assert!(
                ((x - y) / x).abs() < 1e-12,
                "choose({n},{k}) asymmetric: {x} vs {y}"
            ),
            (x, y) => panic!("choose({n},{k}) returned mismatched types: {x:?} vs {y:?}"),
        }
    }
}

// ── power_t_test sample size ─────────────────────────────────────────────────
//
// n per group for a two-sample comparison is 2*((z_a/2 + z_b)/d)^2. The factor
// of 2 was missing, so every answer was half the real requirement - a sample
// size calculator advising experiments at half the size they need. Reference
// values are R's power.t.test(delta = d, sd = 1, power = p).

fn required_n(effect: f64, alpha: f64, power: f64) -> i64 {
    let r = call_stats_builtin(
        "power_t_test",
        vec![
            Value::Float(effect),
            Value::Float(alpha),
            Value::Float(power),
        ],
    )
    .unwrap();
    match r {
        Value::Record(m) => match m.get("n") {
            Some(Value::Int(n)) => *n,
            other => panic!("expected Int n, got {other:?}"),
        },
        other => panic!("expected Record, got {other:?}"),
    }
}

#[test]
fn power_t_test_matches_r_sample_sizes() {
    // R: power.t.test(delta=0.5, sd=1, power=0.80)$n  ->  63.77, so 64
    let n = required_n(0.5, 0.05, 0.80);
    assert!(
        (63..=65).contains(&n),
        "d=0.5 power=0.80 gave n={n}, expected ~64"
    );

    // R: power.t.test(delta=0.8, sd=1, power=0.80)$n  ->  25.52, so 26
    let n = required_n(0.8, 0.05, 0.80);
    assert!(
        (25..=27).contains(&n),
        "d=0.8 power=0.80 gave n={n}, expected ~26"
    );

    // R: power.t.test(delta=1.0, sd=1, power=0.90)$n  ->  22.02, so 23
    let n = required_n(1.0, 0.05, 0.90);
    assert!(
        (21..=24).contains(&n),
        "d=1.0 power=0.90 gave n={n}, expected ~23"
    );
}

#[test]
fn power_t_test_matches_r_noncentral_t_and_retains_normal_option() {
    let effect = 1.0 / 0.7;
    let exact = call_stats_builtin(
        "power_t_test",
        vec![Value::Float(effect), Value::Float(0.05), Value::Float(0.8)],
    )
    .unwrap();
    assert_eq!(get_record_float(&exact, "n"), 9.0);
    let exact_n_raw = get_record_float(&exact, "n_raw");
    let achieved = bl_core::bio_core::stats_ops::two_sample_t_power(exact_n_raw, effect, 0.05);
    assert!(
        (exact_n_raw - 8.76471066481821).abs() < 2e-5,
        "noncentral-t solver returned {exact_n_raw}"
    );
    assert!((achieved - 0.8).abs() < 1e-12);
    assert_eq!(get_record_str(&exact, "method"), "noncentral_t");

    let normal = call_stats_builtin(
        "power_t_test",
        vec![
            Value::Float(effect),
            Value::Float(0.05),
            Value::Float(0.8),
            option_record(&[("method", Value::Str("normal".into()))]),
        ],
    )
    .unwrap();
    assert_eq!(get_record_float(&normal, "n"), 8.0);
    assert!((get_record_float(&normal, "n_raw") - 7.691902139662104).abs() < 1e-7);
    assert_eq!(get_record_str(&normal, "method"), "normal");
}

#[test]
fn power_t_test_needs_more_samples_for_smaller_effects() {
    let small = required_n(0.25, 0.05, 0.80);
    let large = required_n(1.0, 0.05, 0.80);
    assert!(
        small > large * 4,
        "n should scale as 1/d^2: {small} vs {large}"
    );
}

#[test]
fn power_t_test_needs_more_samples_for_higher_power() {
    assert!(required_n(0.5, 0.05, 0.95) > required_n(0.5, 0.05, 0.80));
}

// ── permutation_test ────────────────────────────────────────────────
//
// It had no test against the live implementation at all. It had two against a
// second copy in `statistics.rs` that dispatch never reached -- and because
// both registries feed one arity table, with that one appended second, its
// `Exact(3)` decided the arity while `stats.rs` decided the behaviour. The
// documented optional fourth argument was rejected before any code that could
// have used it ran.

#[test]
fn power_prop_test_matches_r_power_prop_test_examples() {
    let achieved = call_stats_builtin(
        "power_prop_test",
        vec![
            Value::Float(0.8),
            Value::Float(0.2),
            option_record(&[("n", Value::Int(5))]),
        ],
    )
    .unwrap();
    assert!((get_record_float(&achieved, "power") - 0.4688159).abs() < 1e-7);

    let required = call_stats_builtin(
        "power_prop_test",
        vec![
            Value::Float(0.8),
            Value::Float(0.2),
            option_record(&[("power", Value::Float(0.9))]),
        ],
    )
    .unwrap();
    assert!((get_record_float(&required, "n_raw") - 12.37701).abs() < 1e-5);
    assert_eq!(get_record_float(&required, "n"), 13.0);
}

#[test]
fn power_t_test_can_compute_achieved_power_for_a_fixed_n() {
    let result = call_stats_builtin(
        "power_t_test",
        vec![
            Value::Float(2.0 / 2.3),
            Value::Float(0.05),
            Value::Nil,
            option_record(&[("n", Value::Int(20))]),
        ],
    )
    .unwrap();
    let power = get_record_float(&result, "power");
    assert!(
        (power - 0.7641668).abs() < 3e-6,
        "fixed-n power was {power}"
    );
}

fn permutation_p(args: Vec<Value>) -> f64 {
    let result = call_stats_builtin("permutation_test", args).expect("permutation_test runs");
    match result {
        Value::Record(fields) => match fields.get("p_value") {
            Some(Value::Float(p)) => *p,
            other => panic!("no p_value: {other:?}"),
        },
        other => panic!("expected a Record, got {other:?}"),
    }
}

#[test]
fn permutation_test_takes_its_optional_permutation_count() {
    let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let p = permutation_p(vec![a, b, Value::Str("mean_diff".into()), Value::Int(500)]);
    assert!((0.0..=1.0).contains(&p), "p outside [0, 1]: {p}");
}

#[test]
fn permutation_test_works_without_the_optional_count() {
    let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let p = permutation_p(vec![a, b, Value::Str("mean_diff".into())]);
    assert!((0.0..=1.0).contains(&p), "p outside [0, 1]: {p}");
}

#[test]
fn identical_groups_are_not_significant() {
    let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let p = permutation_p(vec![a, b, Value::Str("mean_diff".into()), Value::Int(500)]);
    assert!(p > 0.2, "identical groups should have a high p, got {p}");
}

#[test]
fn clearly_different_groups_are_significant() {
    let a = float_list(&[100.0, 110.0, 105.0, 108.0, 102.0]);
    let b = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let p = permutation_p(vec![a, b, Value::Str("mean_diff".into()), Value::Int(500)]);
    assert!(
        p < 0.1,
        "very different groups should have a low p, got {p}"
    );
}

#[test]
fn every_documented_statistic_is_accepted() {
    // Discoverable only by triggering the error, which is its own problem, but
    // at least the four the error names must all work.
    for statistic in ["mean_diff", "median_diff", "ks", "t"] {
        let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let b = float_list(&[2.0, 3.0, 4.0, 5.0, 6.0]);
        let p = permutation_p(vec![a, b, Value::Str(statistic.into()), Value::Int(200)]);
        assert!((0.0..=1.0).contains(&p), "{statistic} gave p = {p}");
    }
}

#[test]
fn an_unknown_statistic_names_the_ones_that_exist() {
    let a = float_list(&[1.0, 2.0, 3.0]);
    let b = float_list(&[4.0, 5.0, 6.0]);
    let error = call_stats_builtin(
        "permutation_test",
        vec![a, b, Value::Str("wilcoxon".into())],
    )
    .expect_err("wilcoxon is not one of the statistics");
    let message = error.to_string();
    for statistic in ["mean_diff", "median_diff", "ks", "t"] {
        assert!(
            message.contains(statistic),
            "{statistic} unlisted: {message}"
        );
    }
}

// ── Tail probabilities, contingency tables and odds ratios ──────────────
//
// Every reference value here is R 4.6.1, printed to seventeen significant
// figures. They cover six things the book found by trying to use the runtime
// for a statistics course.

fn record_float(value: &Value, field: &str) -> f64 {
    match value {
        Value::Record(fields) => match fields.get(field) {
            Some(Value::Float(f)) => *f,
            Some(Value::Int(n)) => *n as f64,
            other => panic!("field {field} is {other:?}"),
        },
        other => panic!("expected a Record, got {other:?}"),
    }
}

fn relative_error(got: f64, expected: f64) -> f64 {
    if expected == 0.0 {
        got.abs()
    } else {
        ((got - expected) / expected).abs()
    }
}

/// Two cells whose chi-square statistic is exactly `chi2` on one df.
fn one_df_table(chi2: f64) -> (Value, Value) {
    let deviation = (chi2 * 50.0).sqrt();
    (
        float_list(&[100.0 + deviation, 100.0 - deviation]),
        float_list(&[100.0, 100.0]),
    )
}

#[test]
fn chi_square_p_values_survive_the_far_tail() {
    // `pchisq(x, 1, lower.tail = FALSE)`. Computed as `1 - cdf` these lost
    // precision below 1e-15 and underflowed to exactly 0.0 past it -- and a
    // p-value of zero is not a stronger result, it is a missing one. It also
    // breaks every -log10(p), which is the y axis of a volcano plot.
    for (chi2, expected) in [
        (4.0, 0.045500263896358473),
        (16.0, 6.3342483666239808e-05),
        (36.0, 1.973175290075397e-09),
        (64.0, 1.244192114854357e-15),
        (81.0, 2.2571768119076811e-19),
        (200.0, 2.0884875837625449e-45),
        (500.0, 9.5053977665540927e-111),
    ] {
        let (observed, expected_counts) = one_df_table(chi2);
        let result = call_stats_builtin("chi_square", vec![observed, expected_counts]).unwrap();
        let p = record_float(&result, "p_value");
        assert!(
            relative_error(p, expected) < 1e-11,
            "chi2 = {chi2}: R says {expected}, got {p}"
        );
    }
}

#[test]
fn pnorm_is_accurate_rather_than_seven_figures() {
    // The 68/95 rule to R's own digits. The Abramowitz & Stegun approximation
    // this replaced was good to about 1.5e-7 absolute -- fine for reporting,
    // and not fine for a tail.
    let within = |k: f64| {
        let hi = call_stats_builtin("pnorm", vec![Value::Float(k)]).unwrap();
        let lo = call_stats_builtin("pnorm", vec![Value::Float(-k)]).unwrap();
        match (hi, lo) {
            (Value::Float(h), Value::Float(l)) => h - l,
            other => panic!("pnorm should return Float, got {other:?}"),
        }
    };
    assert!(relative_error(within(1.0), 0.68268949213708585) < 1e-12);
    assert!(relative_error(within(2.0), 0.95449973610364158) < 1e-12);
}

#[test]
fn a_deep_normal_tail_is_a_number_rather_than_zero() {
    // pnorm(-10) in R is 7.6198530241605269e-24.
    let p = match call_stats_builtin("pnorm", vec![Value::Float(-10.0)]).unwrap() {
        Value::Float(p) => p,
        other => panic!("{other:?}"),
    };
    assert!(relative_error(p, 7.6198530241605269e-24) < 1e-11, "got {p}");
}

#[test]
fn a_contingency_table_gets_contingency_degrees_of_freedom() {
    // The Berkeley 2x2 aggregate. `chi_square(observed, expected)` is a
    // goodness-of-fit test and reports k - 1 = 3 here, which is wrong for a
    // table whose expected counts came from its own margins.
    //
    // R: chisq.test gives 91.609598 with Yates and 92.205280 without, both on
    // one degree of freedom.
    let table =
        Value::List(vec![float_list(&[1198.0, 1493.0]), float_list(&[557.0, 1278.0])].into());
    let mut yates_options = HashMap::new();
    yates_options.insert("correct".to_string(), Value::Bool(false));

    let corrected = call_stats_builtin("chi_square_contingency", vec![table.clone()]).unwrap();
    assert_eq!(record_float(&corrected, "df"), 1.0);
    assert!(relative_error(record_float(&corrected, "chi2"), 91.609598) < 1e-7);
    assert!(relative_error(record_float(&corrected, "p_value"), 1.0557968087828389e-21) < 1e-10);

    let plain = call_stats_builtin(
        "chi_square_contingency",
        vec![table, Value::Record(std::sync::Arc::new(yates_options))],
    )
    .unwrap();
    assert!(relative_error(record_float(&plain, "chi2"), 92.205280) < 1e-7);
    assert!(relative_error(record_float(&plain, "p_value"), 7.8136003889946405e-22) < 1e-10);
}

#[test]
fn yates_is_for_two_by_two_and_nothing_else() {
    // R applies the correction to 2x2 tables and never to larger ones, so
    // asking for it on a 4x2 must change nothing. Hair colour against eye
    // colour, collapsed to brown and blue: R gives 108.997545 on 3 df.
    let table = Value::List(
        vec![
            float_list(&[68.0, 20.0]),
            float_list(&[119.0, 84.0]),
            float_list(&[26.0, 17.0]),
            float_list(&[7.0, 94.0]),
        ]
        .into(),
    );
    let result = call_stats_builtin("chi_square_contingency", vec![table]).unwrap();
    assert_eq!(record_float(&result, "df"), 3.0);
    assert!(relative_error(record_float(&result, "chi2"), 108.997545) < 1e-7);
    assert!(relative_error(record_float(&result, "p_value"), 1.8032802150609905e-23) < 1e-10);
    match &result {
        Value::Record(fields) => {
            assert_eq!(fields.get("yates_correction"), Some(&Value::Bool(false)))
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn yates_never_pushes_a_deviation_past_zero() {
    // Every cell of this table sits within half a count of its expected value,
    // so the correction takes all four deviations to zero and R's chisq.test
    // returns a statistic of exactly 0 with p = 1. Subtracting the half without
    // clamping leaves small negative deviations that square back to something
    // positive, which is a statistic pointing away from the null it is measured
    // against. Uncorrected, R gives 0.023242630385487569.
    let table = Value::List(vec![float_list(&[10.0, 10.0]), float_list(&[10.0, 11.0])].into());
    let corrected = call_stats_builtin("chi_square_contingency", vec![table.clone()]).unwrap();
    assert_eq!(
        record_float(&corrected, "chi2"),
        0.0,
        "every deviation here is under half a count"
    );
    assert_eq!(record_float(&corrected, "p_value"), 1.0);

    let mut options = HashMap::new();
    options.insert("correct".to_string(), Value::Bool(false));
    let plain = call_stats_builtin(
        "chi_square_contingency",
        vec![table, Value::Record(std::sync::Arc::new(options))],
    )
    .unwrap();
    assert!(
        relative_error(record_float(&plain, "chi2"), 0.023242630385487569) < 1e-12,
        "got {}",
        record_float(&plain, "chi2")
    );
}

#[test]
fn breslow_day_reports_tarones_adjusted_homogeneity_test() {
    let strata = Value::List(
        vec![
            Value::List(vec![float_list(&[4.0, 5.0]), float_list(&[5.0, 103.0])].into()),
            Value::List(vec![float_list(&[10.0, 3.0]), float_list(&[5.0, 43.0])].into()),
        ]
        .into(),
    );
    let adjusted = call_stats_builtin("breslow_day_test", vec![strata.clone()]).unwrap();
    assert!(
        relative_error(
            record_float(&adjusted, "common_odds_ratio"),
            23.00060975609756
        ) < 1e-12
    );
    assert!(relative_error(record_float(&adjusted, "p_value"), 0.627420741721689) < 1e-10);
    assert!(record_float(&adjusted, "tarone_adjustment") > 0.0);

    let mut options = HashMap::new();
    options.insert("tarone".to_string(), Value::Bool(false));
    let unadjusted = call_stats_builtin(
        "breslow_day_test",
        vec![strata, Value::Record(std::sync::Arc::new(options))],
    )
    .unwrap();
    assert_eq!(
        record_float(&unadjusted, "statistic"),
        record_float(&unadjusted, "breslow_day_statistic")
    );
    assert_eq!(
        record_float(&unadjusted, "p_value"),
        record_float(&unadjusted, "breslow_day_p_value")
    );
}

#[test]
fn fisher_reports_both_odds_ratios() {
    // The sample cross-product and R's conditional MLE are different
    // estimators of different things, and quoting only one left anyone
    // cross-checking to work out which was wrong.
    //
    // The reference is not `fisher.test`'s printed estimate: that solves the
    // same root with `uniroot`'s default tolerance of about 1.2e-4, and on the
    // first table below it lands on 9.965185 where the root is 9.963354209.
    // These are R's own numbers computed with tol = 1e-12.
    for (a, b, c, d, sample, conditional) in [
        (
            308u64,
            142u64,
            154u64,
            709u64,
            9.9859154929577461,
            9.963354209,
        ),
        (3, 1, 1, 3, 9.0, 6.408319658),
        (10, 2, 3, 15, 25.0, 21.305317557),
    ] {
        let result = call_stats_builtin(
            "fisher_exact",
            vec![
                Value::Int(a as i64),
                Value::Int(b as i64),
                Value::Int(c as i64),
                Value::Int(d as i64),
            ],
        )
        .unwrap();
        assert!(
            relative_error(record_float(&result, "odds_ratio"), sample) < 1e-12,
            "sample odds ratio for {a},{b},{c},{d}"
        );
        assert!(
            relative_error(record_float(&result, "conditional_odds_ratio"), conditional) < 1e-8,
            "conditional odds ratio for {a},{b},{c},{d}: expected {conditional}, got {}",
            record_float(&result, "conditional_odds_ratio")
        );
    }
}

#[test]
fn the_exact_interval_is_not_the_wald_one() {
    // R's conf.int, again at tol = 1e-14 rather than fisher.test's default:
    // [2.753383, 301.462338] for 10/2/3/15. The Wald interval on the log of
    // the sample ratio is a different and symmetric thing, and both are
    // reported under names that say which is which.
    let result = call_stats_builtin(
        "fisher_exact",
        vec![Value::Int(10), Value::Int(2), Value::Int(3), Value::Int(15)],
    )
    .unwrap();
    assert!(
        relative_error(
            record_float(&result, "conditional_confidence_lower"),
            2.753383
        ) < 1e-6
    );
    assert!(
        relative_error(
            record_float(&result, "conditional_confidence_upper"),
            301.462338
        ) < 1e-6
    );
    assert!(
        record_float(&result, "confidence_upper")
            != record_float(&result, "conditional_confidence_upper"),
        "the two intervals should not be the same interval"
    );
}

#[test]
fn wilcoxon_defaults_to_the_test_r_would_choose() {
    // PlantGrowth ctrl against trt2: twenty untied values in two groups of ten,
    // so R uses the exact distribution and gets 0.063012838554634215. The old
    // default was the normal approximation with no continuity correction --
    // 0.05878, the least accurate of the three available -- chosen for none of
    // them.
    let ctrl = float_list(&[4.17, 5.58, 5.18, 6.11, 4.50, 4.61, 5.17, 4.53, 5.33, 5.14]);
    let trt2 = float_list(&[6.31, 5.12, 5.54, 5.50, 5.37, 5.29, 4.92, 6.15, 5.80, 5.26]);

    let auto = call_stats_builtin("wilcoxon", vec![ctrl.clone(), trt2.clone()]).unwrap();
    assert!(
        relative_error(record_float(&auto, "p_value"), 0.063012838554634215) < 1e-12,
        "default: got {}",
        record_float(&auto, "p_value")
    );

    let with_correction = |key: &str, value: Value| {
        let mut options = HashMap::new();
        options.insert(key.to_string(), value);
        call_stats_builtin(
            "wilcoxon",
            vec![
                ctrl.clone(),
                trt2.clone(),
                Value::Record(std::sync::Arc::new(options)),
            ],
        )
        .unwrap()
    };

    // R, exact = FALSE: 0.064022101283026933 corrected, 0.058781721355358897 not.
    let corrected = with_correction("continuity", Value::Bool(true));
    assert!(
        relative_error(record_float(&corrected, "p_value"), 0.064022101283026933) < 1e-12,
        "continuity: got {}",
        record_float(&corrected, "p_value")
    );
    let uncorrected = with_correction("continuity", Value::Bool(false));
    assert!(
        relative_error(record_float(&uncorrected, "p_value"), 0.058781721355358897) < 1e-12,
        "no continuity: got {}",
        record_float(&uncorrected, "p_value")
    );
}

#[test]
fn ties_send_wilcoxon_to_the_approximation() {
    // The exact rank distribution is not valid with ties, so the automatic
    // choice must not pick it -- R warns and falls back for the same reason.
    let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = float_list(&[5.0, 6.0, 7.0, 8.0, 9.0]);
    let result = call_stats_builtin("wilcoxon", vec![a, b]).unwrap();
    match &result {
        Value::Record(fields) => assert_eq!(
            fields.get("method"),
            Some(&Value::Str("mann_whitney_normal".into())),
            "a tied sample should not get an exact p-value"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_t_test_on_a_large_sample_uses_the_t_distribution() {
    // Above 100 degrees of freedom the CDF used to hand the question to the
    // normal outright. The t has heavier tails at every finite df, so that
    // returned p-values too small -- overstating significance. R:
    // 2 * pt(5, 101, lower.tail = FALSE) = 2.419870267043516e-06, where twice the
    // normal tail is 5.7330314e-07, a factor of four out.
    let n = 102;
    let mut a: Vec<f64> = (0..n).map(|i| f64::from(i) / f64::from(n)).collect();
    let mean: f64 = a.iter().sum::<f64>() / f64::from(n);
    for value in &mut a {
        *value -= mean;
    }
    // Two-sample with df = 202 would need a second group; the one-sample form
    // is the direct route to a large df.
    let shift =
        5.0 * (a.iter().map(|v| v * v).sum::<f64>() / (f64::from(n) - 1.0) / f64::from(n)).sqrt();
    let shifted: Vec<f64> = a.iter().map(|v| v + shift).collect();
    let result =
        call_stats_builtin("ttest_one", vec![float_list(&shifted), Value::Float(0.0)]).unwrap();
    let p = record_float(&result, "p_value");
    // R: 2 * pt(5, 101, lower.tail = FALSE)
    assert!(
        relative_error(p, 2.419870267043516e-06) < 1e-8,
        "expected the t tail, got {p}"
    );
}
