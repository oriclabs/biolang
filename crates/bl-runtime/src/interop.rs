//! Inline Python and R evaluation.
//!
//! Nobody abandons DESeq2, Seurat, or scanpy on the day they try a new
//! language, and a migration that demands it loses. `py()` and `r()` let an
//! analysis move over a piece at a time: port what BioLang does better, keep
//! the calls that have no equivalent yet, delete them as the equivalents land.
//!
//! ```biolang
//! let shrunk = r("DESeq2::lfcShrink(dds, coef=2)", {dds: counts})
//! let scaled = py("import numpy as np; np.log1p(counts).tolist()", {counts: values})
//! ```
//!
//! Values cross as JSON. Tables, records, lists, numbers, strings, and booleans
//! travel both ways; anything that only exists inside the other runtime does
//! not, and says so rather than arriving silently mangled.

use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

use crate::json::{json_to_value, value_to_json};

/// How long a snippet may run before it is treated as hung.
const TIMEOUT_SECONDS: u64 = 300;

pub fn interop_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("py", Arity::AtLeast(1)),
        ("r", Arity::AtLeast(1)),
        ("interop_status", Arity::Exact(0)),
    ]
}

pub fn is_interop_builtin(name: &str) -> bool {
    matches!(name, "py" | "r" | "interop_status")
}

/// The Python driver.
///
/// The last expression is the result, matching what a notebook cell does — the
/// mental model every Python user already has. When the snippet ends in a
/// statement instead, a variable named `result` is used if it exists.
const PYTHON_DRIVER: &str = r#"
import ast, json, sys

request = json.load(sys.stdin)
namespace = {}
namespace.update(request.get("bindings", {}))

def encode(value):
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, dict):
        return {str(k): encode(v) for k, v in value.items()}
    if isinstance(value, (list, tuple, set)):
        return [encode(v) for v in value]
    # pandas and numpy are the shapes that actually come back from bio code.
    module = type(value).__module__.split(".")[0]
    if module == "pandas":
        if hasattr(value, "to_dict"):
            try:
                return json.loads(value.to_json(orient="records"))
            except (TypeError, ValueError):
                return json.loads(value.to_json())
    if module == "numpy":
        if hasattr(value, "tolist"):
            return encode(value.tolist())
        if hasattr(value, "item"):
            return encode(value.item())
    raise TypeError(
        "cannot return a %s to BioLang - convert it to a list, dict, or DataFrame first"
        % type(value).__name__
    )

try:
    tree = ast.parse(request["code"])
    if tree.body and isinstance(tree.body[-1], ast.Expr):
        final = ast.Expression(tree.body.pop().value)
        exec(compile(tree, "<biolang>", "exec"), namespace)
        value = eval(compile(final, "<biolang>", "eval"), namespace)
    else:
        exec(compile(tree, "<biolang>", "exec"), namespace)
        value = namespace.get("result")
    print(json.dumps({"ok": True, "value": encode(value)}))
except Exception as error:
    print(json.dumps({"ok": False, "error": "%s: %s" % (type(error).__name__, error)}))
"#;

/// The R driver.
///
/// `eval(parse(...))` already yields the last value, which is R's own
/// convention, so no equivalent of the Python last-expression dance is needed.
const R_DRIVER: &str = r#"
if (!requireNamespace("jsonlite", quietly = TRUE)) {
  cat('{"ok":false,"error":"the jsonlite package is required for r() - install.packages(\"jsonlite\")"}')
  quit(status = 0)
}
request <- jsonlite::fromJSON(file("stdin"), simplifyVector = FALSE)
environment <- new.env(parent = globalenv())
bindings <- request$bindings
if (!is.null(bindings)) {
  for (name in names(bindings)) {
    value <- bindings[[name]]
    # A list of uniform records is a data frame to anyone writing R.
    if (is.list(value) && length(value) > 0 && all(vapply(value, is.list, logical(1)))) {
      value <- tryCatch(
        do.call(rbind.data.frame, c(lapply(value, as.data.frame), stringsAsFactors = FALSE)),
        error = function(e) value
      )
    } else if (is.list(value) && all(vapply(value, function(x) length(x) == 1, logical(1)))) {
      value <- unlist(value)
    }
    assign(name, value, envir = environment)
  }
}
outcome <- tryCatch({
  value <- eval(parse(text = request$code), envir = environment)
  list(ok = TRUE, value = value)
}, error = function(e) list(ok = FALSE, error = conditionMessage(e)))
cat(jsonlite::toJSON(outcome, auto_unbox = TRUE, dataframe = "rows", null = "null", digits = NA))
"#;

