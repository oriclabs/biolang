//! Capability registry, environment probe, and backend selection.
//!
//! A *capability* is an analysis step (e.g. `read_anndata_h5ad`,
//! `differential_expression`) that can be served by one or more *backends*
//! (native Rust, a local tool, a Python module, or a container image), listed
//! in precedence order. This module:
//!
//! - probes the live environment ([`probe_env`]),
//! - reports per-capability readiness (`bl doctor` → [`doctor_report`]),
//! - and picks a backend for a given context, with a human-readable reason
//!   for provenance/notification ([`select_backend`]).
//!
//! It is pure `std` — no interpreter dependency — so both the CLI and the
//! runtime dispatcher can share it.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ── Model ────────────────────────────────────────────────────────────────────

/// Where a capability can run, in the author's order of preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backend {
    /// Built into `bl` unconditionally.
    Native,
    /// Built into `bl` only when compiled with the named cargo feature.
    NativeFeature(&'static str),
    /// An external executable expected on `PATH`.
    LocalTool(&'static str),
    /// A Python module (implies a working `python`).
    Python(&'static str),
    /// A container image / BioContainers tool (implies a container runtime).
    Container(&'static str),
}

impl Backend {
    /// Short human label for reports.
    pub fn label(&self) -> String {
        match self {
            Backend::Native => "native".into(),
            Backend::NativeFeature(f) => format!("native (--features {f})"),
            Backend::LocalTool(t) => format!("tool: {t}"),
            Backend::Python(m) => format!("python: {m}"),
            Backend::Container(i) => format!("container: {i}"),
        }
    }

    /// What the user must do to make this backend available.
    pub fn fix_hint(&self) -> String {
        match self {
            Backend::Native => "already available".into(),
            Backend::NativeFeature(f) => format!("rebuild with `cargo build --features {f}`"),
            Backend::LocalTool(t) => format!("install `{t}` and put it on PATH"),
            Backend::Python(m) => format!("`pip install {m}` (or use a container)"),
            Backend::Container(i) => format!("install a container runtime, then `bl` will pull `{i}`"),
        }
    }
}

/// How native output relates to the reference implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fidelity {
    /// Native result is numerically equivalent to the reference tool.
    ReferenceEquivalent,
    /// Native result is fine for exploration but not a citeable reference.
    Exploration,
    /// Must use the reference tool; there is no trustworthy native path.
    ReferenceOnly,
}

impl Fidelity {
    pub fn label(&self) -> &'static str {
        match self {
            Fidelity::ReferenceEquivalent => "reference-equivalent",
            Fidelity::Exploration => "exploration-grade",
            Fidelity::ReferenceOnly => "reference-only",
        }
    }
}

/// An analysis step and the backends that can serve it.
#[derive(Debug, Clone)]
pub struct Capability {
    pub name: &'static str,
    pub description: &'static str,
    pub backends: Vec<Backend>,
    pub fidelity: Fidelity,
}

/// The registry of known capabilities.
pub fn registry() -> Vec<Capability> {
    use Backend::*;
    use Fidelity::*;
    vec![
        Capability {
            name: "read_10x",
            description: "Load 10x Genomics MTX matrices",
            backends: vec![Native],
            fidelity: ReferenceEquivalent,
        },
        Capability {
            name: "read_anndata_zarr",
            description: "Read/write AnnData in Zarr form (pure-Rust, no C deps)",
            backends: vec![Native, Python("anndata")],
            fidelity: ReferenceEquivalent,
        },
        Capability {
            name: "read_anndata_h5ad",
            description: "Read/write AnnData .h5ad (HDF5)",
            backends: vec![
                NativeFeature("h5ad"),
                Python("anndata"),
                Container("biocontainers/anndata"),
            ],
            fidelity: ReferenceEquivalent,
        },
        Capability {
            name: "differential_expression",
            description: "Wilcoxon rank-sum marker genes with BH-FDR",
            backends: vec![Native],
            fidelity: ReferenceEquivalent,
        },
        Capability {
            name: "dimreduce_pca",
            description: "Principal component analysis",
            backends: vec![Native],
            fidelity: ReferenceEquivalent,
        },
        Capability {
            name: "embed_umap",
            description: "UMAP / t-SNE embedding",
            backends: vec![Native, Python("scanpy")],
            fidelity: Exploration,
        },
        Capability {
            name: "cluster_leiden",
            description: "Community-detection clustering",
            backends: vec![Native, Python("leidenalg")],
            fidelity: Exploration,
        },
        Capability {
            name: "integrate_scvi",
            description: "Deep-learning batch integration (scVI/scANVI)",
            backends: vec![Python("scvi"), Container("biocontainers/scvi-tools")],
            fidelity: ReferenceOnly,
        },
        Capability {
            name: "pseudobulk_deseq2",
            description: "Pseudobulk differential expression (DESeq2)",
            backends: vec![Container("biocontainers/bioconductor-deseq2")],
            fidelity: ReferenceOnly,
        },
        Capability {
            name: "align_fastq",
            description: "FASTQ → count matrix (upstream alignment)",
            backends: vec![Container("biocontainers/star"), LocalTool("STAR")],
            fidelity: ReferenceOnly,
        },
        Capability {
            name: "notebook_pdf",
            description: "Compile notebooks to PDF via Typst",
            backends: vec![LocalTool("typst")],
            fidelity: ReferenceEquivalent,
        },
        Capability {
            name: "code_import",
            description: "Convert Python/R/notebooks to BioLang",
            backends: vec![Native],
            fidelity: ReferenceEquivalent,
        },
    ]
}

// ── Environment probe ────────────────────────────────────────────────────────

/// A snapshot of what the current machine can run.
#[derive(Debug, Clone, Default)]
pub struct EnvReport {
    pub os: String,
    pub c_compiler: Option<String>,
    pub cmake: bool,
    pub python: Option<String>,
    pub python_modules: BTreeMap<String, bool>,
    pub container_runtime: Option<String>,
    pub container_daemon_up: bool,
    pub compiled_features: Vec<&'static str>,
    pub tools_on_path: BTreeMap<String, bool>,
    pub ram_gb: Option<f64>,
}

/// True if the binary runs at all (found on PATH), regardless of exit code.
fn tool_present(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// True if the command runs *and* exits 0.
fn tool_succeeds(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Cargo features compiled into this build that gate native capabilities.
fn compiled_features() -> Vec<&'static str> {
    let mut features = Vec::new();
    if cfg!(feature = "h5ad") {
        features.push("h5ad");
    }
    if cfg!(feature = "zarr") {
        features.push("zarr");
    }
    features
}

/// Best-effort total physical RAM in GB.
fn total_ram_gb() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string("/proc/meminfo").ok()?;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                let kb: f64 = rest.trim().trim_end_matches("kB").trim().parse().ok()?;
                return Some(kb / 1_048_576.0);
            }
        }
        None
    }
    #[cfg(target_os = "windows")]
    {
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
            ])
            .stderr(Stdio::null())
            .output()
            .ok()?;
        let bytes: f64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        Some(bytes / 1_073_741_824.0)
    }
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()?;
        let bytes: f64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        Some(bytes / 1_073_741_824.0)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

