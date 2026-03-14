use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

/// Returns the list of (name, arity) for all stats/math/string builtins.
pub fn stats_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        // Statistics
        ("mean", Arity::Exact(1)),
        ("median", Arity::Exact(1)),
        ("mode", Arity::Exact(1)),
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
        ("set_seed", Arity::Exact(1)),
        ("power_t_test", Arity::Range(1, 3)),
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
        // GLM & clustering
        ("glm", Arity::Range(2, 3)),
        ("kmeans", Arity::Range(2, 3)),
        // Distribution functions
        ("dnorm", Arity::Range(1, 3)),
        ("pnorm", Arity::Range(1, 3)),
        ("qnorm", Arity::Range(1, 3)),
        ("dbinom", Arity::Exact(3)),
        ("pbinom", Arity::Exact(3)),
        ("dpois", Arity::Exact(2)),
        ("ppois", Arity::Exact(2)),
        ("dunif", Arity::Range(1, 3)),
        ("punif", Arity::Range(1, 3)),
        ("dexp", Arity::Range(1, 2)),
        ("pexp", Arity::Range(1, 2)),
        ("rnorm", Arity::Range(1, 3)),
        ("rbinom", Arity::Exact(3)),
        ("rpois", Arity::Exact(2)),
    ]
}

/// Check if a name is a known stats/math/string builtin.
pub fn is_stats_builtin(name: &str) -> bool {
    matches!(
        name,
        "mean"
            | "median"
            | "mode"
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
            | "set_seed"
            | "power_t_test"
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
            | "glm"
            | "kmeans"
            | "dnorm"
            | "pnorm"
            | "qnorm"
            | "dbinom"
            | "pbinom"
            | "dpois"
            | "ppois"
            | "dunif"
            | "punif"
            | "dexp"
            | "pexp"
            | "rnorm"
            | "rbinom"
            | "rpois"
    )
}

/// Execute a stats/math/string builtin by name.
pub fn call_stats_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "mean" => builtin_mean(args),
        "median" => builtin_median(args),
        "mode" => builtin_mode(args),
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
        "set_seed" => builtin_set_seed(args),
        "power_t_test" => builtin_power_t_test(args),
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
        "kaplan_meier" => {
            // Route Table/Record input to bio_plots (visualization), List input to stats
            if matches!(&args[0], Value::Table(_) | Value::Record(_)) {
                crate::bio_plots::call_bio_plots_builtin("kaplan_meier", args)
            } else {
                builtin_kaplan_meier(args)
            }
        }
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
        "glm" => builtin_glm(args),
        "kmeans" => builtin_kmeans(args),
        "dnorm" => builtin_dnorm(args),
        "pnorm" => builtin_pnorm(args),
        "qnorm" => builtin_qnorm(args),
        "dbinom" => builtin_dbinom(args),
        "pbinom" => builtin_pbinom(args),
        "dpois" => builtin_dpois(args),
        "ppois" => builtin_ppois(args),
        "dunif" => builtin_dunif(args),
        "punif" => builtin_punif(args),
        "dexp" => builtin_dexp(args),
        "pexp" => builtin_pexp(args),
        "rnorm" => builtin_rnorm(args),
        "rbinom" => builtin_rbinom(args),
        "rpois" => builtin_rpois(args),
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

