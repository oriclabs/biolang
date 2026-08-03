//! Hidden Markov model builtins.
//!
//! Functions: viterbi, hmm_likelihood, hmm_posterior, hmm_path_probability,
//!            hmm_emission_probability, hmm_estimate, hmm_viterbi_learning,
//!            hmm_baum_welch, hmm_profile, hmm_profile_align.
//!
//! A model is an ordinary record, so it can be written as a literal, read from
//! JSON, or built up in a loop without any special syntax:
//!
//! ```text
//! let model = {
//!     states: ["A", "B"],
//!     symbols: ["x", "y", "z"],
//!     transition: { A: { A: 0.641, B: 0.359 }, B: { A: 0.729, B: 0.271 } },
//!     emission:   { A: { x: 0.117, y: 0.691, z: 0.192 },
//!                   B: { x: 0.097, y: 0.42,  z: 0.483 } },
//! }
//! viterbi("xyxzzxyxyy", model)   # ["A", "A", "A", "B", "B", ...]
//! ```
//!
//! The matrices are keyed by name rather than positional, because a transposed
//! transition matrix is otherwise a silent wrong answer rather than an error —
//! and it is the mistake everyone makes first.
//!
//! `initial` is optional and defaults to uniform, which is what Rosalind's
//! formulations assume.

use bl_core::bio_core::hmm::Hmm;
use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;
use std::sync::Arc;

// ── Registry ──────────────────────────────────────────────────────────

pub fn hmm_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("viterbi", Arity::Exact(2)),
        ("hmm_likelihood", Arity::Exact(2)),
        ("hmm_posterior", Arity::Exact(2)),
        ("hmm_path_probability", Arity::Exact(2)),
        ("hmm_emission_probability", Arity::Exact(3)),
        ("hmm_estimate", Arity::Exact(3)),
        ("hmm_viterbi_learning", Arity::Exact(3)),
        ("hmm_baum_welch", Arity::Exact(3)),
        ("hmm_profile", Arity::Range(3, 4)),
        ("hmm_profile_align", Arity::Exact(2)),
    ]
}

pub fn is_hmm_builtin(name: &str) -> bool {
    matches!(
        name,
        "viterbi"
            | "hmm_likelihood"
            | "hmm_posterior"
            | "hmm_path_probability"
            | "hmm_emission_probability"
            | "hmm_estimate"
            | "hmm_viterbi_learning"
            | "hmm_baum_welch"
            | "hmm_profile"
            | "hmm_profile_align"
    )
}

pub fn call_hmm_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "viterbi" => builtin_viterbi(args),
        "hmm_likelihood" => builtin_hmm_likelihood(args),
        "hmm_posterior" => builtin_hmm_posterior(args),
        "hmm_path_probability" => builtin_hmm_path_probability(args),
        "hmm_emission_probability" => builtin_hmm_emission_probability(args),
        "hmm_estimate" => builtin_hmm_estimate(args),
        "hmm_viterbi_learning" => builtin_hmm_viterbi_learning(args),
        "hmm_baum_welch" => builtin_hmm_baum_welch(args),
        "hmm_profile" => builtin_hmm_profile(args),
        "hmm_profile_align" => builtin_hmm_profile_align(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown HMM builtin '{name}'"),
            None,
        )),
    }
}

// ── Reading a model out of a record ───────────────────────────────────

fn type_error(message: impl Into<String>) -> BioLangError {
    BioLangError::type_error(message.into(), None)
}

/// A well-formed value that says the wrong thing — a missing matrix row, a
/// symbol outside the alphabet. `ErrorKind` has no `ValueError`, and the rest of
/// the runtime reports these as type errors, so this one does too rather than
/// widening the enum for one module.
fn value_error(message: impl Into<String>) -> BioLangError {
    BioLangError::type_error(message.into(), None)
}

fn as_fields<'a>(value: &'a Value, what: &str) -> Result<&'a HashMap<String, Value>> {
    match value {
        Value::Map(fields) | Value::Record(fields) => Ok(fields),
        other => Err(type_error(format!(
            "{what} must be a record or map, got {}",
            other.type_of()
        ))),
    }
}

fn as_number(value: &Value, what: &str) -> Result<f64> {
    match value {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        other => Err(type_error(format!(
            "{what} must be a number, got {}",
            other.type_of()
        ))),
    }
}

/// The labels in `states` / `symbols`, which fix the order everything else uses.
fn as_labels(value: &Value, what: &str) -> Result<Vec<String>> {
    let items = match value {
        Value::List(items) => items,
        other => {
            return Err(type_error(format!(
                "{what} must be a list of names, got {}",
                other.type_of()
            )));
        }
    };
    items
        .iter()
        .map(|item| match item {
            Value::Str(s) => Ok(s.clone()),
            other => Err(type_error(format!(
                "{what} must contain names, got {}",
                other.type_of()
            ))),
        })
        .collect()
}

