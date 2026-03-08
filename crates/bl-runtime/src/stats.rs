use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

/// Returns the list of (name, arity) for all stats/math/string builtins.
pub fn stats_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        // Statistics
        ("mean", Arity::Exact(1)),
        ("median", Arity::Exact(1)),
        ("stdev", Arity::Exact(1)),
        ("variance", Arity::Exact(1)),
        ("sum", Arity::Exact(1)),
        ("quantile", Arity::Exact(2)),
        ("cor", Arity::Exact(2)),
        ("unique", Arity::Exact(1)),
        // sample and cumsum are registered via table_ops with flexible arity
        // to support both List and Table dispatch (see call_builtin)
        ("summary", Arity::Exact(1)),
        // Math
        ("sqrt", Arity::Exact(1)),
        ("pow", Arity::Exact(2)),
        ("log", Arity::Exact(1)),
        ("log2", Arity::Exact(1)),
        ("log10", Arity::Exact(1)),
        ("exp", Arity::Exact(1)),
        ("ceil", Arity::Exact(1)),
        ("floor", Arity::Exact(1)),
        ("round", Arity::Range(1, 2)),
        // String
        ("upper", Arity::Exact(1)),
        ("lower", Arity::Exact(1)),
        ("trim", Arity::Exact(1)),
        ("starts_with", Arity::Exact(2)),
        ("ends_with", Arity::Exact(2)),
        ("str_replace", Arity::Exact(3)),
        ("substr", Arity::Exact(3)),
        // String (extended)
        ("char_at", Arity::Exact(2)),
        ("index_of", Arity::Exact(2)),
        ("str_repeat", Arity::Exact(2)),
        ("pad_left", Arity::Exact(3)),
        ("pad_right", Arity::Exact(3)),
        ("trim_left", Arity::Exact(1)),
        ("trim_right", Arity::Exact(1)),
        ("str_len", Arity::Exact(1)),
        ("format", Arity::AtLeast(1)),
        // Math (extended)
        ("sign", Arity::Exact(1)),
        ("clamp", Arity::Exact(3)),
        ("sin", Arity::Exact(1)),
        ("cos", Arity::Exact(1)),
        ("tan", Arity::Exact(1)),
        ("asin", Arity::Exact(1)),
        ("acos", Arity::Exact(1)),
        ("atan", Arity::Exact(1)),
        ("atan2", Arity::Exact(2)),
        ("is_nan", Arity::Exact(1)),
        ("is_finite", Arity::Exact(1)),
        ("pi", Arity::Exact(0)),
        ("euler", Arity::Exact(0)),
        ("random", Arity::Exact(0)),
        ("random_int", Arity::Exact(2)),
        // ASCII plotting
        ("hist", Arity::Range(1, 2)),
        ("scatter", Arity::Exact(2)),
        // Statistical testing (wraps bio_core::stats_ops)
        ("ttest", Arity::Exact(2)),
        ("ttest_paired", Arity::Exact(2)),
        ("ttest_one", Arity::Exact(2)),
        ("anova", Arity::Exact(1)),
        ("chi_square", Arity::Exact(2)),
        ("fisher_exact", Arity::Exact(4)),
        ("wilcoxon", Arity::Exact(2)),
        ("p_adjust", Arity::Exact(2)),
        ("normalize", Arity::Exact(2)),
        ("lm", Arity::Exact(2)),
        // New stats builtins
        ("ks_test", Arity::Exact(2)),
        ("spearman", Arity::Exact(2)),
        ("kendall", Arity::Exact(2)),
        ("kaplan_meier", Arity::Exact(2)),
        ("cox_ph", Arity::Exact(3)),
        ("mean_phred", Arity::Exact(1)),
        ("min_phred", Arity::Exact(1)),
        ("error_rate", Arity::Exact(1)),
        ("trim_quality", Arity::Exact(2)),
        ("ode_solve", Arity::Exact(3)),
    ]
}

/// Check if a name is a known stats/math/string builtin.
pub fn is_stats_builtin(name: &str) -> bool {
    matches!(
        name,
        "mean"
            | "median"
            | "stdev"
            | "variance"
            | "sum"
            | "quantile"
            | "cor"
            | "unique"
            | "sample"
            | "cumsum"
            | "summary"
            | "sqrt"
            | "pow"
            | "log"
            | "log2"
            | "log10"
            | "exp"
            | "ceil"
            | "floor"
            | "round"
            | "upper"
            | "lower"
            | "trim"
            | "starts_with"
            | "ends_with"
            | "str_replace"
            | "substr"
            | "char_at"
            | "index_of"
            | "str_repeat"
            | "pad_left"
            | "pad_right"
            | "trim_left"
            | "trim_right"
            | "str_len"
            | "format"
            | "sign"
            | "clamp"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "is_nan"
            | "is_finite"
            | "pi"
            | "euler"
            | "random"
            | "random_int"
            | "hist"
            | "scatter"
            | "ttest"
            | "ttest_paired"
            | "ttest_one"
            | "anova"
            | "chi_square"
            | "fisher_exact"
            | "wilcoxon"
            | "p_adjust"
            | "normalize"
            | "lm"
            | "ks_test"
            | "spearman"
            | "kendall"
            | "kaplan_meier"
            | "cox_ph"
            | "mean_phred"
            | "min_phred"
            | "error_rate"
            | "trim_quality"
            | "ode_solve"
    )
}