/// Candidate commands for each language, most specific first.
fn candidates(language: &str) -> &'static [&'static str] {
    match language {
        "py" => &["python3", "python"],
        _ => &["Rscript"],
    }
}

/// True when `command` is a working interpreter.
///
/// Exit status is checked, not merely whether the process started: on Windows
/// `python3` resolves to a Microsoft Store stub that launches happily, prints
/// an advert, and exits non-zero. Treating that as an interpreter sends every
/// snippet to a program that cannot run it.
fn works(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// The interpreter to use for a language, if one is installed.
pub fn interpreter_for(language: &str) -> Option<String> {
    candidates(language)
        .iter()
        .find(|command| works(command))
        .map(|command| (*command).to_string())
}

fn missing_interpreter(language: &str) -> BioLangError {
    let (what, hint) = if language == "py" {
        (
            "Python",
            "install Python 3 and make sure `python3` or `python` is on PATH",
        )
    } else {
        ("R", "install R and make sure `Rscript` is on PATH")
    };
    BioLangError::new(
        ErrorKind::PluginError,
        format!("{what} is not available for {language}() - {hint}"),
        None,
    )
}

/// Convert an argument for the boundary, refusing what cannot survive it.
///
/// `value_to_json` falls back to a display string for anything it does not
/// know, which would send `Stream(...)` into Python as a meaningless string.
/// Failing here names the offending binding instead.
fn encode_binding(name: &str, value: &Value) -> Result<serde_json::Value> {
    match value {
        Value::Nil
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Str(_)
        | Value::List(_)
        | Value::Map(_)
        | Value::Record(_)
        | Value::DNA(_)
        | Value::RNA(_)
        | Value::Protein(_)
        | Value::Table(_) => Ok(value_to_json(value)),
        other => Err(BioLangError::new(
            ErrorKind::TypeError,
            format!(
                "cannot pass `{name}` across the language boundary: {} does not convert to JSON. \
                 Collect it into a table or list first.",
                other.type_of()
            ),
            None,
        )),
    }
}

/// Trailing records become bindings in the other runtime.
///
/// Bindings are written as a record literal — `py("...", {counts: table})` —
/// rather than as named call arguments, because BioLang does not forward named
/// arguments to builtins.
fn bindings_from(args: &[Value]) -> Result<HashMap<String, serde_json::Value>> {
    let mut bindings = HashMap::new();
    for value in args.iter().skip(1) {
        let Value::Record(record) = value else {
            return Err(BioLangError::new(
                ErrorKind::TypeError,
                "py() and r() take the code first, then a record of bindings such as                  `py(\"...\", {counts: table})`"
                    .to_string(),
                None,
            ));
        };
        for (name, bound) in record.iter() {
            bindings.insert(name.clone(), encode_binding(name, bound)?);
        }
    }
    Ok(bindings)
}

fn run_driver(
    language: &str,
    code: &str,
    bindings: HashMap<String, serde_json::Value>,
) -> Result<Value> {
    let interpreter = interpreter_for(language).ok_or_else(|| missing_interpreter(language))?;
    let driver = if language == "py" {
        PYTHON_DRIVER
    } else {
        R_DRIVER
    };

    // The driver goes to a temp file rather than through `-c`/`-e`: quoting a
    // multi-line program through a Windows command line mangles it. The name
    // carries a per-call counter as well as the pid, because two `py()` calls
    // in one process would otherwise share a path and delete each other's
    // driver mid-run.
    static NEXT_CALL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let call = NEXT_CALL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "biolang-interop-{}-{call}-{}.{}",
        std::process::id(),
        language,
        if language == "py" { "py" } else { "R" }
    ));
    std::fs::write(&path, driver).map_err(|error| {
        BioLangError::new(
            ErrorKind::IOError,
            format!("cannot stage the {language} bridge: {error}"),
            None,
        )
    })?;

    let request = serde_json::json!({ "code": code, "bindings": bindings });
    let mut child = Command::new(&interpreter)
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            BioLangError::new(
                ErrorKind::PluginError,
                format!("cannot start {interpreter}: {error}"),
                None,
            )
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(request.to_string().as_bytes());
    }

    let output = wait_with_timeout(child, &path)?;
    let _ = std::fs::remove_file(&path);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).map_err(|_| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        BioLangError::new(
            ErrorKind::PluginError,
            format!("the {language} bridge returned nothing usable: {detail}"),
            None,
        )
    })?;

    if response.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        let message = response
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("evaluation failed");
        return Err(BioLangError::new(
            ErrorKind::PluginError,
            format!("{language}(): {message}"),
            None,
        ));
    }

    Ok(json_to_value(
        response
            .get("value")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    ))
}

