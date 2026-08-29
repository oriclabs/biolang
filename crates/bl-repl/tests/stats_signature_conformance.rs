//! Every documented return field of a guided statistics builtin must exist.
//!
//! `BUILTIN_CATALOG` signatures are what `bl help`, the REPL completer and the
//! generated website docs show, so a field named there is a promise. Nothing
//! checked that promise against the record the builtin actually returns, and
//! twelve of the thirty-nine guided signatures named fields that were never
//! produced -- `stats_missingness` advertised `by_row,by_column,by_pair` while
//! returning `missing_by_row,columns,co_missing`, and a reader following the
//! signature got Nil.
//!
//! Requiring a fixture for every `stats_*` builtin is the point rather than an
//! inconvenience: a new guided builtin cannot be added without also being
//! invoked here at least once, which is how `stats_design_check` and
//! `stats_decision_map` came to ship with no test of any kind.

use bl_core::value::{Table, Value};
use bl_runtime::stats::call_stats_builtin;
use std::collections::HashMap;

fn numbers(values: &[f64]) -> Value {
    Value::List(
        values
            .iter()
            .copied()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    )
}

fn strings(values: &[&str]) -> Value {
    Value::List(
        values
            .iter()
            .map(|value| Value::Str((*value).into()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn options(entries: &[(&str, Value)]) -> Value {
    Value::Record(
        entries
            .iter()
            .map(|(name, value)| ((*name).to_string(), value.clone()))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn numeric_table(columns: &[(&str, &[f64])]) -> Value {
    let names = columns.iter().map(|(name, _)| (*name).into()).collect();
    let height = columns.first().map(|(_, values)| values.len()).unwrap_or(0);
    let rows = (0..height)
        .map(|row| {
            columns
                .iter()
                .map(|(_, values)| Value::Float(values[row]))
                .collect()
        })
        .collect();
    Value::Table(Table::new(names, rows))
}

/// A small study table with a subject, a group, a batch and a missing value,
/// which is what the table-shaped builtins are designed to comment on.
fn study_table() -> Value {
    Value::Table(Table::new(
        vec![
            "subject".into(),
            "group".into(),
            "batch".into(),
            "age".into(),
            "response".into(),
        ],
        vec![
            vec![
                Value::Str("s1".into()),
                Value::Str("A".into()),
                Value::Str("b1".into()),
                Value::Int(30),
                Value::Float(1.2),
            ],
            vec![
                Value::Str("s1".into()),
                Value::Str("A".into()),
                Value::Str("b1".into()),
                Value::Nil,
                Value::Float(1.7),
            ],
            vec![
                Value::Str("s2".into()),
                Value::Str("B".into()),
                Value::Str("b2".into()),
                Value::Int(41),
                Value::Float(8.0),
            ],
            vec![
                Value::Str("s3".into()),
                Value::Str("B".into()),
                Value::Str("b2".into()),
                Value::Int(55),
                Value::Float(9.1),
            ],
        ],
    ))
}

fn matrix() -> Value {
    Value::List(
        vec![
            numbers(&[4.0, 0.0, 3.0, 1.0]),
            numbers(&[0.0, 7.0, 2.0, 0.0]),
            numbers(&[5.0, 1.0, 0.0, 6.0]),
        ]
        .into(),
    )
}

fn exploration_report() -> Value {
    call_stats_builtin(
        "stats_explore",
        vec![numbers(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 20.0])],
    )
    .expect("stats_explore should accept a plain numeric list")
}

/// Arguments that exercise each guided builtin at least once.
///
/// Deliberately total over the `stats_*` surface: an unmatched name fails the
/// test rather than being skipped.
fn fixture(name: &str) -> Option<Vec<Value>> {
    let values = numbers(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 20.0]);
    let x = numbers(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let y = numbers(&[2.1, 3.9, 6.2, 8.1, 9.8, 12.2, 13.9, 16.1]);
    let groups = strings(&["a", "a", "a", "a", "b", "b", "b", "b"]);
    let categories = strings(&["red", "blue", "red", "green", "red", "blue"]);

    Some(match name {
        "stats_explore" => vec![values],
        "stats_means" => vec![values],
        "stats_shape" => vec![values],
        "stats_uncertainty" => vec![values],
        "stats_preprocess" => vec![values],
        "stats_distribution_clues" => {
            vec![numbers(&[0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 4.0, 9.0, 15.0])]
        }
        "stats_distribution_plot" => vec![values],
        "stats_distribution_ascii" => vec![values],
        "stats_normal_qq_plot" => vec![values],
        "stats_time_series_diagnostics" => {
            vec![numbers(&[
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0,
            ])]
        }
        "stats_transform_preview" => vec![values, Value::Str("log".into())],
        "stats_categories" | "stats_categorical_plot" => vec![categories],

        "stats_relationship"
        | "stats_relationship_plot"
        | "stats_linear_diagnostics"
        | "stats_linear_diagnostic_plot" => vec![x, y],
        "stats_compare" | "stats_group_plot" | "stats_facet_plot" => vec![values, groups],
        "stats_cluster_diagnostics" => vec![
            numbers(&[1.0, 1.2, 5.0, 5.2, 9.0, 9.2, 13.0, 13.2]),
            strings(&["a", "a", "b", "b", "c", "c", "d", "d"]),
        ],
        "stats_weighted_summary" => vec![numbers(&[1.0, 2.0, 10.0]), numbers(&[1.0, 1.0, 8.0])],

        "stats_profile"
        | "stats_missingness"
        | "stats_missingness_plot"
        | "stats_design_check"
        | "stats_associations"
        | "stats_scan"
        | "stats_overview_ascii"
        | "stats_report" => {
            vec![
                study_table(),
                options(&[
                    ("subject_column", Value::Str("subject".into())),
                    ("group_column", Value::Str("group".into())),
                    ("batch_column", Value::Str("batch".into())),
                ]),
            ]
        }

        "stats_normalization_guide" | "stats_omics_profile" => vec![matrix()],

        "stats_multiple_linear_diagnostics" | "stats_robust_linear_diagnostics" => vec![
            numeric_table(&[("x", &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])]),
            numbers(&[2.0, 4.1, 5.9, 8.0, 10.1, 12.0, 14.0, 16.1, 18.0, 20.2]),
        ],
        "stats_glm_diagnostics" => vec![
            numeric_table(&[(
                "age",
                &[20.0, 24.0, 28.0, 32.0, 36.0, 40.0, 44.0, 48.0, 52.0, 56.0],
            )]),
            numbers(&[0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]),
            options(&[("family", Value::Str("binomial".into()))]),
        ],
        "stats_random_intercept_model" => vec![
            numeric_table(&[(
                "time",
                &[0.0, 1.0, 2.0, 3.0, 0.0, 1.0, 2.0, 3.0, 0.0, 1.0, 2.0, 3.0],
            )]),
            numbers(&[
                10.4, 12.1, 14.6, 15.9, 15.0, 17.9, 18.4, 21.2, 6.3, 8.1, 9.4, 12.2,
            ]),
            strings(&["a", "a", "a", "a", "b", "b", "b", "b", "c", "c", "c", "c"]),
        ],
        "stats_cox_diagnostics" => vec![
            numbers(&[5.0, 8.0, 9.0, 12.0, 15.0, 18.0, 20.0, 23.0, 26.0, 30.0]),
            numbers(&[1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0]),
            numeric_table(&[(
                "age",
                &[51.0, 62.0, 57.0, 70.0, 45.0, 66.0, 54.0, 73.0, 49.0, 61.0],
            )]),
        ],

        "stats_explain" | "stats_guide" | "stats_visualize" => vec![exploration_report()],
        "stats_decision_map" => vec![],
        "stats_normal_diagram" => vec![],

        _ => return None,
    })
}

/// The classic hypothesis-test surface, which documents record shapes on the
/// same catalog and drifted the same way.
const CLASSIC_STATISTICS: &[&str] = &[
    "ttest",
    "ttest_one",
    "ttest_paired",
    "wilcoxon",
    "wilcoxon_paired",
    "fisher_exact",
    "chi_square",
    "anova",
    "kruskal_wallis",
    "tukey_hsd",
    "pairwise_ttest",
    "lm",
    "summary",
];

fn classic_fixture(name: &str) -> Option<Vec<Value>> {
    let a = numbers(&[1.2, 2.4, 3.1, 4.8, 5.5]);
    let b = numbers(&[2.0, 3.3, 4.1, 6.2, 7.9]);
    // The multi-group tests take a list of groups, not a value/label pairing.
    let grouped = Value::List(
        vec![
            numbers(&[1.0, 2.0, 3.0]),
            numbers(&[5.0, 6.0, 7.0]),
            numbers(&[9.0, 10.0, 11.0]),
        ]
        .into(),
    );

    Some(match name {
        "ttest" | "wilcoxon" | "ttest_paired" | "wilcoxon_paired" | "lm" => vec![a, b],
        "ttest_one" => vec![a, Value::Float(2.0)],
        "fisher_exact" => vec![Value::Int(8), Value::Int(2), Value::Int(1), Value::Int(5)],
        "chi_square" => vec![numbers(&[10.0, 20.0, 30.0]), numbers(&[20.0, 20.0, 20.0])],
        "anova" | "kruskal_wallis" | "tukey_hsd" | "pairwise_ttest" => vec![grouped],
        "summary" => vec![a],
        _ => return None,
    })
}

/// Field names inside the `Record{...}` of a catalog signature.
fn documented_fields(signature: &str) -> Vec<String> {
    let Some(start) = signature.find("Record{") else {
        return Vec::new();
    };
    let rest = &signature[start + "Record{".len()..];
    let Some(end) = rest.find('}') else {
        return Vec::new();
    };
    rest[..end]
        .split(',')
        .map(str::trim)
        // `...` marks a signature that names only its leading fields, and
        // `col → type_str` describes a key shape rather than a literal key.
        .filter(|field| !field.is_empty() && *field != "..." && !field.contains('\u{2192}'))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_guided_statistics_builtin_has_a_fixture() {
    let unfixtured = bl_repl::biolang_metadata()
        .builtins
        .into_iter()
        .filter(|builtin| builtin.name.starts_with("stats_"))
        .filter(|builtin| fixture(&builtin.name).is_none())
        .map(|builtin| builtin.name)
        .collect::<Vec<_>>();

    assert!(
        unfixtured.is_empty(),
        "these guided statistics builtins have no fixture in this file, so nothing \
         invokes them anywhere in the test suite: {unfixtured:?}"
    );
}

#[test]
fn every_documented_return_field_is_actually_returned() {
    let mut failures = Vec::new();

    for builtin in bl_repl::biolang_metadata().builtins {
        let guided = builtin.name.starts_with("stats_");
        if !guided && !CLASSIC_STATISTICS.contains(&builtin.name.as_str()) {
            continue;
        }
        let fields = documented_fields(&builtin.signature);
        if fields.is_empty() {
            continue;
        }
        let args = if guided {
            fixture(&builtin.name)
        } else {
            classic_fixture(&builtin.name)
        };
        let Some(args) = args else {
            failures.push(format!("{}: no fixture", builtin.name));
            continue;
        };

        let returned = match call_stats_builtin(&builtin.name, args) {
            Ok(value) => value,
            Err(error) => {
                failures.push(format!("{}: fixture rejected -- {error}", builtin.name));
                continue;
            }
        };
        let Value::Record(record) = &returned else {
            failures.push(format!(
                "{}: signature documents a Record but the builtin returned {returned:?}",
                builtin.name
            ));
            continue;
        };
        for field in fields {
            if !record.contains_key(&field) {
                let mut present = record.keys().cloned().collect::<Vec<_>>();
                present.sort();
                failures.push(format!(
                    "{}: signature promises `{field}`, which is absent. Returned: {present:?}",
                    builtin.name
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "documented return fields that do not exist:\n  {}",
        failures.join("\n  ")
    );
}

/// For a guided-exploration API the example is the field that carries the most
/// weight: the whole difficulty is knowing which entry point answers your
/// question. Six of the forty-one had one.
#[test]
fn every_guided_statistics_builtin_has_an_example() {
    let missing = bl_repl::biolang_metadata()
        .builtins
        .into_iter()
        .filter(|builtin| builtin.name.starts_with("stats_"))
        .filter(|builtin| {
            builtin
                .example
                .as_deref()
                .is_none_or(|example| example.trim().is_empty())
        })
        .map(|builtin| builtin.name)
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "guided statistics builtins with no example: {missing:?}"
    );
}

/// The fixtures the shipped examples are written against.
///
/// Lengths are matched to each other deliberately: the paired builtins reject
/// mismatched inputs, which is how the harness first caught that the examples
/// were being checked against inconsistent data rather than being wrong.
const EXAMPLE_PREAMBLE: &str = r#"
let values = [12.1, 12.4, 13.0, 14.2, 15.8, 29.0]
let groups = ["control", "control", "control", "drug", "drug", "drug"]
let labels = ["red", "blue", "red", "green", "red", "blue"]
let counts = [0, 0, 1, 1, 2, 4, 9, 15]
let clusters = ["a", "a", "b", "b", "c", "c"]
let weights = [1.0, 1.0, 1.0, 2.0, 2.0, 8.0]
let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
let y = [2.1, 3.9, 6.2, 8.1, 9.8, 12.2, 13.9, 16.1]
let series = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
let counts_matrix = [[4.0, 0.0, 3.0], [0.0, 7.0, 2.0], [5.0, 1.0, 0.0]]
let predictors = table({age: [51.0, 62.0, 57.0, 70.0, 45.0, 66.0, 54.0, 73.0]})
let outcome = [0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0]
let time = [5.0, 8.0, 9.0, 12.0, 15.0, 18.0, 20.0, 23.0]
let event = [1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0]
let subject_ids = ["a", "a", "b", "b", "c", "c", "d", "d"]
let trial = table({
    patient: ["p1", "p2", "p3", "p4"],
    arm: ["control", "control", "drug", "drug"],
    run: ["r1", "r2", "r1", "r2"],
    response: [1.2, 1.7, 8.0, 9.1]
})
let report = stats_explore(values)
"#;

fn run_source(source: &str) -> Result<(), String> {
    let tokens = bl_lexer::Lexer::new(source)
        .tokenize()
        .map_err(|error| format!("lex: {}", error.message))?;
    let parsed = bl_parser::Parser::new(tokens)
        .parse()
        .map_err(|error| format!("parse: {}", error.message))?;
    if parsed.has_errors() {
        return Err(format!("parse: {}", parsed.errors[0].message));
    }
    bl_runtime::interpreter::Interpreter::new()
        .run(&parsed.program)
        .map(|_| ())
        .map_err(|error| error.message)
}

/// Every example shipped in `bl help` must run.
///
/// This is not pedantry. The examples previously called the `statistics`
/// package wrapper (`stat.missingness(...)`), which needs
/// `import "statistics" as stat` and the package installed -- so on a clean
/// machine every one of them failed with `ImportError`, and one of them passed
/// the builtin `table` constructor where a Table was wanted. Both were found by
/// running them rather than reading them.
#[test]
fn every_shipped_statistics_example_runs() {
    if run_source(EXAMPLE_PREAMBLE).is_err() {
        panic!("the example preamble itself does not run");
    }

    let mut failures = Vec::new();
    for builtin in bl_repl::biolang_metadata().builtins {
        if !builtin.name.starts_with("stats_") {
            continue;
        }
        let Some(example) = builtin.example else {
            continue; // reported by the example-coverage test
        };
        let source = format!("{EXAMPLE_PREAMBLE}\n{example}\n");
        if let Err(error) = run_source(&source) {
            failures.push(format!("{}: `{example}` -- {error}", builtin.name));
        }
    }

    assert!(
        failures.is_empty(),
        "shipped examples that do not run:\n  {}",
        failures.join("\n  ")
    );
}