/// Execute a stats/math/string builtin by name.
pub fn call_stats_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "mean" => builtin_mean(args),
        "median" => builtin_median(args),
        "stdev" => builtin_stdev(args),
        "variance" => builtin_variance(args),
        "sum" => builtin_sum(args),
        "quantile" => builtin_quantile(args),
        "cor" => builtin_cor(args),
        "unique" => builtin_unique(args),
        "sample" => builtin_sample(args),
        "cumsum" => builtin_cumsum(args),
        "summary" => builtin_summary(args),
        "sqrt" => builtin_sqrt(args),
        "pow" => builtin_pow(args),
        "log" => builtin_log(args),
        "log2" => builtin_log2(args),
        "log10" => builtin_log10(args),
        "exp" => builtin_exp(args),
        "ceil" => builtin_ceil(args),
        "floor" => builtin_floor(args),
        "round" => builtin_round(args),
        "upper" => builtin_upper(args),
        "lower" => builtin_lower(args),
        "trim" => builtin_trim(args),
        "starts_with" => builtin_starts_with(args),
        "ends_with" => builtin_ends_with(args),
        "str_replace" => builtin_str_replace(args),
        "substr" => builtin_substr(args),
        "char_at" => builtin_char_at(args),
        "index_of" => builtin_index_of(args),
        "str_repeat" => builtin_str_repeat(args),
        "pad_left" => builtin_pad_left(args),
        "pad_right" => builtin_pad_right(args),
        "trim_left" => builtin_trim_left(args),
        "trim_right" => builtin_trim_right(args),
        "str_len" => builtin_str_len(args),
        "format" => builtin_format(args),
        "sign" => builtin_sign(args),
        "clamp" => builtin_clamp(args),
        "sin" => builtin_trig(args, f64::sin),
        "cos" => builtin_trig(args, f64::cos),
        "tan" => builtin_trig(args, f64::tan),
        "asin" => builtin_trig(args, f64::asin),
        "acos" => builtin_trig(args, f64::acos),
        "atan" => builtin_trig(args, f64::atan),
        "atan2" => builtin_atan2(args),
        "is_nan" => builtin_is_nan(args),
        "is_finite" => builtin_is_finite(args),
        "pi" => Ok(Value::Float(std::f64::consts::PI)),
        "euler" => Ok(Value::Float(std::f64::consts::E)),
        "random" => builtin_random(),
        "random_int" => builtin_random_int(args),
        "hist" => builtin_hist(args),
        "scatter" => builtin_scatter(args),
        "ttest" => builtin_ttest(args),
        "ttest_paired" => builtin_ttest_paired(args),
        "ttest_one" => builtin_ttest_one(args),
        "anova" => builtin_anova(args),
        "chi_square" => builtin_chi_square(args),
        "fisher_exact" => builtin_fisher_exact(args),
        "wilcoxon" => builtin_wilcoxon(args),
        "p_adjust" => builtin_p_adjust(args),
        "normalize" => builtin_normalize(args),
        "lm" => builtin_lm(args),
        "ks_test" => builtin_ks_test(args),
        "spearman" => builtin_spearman(args),
        "kendall" => builtin_kendall(args),
        "kaplan_meier" => builtin_kaplan_meier(args),
        "cox_ph" => builtin_cox_ph(args),
        "mean_phred" => builtin_mean_phred(args),
        "min_phred" => builtin_min_phred(args),
        "error_rate" => builtin_error_rate(args),
        "trim_quality" => builtin_trim_quality(args),
        "ode_solve" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "ode_solve() requires a closure and must be called via HOF dispatch",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown stats builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn to_f64(val: &Value) -> Option<f64> {
    match val {
        Value::Int(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

fn require_num_list(val: &Value, func: &str) -> Result<Vec<f64>> {
    match val {
        Value::List(items) => {
            let mut nums = Vec::with_capacity(items.len());
            for item in items {
                match to_f64(item) {
                    Some(f) => nums.push(f),
                    None => {
                        return Err(BioLangError::type_error(
                            format!(
                                "{func}() requires List of numbers, found {}",
                                item.type_of()
                            ),
                            None,
                        ))
                    }
                }
            }
            if nums.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("{func}() requires non-empty List"),
                    None,
                ));
            }
            Ok(nums)
        }
        // Quality scores → convert Phred+33 encoded bytes to numeric values
        Value::Quality(scores) => {
            if scores.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("{func}() requires non-empty Quality"),
                    None,
                ));
            }
            Ok(scores.iter().map(|&b| (b.saturating_sub(33)) as f64).collect())
        }
        other => Err(BioLangError::type_error(
            format!("{func}() requires List or Quality, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_num(val: &Value, func: &str) -> Result<f64> {
    to_f64(val).ok_or_else(|| {
        BioLangError::type_error(
            format!("{func}() requires number, got {}", val.type_of()),
            None,
        )
    })
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Int, got {}", other.type_of()),
            None,
        )),
    }
}

// ── Statistics ───────────────────────────────────────────────────

fn builtin_mean(args: Vec<Value>) -> Result<Value> {
    let nums = require_num_list(&args[0], "mean")?;
    let total: f64 = nums.iter().sum();
    Ok(Value::Float(total / nums.len() as f64))
}

fn builtin_median(args: Vec<Value>) -> Result<Value> {
    let mut nums = require_num_list(&args[0], "median")?;
    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = nums.len();
    let med = if len % 2 == 0 {
        (nums[len / 2 - 1] + nums[len / 2]) / 2.0
    } else {
        nums[len / 2]
    };
    Ok(Value::Float(med))
}

fn builtin_variance(args: Vec<Value>) -> Result<Value> {
    let nums = require_num_list(&args[0], "variance")?;
    if nums.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "variance() requires at least 2 values",
            None,
        ));
    }
    let mean: f64 = nums.iter().sum::<f64>() / nums.len() as f64;
    let var: f64 =
        nums.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (nums.len() - 1) as f64;
    Ok(Value::Float(var))
}

fn builtin_stdev(args: Vec<Value>) -> Result<Value> {
    let var = builtin_variance(args)?;
    match var {
        Value::Float(v) => Ok(Value::Float(v.sqrt())),
        _ => unreachable!(),
    }
}

