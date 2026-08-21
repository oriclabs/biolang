use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn bl_convert() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bl-convert"))
}

#[test]
fn formats_lists_the_supported_contract() {
    let output = bl_convert().arg("formats").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("vcf      bed      yes"));
    assert!(stdout.contains("fastq    fasta    yes"));
}

#[test]
fn convert_and_inspect_work_through_the_public_cli() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("genes.csv");
    let output = directory.path().join("genes.tsv");
    let report = directory.path().join("report.json");
    fs::write(&input, "gene,count\nTP53,3\nBRCA1,5\n").unwrap();

    let converted = bl_convert()
        .args(["convert"])
        .arg(&input)
        .arg(&output)
        .args(["--json", "--report"])
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        converted.status.success(),
        "{}",
        String::from_utf8_lossy(&converted.stderr)
    );
    let conversion: Value = serde_json::from_slice(&converted.stdout).unwrap();
    assert_eq!(conversion["records_written"], 2);
    assert_eq!(conversion["output_validated"], true);
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        "gene\tcount\nTP53\t3\nBRCA1\t5\n"
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&report).unwrap()).unwrap()["schema"],
        "bl-convert.report/v1"
    );

    let inspected = bl_convert()
        .arg("inspect")
        .arg(&output)
        .arg("--json")
        .output()
        .unwrap();
    assert!(inspected.status.success());
    let inspection: Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(inspection["format"], "tsv");
    assert_eq!(inspection["records"], 2);
    assert_eq!(inspection["valid"], true);
}

#[test]
fn report_cannot_replace_the_conversion_output() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("genes.csv");
    let output = directory.path().join("genes.tsv");
    fs::write(&input, "gene\nTP53\n").unwrap();

    let result = bl_convert()
        .arg("convert")
        .arg(&input)
        .arg(&output)
        .arg("--report")
        .arg(&output)
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("--report must differ"));
    assert!(!output.exists());
}

#[test]
fn tool_catalog_is_machine_readable_and_uses_pinned_images() {
    let output = bl_convert()
        .args(["tool", "catalog", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let tools: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(tools.len() >= 10);
    assert!(tools.iter().any(|tool| tool["name"] == "samtools"));
    assert!(tools.iter().all(|tool| {
        tool["image"].as_str().is_some_and(|image| {
            image.starts_with("quay.io/biocontainers/") && !image.ends_with(":latest")
        })
    }));
}

#[test]
fn doctor_and_empty_tool_list_need_no_container_runtime() {
    let directory = tempdir().unwrap();
    let doctor = bl_convert()
        .arg("doctor")
        .arg("--json")
        .env("BL_CONVERT_HOME", directory.path())
        .env("PATH", "")
        .output()
        .unwrap();

    assert!(doctor.status.success());
    let report: Value = serde_json::from_slice(&doctor.stdout).unwrap();
    assert_eq!(report["installed_tools"], 0);
    assert!(report["runtimes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|runtime| runtime["available"] == false && runtime["ready"] == false));

    let listed = bl_convert()
        .args(["tool", "list", "--json"])
        .env("BL_CONVERT_HOME", directory.path())
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(listed.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&listed.stdout).unwrap(),
        serde_json::json!([])
    );
}

#[test]
fn install_is_explicit_and_fails_cleanly_without_the_requested_runtime() {
    let directory = tempdir().unwrap();
    let result = bl_convert()
        .args(["tool", "install", "samtools", "--runtime", "docker"])
        .env("BL_CONVERT_HOME", directory.path())
        .env("PATH", "")
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("docker' is not ready"));
    assert!(!directory.path().join("tools.json").exists());
}

#[test]
fn existing_native_tools_can_be_registered_run_and_unregistered() {
    let directory = tempdir().unwrap();
    let fake_tool = directory
        .path()
        .join(format!("samtools{}", std::env::consts::EXE_SUFFIX));
    fs::copy(env!("CARGO_BIN_EXE_bl-convert"), &fake_tool).unwrap();

    let registered = bl_convert()
        .args(["tool", "register", "samtools", "--path"])
        .arg(&fake_tool)
        .env("BL_CONVERT_HOME", directory.path())
        .output()
        .unwrap();
    assert!(
        registered.status.success(),
        "{}",
        String::from_utf8_lossy(&registered.stderr)
    );

    let status = bl_convert()
        .args(["tool", "status", "samtools", "--json"])
        .env("BL_CONVERT_HOME", directory.path())
        .output()
        .unwrap();
    assert!(status.status.success());
    let installed: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(installed["backend"], "local");
    assert_eq!(installed["image"], Value::Null);

    let run_report = directory.path().join("run-report.json");
    let ran = bl_convert()
        .args(["tool", "run", "samtools", "--workdir"])
        .arg(directory.path())
        .arg("--report")
        .arg(&run_report)
        .args(["--", "--version"])
        .env("BL_CONVERT_HOME", directory.path())
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(String::from_utf8_lossy(&ran.stdout).contains("bl-convert"));
    let provenance: Value = serde_json::from_slice(&fs::read(run_report).unwrap()).unwrap();
    assert_eq!(provenance["schema"], "bl-convert.tool-run/v1");
    assert_eq!(provenance["backend"], "local");
    assert_eq!(provenance["arguments"], serde_json::json!(["--version"]));
    assert_eq!(provenance["exit_code"], 0);

    let removed = bl_convert()
        .args(["tool", "remove", "samtools"])
        .env("BL_CONVERT_HOME", directory.path())
        .output()
        .unwrap();
    assert!(removed.status.success());
    assert!(fake_tool.exists());
}

#[test]
fn missing_wsl_tool_fails_without_changing_the_manifest() {
    let directory = tempdir().unwrap();
    let result = bl_convert()
        .args(["tool", "register", "samtools", "--wsl"])
        .env("BL_CONVERT_HOME", directory.path())
        .env("PATH", "")
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("cannot start WSL") || stderr.contains("is not installed"));
    assert!(!directory.path().join("tools.json").exists());
}

#[test]
fn local_backends_reject_container_only_resource_controls() {
    let directory = tempdir().unwrap();
    let fake_tool = directory
        .path()
        .join(format!("samtools{}", std::env::consts::EXE_SUFFIX));
    fs::copy(env!("CARGO_BIN_EXE_bl-convert"), &fake_tool).unwrap();
    let registered = bl_convert()
        .args(["tool", "register", "samtools", "--path"])
        .arg(&fake_tool)
        .env("BL_CONVERT_HOME", directory.path())
        .output()
        .unwrap();
    assert!(registered.status.success());

    let result = bl_convert()
        .args(["tool", "run", "samtools", "--cpus", "2", "--", "--version"])
        .env("BL_CONVERT_HOME", directory.path())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("apply only to container"));
}