/// Probe the live environment. Runs a handful of quick subprocesses.
pub fn probe_env() -> EnvReport {
    let c_compiler = if tool_present("cl", &[]) {
        Some("cl (MSVC)".into())
    } else if tool_succeeds("gcc", &["--version"]) {
        Some("gcc".into())
    } else if tool_succeeds("clang", &["--version"]) {
        Some("clang".into())
    } else {
        None
    };

    let python: Option<String> = if tool_succeeds("python", &["--version"]) {
        Some("python".into())
    } else if tool_succeeds("python3", &["--version"]) {
        Some("python3".into())
    } else {
        None
    };

    let py = python.clone().unwrap_or_else(|| "python".into());
    let mut python_modules = BTreeMap::new();
    if python.is_some() {
        for module in ["anndata", "scanpy", "scvi", "h5py"] {
            python_modules.insert(
                module.to_string(),
                tool_succeeds(&py, &["-c", &format!("import {module}")]),
            );
        }
    }

    let container_runtime = ["docker", "podman", "singularity", "apptainer"]
        .into_iter()
        .find(|rt| tool_present(rt, &["--version"]))
        .map(String::from);

    // Daemon check only meaningful for docker/podman.
    let container_daemon_up = match container_runtime.as_deref() {
        Some(rt @ ("docker" | "podman")) => tool_succeeds(rt, &["info"]),
        Some(_) => true, // singularity/apptainer are daemonless
        None => false,
    };

    let mut tools_on_path = BTreeMap::new();
    for tool in ["typst", "STAR", "samtools", "cmake"] {
        tools_on_path.insert(tool.to_string(), tool_present(tool, &["--version"]));
    }

    EnvReport {
        os: std::env::consts::OS.to_string(),
        c_compiler,
        cmake: tool_succeeds("cmake", &["--version"]),
        python,
        python_modules,
        container_runtime,
        container_daemon_up,
        compiled_features: compiled_features(),
        tools_on_path,
        ram_gb: total_ram_gb(),
    }
}

/// Probe the environment once per process (subprocess probes are cached).
pub fn cached_env() -> &'static EnvReport {
    static ENV: OnceLock<EnvReport> = OnceLock::new();
    ENV.get_or_init(probe_env)
}

