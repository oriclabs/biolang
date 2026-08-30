use std::fs;
use std::process::Command;

#[test]
fn exported_lesson_project_runs_from_its_own_directory() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("lesson.bl"), "40 + 2\n").unwrap();
    fs::write(
        directory.path().join("lesson.bln"),
        "```biolang\n40 + 2\n```\n",
    )
    .unwrap();
    let lock = directory.path().join("lesson-data.json");
    fs::write(
        &lock,
        r#"{"schema":1,"kind":"biolang-lesson-data-lock","project":{"notebook":"lesson.bln","script":"lesson.bl"},"files":[]}"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bl"))
        .args([
            "lesson",
            "run",
            lock.to_str().unwrap(),
            "--offline",
            "--print-result",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("42"));
}