fn builtin_mode(args: Vec<Value>) -> Result<Value> {
    let nums = require_num_list(&args[0], "mode")?;
    if nums.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "mode() requires a non-empty list",
            None,
        ));
    }
    // Round to nearest integer for frequency counting (standard for continuous data)
    let mut freq: HashMap<i64, usize> = HashMap::new();
    for &v in &nums {
        *freq.entry(v.round() as i64).or_insert(0) += 1;
    }
    let max_count = freq.values().copied().max().unwrap_or(0);
    let mode_val = freq
        .into_iter()
        .filter(|&(_, c)| c == max_count)
        .map(|(v, _)| v)
        .min()
        .unwrap_or(0);
    Ok(Value::Int(mode_val))
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
    // Handle List input: return a record with basic numeric stats
    if let Value::List(items) = &args[0] {
        let nums: Vec<f64> = items.iter().filter_map(|v| to_f64(v)).collect();
        if nums.is_empty() {
            let mut m = std::collections::HashMap::new();
            m.insert("count".to_string(), Value::Int(items.len() as i64));
            m.insert("numeric".to_string(), Value::Int(0));
            return Ok(Value::Record(m));
        }
        let count = nums.len();
        let sum: f64 = nums.iter().sum();
        let mean_val = sum / count as f64;
        let mut sorted = nums.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_val = if count % 2 == 0 {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
        } else {
            sorted[count / 2]
        };
        let variance: f64 = nums.iter().map(|x| (x - mean_val).powi(2)).sum::<f64>() / (count as f64 - 1.0).max(1.0);
        let sd_val = variance.sqrt();
        let mut m = std::collections::HashMap::new();
        m.insert("count".to_string(), Value::Int(count as i64));
        m.insert("min".to_string(), Value::Float(sorted[0]));
        m.insert("max".to_string(), Value::Float(sorted[count - 1]));
        m.insert("mean".to_string(), Value::Float(mean_val));
        m.insert("median".to_string(), Value::Float(median_val));
        m.insert("sd".to_string(), Value::Float(sd_val));
        return Ok(Value::Record(m));
    }

    let table = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("summary() requires Table or List, got {}", other.type_of()),
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
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "scatter() received empty lists",
            None,
        ));
    }

    // Build a 2-column table and delegate to the SVG plot function
    let rows: Vec<Vec<Value>> = xs
        .iter()
        .zip(ys.iter())
        .map(|(x, y)| vec![Value::Float(*x), Value::Float(*y)])
        .collect();
    let table = Value::Table(bl_core::value::Table::new(
        vec!["x".into(), "y".into()],
        rows,
    ));
    crate::plot::call_plot_builtin("plot", vec![table])
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
        ("statistic", Value::Float(res.statistic)),
        ("p_value", Value::Float(res.p_value)),
        ("pvalue", Value::Float(res.p_value)),
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
        ("statistic", Value::Float(t)),
        ("p_value", Value::Float(p)),
        ("pvalue", Value::Float(p)),
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
        ("statistic", Value::Float(t)),
        ("p_value", Value::Float(p)),
        ("pvalue", Value::Float(p)),
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
        ("statistic", Value::Float(res.f_statistic)),
        ("p_value", Value::Float(res.p_value)),
        ("pvalue", Value::Float(res.p_value)),
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
        ("statistic", Value::Float(chi2)),
        ("p_value", Value::Float(p_value)),
        ("pvalue", Value::Float(p_value)),
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
        ("pvalue", Value::Float(res.p_value)),
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
        ("statistic", Value::Float(res.statistic)),
        ("p_value", Value::Float(res.p_value)),
        ("pvalue", Value::Float(res.p_value)),
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
                    ("pvalue", Value::Float(res.p_value)),
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
                ("pvalue", Value::Float(res.p_value)),
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
        ("p_value", Value::Float(res.p_value)),
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
        ("statistic", Value::Float(res.correlation)),
        ("pvalue", Value::Float(res.p_value)),
        ("p_value", Value::Float(res.p_value)),
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
        ("statistic", Value::Float(res.correlation)),
        ("pvalue", Value::Float(res.p_value)),
        ("p_value", Value::Float(res.p_value)),
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

fn str_to_phred(s: &str) -> Vec<u8> {
    s.bytes().map(|b| b.saturating_sub(33)).collect()
}

fn builtin_mean_phred(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Quality(scores) => Ok(Value::Float(bl_core::bio_core::QualityOps::mean_phred(scores))),
        Value::Str(s) => {
            let scores = str_to_phred(s);
            Ok(Value::Float(bl_core::bio_core::QualityOps::mean_phred(&scores)))
        }
        _ => Err(BioLangError::type_error("mean_phred() requires Quality or Str value", None)),
    }
}

fn builtin_min_phred(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Quality(scores) => Ok(Value::Int(bl_core::bio_core::QualityOps::min_phred(scores) as i64)),
        Value::Str(s) => {
            let scores = str_to_phred(s);
            Ok(Value::Int(bl_core::bio_core::QualityOps::min_phred(&scores) as i64))
        }
        _ => Err(BioLangError::type_error("min_phred() requires Quality or Str value", None)),
    }
}