fn builtin_sum(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::List(items) => {
            if items.is_empty() {
                return Ok(Value::Int(0));
            }
            let mut has_float = false;
            let mut int_sum: i64 = 0;
            let mut float_sum: f64 = 0.0;
            for item in items {
                match item {
                    Value::Int(n) => {
                        int_sum += n;
                        float_sum += *n as f64;
                    }
                    Value::Float(f) => {
                        has_float = true;
                        float_sum += f;
                    }
                    other => {
                        return Err(BioLangError::type_error(
                            format!(
                                "sum() requires List of numbers, found {}",
                                other.type_of()
                            ),
                            None,
                        ))
                    }
                }
            }
            if has_float {
                Ok(Value::Float(float_sum))
            } else {
                Ok(Value::Int(int_sum))
            }
        }
        other => Err(BioLangError::type_error(
            format!("sum() requires List, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_quantile(args: Vec<Value>) -> Result<Value> {
    let mut nums = require_num_list(&args[0], "quantile")?;
    let q = require_num(&args[1], "quantile")?;
    if !(0.0..=1.0).contains(&q) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "quantile() q must be between 0 and 1",
            None,
        ));
    }
    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pos = q * (nums.len() - 1) as f64;
    let lower = pos.floor() as usize;
    let upper = pos.ceil() as usize;
    let frac = pos - lower as f64;
    let result = nums[lower] * (1.0 - frac) + nums[upper] * frac;
    Ok(Value::Float(result))
}

fn builtin_cor(args: Vec<Value>) -> Result<Value> {
    let xs = require_num_list(&args[0], "cor")?;
    let ys = require_num_list(&args[1], "cor")?;
    if xs.len() != ys.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cor() requires Lists of equal length",
            None,
        ));
    }
    if xs.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cor() requires at least 2 values",
            None,
        ));
    }
    let n = xs.len() as f64;
    let mean_x: f64 = xs.iter().sum::<f64>() / n;
    let mean_y: f64 = ys.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for i in 0..xs.len() {
        let dx = xs[i] - mean_x;
        let dy = ys[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    let denom = (var_x * var_y).sqrt();
    if denom == 0.0 {
        return Ok(Value::Float(f64::NAN));
    }
    Ok(Value::Float(cov / denom))
}

fn builtin_unique(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::List(items) => {
            let mut seen = Vec::new();
            for item in items {
                if !seen.contains(item) {
                    seen.push(item.clone());
                }
            }
            Ok(Value::List(seen))
        }
        other => Err(BioLangError::type_error(
            format!("unique() requires List, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_sample(args: Vec<Value>) -> Result<Value> {
    let items = match &args[0] {
        Value::List(l) => l,
        other => {
            return Err(BioLangError::type_error(
                format!("sample() requires List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let n = require_int(&args[1], "sample")? as usize;
    if n > items.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "sample() n ({n}) exceeds list length ({})",
                items.len()
            ),
            None,
        ));
    }

    let mut indices: Vec<usize> = (0..items.len()).collect();
    // Simple xorshift PRNG seeded from system time
    let mut rng = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    if rng == 0 {
        rng = 42;
    }

    // Fisher-Yates partial shuffle
    for i in 0..n {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let j = i + (rng as usize) % (items.len() - i);
        indices.swap(i, j);
    }

    let result: Vec<Value> = indices[..n].iter().map(|&i| items[i].clone()).collect();
    Ok(Value::List(result))
}

fn builtin_cumsum(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::List(items) => {
            let mut result = Vec::with_capacity(items.len());
            let mut has_float = false;
            let mut int_sum: i64 = 0;
            let mut float_sum: f64 = 0.0;

            for item in items {
                match item {
                    Value::Int(n) => {
                        int_sum += n;
                        float_sum += *n as f64;
                        if has_float {
                            result.push(Value::Float(float_sum));
                        } else {
                            result.push(Value::Int(int_sum));
                        }
                    }
                    Value::Float(f) => {
                        has_float = true;
                        float_sum += f;
                        // Convert previous Int entries to Float
                        for v in &mut result {
                            if let Value::Int(n) = v {
                                *v = Value::Float(*n as f64);
                            }
                        }
                        result.push(Value::Float(float_sum));
                    }
                    other => {
                        return Err(BioLangError::type_error(
                            format!(
                                "cumsum() requires List of numbers, found {}",
                                other.type_of()
                            ),
                            None,
                        ))
                    }
                }
            }
            Ok(Value::List(result))
        }
        other => Err(BioLangError::type_error(
            format!("cumsum() requires List, got {}", other.type_of()),
            None,
        )),
    }
}

// ── summary(table) ──────────────────────────────────────────────

fn builtin_summary(args: Vec<Value>) -> Result<Value> {
    let table = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("summary() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };

    let out_columns = vec![
        "column".to_string(),
        "type".to_string(),
        "count".to_string(),
        "min".to_string(),
        "max".to_string(),
        "mean".to_string(),
    ];

    let mut out_rows: Vec<Vec<Value>> = Vec::new();

    for (ci, col_name) in table.columns.iter().enumerate() {
        let mut count = 0i64;
        let mut nums: Vec<f64> = Vec::new();
        let mut first_type: Option<String> = None;
        let mut all_same_type = true;

        for row in &table.rows {
            let val = &row[ci];
            if !matches!(val, Value::Nil) {
                count += 1;
                let t = format!("{}", val.type_of());
                if let Some(ref ft) = first_type {
                    if t != *ft {
                        all_same_type = false;
                    }
                } else {
                    first_type = Some(t);
                }
                if let Some(n) = to_f64(val) {
                    nums.push(n);
                }
            }
        }

        let type_name = if all_same_type {
            first_type.unwrap_or_else(|| "nil".to_string())
        } else {
            "mixed".to_string()
        };

        let (min_val, max_val, mean_val) = if nums.is_empty() {
            (Value::Nil, Value::Nil, Value::Nil)
        } else {
            let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mean = nums.iter().sum::<f64>() / nums.len() as f64;
            (Value::Float(min), Value::Float(max), Value::Float(mean))
        };

        out_rows.push(vec![
            Value::Str(col_name.clone()),
            Value::Str(type_name),
            Value::Int(count),
            min_val,
            max_val,
            mean_val,
        ]);
    }

    Ok(Value::Table(Table::new(out_columns, out_rows)))
}

// ── Math ─────────────────────────────────────────────────────────

fn builtin_sqrt(args: Vec<Value>) -> Result<Value> {
    let n = require_num(&args[0], "sqrt")?;
    Ok(Value::Float(n.sqrt()))
}

fn builtin_pow(args: Vec<Value>) -> Result<Value> {
    let base = require_num(&args[0], "pow")?;
    let e = require_num(&args[1], "pow")?;
    Ok(Value::Float(base.powf(e)))
}

fn builtin_log(args: Vec<Value>) -> Result<Value> {
    let n = require_num(&args[0], "log")?;
    Ok(Value::Float(n.ln()))
}

fn builtin_log2(args: Vec<Value>) -> Result<Value> {
    let n = require_num(&args[0], "log2")?;
    Ok(Value::Float(n.log2()))
}

fn builtin_log10(args: Vec<Value>) -> Result<Value> {
    let n = require_num(&args[0], "log10")?;
    Ok(Value::Float(n.log10()))
}

fn builtin_exp(args: Vec<Value>) -> Result<Value> {
    let n = require_num(&args[0], "exp")?;
    Ok(Value::Float(n.exp()))
}

fn builtin_ceil(args: Vec<Value>) -> Result<Value> {
    let n = require_num(&args[0], "ceil")?;
    Ok(Value::Int(n.ceil() as i64))
}

fn builtin_floor(args: Vec<Value>) -> Result<Value> {
    let n = require_num(&args[0], "floor")?;
    Ok(Value::Int(n.floor() as i64))
}

fn builtin_round(args: Vec<Value>) -> Result<Value> {
    let n = require_num(&args[0], "round")?;
    if args.len() > 1 {
        let places = require_int(&args[1], "round")?;
        let factor = 10f64.powi(places as i32);
        Ok(Value::Float((n * factor).round() / factor))
    } else {
        Ok(Value::Int(n.round() as i64))
    }
}

// ── String ───────────────────────────────────────────────────────

fn builtin_upper(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "upper")?;
    Ok(Value::Str(s.to_uppercase()))
}

fn builtin_lower(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "lower")?;
    Ok(Value::Str(s.to_lowercase()))
}

fn builtin_trim(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "trim")?;
    Ok(Value::Str(s.trim().to_string()))
}

fn builtin_starts_with(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "starts_with")?;
    let prefix = require_str(&args[1], "starts_with")?;
    Ok(Value::Bool(s.starts_with(prefix)))
}

fn builtin_ends_with(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "ends_with")?;
    let suffix = require_str(&args[1], "ends_with")?;
    Ok(Value::Bool(s.ends_with(suffix)))
}

