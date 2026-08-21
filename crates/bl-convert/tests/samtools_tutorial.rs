//! Real samtools workflow coverage for the BL Convert tool runner.
//!
//! This test is ignored by default because it requires a real external tool
//! and may pull a container image. See `tests/README.md` for invocation.

use serde_json::Value;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

fn bl_convert() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bl-convert"))
}

fn output_text(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_success(output: &Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation} failed with {:?}\n{}",
        output.status.code(),
        output_text(output)
    );
}

fn native_samtools_is_ready() -> bool {
    Command::new("samtools")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn configure_backend(home: &Path) -> String {
    let output = if let Some(path) = env::var_os("BL_CONVERT_TEST_SAMTOOLS") {
        bl_convert()
            .args(["tool", "register", "samtools", "--path"])
            .arg(path)
            .env("BL_CONVERT_HOME", home)
            .output()
            .expect("start BL Convert native registration")
    } else if native_samtools_is_ready() {
        bl_convert()
            .args(["tool", "register", "samtools", "--local"])
            .env("BL_CONVERT_HOME", home)
            .output()
            .expect("start BL Convert PATH registration")
    } else if let Ok(runtime) = env::var("BL_CONVERT_TEST_RUNTIME") {
        bl_convert()
            .args(["tool", "install", "samtools", "--runtime", &runtime])
            .env("BL_CONVERT_HOME", home)
            .output()
            .expect("start BL Convert container installation")
    } else {
        panic!(
            "real samtools is not configured; install samtools on PATH, set \
             BL_CONVERT_TEST_SAMTOOLS to its executable, or explicitly set \
             BL_CONVERT_TEST_RUNTIME=docker|podman|apptainer|singularity"
        );
    };
    assert_success(&output, "configure real samtools backend");

    let status = bl_convert()
        .args(["tool", "status", "samtools", "--json"])
        .env("BL_CONVERT_HOME", home)
        .output()
        .expect("read samtools registration");
    assert_success(&status, "read samtools registration");
    let status: Value = serde_json::from_slice(&status.stdout).expect("status JSON");
    let backend = status["backend"]
        .as_str()
        .expect("registered backend name")
        .to_owned();
    let version = status["tool_version"]
        .as_str()
        .expect("registered samtools version");
    assert!(
        version.to_ascii_lowercase().contains("samtools"),
        "unexpected version text: {version}"
    );
    backend
}

fn run_tool<I, S>(home: &Path, workdir: &Path, report: &Path, arguments: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = bl_convert();
    command
        .args(["tool", "run", "samtools", "--workdir"])
        .arg(workdir)
        .arg("--report")
        .arg(report)
        .arg("--force-report")
        .arg("--")
        .args(arguments)
        .env("BL_CONVERT_HOME", home);
    command.output().expect("start samtools through BL Convert")
}

fn write_fixtures(workdir: &Path) {
    fs::create_dir_all(workdir).expect("create fixture directory");
    let sam = concat!(
        "@HD\tVN:1.6\tSO:unsorted\n",
        "@SQ\tSN:chr1\tLN:1000\n",
        "@SQ\tSN:chr2\tLN:500\n",
        "@RG\tID:rg1\tSM:tutorial\n",
        "read_chr2\t0\tchr2\t50\t60\t5M\t*\t0\t0\tGGGGG\tIIIII\tRG:Z:rg1\n",
        "read_chr1_b\t0\tchr1\t20\t60\t5M\t*\t0\t0\tCCCCC\tIIIII\tRG:Z:rg1\n",
        "read_unmapped\t4\t*\t0\t0\t*\t*\t0\t0\tTTTTT\tIIIII\tRG:Z:rg1\n",
        "read_chr1_a\t0\tchr1\t10\t60\t5M\t*\t0\t0\tAAAAA\tIIIII\tRG:Z:rg1\n",
    );
    fs::write(workdir.join("tutorial.sam"), sam).expect("write SAM fixture");
    fs::write(
        workdir.join("malformed.sam"),
        "@HD\tVN:1.6\nthis-is-not-a-valid-alignment\n",
    )
    .expect("write malformed SAM fixture");

    let reference = format!(">chr1\n{}\n>chr2\n{}\n", "A".repeat(1000), "C".repeat(500));
    fs::write(workdir.join("reference.fa"), reference).expect("write FASTA fixture");
}

fn inferred_bl_executable() -> Option<PathBuf> {
    let converter = Path::new(env!("CARGO_BIN_EXE_bl-convert"));
    let sibling = converter
        .parent()?
        .join(format!("bl{}", env::consts::EXE_SUFFIX));
    sibling.is_file().then_some(sibling)
}

#[test]
#[ignore = "requires real samtools; see crates/bl-convert/tests/README.md"]
fn official_samtools_tutorial_commands_work_end_to_end() {
    let temporary = tempdir().expect("create integration-test directory");
    let home = temporary.path().join("state");
    let workdir = temporary.path().join("samtools tutorial with spaces");
    let report = temporary.path().join("samtools-run.json");
    write_fixtures(&workdir);

    let backend = configure_backend(&home);
    eprintln!("testing samtools through {backend}");

    // Official `samtools view -bo` compound-option form: SAM -> BAM.
    let view = run_tool(
        &home,
        &workdir,
        &report,
        ["view", "-bo", "aln with spaces.bam", "tutorial.sam"],
    );
    assert_success(&view, "SAM to BAM");
    assert!(workdir.join("aln with spaces.bam").is_file());
    let provenance: Value =
        serde_json::from_slice(&fs::read(&report).expect("read provenance report"))
            .expect("provenance JSON");
    assert_eq!(provenance["schema"], "bl-convert.tool-run/v1");
    assert_eq!(provenance["backend"], backend);
    assert_eq!(
        provenance["arguments"],
        serde_json::json!(["view", "-bo", "aln with spaces.bam", "tutorial.sam"])
    );
    assert_eq!(provenance["exit_code"], 0);
    assert!(provenance["elapsed_ms"].as_u64().is_some());

    // Coordinate sort with tutorial-style threading and memory parameters.
    let sort = run_tool(
        &home,
        &workdir,
        &report,
        [
            "sort",
            "-@",
            "2",
            "-m",
            "1M",
            "-o",
            "aln sorted.bam",
            "aln with spaces.bam",
        ],
    );
    assert_success(&sort, "coordinate sort");
    assert!(workdir.join("aln sorted.bam").is_file());

    let index = run_tool(&home, &workdir, &report, ["index", "aln sorted.bam"]);
    assert_success(&index, "BAM index");
    assert!(workdir.join("aln sorted.bam.bai").is_file());

    let quickcheck = run_tool(
        &home,
        &workdir,
        &report,
        ["quickcheck", "-v", "aln sorted.bam"],
    );
    assert_success(&quickcheck, "BAM quickcheck");
    assert!(quickcheck.stdout.is_empty());

    let count = run_tool(&home, &workdir, &report, ["view", "-c", "aln sorted.bam"]);
    assert_success(&count, "BAM count");
    assert_eq!(String::from_utf8_lossy(&count.stdout).trim(), "4");

    let region = run_tool(
        &home,
        &workdir,
        &report,
        ["view", "aln sorted.bam", "chr1:10-24"],
    );
    assert_success(&region, "indexed region query");
    let region_text = String::from_utf8_lossy(&region.stdout);
    let region_names: Vec<_> = region_text
        .lines()
        .map(|line| line.split('\t').next().expect("SAM query name"))
        .collect();
    assert_eq!(region_names, ["read_chr1_a", "read_chr1_b"]);

    let flagstat = run_tool(
        &home,
        &workdir,
        &report,
        ["flagstat", "-O", "json", "aln sorted.bam"],
    );
    assert_success(&flagstat, "flagstat JSON");
    let flagstat: Value = serde_json::from_slice(&flagstat.stdout).expect("flagstat JSON output");
    assert!(flagstat.is_object());

    let stats = run_tool(&home, &workdir, &report, ["stats", "aln sorted.bam"]);
    assert_success(&stats, "samtools stats");
    assert!(String::from_utf8_lossy(&stats.stdout).contains("SN\traw total sequences:\t4"));

    // Reference indexing and a CRAM round trip exercise reference mounts and
    // network-independent decoding.
    let faidx = run_tool(&home, &workdir, &report, ["faidx", "reference.fa"]);
    assert_success(&faidx, "FASTA index");
    assert!(workdir.join("reference.fa.fai").is_file());

    let cram = run_tool(
        &home,
        &workdir,
        &report,
        [
            "view",
            "-C",
            "-T",
            "reference.fa",
            "-o",
            "aln.cram",
            "aln sorted.bam",
        ],
    );
    assert_success(&cram, "BAM to CRAM");
    assert!(workdir.join("aln.cram").is_file());

    let cram_count = run_tool(
        &home,
        &workdir,
        &report,
        ["view", "-T", "reference.fa", "-c", "aln.cram"],
    );
    assert_success(&cram_count, "CRAM decode");
    assert_eq!(String::from_utf8_lossy(&cram_count.stdout).trim(), "4");

    // Upstream failures must remain failures at the BL Convert boundary.
    let bad_option = run_tool(
        &home,
        &workdir,
        &report,
        [
            "view",
            "--definitely-not-a-samtools-option",
            "aln sorted.bam",
        ],
    );
    assert!(
        !bad_option.status.success(),
        "invalid option unexpectedly passed"
    );
    assert!(String::from_utf8_lossy(&bad_option.stderr).contains("exited with code"));

    let malformed = run_tool(
        &home,
        &workdir,
        &report,
        ["view", "-b", "-o", "malformed.bam", "malformed.sam"],
    );
    assert!(
        !malformed.status.success(),
        "malformed SAM unexpectedly passed"
    );

    // If the workspace `bl` binary was built, verify its delegating shortcut
    // against the same registered backend and state directory.
    if let Some(bl) = inferred_bl_executable() {
        let delegated = Command::new(bl)
            .args(["convert", "tool", "status", "samtools", "--json"])
            .env("BL_CONVERT_HOME", &home)
            .output()
            .expect("start bl convert delegation");
        assert_success(&delegated, "bl convert delegation");
        let delegated: Value =
            serde_json::from_slice(&delegated.stdout).expect("delegated status JSON");
        assert_eq!(delegated["backend"], backend);
    } else {
        eprintln!("workspace bl executable not built; delegation check not run");
    }
}