fn builtin_error_rate(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Quality(scores) => Ok(Value::Float(bl_core::bio_core::QualityOps::error_rate(scores))),
        Value::Str(s) => {
            let scores = str_to_phred(s);
            Ok(Value::Float(bl_core::bio_core::QualityOps::error_rate(&scores)))
        }
        _ => Err(BioLangError::type_error("error_rate() requires Quality or Str value", None)),
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

// ── GLM (Generalized Linear Model) ─────────────────────────────────────────

fn builtin_glm(args: Vec<Value>) -> Result<Value> {
    // glm(formula, table) or glm(formula, table, family)
    let family = if args.len() >= 3 {
        match &args[2] {
            Value::Str(s) => s.as_str().to_string(),
            Value::Record(map) => {
                map.get("family")
                    .and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_else(|| "binomial".to_string())
            }
            _ => "binomial".to_string(),
        }
    } else {
        "binomial".to_string()
    };

    let (formula_expr, table) = match (&args[0], &args[1]) {
        (Value::Formula(f), Value::Table(t)) => (f, t),
        _ => return Err(BioLangError::type_error(
            format!("glm() requires (formula, Table), got ({}, {})", args[0].type_of(), args[1].type_of()),
            None,
        )),
    };

    let (response_col, predictor_cols) = parse_formula_columns(formula_expr, table)?;
    let y = extract_column_floats(table, &response_col, "glm")?;

    match family.as_str() {
        "binomial" | "logistic" => {
            if predictor_cols.len() == 1 {
                // Simple logistic regression
                let x = extract_column_floats(table, &predictor_cols[0], "glm")?;
                let res = bl_core::bio_core::stats_ops::logistic_regression(&x, &y)
                    .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;

                Ok(make_record(vec![
                    ("coefficients", Value::List(res.coefficients.into_iter().map(Value::Float).collect())),
                    ("p_values", Value::List(res.p_values.into_iter().map(Value::Float).collect())),
                    ("log_likelihood", Value::Float(res.log_likelihood)),
                    ("aic", Value::Float(res.aic)),
                    ("family", Value::Str("binomial".into())),
                ]))
            } else {
                // Multi-predictor: use iteratively reweighted least squares
                let x_matrix: Vec<Vec<f64>> = (0..table.num_rows())
                    .map(|i| {
                        predictor_cols.iter().map(|col| {
                            let ci = table.col_index(col).unwrap();
                            to_f64(&table.rows[i][ci]).unwrap_or(0.0)
                        }).collect()
                    })
                    .collect();

                let res = logistic_regression_multi(&y, &x_matrix)
                    .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;

                Ok(make_record(vec![
                    ("coefficients", Value::List(res.0.into_iter().map(Value::Float).collect())),
                    ("p_values", Value::List(res.1.into_iter().map(Value::Float).collect())),
                    ("log_likelihood", Value::Float(res.2)),
                    ("aic", Value::Float(res.3)),
                    ("family", Value::Str("binomial".into())),
                ]))
            }
        }
        "gaussian" | "linear" => {
            // Falls back to lm()
            builtin_lm(args[..2].to_vec())
        }
        "poisson" => {
            // Simple Poisson regression via IRLS
            let x_matrix: Vec<Vec<f64>> = (0..table.num_rows())
                .map(|i| {
                    predictor_cols.iter().map(|col| {
                        let ci = table.col_index(col).unwrap();
                        to_f64(&table.rows[i][ci]).unwrap_or(0.0)
                    }).collect()
                })
                .collect();

            let res = poisson_regression(&y, &x_matrix)
                .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;

            Ok(make_record(vec![
                ("coefficients", Value::List(res.0.into_iter().map(Value::Float).collect())),
                ("p_values", Value::List(res.1.into_iter().map(Value::Float).collect())),
                ("deviance", Value::Float(res.2)),
                ("aic", Value::Float(res.3)),
                ("family", Value::Str("poisson".into())),
            ]))
        }
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("glm() unknown family '{family}'. Supported: binomial, gaussian, poisson"),
            None,
        )),
    }
}

