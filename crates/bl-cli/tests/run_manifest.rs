use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bl"))
}

fn run_recorded(script: &Path, record: &Path, input: &Path, output: &Path) -> Output {
    bl().args(["--no-gpu", "run"])
        .arg(script)
        .args(["--record"])
        .arg(record)
        .args(["--input"])
        .arg(input)
        .args(["--output"])
        .arg(output)
        .args(["--param", "count=3", "--param", "label=treated"])
        .args(["--param"])
        .arg(format!("out={}", output.display()))
        .args(["--seed", "17", "--print-result"])
        .output()
        .expect("run bl")
}

#[test]
fn successful_run_records_typed_parameters_hashes_backend_memory_and_outputs() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input");
    fs::create_dir(&input).unwrap();
    fs::write(input.join("a.txt"), "alpha").unwrap();
    fs::write(input.join("b.txt"), "beta").unwrap();
    fs::write(directory.path().join("helper.bl"), "fn answer() { 42 }\n").unwrap();
    let script = directory.path().join("analysis.bl");
    fs::write(
        &script,
        r#"import "helper.bl" as helper
assert(helper.answer() == 42)
assert(run_param("count") == 3)
assert(run_param("label") == "treated")
write_text(run_param("out"), "finished")
random()
"#,
    )
    .unwrap();
    let record = directory.path().join("run.json");
    let output_path = directory.path().join("result.txt");

    let first = run_recorded(&script, &record, &input, &output_path);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_stdout = String::from_utf8_lossy(&first.stdout).trim().to_string();
    let manifest: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();

    assert_eq!(manifest["schema"], "biolang.run/v1");
    assert_eq!(manifest["status"], "succeeded");
    assert_eq!(manifest["inputTracking"], "declared");
    assert_eq!(manifest["parameters"]["count"], 3);
    assert_eq!(manifest["parameters"]["label"], "treated");
    assert!(manifest["options"].get("parameters").is_none());
    assert_eq!(manifest["options"]["seed"], 17);
    assert_eq!(manifest["inputs"][0]["kind"], "directory");
    assert_eq!(manifest["inputs"][0]["fileCount"], 2);
    assert_eq!(manifest["outputs"][0]["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(manifest["script"]["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(manifest["loadedModules"].as_array().unwrap().len(), 1);
    assert_eq!(
        manifest["loadedModules"][0]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        manifest["runtime"]["executableSha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert!(manifest["compute"]["backend"]
        .as_str()
        .unwrap()
        .contains("CPU f64"));
    assert!(manifest["durationMs"].as_u64().is_some());
    assert!(
        manifest["resources"]["peakResidentBytes"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );
    assert_eq!(fs::read_to_string(&output_path).unwrap(), "finished");

    let second_record = directory.path().join("run-2.json");
    let second = run_recorded(&script, &second_record, &input, &output_path);
    assert!(second.status.success());
    assert_eq!(first_stdout, String::from_utf8_lossy(&second.stdout).trim());
}

#[test]
fn failed_execution_still_writes_the_run_record() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("failure.bl");
    let record = directory.path().join("failed.json");
    fs::write(&script, "assert(false)\n").unwrap();

    let output = bl()
        .args(["--no-gpu", "run"])
        .arg(&script)
        .arg("--record")
        .arg(&record)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let manifest: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(manifest["status"], "failed");
    assert!(manifest["error"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
        .contains("assert"));
}

#[test]
fn missing_declared_input_fails_before_the_script_runs_and_is_recorded() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("should-not-run.bl");
    let side_effect = directory.path().join("side-effect.txt");
    let record = directory.path().join("preflight.json");
    let side_effect_source = side_effect.to_string_lossy().replace('\\', "/");
    fs::write(
        &script,
        format!("write_text(\"{side_effect_source}\", \"bad\")\n"),
    )
    .unwrap();

    let output = bl()
        .args(["--no-gpu", "run"])
        .arg(&script)
        .arg("--record")
        .arg(&record)
        .arg("--input")
        .arg(directory.path().join("missing.fastq"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!side_effect.exists());
    let manifest: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(manifest["status"], "failed");
    assert_eq!(manifest["inputs"][0]["exists"], false);
    assert!(manifest["error"].as_str().unwrap().contains("preflight"));
}

#[test]
fn missing_declared_output_fails_after_execution_and_is_recorded() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("missing-output.bl");
    let record = directory.path().join("postflight.json");
    let expected = directory.path().join("never-created.tsv");
    fs::write(&script, "42\n").unwrap();

    let output = bl()
        .args(["--no-gpu", "run"])
        .arg(&script)
        .arg("--record")
        .arg(&record)
        .arg("--output")
        .arg(&expected)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let manifest: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(manifest["status"], "failed");
    assert!(manifest["error"].as_str().unwrap().contains("postflight"));
    assert_eq!(manifest["outputs"][0]["exists"], false);
}

#[test]
fn record_cannot_live_inside_a_declared_directory() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("analysis.bl");
    let results = directory.path().join("results");
    fs::create_dir(&results).unwrap();
    fs::write(&script, "42\n").unwrap();
    let record = results.join("run.json");

    let output = bl()
        .args(["--no-gpu", "run"])
        .arg(&script)
        .arg("--record")
        .arg(&record)
        .arg("--output")
        .arg(&results)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!record.exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be inside"));
}