fn builtin_str_replace(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "str_replace")?;
    let from = require_str(&args[1], "str_replace")?;
    let to = require_str(&args[2], "str_replace")?;
    Ok(Value::Str(s.replace(from, to)))
}

fn builtin_substr(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "substr")?;
    let start = require_int(&args[1], "substr")? as usize;
    let length = require_int(&args[2], "substr")? as usize;
    let chars: Vec<char> = s.chars().collect();
    let start = start.min(chars.len());
    let end = (start + length).min(chars.len());
    let result: String = chars[start..end].iter().collect();
    Ok(Value::Str(result))
}

// ── ASCII Plotting ───────────────────────────────────────────────

fn builtin_hist(args: Vec<Value>) -> Result<Value> {
    let nums = require_num_list(&args[0], "hist")?;
    let bins = if args.len() > 1 {
        require_int(&args[1], "hist")? as usize
    } else {
        10
    };

    if bins == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "hist() bins must be > 0",
            None,
        ));
    }

    let min_val = nums.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if min_val == max_val {
        println!("All values = {min_val} (n={})", nums.len());
        return Ok(Value::Nil);
    }

    let bin_width = (max_val - min_val) / bins as f64;
    let mut counts = vec![0usize; bins];

    for &val in &nums {
        let mut idx = ((val - min_val) / bin_width) as usize;
        if idx >= bins {
            idx = bins - 1;
        }
        counts[idx] += 1;
    }

    let max_count = *counts.iter().max().unwrap_or(&1);
    let bar_max = 40;

    println!("Histogram (n={}, bins={}):", nums.len(), bins);
    for (i, &count) in counts.iter().enumerate() {
        let lo = min_val + i as f64 * bin_width;
        let hi = lo + bin_width;
        let bar_len = if max_count > 0 {
            (count as f64 / max_count as f64 * bar_max as f64) as usize
        } else {
            0
        };
        let bar: String = "\u{2588}".repeat(bar_len);
        let bracket = if i == bins - 1 { "]" } else { ")" };
        println!("  [{lo:>8.1}, {hi:>8.1}{bracket} |{bar} {count}");
    }

    Ok(Value::Nil)
}

fn builtin_scatter(args: Vec<Value>) -> Result<Value> {
    let xs = require_num_list(&args[0], "scatter")?;
    let ys = require_num_list(&args[1], "scatter")?;

    if xs.len() != ys.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "scatter() requires Lists of equal length",
            None,
        ));
    }

    if xs.is_empty() {
        println!("(empty)");
        return Ok(Value::Nil);
    }

    let width = 60usize;
    let height = 20usize;

    let x_min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let x_max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_min = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let y_max = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let x_range = if (x_max - x_min).abs() < f64::EPSILON {
        1.0
    } else {
        x_max - x_min
    };
    let y_range = if (y_max - y_min).abs() < f64::EPSILON {
        1.0
    } else {
        y_max - y_min
    };

    // Create grid
    let mut grid = vec![vec![' '; width]; height];

    // Plot points
    for i in 0..xs.len() {
        let gx = ((xs[i] - x_min) / x_range * (width - 1) as f64) as usize;
        let gy = ((ys[i] - y_min) / y_range * (height - 1) as f64) as usize;
        let gy = (height - 1).saturating_sub(gy); // Invert Y axis
        let gx = gx.min(width - 1);
        let gy = gy.min(height - 1);
        grid[gy][gx] = '*';
    }

    // Render
    let y_w = format!("{:.0}", y_max)
        .len()
        .max(format!("{:.0}", y_min).len());

    println!("Scatter plot (n={}):", xs.len());
    for (row_idx, row) in grid.iter().enumerate() {
        let label = if row_idx == 0 {
            format!("{:>w$}", y_max as i64, w = y_w)
        } else if row_idx == height - 1 {
            format!("{:>w$}", y_min as i64, w = y_w)
        } else {
            " ".repeat(y_w)
        };
        let line: String = row.iter().collect();
        println!("  {label}|{line}");
    }
    let pad = " ".repeat(y_w);
    println!("  {pad}+{}", "-".repeat(width));
    println!(
        "  {pad} {:<1}{:>w$}",
        x_min as i64,
        x_max as i64,
        w = width - 1
    );

    Ok(Value::Nil)
}