/// Multi-predictor logistic regression via IRLS.
/// Returns (coefficients, p_values, log_likelihood, aic).
fn logistic_regression_multi(y: &[f64], x: &[Vec<f64>]) -> std::result::Result<(Vec<f64>, Vec<f64>, f64, f64), String> {
    let n = y.len();
    if n < 2 || x.is_empty() || x[0].is_empty() { return Err("insufficient data".into()); }
    let p = x[0].len() + 1; // +1 for intercept

    // Build design matrix with intercept
    let mut beta = vec![0.0; p];
    let max_iter = 50;

    for _ in 0..max_iter {
        let mut mu: Vec<f64> = Vec::with_capacity(n);
        for i in 0..n {
            let eta = beta[0] + x[i].iter().zip(&beta[1..]).map(|(xi, bi)| xi * bi).sum::<f64>();
            mu.push(1.0 / (1.0 + (-eta).exp()));
        }

        // Weights W = mu * (1 - mu)
        let w: Vec<f64> = mu.iter().map(|&m| {
            let v = m * (1.0 - m);
            if v < 1e-10 { 1e-10 } else { v }
        }).collect();

        // Working response z = eta + (y - mu) / w
        let z: Vec<f64> = (0..n).map(|i| {
            let eta = beta[0] + x[i].iter().zip(&beta[1..]).map(|(xi, bi)| xi * bi).sum::<f64>();
            eta + (y[i] - mu[i]) / w[i]
        }).collect();

        // Weighted least squares: (X'WX)^-1 X'Wz
        let mut xtwx = vec![vec![0.0; p]; p];
        let mut xtwz = vec![0.0; p];

        for i in 0..n {
            let xi = std::iter::once(1.0).chain(x[i].iter().copied()).collect::<Vec<_>>();
            for j in 0..p {
                xtwz[j] += xi[j] * w[i] * z[i];
                for k in 0..p {
                    xtwx[j][k] += xi[j] * w[i] * xi[k];
                }
            }
        }

        let new_beta = solve_linear_system(&xtwx, &xtwz).ok_or("singular matrix in GLM")?;
        let max_change: f64 = beta.iter().zip(&new_beta).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
        beta = new_beta;
        if max_change < 1e-8 { break; }
    }

    // Log-likelihood
    let ll: f64 = (0..n).map(|i| {
        let eta = beta[0] + x[i].iter().zip(&beta[1..]).map(|(xi, bi)| xi * bi).sum::<f64>();
        let mu = 1.0 / (1.0 + (-eta).exp());
        let mu = mu.clamp(1e-15, 1.0 - 1e-15);
        y[i] * mu.ln() + (1.0 - y[i]) * (1.0 - mu).ln()
    }).sum();

    let aic = -2.0 * ll + 2.0 * p as f64;

    // Approximate p-values from Wald test (using diagonal of (X'WX)^-1)
    let mut mu: Vec<f64> = Vec::with_capacity(n);
    for i in 0..n {
        let eta = beta[0] + x[i].iter().zip(&beta[1..]).map(|(xi, bi)| xi * bi).sum::<f64>();
        mu.push(1.0 / (1.0 + (-eta).exp()));
    }
    let w: Vec<f64> = mu.iter().map(|&m| { let v = m * (1.0 - m); if v < 1e-10 { 1e-10 } else { v } }).collect();
    let mut xtwx = vec![vec![0.0; p]; p];
    for i in 0..n {
        let xi = std::iter::once(1.0).chain(x[i].iter().copied()).collect::<Vec<_>>();
        for j in 0..p {
            for k in 0..p {
                xtwx[j][k] += xi[j] * w[i] * xi[k];
            }
        }
    }
    let inv = invert_matrix(&xtwx).unwrap_or_else(|| vec![vec![0.0; p]; p]);
    let p_values: Vec<f64> = (0..p).map(|j| {
        let se = inv[j][j].abs().sqrt();
        if se > 0.0 {
            let z = beta[j] / se;
            2.0 * (1.0 - bl_core::bio_core::stats_ops::normal_cdf(z.abs()))
        } else { 1.0 }
    }).collect();

    Ok((beta, p_values, ll, aic))
}