/// Read a `rows × columns` matrix keyed by name in both directions.
///
/// Every entry is required. A missing one is far more often a typo in a state
/// name than a deliberate zero, and reporting it beats silently scoring that
/// transition as impossible.
fn as_matrix(
    value: &Value,
    rows: &[String],
    columns: &[String],
    what: &str,
) -> Result<Vec<Vec<f64>>> {
    let outer = as_fields(value, what)?;
    rows.iter()
        .map(|row| {
            let inner_value = outer
                .get(row)
                .ok_or_else(|| value_error(format!("{what} has no row for '{row}'")))?;
            let inner = as_fields(inner_value, &format!("{what}['{row}']"))?;
            columns
                .iter()
                .map(|column| {
                    inner.get(column).map_or_else(
                        || {
                            Err(value_error(format!(
                                "{what}['{row}'] has no entry for '{column}'"
                            )))
                        },
                        |entry| as_number(entry, &format!("{what}['{row}']['{column}']")),
                    )
                })
                .collect()
        })
        .collect()
}

/// Build a model from a record, given which parts this caller actually needs.
///
/// BA10A asks for the probability of a path with no emissions in sight, and
/// BA10B for the reverse, so requiring both matrices every time would force
/// callers to invent one.
fn read_model(value: &Value, need_transition: bool, need_emission: bool) -> Result<Hmm> {
    let fields = as_fields(value, "the model")?;

    let states = fields
        .get("states")
        .ok_or_else(|| value_error("the model has no `states`"))
        .and_then(|v| as_labels(v, "`states`"))?;
    if states.is_empty() {
        return Err(value_error("the model has no states"));
    }

    let symbols = match fields.get("symbols") {
        Some(v) => as_labels(v, "`symbols`")?,
        None if need_emission => {
            return Err(value_error("the model has no `symbols`"));
        }
        None => Vec::new(),
    };

    let transition = match fields.get("transition") {
        Some(v) => as_matrix(v, &states, &states, "`transition`")?,
        None if need_transition => {
            return Err(value_error("the model has no `transition`"));
        }
        None => Vec::new(),
    };

    let emission = match fields.get("emission") {
        Some(v) => as_matrix(v, &states, &symbols, "`emission`")?,
        None if need_emission => {
            return Err(value_error("the model has no `emission`"));
        }
        None => Vec::new(),
    };

    let mut model = Hmm::with_uniform_start(states, symbols, transition, emission);
    if let Some(start) = fields.get("initial") {
        let given = as_fields(start, "`initial`")?;
        model.initial = model
            .states
            .iter()
            .map(|state| {
                given.get(state).map_or_else(
                    || Err(value_error(format!("`initial` has no entry for '{state}'"))),
                    |v| as_number(v, &format!("`initial`['{state}']")),
                )
            })
            .collect::<Result<Vec<f64>>>()?;
    }
    Ok(model)
}

/// Turn an observed string or list into indices into `model.symbols`.
fn encode(value: &Value, labels: &[String], what: &str) -> Result<Vec<usize>> {
    let pieces: Vec<String> = match value {
        Value::Str(text) => text.chars().map(|c| c.to_string()).collect(),
        Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => {
            seq.data.chars().map(|c| c.to_string()).collect()
        }
        Value::List(items) => items
            .iter()
            .map(|item| match item {
                Value::Str(s) => Ok(s.clone()),
                other => Err(type_error(format!(
                    "{what} must contain names, got {}",
                    other.type_of()
                ))),
            })
            .collect::<Result<Vec<String>>>()?,
        other => {
            return Err(type_error(format!(
                "{what} must be a string or a list, got {}",
                other.type_of()
            )));
        }
    };

    pieces
        .iter()
        .map(|piece| {
            labels
                .iter()
                .position(|label| label == piece)
                .ok_or_else(|| {
                    value_error(format!(
                        "{what} contains '{piece}', which is not one of {labels:?}"
                    ))
                })
        })
        .collect()
}

fn to_state_list(path: &[usize], states: &[String]) -> Value {
    Value::List(Arc::new(
        path.iter()
            .map(|&index| Value::Str(states[index].clone()))
            .collect(),
    ))
}