// ── Statistical Testing (wraps bio_core::stats_ops) ─────────────

fn make_record(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Value::Record(map)
}

// ── String builtins (extended) ──────────────────────────────────

fn builtin_char_at(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "char_at")?;
    let idx = require_int(&args[1], "char_at")? as usize;
    match s.chars().nth(idx) {
        Some(ch) => Ok(Value::Str(ch.to_string())),
        None => Ok(Value::Nil),
    }
}

fn builtin_index_of(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "index_of")?;
    let sub = require_str(&args[1], "index_of")?;
    match s.find(sub) {
        Some(pos) => Ok(Value::Int(pos as i64)),
        None => Ok(Value::Int(-1)),
    }
}

fn builtin_str_repeat(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "str_repeat")?;
    let n = require_int(&args[1], "str_repeat")? as usize;
    Ok(Value::Str(s.repeat(n)))
}

fn builtin_pad_left(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "pad_left")?;
    let width = require_int(&args[1], "pad_left")? as usize;
    let pad = require_str(&args[2], "pad_left")?;
    let pad_char = pad.chars().next().unwrap_or(' ');
    if s.len() >= width {
        Ok(Value::Str(s.to_string()))
    } else {
        let padding: String = std::iter::repeat_n(pad_char, width - s.len()).collect();
        Ok(Value::Str(format!("{padding}{s}")))
    }
}

fn builtin_pad_right(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "pad_right")?;
    let width = require_int(&args[1], "pad_right")? as usize;
    let pad = require_str(&args[2], "pad_right")?;
    let pad_char = pad.chars().next().unwrap_or(' ');
    if s.len() >= width {
        Ok(Value::Str(s.to_string()))
    } else {
        let padding: String = std::iter::repeat_n(pad_char, width - s.len()).collect();
        Ok(Value::Str(format!("{s}{padding}")))
    }
}

fn builtin_trim_left(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "trim_left")?;
    Ok(Value::Str(s.trim_start().to_string()))
}

fn builtin_trim_right(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "trim_right")?;
    Ok(Value::Str(s.trim_end().to_string()))
}

fn builtin_str_len(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "str_len")?;
    Ok(Value::Int(s.chars().count() as i64))
}

fn builtin_format(args: Vec<Value>) -> Result<Value> {
    let template = require_str(&args[0], "format")?;
    let mut result = template.to_string();
    for (i, arg) in args[1..].iter().enumerate() {
        let placeholder = format!("{{{i}}}");
        result = result.replace(&placeholder, &format!("{arg}"));
    }
    // Also support {} for sequential replacement
    for arg in &args[1..] {
        if let Some(pos) = result.find("{}") {
            result = format!("{}{}{}", &result[..pos], arg, &result[pos + 2..]);
        }
    }
    Ok(Value::Str(result))
}

// ── Math builtins (extended) ────────────────────────────────────