/// Poisson regression via IRLS.
/// Returns (coefficients, p_values, deviance, aic).
fn poisson_regression(y: &[f64], x: &[Vec<f64>]) -> std::result::Result<(Vec<f64>, Vec<f64>, f64, f64), String> {
    let n = y.len();
    if n < 2 || x.is_empty() || x[0].is_empty() { return Err("insufficient data".into()); }
    let p = x[0].len() + 1;

    let mut beta = vec![0.0; p];
    beta[0] = y.iter().map(|&yi| (yi.max(0.01)).ln()).sum::<f64>() / n as f64;

    for _ in 0..50 {
        let mu: Vec<f64> = (0..n).map(|i| {
            let eta = beta[0] + x[i].iter().zip(&beta[1..]).map(|(xi, bi)| xi * bi).sum::<f64>();
            eta.exp().min(1e10)
        }).collect();

        let z: Vec<f64> = (0..n).map(|i| {
            let eta = beta[0] + x[i].iter().zip(&beta[1..]).map(|(xi, bi)| xi * bi).sum::<f64>();
            eta + (y[i] - mu[i]) / mu[i].max(1e-10)
        }).collect();

        let mut xtwx = vec![vec![0.0; p]; p];
        let mut xtwz = vec![0.0; p];
        for i in 0..n {
            let xi = std::iter::once(1.0).chain(x[i].iter().copied()).collect::<Vec<_>>();
            let w = mu[i].max(1e-10);
            for j in 0..p {
                xtwz[j] += xi[j] * w * z[i];
                for k in 0..p {
                    xtwx[j][k] += xi[j] * w * xi[k];
                }
            }
        }

        let new_beta = solve_linear_system(&xtwx, &xtwz).ok_or("singular matrix in Poisson GLM")?;
        let max_change: f64 = beta.iter().zip(&new_beta).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
        beta = new_beta;
        if max_change < 1e-8 { break; }
    }

    // Deviance
    let deviance: f64 = (0..n).map(|i| {
        let eta = beta[0] + x[i].iter().zip(&beta[1..]).map(|(xi, bi)| xi * bi).sum::<f64>();
        let mu = eta.exp().max(1e-15);
        if y[i] > 0.0 { 2.0 * (y[i] * (y[i] / mu).ln() - (y[i] - mu)) } else { 2.0 * mu }
    }).sum();

    // Log-likelihood for AIC
    let ll: f64 = (0..n).map(|i| {
        let eta = beta[0] + x[i].iter().zip(&beta[1..]).map(|(xi, bi)| xi * bi).sum::<f64>();
        let mu = eta.exp().max(1e-15);
        y[i] * mu.ln() - mu
    }).sum();
    let aic = -2.0 * ll + 2.0 * p as f64;

    // P-values
    let mu: Vec<f64> = (0..n).map(|i| {
        let eta = beta[0] + x[i].iter().zip(&beta[1..]).map(|(xi, bi)| xi * bi).sum::<f64>();
        eta.exp().max(1e-10)
    }).collect();
    let mut xtwx = vec![vec![0.0; p]; p];
    for i in 0..n {
        let xi = std::iter::once(1.0).chain(x[i].iter().copied()).collect::<Vec<_>>();
        for j in 0..p {
            for k in 0..p { xtwx[j][k] += xi[j] * mu[i] * xi[k]; }
        }
    }
    let inv = invert_matrix(&xtwx).unwrap_or_else(|| vec![vec![0.0; p]; p]);
    let p_values: Vec<f64> = (0..p).map(|j| {
        let se = inv[j][j].abs().sqrt();
        if se > 0.0 {
            let z = beta[j] / se;
            2.0 * (1.0 - bl_core::bio_core::stats_ops::normal_cdf(z.abs()))
        } else { 1.0 }
    }).collect();

    Ok((beta, p_values, deviance, aic))
}

/// Solve Ax = b via Gaussian elimination with partial pivoting.
fn solve_linear_system(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = a.len();
    let mut aug: Vec<Vec<f64>> = a.iter().enumerate().map(|(i, row)| {
        let mut r = row.clone();
        r.push(b[i]);
        r
    }).collect();

    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j|
            aug[i][col].abs().partial_cmp(&aug[j][col].abs()).unwrap_or(std::cmp::Ordering::Equal)
        )?;
        aug.swap(col, pivot);
        if aug[col][col].abs() < 1e-15 { return None; }
        let div = aug[col][col];
        for j in col..=n { aug[col][j] /= div; }
        for i in 0..n {
            if i == col { continue; }
            let factor = aug[i][col];
            for j in col..=n { aug[i][j] -= factor * aug[col][j]; }
        }
    }

    Some(aug.iter().map(|row| row[n]).collect())
}