/// A `rows × columns` matrix back as nested records keyed by name.
fn matrix_to_record(rows: &[Vec<f64>], row_names: &[String], column_names: &[String]) -> Value {
    let fields: HashMap<String, Value> = row_names
        .iter()
        .zip(rows)
        .map(|(name, row)| {
            let inner: HashMap<String, Value> = column_names
                .iter()
                .cloned()
                .zip(row.iter().map(|&v| Value::Float(v)))
                .collect();
            (name.clone(), Value::Record(Arc::new(inner)))
        })
        .collect();
    Value::Record(Arc::new(fields))
}

/// A learned model, in the same shape the builtins accept — so the result of
/// learning can be fed straight back into `viterbi` or another round.
fn model_to_record(model: &Hmm) -> Value {
    let states = Value::List(Arc::new(
        model
            .states
            .iter()
            .cloned()
            .map(Value::Str)
            .collect::<Vec<_>>(),
    ));
    let symbols = Value::List(Arc::new(
        model
            .symbols
            .iter()
            .cloned()
            .map(Value::Str)
            .collect::<Vec<_>>(),
    ));
    let initial: HashMap<String, Value> = model
        .states
        .iter()
        .cloned()
        .zip(model.initial.iter().map(|&v| Value::Float(v)))
        .collect();

    let fields: HashMap<String, Value> = [
        ("states".to_string(), states),
        ("symbols".to_string(), symbols),
        ("initial".to_string(), Value::Record(Arc::new(initial))),
        (
            "transition".to_string(),
            matrix_to_record(&model.transition, &model.states, &model.states),
        ),
        (
            "emission".to_string(),
            matrix_to_record(&model.emission, &model.states, &model.symbols),
        ),
    ]
    .into_iter()
    .collect();
    Value::Record(Arc::new(fields))
}

fn require_count(value: &Value, what: &str) -> Result<usize> {
    match value {
        Value::Int(n) if *n >= 0 => Ok(*n as usize),
        Value::Int(n) => Err(value_error(format!("{what} cannot be negative, got {n}"))),
        other => Err(type_error(format!(
            "{what} must be an integer, got {}",
            other.type_of()
        ))),
    }
}

// ── Builtins ──────────────────────────────────────────────────────────

/// `viterbi(observations, model)` — the most likely hidden path, as a list of
/// state names.
fn builtin_viterbi(args: Vec<Value>) -> Result<Value> {
    let model = read_model(&args[1], true, true)?;
    let observations = encode(&args[0], &model.symbols, "the observations")?;
    Ok(to_state_list(&model.viterbi(&observations), &model.states))
}

/// `hmm_likelihood(observations, model)` — `P(observations)`, summed over every
/// hidden path.
fn builtin_hmm_likelihood(args: Vec<Value>) -> Result<Value> {
    let model = read_model(&args[1], true, true)?;
    let observations = encode(&args[0], &model.symbols, "the observations")?;
    Ok(Value::Float(model.likelihood(&observations)))
}

/// `hmm_posterior(observations, model)` — per position, the probability of each
/// state given the whole observation.
fn builtin_hmm_posterior(args: Vec<Value>) -> Result<Value> {
    let model = read_model(&args[1], true, true)?;
    let observations = encode(&args[0], &model.symbols, "the observations")?;
    let rows = model
        .posterior(&observations)
        .into_iter()
        .map(|row| {
            let fields: HashMap<String, Value> = model
                .states
                .iter()
                .cloned()
                .zip(row.into_iter().map(Value::Float))
                .collect();
            Value::Record(Arc::new(fields))
        })
        .collect();
    Ok(Value::List(Arc::new(rows)))
}

/// `hmm_path_probability(path, model)` — `P(path)`, ignoring emissions.
fn builtin_hmm_path_probability(args: Vec<Value>) -> Result<Value> {
    let model = read_model(&args[1], true, false)?;
    let path = encode(&args[0], &model.states, "the path")?;
    Ok(Value::Float(model.path_probability(&path)))
}

/// `hmm_emission_probability(observations, path, model)` — `P(observations | path)`.
fn builtin_hmm_emission_probability(args: Vec<Value>) -> Result<Value> {
    let model = read_model(&args[2], false, true)?;
    let observations = encode(&args[0], &model.symbols, "the observations")?;
    let path = encode(&args[1], &model.states, "the path")?;
    model
        .emission_probability(&observations, &path)
        .map(Value::Float)
        .ok_or_else(|| {
            value_error(format!(
                "the observations are {} long but the path is {} — they have to match",
                observations.len(),
                path.len()
            ))
        })
}