fn builtin_sign(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n.signum())),
        Value::Float(f) => {
            if f.is_nan() {
                Ok(Value::Float(f64::NAN))
            } else {
                Ok(Value::Float(f.signum()))
            }
        }
        other => Err(BioLangError::type_error(
            format!("sign() requires number, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_clamp(args: Vec<Value>) -> Result<Value> {
    let val = to_f64(&args[0]).ok_or_else(|| {
        BioLangError::type_error(format!("clamp() requires number, got {}", args[0].type_of()), None)
    })?;
    let lo = to_f64(&args[1]).ok_or_else(|| {
        BioLangError::type_error("clamp() min must be number", None)
    })?;
    let hi = to_f64(&args[2]).ok_or_else(|| {
        BioLangError::type_error("clamp() max must be number", None)
    })?;
    Ok(Value::Float(val.clamp(lo, hi)))
}

fn builtin_trig(args: Vec<Value>, f: fn(f64) -> f64) -> Result<Value> {
    let x = to_f64(&args[0]).ok_or_else(|| {
        BioLangError::type_error(
            format!("trig function requires number, got {}", args[0].type_of()),
            None,
        )
    })?;
    Ok(Value::Float(f(x)))
}

fn builtin_atan2(args: Vec<Value>) -> Result<Value> {
    let y = to_f64(&args[0]).ok_or_else(|| {
        BioLangError::type_error("atan2() requires number", None)
    })?;
    let x = to_f64(&args[1]).ok_or_else(|| {
        BioLangError::type_error("atan2() requires number", None)
    })?;
    Ok(Value::Float(y.atan2(x)))
}

fn builtin_is_nan(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Float(f) => Ok(Value::Bool(f.is_nan())),
        Value::Int(_) => Ok(Value::Bool(false)),
        other => Err(BioLangError::type_error(
            format!("is_nan() requires number, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_is_finite(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Float(f) => Ok(Value::Bool(f.is_finite())),
        Value::Int(_) => Ok(Value::Bool(true)),
        other => Err(BioLangError::type_error(
            format!("is_finite() requires number, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_random() -> Result<Value> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    // xorshift64
    let mut x = seed.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    Ok(Value::Float((x as f64) / (u64::MAX as f64)))
}

fn builtin_random_int(args: Vec<Value>) -> Result<Value> {
    let lo = require_int(&args[0], "random_int")?;
    let hi = require_int(&args[1], "random_int")?;
    if lo >= hi {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "random_int() requires lo < hi",
            None,
        ));
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut x = seed.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    let range = (hi - lo) as u64;
    let val = lo + (x % range) as i64;
    Ok(Value::Int(val))
}

// ── Statistical testing (wraps bio_core::stats_ops) ─────────────

fn builtin_ttest(args: Vec<Value>) -> Result<Value> {
    let a = require_num_list(&args[0], "ttest")?;
    let b = require_num_list(&args[1], "ttest")?;
    let res = bl_core::bio_core::stats_ops::t_test(&a, &b, "two_sided")
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(make_record(vec![
        ("t_statistic", Value::Float(res.statistic)),
        ("p_value", Value::Float(res.p_value)),
        ("df", Value::Float(res.df)),
        ("mean_diff", Value::Float(res.mean_a - res.mean_b)),
    ]))
}

fn builtin_ttest_paired(args: Vec<Value>) -> Result<Value> {
    let a = require_num_list(&args[0], "ttest_paired")?;
    let b = require_num_list(&args[1], "ttest_paired")?;
    if a.len() != b.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "ttest_paired() requires lists of equal length",
            None,
        ));
    }
    let diffs: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x - y).collect();
    let n = diffs.len() as f64;
    let mean_d = diffs.iter().sum::<f64>() / n;
    let var_d = diffs.iter().map(|d| (d - mean_d).powi(2)).sum::<f64>() / (n - 1.0);
    let se = (var_d / n).sqrt();
    let t = if se > 0.0 { mean_d / se } else { 0.0 };
    let df = n - 1.0;
    let p = 2.0 * (1.0 - bl_core::bio_core::stats_ops::students_t_cdf(t.abs(), df));
    Ok(make_record(vec![
        ("t_statistic", Value::Float(t)),
        ("p_value", Value::Float(p)),
        ("df", Value::Float(df)),
        ("mean_diff", Value::Float(mean_d)),
    ]))
}

fn builtin_ttest_one(args: Vec<Value>) -> Result<Value> {
    let data = require_num_list(&args[0], "ttest_one")?;
    let mu = require_num(&args[1], "ttest_one")?;
    let n = data.len() as f64;
    let mean = data.iter().sum::<f64>() / n;
    let var = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let se = (var / n).sqrt();
    let t = if se > 0.0 { (mean - mu) / se } else { 0.0 };
    let df = n - 1.0;
    let p = 2.0 * (1.0 - bl_core::bio_core::stats_ops::students_t_cdf(t.abs(), df));
    Ok(make_record(vec![
        ("t_statistic", Value::Float(t)),
        ("p_value", Value::Float(p)),
        ("df", Value::Float(df)),
    ]))
}

fn builtin_anova(args: Vec<Value>) -> Result<Value> {
    let groups = match &args[0] {
        Value::List(items) => {
            let mut gs: Vec<Vec<f64>> = Vec::new();
            for item in items {
                let g = match item {
                    Value::List(inner) => {
                        let mut nums = Vec::new();
                        for v in inner {
                            nums.push(to_f64(v).ok_or_else(|| {
                                BioLangError::type_error(
                                    "anova() requires list of numeric lists",
                                    None,
                                )
                            })?);
                        }
                        nums
                    }
                    _ => {
                        return Err(BioLangError::type_error(
                            "anova() requires list of lists",
                            None,
                        ))
                    }
                };
                gs.push(g);
            }
            gs
        }
        _ => {
            return Err(BioLangError::type_error(
                "anova() requires List of Lists",
                None,
            ))
        }
    };
    let res = bl_core::bio_core::stats_ops::anova(&groups)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(make_record(vec![
        ("f_statistic", Value::Float(res.f_statistic)),
        ("p_value", Value::Float(res.p_value)),
        ("df_between", Value::Float(res.df_between)),
        ("df_within", Value::Float(res.df_within)),
    ]))
}

fn builtin_chi_square(args: Vec<Value>) -> Result<Value> {
    let observed = require_num_list(&args[0], "chi_square")?;
    let expected = require_num_list(&args[1], "chi_square")?;
    if observed.len() != expected.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "chi_square() requires lists of equal length",
            None,
        ));
    }
    // Chi-square goodness-of-fit test
    let mut chi2 = 0.0;
    for (o, e) in observed.iter().zip(expected.iter()) {
        if *e <= 0.0 {
            continue;
        }
        chi2 += (o - e).powi(2) / e;
    }
    let df = (observed.len() - 1).max(1);
    let p_value = 1.0 - bl_core::bio_core::stats_ops::chi_square_cdf(chi2, df);
    Ok(make_record(vec![
        ("chi2", Value::Float(chi2)),
        ("p_value", Value::Float(p_value)),
        ("df", Value::Int(df as i64)),
    ]))
}

fn builtin_fisher_exact(args: Vec<Value>) -> Result<Value> {
    let a = require_num(&args[0], "fisher_exact")? as u64;
    let b = require_num(&args[1], "fisher_exact")? as u64;
    let c = require_num(&args[2], "fisher_exact")? as u64;
    let d = require_num(&args[3], "fisher_exact")? as u64;
    let table = vec![vec![a as f64, b as f64], vec![c as f64, d as f64]];
    let res = bl_core::bio_core::stats_ops::fishers_exact_test(&table)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(make_record(vec![
        ("p_value", Value::Float(res.p_value)),
        ("odds_ratio", Value::Float(res.odds_ratio)),
    ]))
}

fn builtin_wilcoxon(args: Vec<Value>) -> Result<Value> {
    let a = require_num_list(&args[0], "wilcoxon")?;
    let b = require_num_list(&args[1], "wilcoxon")?;
    let res = bl_core::bio_core::stats_ops::mann_whitney_test(&a, &b, "two_sided")
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(make_record(vec![
        ("u_statistic", Value::Float(res.statistic)),
        ("p_value", Value::Float(res.p_value)),
    ]))
}

fn builtin_p_adjust(args: Vec<Value>) -> Result<Value> {
    let pvals = require_num_list(&args[0], "p_adjust")?;
    let method = require_str(&args[1], "p_adjust")?;
    let adjusted = match method {
        "bh" | "BH" | "fdr" => {
            bl_core::bio_core::stats_ops::benjamini_hochberg_correction(&pvals, 0.05)
                .adjusted_p_values
        }
        "bonferroni" => {
            bl_core::bio_core::stats_ops::bonferroni_correction(&pvals).adjusted_p_values
        }
        "holm" => {
            bl_core::bio_core::stats_ops::holm_bonferroni_correction(&pvals).adjusted_p_values
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("p_adjust() unknown method '{method}', expected 'bh', 'bonferroni', or 'holm'"),
                None,
            ))
        }
    };
    Ok(Value::List(adjusted.into_iter().map(Value::Float).collect()))
}