/// Invert a square matrix via Gauss-Jordan elimination.
fn invert_matrix(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut aug: Vec<Vec<f64>> = (0..n).map(|i| {
        let mut row = a[i].clone();
        row.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
        row
    }).collect();

    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j|
            aug[i][col].abs().partial_cmp(&aug[j][col].abs()).unwrap_or(std::cmp::Ordering::Equal)
        )?;
        aug.swap(col, pivot);
        if aug[col][col].abs() < 1e-15 { return None; }
        let div = aug[col][col];
        for j in 0..2*n { aug[col][j] /= div; }
        for i in 0..n {
            if i == col { continue; }
            let factor = aug[i][col];
            for j in 0..2*n { aug[i][j] -= factor * aug[col][j]; }
        }
    }

    Some(aug.iter().map(|row| row[n..].to_vec()).collect())
}

// ── K-Means Builtin ─────────────────────────────────────────────────────────

fn builtin_kmeans(args: Vec<Value>) -> Result<Value> {
    let k = require_int(&args[1], "kmeans")? as usize;
    let max_iter = if args.len() >= 3 { require_int(&args[2], "kmeans")? as usize } else { 100 };

    // Accept Table or Matrix
    let data: Vec<Vec<f64>> = match &args[0] {
        Value::Table(t) => {
            // Use all numeric columns
            let numeric_cols: Vec<usize> = (0..t.columns.len())
                .filter(|&ci| t.rows.iter().any(|r| matches!(&r[ci], Value::Int(_) | Value::Float(_))))
                .collect();
            if numeric_cols.is_empty() {
                return Err(BioLangError::type_error("kmeans() table has no numeric columns", None));
            }
            t.rows.iter().map(|row| {
                numeric_cols.iter().map(|&ci| to_f64(&row[ci]).unwrap_or(0.0)).collect()
            }).collect()
        }
        Value::Matrix(m) => {
            (0..m.nrow).map(|i| m.row(i)).collect()
        }
        Value::List(items) => {
            // List of lists
            items.iter().map(|v| {
                match v {
                    Value::List(inner) => inner.iter().map(|x| to_f64(x).unwrap_or(0.0)).collect(),
                    _ => vec![to_f64(v).unwrap_or(0.0)],
                }
            }).collect()
        }
        other => return Err(BioLangError::type_error(
            format!("kmeans() requires Table, Matrix, or List, got {}", other.type_of()), None)),
    };

    let res = bl_core::bio_core::stats_ops::kmeans(&data, k, max_iter)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;

    let centroid_cols: Vec<String> = (0..res.centroids.first().map(|c| c.len()).unwrap_or(0))
        .map(|i| format!("dim_{}", i + 1))
        .collect();
    let centroid_rows: Vec<Vec<Value>> = res.centroids.iter()
        .map(|c| c.iter().map(|&v| Value::Float(v)).collect())
        .collect();

    Ok(make_record(vec![
        ("clusters", Value::List(res.clusters.into_iter().map(|c| Value::Int(c as i64)).collect())),
        ("centroids", Value::Table(bl_core::value::Table::new(centroid_cols, centroid_rows))),
        ("iterations", Value::Int(res.iterations as i64)),
        ("inertia", Value::Float(res.inertia)),
    ]))
}

// ── Distribution Functions ──────────────────────────────────────────────────

fn opt_float(args: &[Value], idx: usize, default: f64) -> f64 {
    if idx < args.len() { to_f64(&args[idx]).unwrap_or(default) } else { default }
}

fn builtin_dnorm(args: Vec<Value>) -> Result<Value> {
    let x = require_num(&args[0], "dnorm")?;
    let mean = opt_float(&args, 1, 0.0);
    let sd = opt_float(&args, 2, 1.0);
    if sd <= 0.0 { return Err(BioLangError::type_error("dnorm() sd must be positive", None)); }
    let z = (x - mean) / sd;
    Ok(Value::Float(bl_core::bio_core::stats_ops::normal_pdf(z) / sd))
}