/// Look up a capability by name.
pub fn find_capability(name: &str) -> Option<Capability> {
    registry().into_iter().find(|c| c.name == name)
}

// ── Availability + selection ─────────────────────────────────────────────────

/// Whether a specific backend can run in this environment.
pub fn backend_available(backend: &Backend, env: &EnvReport) -> bool {
    match backend {
        Backend::Native => true,
        Backend::NativeFeature(f) => env.compiled_features.contains(f),
        Backend::LocalTool(t) => *env.tools_on_path.get(*t).unwrap_or(&false),
        Backend::Python(m) => {
            env.python.is_some() && *env.python_modules.get(*m).unwrap_or(&false)
        }
        Backend::Container(_) => env.container_runtime.is_some() && env.container_daemon_up,
    }
}

/// Context for a selection decision.
#[derive(Debug, Clone, Default)]
pub struct SelectionContext {
    /// Prefer a pinned container for reproducibility, even when native exists.
    pub strict: bool,
    /// Dataset size, used to warn about the dense-matrix ceiling.
    pub n_cells: Option<usize>,
    /// Available RAM (GB); falls back to the probed value when None.
    pub ram_gb: Option<f64>,
}

/// The outcome of selecting a backend — carries the *reason*, so callers can
/// log it to a provenance ledger and surface it to the user.
#[derive(Debug, Clone)]
pub struct Decision {
    pub capability: String,
    pub backend: Option<Backend>,
    pub reason: String,
    pub warnings: Vec<String>,
}

/// Rough native dense-matrix cell ceiling for a RAM budget (assuming ~20k genes,
/// f64, and leaving ~half of RAM as headroom).
fn native_cell_ceiling(ram_gb: f64) -> usize {
    let genes = 20_000.0;
    let bytes = ram_gb * 0.5 * 1_073_741_824.0;
    (bytes / (8.0 * genes)) as usize
}

/// Pick a backend for `cap` under `ctx`. Precedence is the registry order
/// (native first), except in `strict` mode where an available container wins
/// for reproducibility.
pub fn select_backend(cap: &Capability, env: &EnvReport, ctx: &SelectionContext) -> Decision {
    let available: Vec<&Backend> = cap
        .backends
        .iter()
        .filter(|b| backend_available(b, env))
        .collect();

    let mut warnings = Vec::new();

    if available.is_empty() {
        let hint = cap
            .backends
            .first()
            .map(|b| b.fix_hint())
            .unwrap_or_default();
        return Decision {
            capability: cap.name.into(),
            backend: None,
            reason: format!("no backend available — {hint}"),
            warnings,
        };
    }

    // Strict mode: prefer a reproducible container if one is available.
    let (chosen, reason) = if ctx.strict {
        if let Some(container) = available
            .iter()
            .find(|b| matches!(b, Backend::Container(_)))
        {
            (
                (*container).clone(),
                "strict mode → reproducible pinned container".to_string(),
            )
        } else {
            (
                available[0].clone(),
                format!(
                    "strict mode, no container available → {}",
                    available[0].label()
                ),
            )
        }
    } else {
        (
            available[0].clone(),
            format!("first available by precedence → {}", available[0].label()),
        )
    };

    // Warn when running an exploration-grade native backend on a dataset that
    // likely exceeds the dense-matrix ceiling.
    if matches!(chosen, Backend::Native) && cap.fidelity == Fidelity::Exploration {
        if let (Some(n), Some(ram)) = (ctx.n_cells, ctx.ram_gb.or(env.ram_gb)) {
            let ceiling = native_cell_ceiling(ram);
            if n > ceiling {
                warnings.push(format!(
                    "{n} cells exceeds the native dense-matrix ceiling (~{ceiling} for {ram:.0} GB RAM); consider a container backend"
                ));
            }
        }
    }

    Decision {
        capability: cap.name.into(),
        backend: Some(chosen),
        reason,
        warnings,
    }
}

// ── `bl doctor` report ───────────────────────────────────────────────────────

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