/// `hmm_estimate(observations, path, model)` — the model that best explains an
/// observation together with the path that produced it.
///
/// `model` supplies only `states` and `symbols`; the matrices are what comes
/// back, so a skeleton is enough to ask with.
fn builtin_hmm_estimate(args: Vec<Value>) -> Result<Value> {
    let skeleton = read_model(&args[2], false, false)?;
    let observations = encode(&args[0], &skeleton.symbols, "the observations")?;
    let path = encode(&args[1], &skeleton.states, "the path")?;
    if observations.len() != path.len() {
        return Err(value_error(format!(
            "the observations are {} long but the path is {} — they have to match",
            observations.len(),
            path.len()
        )));
    }
    let learned = Hmm::estimate(skeleton.states, skeleton.symbols, &observations, &path);
    Ok(model_to_record(&learned))
}

/// `hmm_viterbi_learning(observations, model, iterations)` — decode, re-estimate,
/// repeat.
fn builtin_hmm_viterbi_learning(args: Vec<Value>) -> Result<Value> {
    let model = read_model(&args[1], true, true)?;
    let observations = encode(&args[0], &model.symbols, "the observations")?;
    let iterations = require_count(&args[2], "the iteration count")?;
    Ok(model_to_record(
        &model.viterbi_learning(&observations, iterations),
    ))
}

/// `hmm_baum_welch(observations, model, iterations)` — expectation-maximisation,
/// weighting every path rather than committing to one.
fn builtin_hmm_baum_welch(args: Vec<Value>) -> Result<Value> {
    let model = read_model(&args[1], true, true)?;
    let observations = encode(&args[0], &model.symbols, "the observations")?;
    let iterations = require_count(&args[2], "the iteration count")?;
    Ok(model_to_record(
        &model.baum_welch(&observations, iterations),
    ))
}

/// `hmm_profile(alignment, alphabet, threshold)` or
/// `hmm_profile(alignment, alphabet, threshold, pseudocount)` — a profile HMM
/// built from a multiple alignment.
///
/// The pseudocount is optional because leaving it out is a different published
/// answer, not merely a different default: without it, states the alignment
/// never reaches keep empty rows.
fn builtin_hmm_profile(args: Vec<Value>) -> Result<Value> {
    let alignment = match &args[0] {
        Value::List(rows) => rows
            .iter()
            .map(|row| match row {
                Value::Str(s) => Ok(s.clone()),
                Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => Ok(seq.data.clone()),
                other => Err(type_error(format!(
                    "the alignment must contain strings, got {}",
                    other.type_of()
                ))),
            })
            .collect::<Result<Vec<String>>>()?,
        other => {
            return Err(type_error(format!(
                "the alignment must be a list of strings, got {}",
                other.type_of()
            )));
        }
    };
    if alignment.is_empty() {
        return Err(value_error("the alignment has no sequences"));
    }
    let widths: Vec<usize> = alignment.iter().map(|row| row.chars().count()).collect();
    if widths.iter().any(|w| *w != widths[0]) {
        return Err(value_error(
            "every row of an alignment must be the same length — pad the short ones with '-'",
        ));
    }

    let symbols = as_labels(&args[1], "the alphabet")?;
    let threshold = as_number(&args[2], "the threshold")?;
    let pseudocount = match args.get(3) {
        Some(value) => as_number(value, "the pseudocount")?,
        None => 0.0,
    };

    let model = Hmm::profile(&alignment, symbols, threshold, pseudocount);
    Ok(model_to_record(&model))
}