fn builtin_pnorm(args: Vec<Value>) -> Result<Value> {
    let x = require_num(&args[0], "pnorm")?;
    let mean = opt_float(&args, 1, 0.0);
    let sd = opt_float(&args, 2, 1.0);
    if sd <= 0.0 { return Err(BioLangError::type_error("pnorm() sd must be positive", None)); }
    let z = (x - mean) / sd;
    Ok(Value::Float(bl_core::bio_core::stats_ops::normal_cdf(z)))
}

fn builtin_qnorm(args: Vec<Value>) -> Result<Value> {
    let p = require_num(&args[0], "qnorm")?;
    let mean = opt_float(&args, 1, 0.0);
    let sd = opt_float(&args, 2, 1.0);
    if sd <= 0.0 { return Err(BioLangError::type_error("qnorm() sd must be positive", None)); }
    if p <= 0.0 || p >= 1.0 {
        return Err(BioLangError::type_error("qnorm() p must be in (0, 1)", None));
    }
    Ok(Value::Float(bl_core::bio_core::stats_ops::normal_quantile(p) * sd + mean))
}

fn builtin_dbinom(args: Vec<Value>) -> Result<Value> {
    let k = require_int(&args[0], "dbinom")? as u64;
    let n = require_int(&args[1], "dbinom")? as u64;
    let p = require_num(&args[2], "dbinom")?;
    Ok(Value::Float(bl_core::bio_core::stats_ops::binomial_pmf(k, n, p)))
}

fn builtin_pbinom(args: Vec<Value>) -> Result<Value> {
    let k = require_int(&args[0], "pbinom")? as u64;
    let n = require_int(&args[1], "pbinom")? as u64;
    let p = require_num(&args[2], "pbinom")?;
    Ok(Value::Float(bl_core::bio_core::stats_ops::binom_cdf(k, n, p)))
}

fn builtin_dpois(args: Vec<Value>) -> Result<Value> {
    let k = require_int(&args[0], "dpois")? as u64;
    let lambda = require_num(&args[1], "dpois")?;
    Ok(Value::Float(bl_core::bio_core::stats_ops::poisson_pmf_exact(k, lambda)))
}

fn builtin_ppois(args: Vec<Value>) -> Result<Value> {
    let k = require_int(&args[0], "ppois")? as u64;
    let lambda = require_num(&args[1], "ppois")?;
    Ok(Value::Float(bl_core::bio_core::stats_ops::poisson_cdf_exact(k, lambda)))
}

fn builtin_dunif(args: Vec<Value>) -> Result<Value> {
    let x = require_num(&args[0], "dunif")?;
    let a = opt_float(&args, 1, 0.0);
    let b = opt_float(&args, 2, 1.0);
    Ok(Value::Float(bl_core::bio_core::stats_ops::uniform_pdf(x, a, b)))
}

fn builtin_punif(args: Vec<Value>) -> Result<Value> {
    let x = require_num(&args[0], "punif")?;
    let a = opt_float(&args, 1, 0.0);
    let b = opt_float(&args, 2, 1.0);
    Ok(Value::Float(bl_core::bio_core::stats_ops::uniform_cdf(x, a, b)))
}

fn builtin_dexp(args: Vec<Value>) -> Result<Value> {
    let x = require_num(&args[0], "dexp")?;
    let rate = opt_float(&args, 1, 1.0);
    Ok(Value::Float(bl_core::bio_core::stats_ops::exponential_pdf(x, rate)))
}

fn builtin_pexp(args: Vec<Value>) -> Result<Value> {
    let x = require_num(&args[0], "pexp")?;
    let rate = opt_float(&args, 1, 1.0);
    Ok(Value::Float(bl_core::bio_core::stats_ops::exponential_cdf(x, rate)))
}

// ── Random Variate Generators ───────────────────────────────────────────────