fn builtin_normalize(args: Vec<Value>) -> Result<Value> {
    let nums = require_num_list(&args[0], "normalize")?;
    let method = require_str(&args[1], "normalize")?;
    let result = match method {
        "zscore" | "z" => {
            let n = nums.len() as f64;
            let mean = nums.iter().sum::<f64>() / n;
            let sd = (nums.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
            if sd == 0.0 {
                vec![0.0; nums.len()]
            } else {
                nums.iter().map(|x| (x - mean) / sd).collect()
            }
        }
        "minmax" => {
            let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let range = max - min;
            if range == 0.0 {
                vec![0.0; nums.len()]
            } else {
                nums.iter().map(|x| (x - min) / range).collect()
            }
        }
        "quantile" => {
            let mut sorted: Vec<(usize, f64)> = nums.iter().copied().enumerate().collect();
            sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let n = nums.len() as f64;
            let mut result = vec![0.0; nums.len()];
            for (rank, &(orig_idx, _)) in sorted.iter().enumerate() {
                result[orig_idx] = rank as f64 / (n - 1.0);
            }
            result
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("normalize() unknown method '{method}', expected 'zscore', 'minmax', or 'quantile'"),
                None,
            ))
        }
    };
    Ok(Value::List(result.into_iter().map(Value::Float).collect()))
}

fn builtin_lm(args: Vec<Value>) -> Result<Value> {
    // Support formula-based lm: lm(formula, table) or lm(x_list, y_list)
    match (&args[0], &args[1]) {
        (Value::Formula(formula_expr), Value::Table(table)) => {
            // Parse formula: ~y ~ x1 + x2
            // Extract column names from the formula AST
            let (response_col, predictor_cols) = parse_formula_columns(formula_expr, table)?;
            let y: Vec<f64> = extract_column_floats(table, &response_col, "lm")?;
            if predictor_cols.len() == 1 {
                let x: Vec<f64> = extract_column_floats(table, &predictor_cols[0], "lm")?;
                let res = bl_core::bio_core::stats_ops::linear_regression(&x, &y)
                    .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
                Ok(make_record(vec![
                    ("slope", Value::Float(res.slope)),
                    ("intercept", Value::Float(res.intercept)),
                    ("r_squared", Value::Float(res.r_squared)),
                    ("p_value", Value::Float(res.p_value)),
                    ("std_error", Value::Float(res.std_error)),
                ]))
            } else {
                let x_matrix: Vec<Vec<f64>> = (0..table.num_rows())
                    .map(|i| {
                        predictor_cols.iter().map(|col| {
                            let ci = table.col_index(col).unwrap();
                            to_f64(&table.rows[i][ci]).unwrap_or(0.0)
                        }).collect()
                    })
                    .collect();
                let res = bl_core::bio_core::stats_ops::multiple_linear_regression(&y, &x_matrix)
                    .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
                Ok(make_record(vec![
                    ("coefficients", Value::List(res.coefficients.iter().map(|&c| Value::Float(c)).collect())),
                    ("std_errors", Value::List(res.std_errors.iter().map(|&s| Value::Float(s)).collect())),
                    ("t_values", Value::List(res.t_values.iter().map(|&t| Value::Float(t)).collect())),
                    ("p_values", Value::List(res.p_values.iter().map(|&p| Value::Float(p)).collect())),
                    ("r_squared", Value::Float(res.r_squared)),
                    ("adj_r_squared", Value::Float(res.adj_r_squared)),
                    ("f_statistic", Value::Float(res.f_statistic)),
                    ("f_p_value", Value::Float(res.f_p_value)),
                ]))
            }
        }
        _ => {
            // Original behavior: lm(x_list, y_list)
            let x = require_num_list(&args[0], "lm")?;
            let y = require_num_list(&args[1], "lm")?;
            let res = bl_core::bio_core::stats_ops::linear_regression(&x, &y)
                .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
            Ok(make_record(vec![
                ("slope", Value::Float(res.slope)),
                ("intercept", Value::Float(res.intercept)),
                ("r_squared", Value::Float(res.r_squared)),
                ("p_value", Value::Float(res.p_value)),
                ("std_error", Value::Float(res.std_error)),
            ]))
        }
    }
}

/// Parse a formula AST to extract response and predictor column names.
/// Formula format: `~response ~ predictor1 + predictor2`
fn parse_formula_columns(
    formula_expr: &bl_core::span::Spanned<bl_core::ast::Expr>,
    table: &Table,
) -> Result<(String, Vec<String>)> {
    use bl_core::ast::Expr;
    // The formula is ~(response ~ predictors), where response and predictors are idents
    // or binary Add expressions
    fn collect_idents(expr: &bl_core::span::Spanned<Expr>) -> Vec<String> {
        match &expr.node {
            Expr::Ident(name) => vec![name.clone()],
            Expr::Binary { op: bl_core::ast::BinaryOp::Add, left, right } => {
                let mut v = collect_idents(left);
                v.extend(collect_idents(right));
                v
            }
            _ => vec![],
        }
    }
    match &formula_expr.node {
        Expr::Binary { left, right, .. } => {
            let response_names = collect_idents(left);
            let predictor_names = collect_idents(right);
            if response_names.len() != 1 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError, "formula must have exactly one response variable", None,
                ));
            }
            // Validate columns exist
            let resp = &response_names[0];
            if table.col_index(resp).is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError, format!("column '{resp}' not found in table"), None,
                ));
            }
            for pred in &predictor_names {
                if table.col_index(pred).is_none() {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError, format!("column '{pred}' not found in table"), None,
                    ));
                }
            }
            Ok((resp.clone(), predictor_names))
        }
        Expr::Ident(name) => {
            // Simple case: ~response — use all other columns as predictors
            if table.col_index(name).is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError, format!("column '{name}' not found in table"), None,
                ));
            }
            let predictors: Vec<String> = table.columns.iter()
                .filter(|c| c.as_str() != name.as_str())
                .cloned().collect();
            Ok((name.clone(), predictors))
        }
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError, "lm() formula must be ~response ~ predictors", None,
        )),
    }
}