/// `hmm_profile_align(observations, profile)` — the most likely path through a
/// profile HMM that emits `observations`.
///
/// Separate from `viterbi` because it has to be: a profile's deletion states are
/// silent, so the path is longer than the string and the ordinary recurrence
/// does not apply.
fn builtin_hmm_profile_align(args: Vec<Value>) -> Result<Value> {
    let model = read_model(&args[1], true, true)?;
    let observations = encode(&args[0], &model.symbols, "the observations")?;
    match model.align_to_profile(&observations) {
        Some(path) => Ok(to_state_list(&path, &model.states)),
        None => Err(value_error(
            "hmm_profile_align() needs a model built by hmm_profile(), with states              S, I0, M1, D1, I1, ... and E — and one that can emit the observations at all",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(pairs: Vec<(&str, Value)>) -> Value {
        Value::Record(Arc::new(
            pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        ))
    }

    fn labels(names: &[&str]) -> Value {
        Value::List(Arc::new(
            names.iter().map(|n| Value::Str((*n).to_string())).collect(),
        ))
    }

    fn ba10c_model() -> Value {
        record(vec![
            ("states", labels(&["A", "B"])),
            ("symbols", labels(&["x", "y", "z"])),
            (
                "transition",
                record(vec![
                    (
                        "A",
                        record(vec![("A", Value::Float(0.641)), ("B", Value::Float(0.359))]),
                    ),
                    (
                        "B",
                        record(vec![("A", Value::Float(0.729)), ("B", Value::Float(0.271))]),
                    ),
                ]),
            ),
            (
                "emission",
                record(vec![
                    (
                        "A",
                        record(vec![
                            ("x", Value::Float(0.117)),
                            ("y", Value::Float(0.691)),
                            ("z", Value::Float(0.192)),
                        ]),
                    ),
                    (
                        "B",
                        record(vec![
                            ("x", Value::Float(0.097)),
                            ("y", Value::Float(0.42)),
                            ("z", Value::Float(0.483)),
                        ]),
                    ),
                ]),
            ),
        ])
    }

    #[test]
    fn viterbi_returns_named_states() {
        let got = call_hmm_builtin(
            "viterbi",
            vec![Value::Str("xyxzzxyxyy".into()), ba10c_model()],
        )
        .expect("viterbi");
        let path = match got {
            Value::List(items) => items
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.clone(),
                    other => panic!("expected a state name, got {other:?}"),
                })
                .collect::<String>(),
            other => panic!("expected a list, got {other:?}"),
        };
        assert_eq!(path, "AAABBAAAAA");
    }

    #[test]
    fn a_transposed_matrix_is_not_silently_accepted() {
        // The matrices are keyed by name, so an unknown state name is caught
        // rather than read as a different row.
        let model = record(vec![
            ("states", labels(&["A", "B"])),
            ("symbols", labels(&["x"])),
            (
                "transition",
                record(vec![(
                    "A",
                    record(vec![("A", Value::Float(1.0)), ("B", Value::Float(0.0))]),
                )]),
            ),
            (
                "emission",
                record(vec![
                    ("A", record(vec![("x", Value::Float(1.0))])),
                    ("B", record(vec![("x", Value::Float(1.0))])),
                ]),
            ),
        ]);
        let error = call_hmm_builtin("viterbi", vec![Value::Str("x".into()), model])
            .expect_err("the missing B row should be reported");
        assert!(
            error.to_string().contains("no row for 'B'"),
            "unhelpful error: {error}"
        );
    }

    #[test]
    fn an_unknown_symbol_names_what_was_expected() {
        let error = call_hmm_builtin("viterbi", vec![Value::Str("q".into()), ba10c_model()])
            .expect_err("q is not in the alphabet");
        let text = error.to_string();
        assert!(
            text.contains('q') && text.contains('x'),
            "unhelpful: {text}"
        );
    }

    #[test]
    fn mismatched_lengths_are_reported_rather_than_truncated() {
        let error = call_hmm_builtin(
            "hmm_emission_probability",
            vec![
                Value::Str("xyz".into()),
                Value::Str("AA".into()),
                ba10c_model(),
            ],
        )
        .expect_err("3 against 2");
        assert!(error.to_string().contains("have to match"), "{error}");
    }

    #[test]
    fn an_explicit_initial_distribution_is_used() {
        // Forcing the start into B changes the first call, which is the whole
        // point of the field being settable.
        let uniform = call_hmm_builtin(
            "hmm_likelihood",
            vec![Value::Str("z".into()), ba10c_model()],
        )
        .expect("likelihood");
        let mut fields = match ba10c_model() {
            Value::Record(f) => (*f).clone(),
            _ => unreachable!(),
        };
        fields.insert(
            "initial".into(),
            record(vec![("A", Value::Float(0.0)), ("B", Value::Float(1.0))]),
        );
        let forced = call_hmm_builtin(
            "hmm_likelihood",
            vec![Value::Str("z".into()), Value::Record(Arc::new(fields))],
        )
        .expect("likelihood");
        match (uniform, forced) {
            (Value::Float(u), Value::Float(f)) => {
                // Uniform averages A's 0.192 with B's 0.483; forcing B gives 0.483.
                assert!((u - 0.3375).abs() < 1e-12, "uniform start gave {u}");
                assert!((f - 0.483).abs() < 1e-12, "forced start gave {f}");
            }
            other => panic!("expected floats, got {other:?}"),
        }
    }

    #[test]
    fn posterior_rows_are_keyed_by_state_name() {
        let got = call_hmm_builtin(
            "hmm_posterior",
            vec![Value::Str("xy".into()), ba10c_model()],
        )
        .expect("posterior");
        match got {
            Value::List(rows) => {
                assert_eq!(rows.len(), 2);
                for row in rows.iter() {
                    let fields = as_fields(row, "row").expect("a record");
                    assert!(fields.contains_key("A") && fields.contains_key("B"));
                }
            }
            other => panic!("expected a list, got {other:?}"),
        }
    }
}