/// Wait for the child, killing it if it outlives the timeout.
///
/// A snippet that blocks on input would otherwise hang the whole run with no
/// output and no way to tell why.
fn wait_with_timeout(
    mut child: std::process::Child,
    path: &std::path::Path,
) -> Result<std::process::Output> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(TIMEOUT_SECONDS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = std::fs::remove_file(path);
                    return Err(BioLangError::new(
                        ErrorKind::PluginError,
                        format!("the snippet did not finish within {TIMEOUT_SECONDS}s"),
                        None,
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(error) => {
                let _ = std::fs::remove_file(path);
                return Err(BioLangError::new(
                    ErrorKind::PluginError,
                    format!("the bridge process failed: {error}"),
                    None,
                ));
            }
        }
    }
    child.wait_with_output().map_err(|error| {
        BioLangError::new(
            ErrorKind::PluginError,
            format!("the bridge produced no output: {error}"),
            None,
        )
    })
}

pub fn call_interop_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    if name == "interop_status" {
        let mut status = HashMap::new();
        for language in ["py", "r"] {
            status.insert(
                language.to_string(),
                match interpreter_for(language) {
                    Some(interpreter) => Value::Str(interpreter),
                    None => Value::Nil,
                },
            );
        }
        return Ok(Value::Record(status.into()));
    }

    let Some(Value::Str(code)) = args.first() else {
        return Err(BioLangError::new(
            ErrorKind::TypeError,
            format!("{name}() expects the code to run as its first argument"),
            None,
        ));
    };
    let bindings = bindings_from(&args)?;
    run_driver(name, code, bindings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(entries: &[(&str, Value)]) -> Value {
        Value::Record(
            entries
                .iter()
                .map(|(key, value)| ((*key).to_string(), value.clone()))
                .collect::<HashMap<_, _>>()
                .into(),
        )
    }

    #[test]
    fn interop_builtins_are_registered() {
        assert!(is_interop_builtin("py"));
        assert!(is_interop_builtin("r"));
        assert!(is_interop_builtin("interop_status"));
        assert!(!is_interop_builtin("python"));
    }

    #[test]
    fn scalars_tables_and_records_cross_the_boundary() {
        for value in [
            Value::Int(3),
            Value::Float(1.5),
            Value::Str("ACGT".into()),
            Value::Bool(true),
            Value::Nil,
            Value::List(vec![Value::Int(1), Value::Int(2)].into()),
        ] {
            assert!(encode_binding("x", &value).is_ok(), "{value} should encode");
        }
    }

    #[test]
    fn an_unconvertible_binding_names_itself_rather_than_stringifying() {
        // `value_to_json` would happily turn this into a display string, which
        // arrives in Python as meaningless text.
        let matrix = Value::Matrix(
            bl_core::matrix::Matrix::new(vec![0.0], 1, 1)
                .unwrap()
                .into(),
        );
        let error = encode_binding("reads", &matrix).expect_err("should refuse");
        assert!(error.message.contains("reads"), "{}", error.message);
        assert!(
            error.message.contains("does not convert"),
            "{}",
            error.message
        );
    }

    #[test]
    fn bindings_must_be_named() {
        let error = bindings_from(&[Value::Str("1+1".into()), Value::Int(3)])
            .expect_err("positional binding");
        assert!(
            error.message.contains("record of bindings"),
            "{}",
            error.message
        );
    }

    #[test]
    fn named_bindings_are_collected() {
        let bindings = bindings_from(&[
            Value::Str("x + y".into()),
            record(&[("x", Value::Int(1))]),
            record(&[("y", Value::Int(2))]),
        ])
        .expect("collects");
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings["x"], serde_json::json!(1));
    }

    #[test]
    fn a_missing_interpreter_explains_how_to_get_one() {
        let error = missing_interpreter("py");
        assert!(error.message.contains("PATH"), "{}", error.message);
        assert!(error.message.contains("Python"), "{}", error.message);
    }

    #[test]
    fn interop_status_reports_what_is_installed() {
        let status = call_interop_builtin("interop_status", Vec::new()).expect("status");
        let Value::Record(record) = status else {
            panic!("expected a record");
        };
        assert!(record.contains_key("py"));
        assert!(record.contains_key("r"));
    }

    #[test]
    fn python_evaluates_the_last_expression_and_returns_it() {
        let Some(_) = interpreter_for("py") else {
            eprintln!("skipping: no Python on PATH");
            return;
        };
        let result =
            call_interop_builtin("py", vec![Value::Str("x = 6\nx * 7".into())]).expect("evaluates");
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn python_receives_named_bindings() {
        let Some(_) = interpreter_for("py") else {
            eprintln!("skipping: no Python on PATH");
            return;
        };
        let result = call_interop_builtin(
            "py",
            vec![
                Value::Str("sum(values)".into()),
                record(&[(
                    "values",
                    Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(4)].into()),
                )]),
            ],
        )
        .expect("evaluates");
        assert_eq!(result, Value::Int(7));
    }

    #[test]
    fn a_table_arrives_in_python_as_a_list_of_records() {
        let Some(_) = interpreter_for("py") else {
            eprintln!("skipping: no Python on PATH");
            return;
        };
        let table = Value::Table(
            bl_core::value::Table::new(
                vec!["gene".into(), "n".into()],
                vec![
                    vec![Value::Str("TP53".into()), Value::Int(3)],
                    vec![Value::Str("BRCA1".into()), Value::Int(7)],
                ],
            )
            .into(),
        );
        let result = call_interop_builtin(
            "py",
            vec![
                Value::Str("[row['gene'] for row in counts]".into()),
                record(&[("counts", table)]),
            ],
        )
        .expect("evaluates");
        assert_eq!(
            result,
            Value::List(vec![Value::Str("TP53".into()), Value::Str("BRCA1".into())].into())
        );
    }

    #[test]
    fn a_python_error_comes_back_as_a_biolang_error() {
        let Some(_) = interpreter_for("py") else {
            eprintln!("skipping: no Python on PATH");
            return;
        };
        let error = call_interop_builtin("py", vec![Value::Str("1 / 0".into())])
            .expect_err("division by zero");
        assert!(
            error.message.contains("ZeroDivisionError"),
            "{}",
            error.message
        );
    }

    #[test]
    fn a_value_python_cannot_encode_says_so() {
        let Some(_) = interpreter_for("py") else {
            eprintln!("skipping: no Python on PATH");
            return;
        };
        let error = call_interop_builtin("py", vec![Value::Str("object()".into())])
            .expect_err("opaque object");
        assert!(error.message.contains("cannot return"), "{}", error.message);
    }
}