fn extract_column_floats(table: &Table, col: &str, func: &str) -> Result<Vec<f64>> {
    let ci = table.col_index(col).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, format!("{func}(): column '{col}' not found"), None)
    })?;
    table.rows.iter().map(|row| {
        to_f64(&row[ci]).ok_or_else(|| {
            BioLangError::type_error(format!("{func}(): non-numeric value in column '{col}'"), None)
        })
    }).collect()
}

fn builtin_ks_test(args: Vec<Value>) -> Result<Value> {
    let s1 = require_num_list(&args[0], "ks_test")?;
    let s2 = require_num_list(&args[1], "ks_test")?;
    let res = bl_core::bio_core::stats_ops::kolmogorov_smirnov(&s1, &s2)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(make_record(vec![
        ("statistic", Value::Float(res.statistic)),
        ("pvalue", Value::Float(res.p_value)),
        ("n1", Value::Int(res.n1 as i64)),
        ("n2", Value::Int(res.n2 as i64)),
    ]))
}

fn builtin_spearman(args: Vec<Value>) -> Result<Value> {
    let x = require_num_list(&args[0], "spearman")?;
    let y = require_num_list(&args[1], "spearman")?;
    let res = bl_core::bio_core::stats_ops::correlation(&x, &y, "spearman")
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(make_record(vec![
        ("coefficient", Value::Float(res.correlation)),
        ("pvalue", Value::Float(res.p_value)),
        ("n", Value::Int(res.n as i64)),
    ]))
}

fn builtin_kendall(args: Vec<Value>) -> Result<Value> {
    let x = require_num_list(&args[0], "kendall")?;
    let y = require_num_list(&args[1], "kendall")?;
    let res = bl_core::bio_core::stats_ops::kendall_tau(&x, &y)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(make_record(vec![
        ("coefficient", Value::Float(res.correlation)),
        ("pvalue", Value::Float(res.p_value)),
        ("n", Value::Int(res.n as i64)),
    ]))
}

fn builtin_kaplan_meier(args: Vec<Value>) -> Result<Value> {
    let times = require_num_list(&args[0], "kaplan_meier")?;
    let events_val = match &args[1] {
        Value::List(items) => items.iter().map(|v| match v {
            Value::Bool(b) => Ok(*b),
            Value::Int(n) => Ok(*n != 0),
            _ => Err(BioLangError::type_error("kaplan_meier() events must be Bool or Int", None)),
        }).collect::<Result<Vec<bool>>>()?,
        _ => return Err(BioLangError::type_error("kaplan_meier() events must be a List", None)),
    };
    let res = bl_core::bio_core::stats_ops::kaplan_meier(&times, &events_val)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(make_record(vec![
        ("times", Value::List(res.times.iter().map(|&t| Value::Float(t)).collect())),
        ("survival", Value::List(res.survival.iter().map(|&s| Value::Float(s)).collect())),
        ("ci_lower", Value::List(res.ci_lower.iter().map(|&c| Value::Float(c)).collect())),
        ("ci_upper", Value::List(res.ci_upper.iter().map(|&c| Value::Float(c)).collect())),
        ("at_risk", Value::List(res.at_risk.iter().map(|&n| Value::Int(n as i64)).collect())),
    ]))
}

fn builtin_cox_ph(args: Vec<Value>) -> Result<Value> {
    let time = require_num_list(&args[0], "cox_ph")?;
    let event_val = match &args[1] {
        Value::List(items) => items.iter().map(|v| match v {
            Value::Bool(b) => Ok(*b),
            Value::Int(n) => Ok(*n != 0),
            _ => Err(BioLangError::type_error("cox_ph() event must be Bool or Int", None)),
        }).collect::<Result<Vec<bool>>>()?,
        _ => return Err(BioLangError::type_error("cox_ph() event must be a List", None)),
    };
    // covariates: Table or List of Lists
    let covariates: Vec<Vec<f64>> = match &args[2] {
        Value::Table(t) => {
            (0..t.num_rows()).map(|i| {
                t.rows[i].iter().map(|v| to_f64(v).unwrap_or(0.0)).collect()
            }).collect()
        }
        Value::List(items) => {
            items.iter().map(|v| match v {
                Value::List(inner) => inner.iter().map(|iv| to_f64(iv).unwrap_or(0.0)).collect(),
                _ => vec![to_f64(v).unwrap_or(0.0)],
            }).collect()
        }
        _ => return Err(BioLangError::type_error("cox_ph() covariates must be Table or List", None)),
    };
    let res = bl_core::bio_core::stats_ops::cox_ph(&time, &event_val, &covariates)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(make_record(vec![
        ("coefficients", Value::List(res.coefficients.iter().map(|&c| Value::Float(c)).collect())),
        ("hazard_ratios", Value::List(res.hazard_ratios.iter().map(|&h| Value::Float(h)).collect())),
        ("pvalues", Value::List(res.p_values.iter().map(|&p| Value::Float(p)).collect())),
        ("concordance", Value::Float(res.concordance)),
    ]))
}

fn builtin_mean_phred(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Quality(scores) => Ok(Value::Float(bl_core::bio_core::QualityOps::mean_phred(scores))),
        _ => Err(BioLangError::type_error("mean_phred() requires Quality value", None)),
    }
}

fn builtin_min_phred(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Quality(scores) => Ok(Value::Int(bl_core::bio_core::QualityOps::min_phred(scores) as i64)),
        _ => Err(BioLangError::type_error("min_phred() requires Quality value", None)),
    }
}

fn builtin_error_rate(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Quality(scores) => Ok(Value::Float(bl_core::bio_core::QualityOps::error_rate(scores))),
        _ => Err(BioLangError::type_error("error_rate() requires Quality value", None)),
    }
}

fn builtin_trim_quality(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Quality(scores) => {
            let threshold = require_int(&args[1], "trim_quality")? as u8;
            let trim_pos = bl_core::bio_core::QualityOps::trim_quality(scores, threshold);
            Ok(Value::Quality(scores[..trim_pos].to_vec()))
        }
        _ => Err(BioLangError::type_error("trim_quality() requires Quality value", None)),
    }
}