/// Render the full `bl doctor` report as text.
pub fn doctor_report() -> String {
    let env = probe_env();
    let mut out = String::new();

    out.push_str("BioLang environment check (bl doctor)\n");
    out.push_str("=====================================\n\n");

    out.push_str("Environment\n-----------\n");
    out.push_str(&format!("  os                : {}\n", env.os));
    out.push_str(&format!(
        "  ram               : {}\n",
        env.ram_gb
            .map(|r| format!("{r:.0} GB"))
            .unwrap_or_else(|| "unknown".into())
    ));
    out.push_str(&format!(
        "  c compiler        : {}\n",
        env.c_compiler.clone().unwrap_or_else(|| "none".into())
    ));
    out.push_str(&format!("  cmake             : {}\n", yes_no(env.cmake)));
    out.push_str(&format!(
        "  python            : {}\n",
        env.python.clone().unwrap_or_else(|| "none".into())
    ));
    if !env.python_modules.is_empty() {
        let mods: Vec<String> = env
            .python_modules
            .iter()
            .map(|(m, ok)| format!("{m}={}", yes_no(*ok)))
            .collect();
        out.push_str(&format!("  python modules    : {}\n", mods.join(", ")));
    }
    out.push_str(&format!(
        "  container runtime : {}{}\n",
        env.container_runtime.clone().unwrap_or_else(|| "none".into()),
        match env.container_runtime {
            Some(_) if env.container_daemon_up => " (running)",
            Some(_) => " (not running)",
            None => "",
        }
    ));
    out.push_str(&format!(
        "  build features    : {}\n",
        if env.compiled_features.is_empty() {
            "none".to_string()
        } else {
            env.compiled_features.join(", ")
        }
    ));
    if let Some(ram) = env.ram_gb {
        out.push_str(&format!(
            "  native cell limit : ~{} cells (dense, ~20k genes)\n",
            native_cell_ceiling(ram)
        ));
    }

    out.push_str("\nCapabilities\n------------\n");
    let ctx = SelectionContext::default();
    for cap in registry() {
        let decision = select_backend(&cap, &env, &ctx);
        let (mark, detail) = match &decision.backend {
            Some(b) => ("[ok]  ", b.label()),
            None => ("[--]  ", decision.reason.clone()),
        };
        out.push_str(&format!(
            "  {mark}{:<26} {}\n",
            cap.name,
            detail
        ));
        out.push_str(&format!(
            "        {:<26} ({})\n",
            "",
            cap.fidelity.label()
        ));
        if decision.backend.is_none() {
            // List how to enable each declared backend.
            for b in &cap.backends {
                out.push_str(&format!("          - {}: {}\n", b.label(), b.fix_hint()));
            }
        }
    }

    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with(python: bool, modules: &[(&str, bool)], container: bool) -> EnvReport {
        let mut m = BTreeMap::new();
        for (k, v) in modules {
            m.insert(k.to_string(), *v);
        }
        EnvReport {
            os: "test".into(),
            python: python.then(|| "python".to_string()),
            python_modules: m,
            container_runtime: container.then(|| "docker".to_string()),
            container_daemon_up: container,
            ram_gb: Some(16.0),
            ..Default::default()
        }
    }

    #[test]
    fn native_always_available() {
        let env = env_with(false, &[], false);
        let cap = &registry()[0]; // read_10x, native
        let d = select_backend(cap, &env, &SelectionContext::default());
        assert_eq!(d.backend, Some(Backend::Native));
    }

    #[test]
    fn falls_back_to_python_when_native_feature_missing() {
        // read_anndata_h5ad: [NativeFeature(h5ad), Python(anndata), Container]
        let env = env_with(true, &[("anndata", true)], false);
        let cap = registry().into_iter().find(|c| c.name == "read_anndata_h5ad").unwrap();
        let d = select_backend(&cap, &env, &SelectionContext::default());
        assert_eq!(d.backend, Some(Backend::Python("anndata")));
    }

    #[test]
    fn none_available_reports_fix_hint() {
        let env = env_with(false, &[], false);
        let cap = registry().into_iter().find(|c| c.name == "integrate_scvi").unwrap();
        let d = select_backend(&cap, &env, &SelectionContext::default());
        assert!(d.backend.is_none());
        assert!(d.reason.contains("pip install") || d.reason.contains("container"));
    }

    #[test]
    fn strict_mode_prefers_container() {
        // read_anndata_h5ad with both python and container available.
        let env = env_with(true, &[("anndata", true)], true);
        let cap = registry().into_iter().find(|c| c.name == "read_anndata_h5ad").unwrap();
        let d = select_backend(&cap, &env, &SelectionContext { strict: true, ..Default::default() });
        assert!(matches!(d.backend, Some(Backend::Container(_))));
    }

    #[test]
    fn warns_when_exploration_native_exceeds_ceiling() {
        let env = env_with(false, &[], false);
        let cap = registry().into_iter().find(|c| c.name == "embed_umap").unwrap();
        let ctx = SelectionContext { n_cells: Some(5_000_000), ram_gb: Some(16.0), ..Default::default() };
        let d = select_backend(&cap, &env, &ctx);
        assert_eq!(d.backend, Some(Backend::Native));
        assert!(!d.warnings.is_empty());
    }
}