fn builtin_rnorm(args: Vec<Value>) -> Result<Value> {
    let n = require_int(&args[0], "rnorm")? as usize;
    let mean = opt_float(&args, 1, 0.0);
    let sd = opt_float(&args, 2, 1.0);
    if sd <= 0.0 { return Err(BioLangError::type_error("rnorm() sd must be positive", None)); }
    // Box-Muller transform using the existing xorshift64 RNG
    let mut result = Vec::with_capacity(n);
    for _ in 0..n {
        let u1: f64 = xorshift_f64();
        let u2: f64 = xorshift_f64();
        let z = (-2.0 * u1.max(1e-15).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        result.push(Value::Float(z * sd + mean));
    }
    Ok(Value::List(result))
}

fn builtin_rbinom(args: Vec<Value>) -> Result<Value> {
    let n = require_int(&args[0], "rbinom")? as usize;
    let trials = require_int(&args[1], "rbinom")? as usize;
    let p = require_num(&args[2], "rbinom")?;
    let mut result = Vec::with_capacity(n);
    for _ in 0..n {
        let mut successes = 0i64;
        for _ in 0..trials {
            if xorshift_f64() < p { successes += 1; }
        }
        result.push(Value::Int(successes));
    }
    Ok(Value::List(result))
}

fn builtin_rpois(args: Vec<Value>) -> Result<Value> {
    let n = require_int(&args[0], "rpois")? as usize;
    let lambda = require_num(&args[1], "rpois")?;
    // Knuth's algorithm for Poisson random variates
    let mut result = Vec::with_capacity(n);
    let l = (-lambda).exp();
    for _ in 0..n {
        let mut k = 0i64;
        let mut p_val = 1.0;
        loop {
            k += 1;
            p_val *= xorshift_f64();
            if p_val <= l { break; }
        }
        result.push(Value::Int(k - 1));
    }
    Ok(Value::List(result))
}

/// Thread-local xorshift64 PRNG state, accessible by set_seed().
thread_local! {
    static XORSHIFT_STATE: std::cell::Cell<u64> = const { std::cell::Cell::new(0x12345678_9abcdef0) };
}

/// Thread-local xorshift64 PRNG (same as used in random() builtin).
fn xorshift_f64() -> f64 {
    XORSHIFT_STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 11) as f64 / (1u64 << 53) as f64
    })
}

/// set_seed(n) — set the PRNG state for reproducible random sequences.
fn builtin_set_seed(args: Vec<Value>) -> Result<Value> {
    let seed = require_int(&args[0], "set_seed")? as u64;
    // Mix the seed through splitmix64 to avoid bad initial states
    let mut s = seed;
    s = s.wrapping_add(0x9E3779B97F4A7C15);
    s = (s ^ (s >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    s = (s ^ (s >> 27)).wrapping_mul(0x94D049BB133111EB);
    s ^= s >> 31;
    if s == 0 { s = 1; } // xorshift must not be zero
    XORSHIFT_STATE.with(|state| state.set(s));
    Ok(Value::Nil)
}

/// power_t_test(effect_size, alpha?, power?) — compute required sample size per group.
/// Returns a record with {n, effect_size, alpha, power}.
/// Default alpha=0.05, power=0.80.
fn builtin_power_t_test(args: Vec<Value>) -> Result<Value> {
    let d = require_num(&args[0], "power_t_test")?;
    let alpha = opt_float(&args, 1, 0.05);
    let power = opt_float(&args, 2, 0.80);

    if d <= 0.0 {
        return Err(BioLangError::type_error("power_t_test() effect_size must be positive", None));
    }
    if alpha <= 0.0 || alpha >= 1.0 {
        return Err(BioLangError::type_error("power_t_test() alpha must be in (0, 1)", None));
    }
    if power <= 0.0 || power >= 1.0 {
        return Err(BioLangError::type_error("power_t_test() power must be in (0, 1)", None));
    }

    // Use the formula: n = ((z_alpha/2 + z_beta) / d)^2
    // where z_alpha/2 and z_beta are normal quantiles
    let z_alpha = bl_core::bio_core::stats_ops::normal_quantile(1.0 - alpha / 2.0);
    let z_beta = bl_core::bio_core::stats_ops::normal_quantile(power);
    let n_raw = ((z_alpha + z_beta) / d).powi(2);
    let n = n_raw.ceil() as i64;

    let mut m = std::collections::HashMap::new();
    m.insert("n".to_string(), Value::Int(n));
    m.insert("effect_size".to_string(), Value::Float(d));
    m.insert("alpha".to_string(), Value::Float(alpha));
    m.insert("power".to_string(), Value::Float(power));
    Ok(Value::Record(m))
}

