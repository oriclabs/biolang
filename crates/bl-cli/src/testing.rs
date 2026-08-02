//! The `bl test` runner.
//!
//! Analysis code is famously untested, and part of the reason is that testing
//! it has never been the path of least resistance: there is no runner, so
//! people write a scratch script, eyeball the numbers, and delete it.
//!
//! Tests are plain zero-argument functions whose name starts with `test_`. That
//! needs no new syntax — `assert` already exists — so any existing file can
//! gain tests without changing how it is written or run.

use std::path::Path;
use std::time::Instant;

use bl_core::ast::{Program, Stmt};
use bl_runtime::Interpreter;

/// Prefix that marks a function as a test.
const TEST_PREFIX: &str = "test_";

pub struct TestOutcome {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
    pub duration_ms: u128,
}

/// Names of the zero-argument `test_*` functions declared at the top level.
///
/// Only top-level declarations count: a `test_` function nested inside another
/// function is a helper, not a test, and calling it out of context would fail
/// for reasons that have nothing to do with the code under test.
pub fn discover(program: &Program) -> Vec<String> {
    program
        .stmts
        .iter()
        .filter_map(|statement| match &statement.node {
            Stmt::Fn { name, params, .. }
                if name.starts_with(TEST_PREFIX) && params.is_empty() =>
            {
                Some(name.clone())
            }
            _ => None,
        })
        .collect()
}

/// Turn a test function name into a readable label.
pub fn describe(name: &str) -> String {
    name.trim_start_matches(TEST_PREFIX).replace('_', " ")
}

fn parse(source: &str) -> Result<Program, String> {
    let tokens = bl_lexer::Lexer::new(source)
        .tokenize()
        .map_err(|error| error.format_with_source(source))?;
    let parsed = bl_parser::Parser::new(tokens)
        .parse()
        .map_err(|error| error.format_with_source(source))?;
    if parsed.has_errors() {
        return Err(parsed
            .errors
            .iter()
            .map(|error| error.format_with_source(source))
            .collect::<Vec<_>>()
            .join("\n"));
    }
    Ok(parsed.program)
}

/// Run every test in one file against a fresh interpreter.
///
/// The file's top level runs once to define functions and set up fixtures, then
/// each test is invoked in turn. State is deliberately shared across tests in a
/// file: bio fixtures are expensive to build, and re-reading a reference genome
/// per test would make the runner useless on real data.
pub fn run_file(path: &Path) -> Result<Vec<TestOutcome>, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("{}: cannot read file: {error}", path.display()))?;
    let program = parse(&source).map_err(|error| format!("{}:\n{error}", path.display()))?;
    let names = discover(&program);
    if names.is_empty() {
        return Ok(Vec::new());
    }

    let mut interpreter = Interpreter::new();
    interpreter.set_current_file(Some(path.to_path_buf()));
    interpreter
        .run(&program)
        .map_err(|error| format!("{}:\n{}", path.display(), error.format_with_source(&source)))?;

    let mut outcomes = Vec::with_capacity(names.len());
    for name in names {
        // Each test is a one-line program calling the function, run against the
        // same interpreter so definitions and fixtures remain in scope.
        let call = format!("{name}()");
        let started = Instant::now();
        let outcome = match parse(&call) {
            Ok(program) => match interpreter.run(&program) {
                Ok(_) => (true, None),
                Err(error) => (false, Some(error.message.clone())),
            },
            Err(error) => (false, Some(error)),
        };
        outcomes.push(TestOutcome {
            name,
            passed: outcome.0,
            message: outcome.1,
            duration_ms: started.elapsed().as_millis(),
        });
    }
    Ok(outcomes)
}

/// Files to run: the given paths, expanding directories to `*.bl`.
pub fn collect_files(paths: &[String]) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack: Vec<std::path::PathBuf> =
        paths.iter().map(std::path::PathBuf::from).rev().collect();
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            let Ok(entries) = std::fs::read_dir(&path) else {
                continue;
            };
            let mut children: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
            children.sort();
            stack.extend(children.into_iter().rev());
        } else if path.extension().is_some_and(|extension| extension == "bl") {
            files.push(path);
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(source: &str) -> Program {
        parse(source).expect("parses")
    }

    #[test]
    fn discovers_top_level_test_functions() {
        let found = discover(&program(
            "fn test_one() { assert true }\nfn helper() { }\nfn test_two() { assert true }\n",
        ));
        assert_eq!(found, vec!["test_one", "test_two"]);
    }

    #[test]
    fn ignores_functions_that_take_arguments() {
        // A `test_` function with parameters is a helper the runner cannot call.
        let found = discover(&program("fn test_with(x) { assert x }\n"));
        assert!(found.is_empty());
    }

    #[test]
    fn ignores_functions_without_the_prefix() {
        let found = discover(&program("fn check_one() { assert true }\n"));
        assert!(found.is_empty());
    }

    #[test]
    fn describes_names_as_readable_labels() {
        assert_eq!(describe("test_gc_content_is_a_fraction"), "gc content is a fraction");
    }

    #[test]
    fn runs_passing_and_failing_tests_in_one_file() {
        let directory = std::env::temp_dir().join(format!("bl-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("suite.bl");
        std::fs::write(
            &path,
            "let base = 2\nfn test_passes() { assert base == 2 }\nfn test_fails() { assert base == 3, \"base drifted\" }\n",
        )
        .unwrap();

        let outcomes = run_file(&path).expect("runs");
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes[0].passed, "{:?}", outcomes[0].message);
        assert!(!outcomes[1].passed);
        assert!(outcomes[1]
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("base drifted"));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_file_with_no_tests_reports_none() {
        let directory = std::env::temp_dir().join(format!("bl-test-empty-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("plain.bl");
        std::fs::write(&path, "let x = 1\n").unwrap();
        assert!(run_file(&path).expect("runs").is_empty());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn collect_files_expands_directories_and_skips_other_extensions() {
        let directory = std::env::temp_dir().join(format!("bl-collect-{}", std::process::id()));
        std::fs::create_dir_all(directory.join("nested")).unwrap();
        std::fs::write(directory.join("a.bl"), "").unwrap();
        std::fs::write(directory.join("notes.md"), "").unwrap();
        std::fs::write(directory.join("nested").join("b.bl"), "").unwrap();

        let found = collect_files(&[directory.display().to_string()]);
        let names: Vec<String> = found
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.contains(&"a.bl".to_string()));
        assert!(names.contains(&"b.bl".to_string()));
        let _ = std::fs::remove_dir_all(directory);
    }
}
