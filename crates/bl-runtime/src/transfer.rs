use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;

/// Returns the list of (name, arity) for all transfer builtins.
pub fn transfer_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        // FTP
        ("ftp_download", Arity::Range(1, 2)),
        ("ftp_list", Arity::Exact(1)),
        ("ftp_upload", Arity::Exact(2)),
        // SFTP/SCP
        ("sftp_download", Arity::Range(1, 2)),
        ("sftp_upload", Arity::Exact(2)),
        ("scp", Arity::Exact(2)),
        // S3
        ("s3_download", Arity::Range(1, 2)),
        ("s3_upload", Arity::Exact(2)),
        ("s3_list", Arity::Range(1, 2)),
        // GCS
        ("gcs_download", Arity::Range(1, 2)),
        ("gcs_upload", Arity::Exact(2)),
        // rsync
        ("rsync", Arity::Range(2, 3)),
        // Aspera
        ("aspera_download", Arity::Range(1, 2)),
        // SRA Toolkit
        ("sra_prefetch", Arity::Range(1, 2)),
        ("sra_fastq", Arity::Range(1, 2)),
    ]
}

pub fn is_transfer_builtin(name: &str) -> bool {
    matches!(
        name,
        "ftp_download"
            | "ftp_list"
            | "ftp_upload"
            | "sftp_download"
            | "sftp_upload"
            | "scp"
            | "s3_download"
            | "s3_upload"
            | "s3_list"
            | "gcs_download"
            | "gcs_upload"
            | "rsync"
            | "aspera_download"
            | "sra_prefetch"
            | "sra_fastq"
    )
}

pub fn call_transfer_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "ftp_download" => builtin_ftp_download(args),
        "ftp_list" => builtin_ftp_list(args),
        "ftp_upload" => builtin_ftp_upload(args),
        "sftp_download" => builtin_sftp_download(args),
        "sftp_upload" => builtin_sftp_upload(args),
        "scp" => builtin_scp(args),
        "s3_download" => builtin_s3_download(args),
        "s3_upload" => builtin_s3_upload(args),
        "s3_list" => builtin_s3_list(args),
        "gcs_download" => builtin_gcs_download(args),
        "gcs_upload" => builtin_gcs_upload(args),
        "rsync" => builtin_rsync(args),
        "aspera_download" => builtin_aspera_download(args),
        "sra_prefetch" => builtin_sra_prefetch(args),
        "sra_fastq" => builtin_sra_fastq(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown transfer builtin '{name}'"),
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

fn default_data_dir(subdir: &str) -> String {
    let base = std::env::var("BIOLANG_DATA_DIR").unwrap_or_else(|_| {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        format!("{home}/.biolang")
    });
    let dir = format!("{base}/{subdir}");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1_073_741_824 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    }
}

/// Derive a local filename from a URL path.
fn filename_from_url(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    path.rsplit('/')
        .next()
        .unwrap_or("download")
        .to_string()
}

/// Run a subprocess with live stderr output and return (stdout, exit_code).
fn run_with_progress(cmd: &str, args: &[&str], func: &str) -> Result<(String, i32)> {
    let child = Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit()) // live progress
        .spawn()
        .map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("{func}(): '{}' not found — {e}", cmd),
                None,
            )
        })?;

    let output = child.wait_with_output().map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("{func}() failed: {e}"),
            None,
        )
    })?;

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok((stdout, code))
}

/// Run subprocess, fail if non-zero exit.
fn run_checked(cmd: &str, args: &[&str], func: &str) -> Result<String> {
    let (stdout, code) = run_with_progress(cmd, args, func)?;
    if code != 0 {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("{func}() exited with code {code}"),
            None,
        ));
    }
    Ok(stdout)
}

// ── FTP (native via suppaftp) ───────────────────────────────────

/// Parse an FTP URL into (host, port, path, user, pass).
fn parse_ftp_url(url: &str) -> Result<(String, u16, String, String, String)> {
    // ftp://[user:pass@]host[:port]/path
    let stripped = url
        .strip_prefix("ftp://")
        .or_else(|| url.strip_prefix("ftps://"))
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "ftp URL must start with ftp:// or ftps://",
                None,
            )
        })?;

    let (auth, rest) = if let Some(at_pos) = stripped.find('@') {
        (Some(&stripped[..at_pos]), &stripped[at_pos + 1..])
    } else {
        (None, stripped)
    };

    let (user, pass) = if let Some(auth) = auth {
        if let Some(colon) = auth.find(':') {
            (auth[..colon].to_string(), auth[colon + 1..].to_string())
        } else {
            (auth.to_string(), String::new())
        }
    } else {
        ("anonymous".to_string(), "biolang@".to_string())
    };

    let (host_port, path) = if let Some(slash) = rest.find('/') {
        (&rest[..slash], &rest[slash..])
    } else {
        (rest, "/")
    };

    let (host, port) = if let Some(colon) = host_port.rfind(':') {
        let p = host_port[colon + 1..].parse::<u16>().unwrap_or(21);
        (host_port[..colon].to_string(), p)
    } else {
        (host_port.to_string(), 21)
    };

    Ok((host, port, path.to_string(), user, pass))
}

/// ftp_download(url, path?) → {path, size}
fn builtin_ftp_download(args: Vec<Value>) -> Result<Value> {
    let url = require_str(&args[0], "ftp_download")?;
    let (host, port, remote_path, user, pass) = parse_ftp_url(url)?;

    let local_path = if args.len() > 1 {
        require_str(&args[1], "ftp_download")?.to_string()
    } else {
        let dir = default_data_dir("downloads");
        let fname = filename_from_url(url);
        format!("{dir}/{fname}")
    };

    eprintln!("  ftp: connecting to {host}:{port} ...");

    let addr = format!("{host}:{port}");
    let mut ftp = suppaftp::FtpStream::connect(&addr).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_download() connect failed: {e}"),
            None,
        )
    })?;

    ftp.login(&user, &pass).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_download() login failed: {e}"),
            None,
        )
    })?;

    // Switch to binary mode
    ftp.transfer_type(suppaftp::types::FileType::Binary)
        .map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("ftp_download() binary mode failed: {e}"),
                None,
            )
        })?;

    // Get file size if possible
    let remote_size = ftp.size(&remote_path).ok();

    eprintln!(
        "  ftp: downloading {remote_path} {}",
        remote_size
            .map(|s| format!("({})", format_bytes(s as u64)))
            .unwrap_or_default()
    );

    // Stream the file with progress
    let mut file = std::fs::File::create(&local_path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_download() cannot create file: {e}"),
            None,
        )
    })?;

    let mut reader = ftp.retr_as_stream(&remote_path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_download() retrieve failed: {e}"),
            None,
        )
    })?;

    let mut buf = [0u8; 65536];
    let mut downloaded: u64 = 0;
    let mut last_print: u64 = 0;

    loop {
        let n = std::io::Read::read(&mut reader, &mut buf).map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("ftp_download() read error: {e}"),
                None,
            )
        })?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("ftp_download() write error: {e}"),
                None,
            )
        })?;
        downloaded += n as u64;

        if downloaded - last_print >= 1_048_576 {
            last_print = downloaded;
            if let Some(total) = remote_size {
                let pct = (downloaded as f64 / total as f64 * 100.0) as u32;
                eprint!(
                    "\r  ftp: {:.1} / {:.1} MB ({pct}%)  ",
                    downloaded as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0
                );
            } else {
                eprint!(
                    "\r  ftp: {:.1} MB  ",
                    downloaded as f64 / 1_048_576.0
                );
            }
        }
    }

    // Finalize transfer
    let _ = ftp.finalize_retr_stream(reader);
    let _ = ftp.quit();

    if last_print > 0 {
        eprintln!(
            "\r  ftp: {} ({})                    ",
            local_path,
            format_bytes(downloaded)
        );
    }

    let mut rec = HashMap::new();
    rec.insert("path".to_string(), Value::Str(local_path));
    rec.insert("size".to_string(), Value::Int(downloaded as i64));
    rec.insert("protocol".to_string(), Value::Str("ftp".into()));
    Ok(Value::Record(rec))
}

/// ftp_list(url) → List of Records {name, size, type}
fn builtin_ftp_list(args: Vec<Value>) -> Result<Value> {
    let url = require_str(&args[0], "ftp_list")?;
    let (host, port, remote_path, user, pass) = parse_ftp_url(url)?;

    let addr = format!("{host}:{port}");
    let mut ftp = suppaftp::FtpStream::connect(&addr).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_list() connect failed: {e}"),
            None,
        )
    })?;

    ftp.login(&user, &pass).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_list() login failed: {e}"),
            None,
        )
    })?;

    let entries = ftp.nlst(Some(&remote_path)).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_list() failed: {e}"),
            None,
        )
    })?;

    let _ = ftp.quit();

    let items: Vec<Value> = entries
        .into_iter()
        .filter(|e| !e.is_empty())
        .map(|entry| {
            let name = entry
                .rsplit('/')
                .next()
                .unwrap_or(&entry)
                .to_string();
            let mut rec = HashMap::new();
            rec.insert("name".to_string(), Value::Str(name));
            rec.insert("path".to_string(), Value::Str(entry));
            Value::Record(rec)
        })
        .collect();

    Ok(Value::List(items))
}

/// ftp_upload(path, url) → {size, protocol}
fn builtin_ftp_upload(args: Vec<Value>) -> Result<Value> {
    let local_path = require_str(&args[0], "ftp_upload")?;
    let url = require_str(&args[1], "ftp_upload")?;
    let (host, port, remote_path, user, pass) = parse_ftp_url(url)?;

    let content = std::fs::read(local_path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_upload() cannot read file: {e}"),
            None,
        )
    })?;
    let size = content.len();

    let addr = format!("{host}:{port}");
    let mut ftp = suppaftp::FtpStream::connect(&addr).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_upload() connect failed: {e}"),
            None,
        )
    })?;

    ftp.login(&user, &pass).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_upload() login failed: {e}"),
            None,
        )
    })?;

    ftp.transfer_type(suppaftp::types::FileType::Binary)
        .map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("ftp_upload() binary mode failed: {e}"),
                None,
            )
        })?;

    eprintln!("  ftp: uploading {} ({}) → {remote_path}", local_path, format_bytes(size as u64));

    let mut cursor = std::io::Cursor::new(content);
    ftp.put_file(&remote_path, &mut cursor).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ftp_upload() failed: {e}"),
            None,
        )
    })?;

    let _ = ftp.quit();

    let mut rec = HashMap::new();
    rec.insert("size".to_string(), Value::Int(size as i64));
    rec.insert("protocol".to_string(), Value::Str("ftp".into()));
    Ok(Value::Record(rec))
}

// ── SFTP/SCP (subprocess) ───────────────────────────────────────

/// sftp_download(url, path?) → {path, size}
/// URL format: user@host:/remote/path or sftp://user@host/path
fn builtin_sftp_download(args: Vec<Value>) -> Result<Value> {
    let remote = require_str(&args[0], "sftp_download")?;
    let local = if args.len() > 1 {
        require_str(&args[1], "sftp_download")?.to_string()
    } else {
        let dir = default_data_dir("downloads");
        let fname = filename_from_url(remote);
        format!("{dir}/{fname}")
    };

    // Normalize sftp:// URL to scp format
    let scp_src = normalize_ssh_url(remote);

    eprintln!("  scp: downloading {scp_src} → {local}");
    run_checked("scp", &["-q", &scp_src, &local], "sftp_download")?;

    let size = std::fs::metadata(&local)
        .map(|m| m.len())
        .unwrap_or(0);

    eprintln!("  scp: done ({})", format_bytes(size));

    let mut rec = HashMap::new();
    rec.insert("path".to_string(), Value::Str(local));
    rec.insert("size".to_string(), Value::Int(size as i64));
    rec.insert("protocol".to_string(), Value::Str("scp".into()));
    Ok(Value::Record(rec))
}

/// sftp_upload(path, url) → {size}
fn builtin_sftp_upload(args: Vec<Value>) -> Result<Value> {
    let local = require_str(&args[0], "sftp_upload")?;
    let remote = require_str(&args[1], "sftp_upload")?;

    let size = std::fs::metadata(local)
        .map(|m| m.len())
        .unwrap_or(0);

    let scp_dest = normalize_ssh_url(remote);

    eprintln!("  scp: uploading {local} ({}) → {scp_dest}", format_bytes(size));
    run_checked("scp", &["-q", local, &scp_dest], "sftp_upload")?;

    let mut rec = HashMap::new();
    rec.insert("size".to_string(), Value::Int(size as i64));
    rec.insert("protocol".to_string(), Value::Str("scp".into()));
    Ok(Value::Record(rec))
}

/// scp(source, dest) → {exit_code}  (generic scp wrapper)
fn builtin_scp(args: Vec<Value>) -> Result<Value> {
    let src = require_str(&args[0], "scp")?;
    let dst = require_str(&args[1], "scp")?;
    run_checked("scp", &["-r", src, dst], "scp")?;

    let mut rec = HashMap::new();
    rec.insert("source".to_string(), Value::Str(src.to_string()));
    rec.insert("dest".to_string(), Value::Str(dst.to_string()));
    rec.insert("protocol".to_string(), Value::Str("scp".into()));
    Ok(Value::Record(rec))
}

fn normalize_ssh_url(url: &str) -> String {
    if url.starts_with("sftp://") || url.starts_with("ssh://") {
        // sftp://user@host/path → user@host:/path
        let stripped = url.split("://").nth(1).unwrap_or(url);
        if let Some(slash) = stripped.find('/') {
            format!("{}:{}", &stripped[..slash], &stripped[slash..])
        } else {
            stripped.to_string()
        }
    } else {
        url.to_string() // already user@host:/path format
    }
}

// ── S3 (aws CLI subprocess) ─────────────────────────────────────

/// s3_download(s3_url, path?) → {path, size}
fn builtin_s3_download(args: Vec<Value>) -> Result<Value> {
    let s3_url = require_str(&args[0], "s3_download")?;
    let local = if args.len() > 1 {
        require_str(&args[1], "s3_download")?.to_string()
    } else {
        let dir = default_data_dir("downloads");
        let fname = filename_from_url(s3_url);
        format!("{dir}/{fname}")
    };

    eprintln!("  s3: downloading {s3_url}");
    run_checked("aws", &["s3", "cp", s3_url, &local], "s3_download")?;

    let size = std::fs::metadata(&local)
        .map(|m| m.len())
        .unwrap_or(0);

    let mut rec = HashMap::new();
    rec.insert("path".to_string(), Value::Str(local));
    rec.insert("size".to_string(), Value::Int(size as i64));
    rec.insert("protocol".to_string(), Value::Str("s3".into()));
    Ok(Value::Record(rec))
}

/// s3_upload(path, s3_url) → {size}
fn builtin_s3_upload(args: Vec<Value>) -> Result<Value> {
    let local = require_str(&args[0], "s3_upload")?;
    let s3_url = require_str(&args[1], "s3_upload")?;

    let size = std::fs::metadata(local)
        .map(|m| m.len())
        .unwrap_or(0);

    eprintln!("  s3: uploading {local} ({}) → {s3_url}", format_bytes(size));
    run_checked("aws", &["s3", "cp", local, s3_url], "s3_upload")?;

    let mut rec = HashMap::new();
    rec.insert("size".to_string(), Value::Int(size as i64));
    rec.insert("protocol".to_string(), Value::Str("s3".into()));
    Ok(Value::Record(rec))
}

/// s3_list(s3_url, opts?) → List of Records
fn builtin_s3_list(args: Vec<Value>) -> Result<Value> {
    let s3_url = require_str(&args[0], "s3_list")?;

    let mut cmd_args = vec!["s3", "ls", s3_url];
    // Check for recursive option
    let recursive = if args.len() > 1 {
        match &args[1] {
            Value::Bool(true) => true,
            Value::Record(r) => r
                .get("recursive")
                .map(|v| matches!(v, Value::Bool(true)))
                .unwrap_or(false),
            _ => false,
        }
    } else {
        false
    };
    if recursive {
        cmd_args.push("--recursive");
    }

    let stdout = run_checked("aws", &cmd_args, "s3_list")?;

    let items: Vec<Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let mut rec = HashMap::new();
            rec.insert("entry".to_string(), Value::Str(line.trim().to_string()));
            // Try to parse: date time size name
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                rec.insert("name".to_string(), Value::Str(parts[3..].join(" ")));
                if let Ok(size) = parts[2].parse::<i64>() {
                    rec.insert("size".to_string(), Value::Int(size));
                }
            } else if parts.len() >= 2 && parts[1].starts_with("PRE") {
                // Directory prefix
                let name = parts.get(2).unwrap_or(&"");
                rec.insert("name".to_string(), Value::Str(name.to_string()));
                rec.insert("type".to_string(), Value::Str("dir".into()));
            }
            Value::Record(rec)
        })
        .collect();

    Ok(Value::List(items))
}

// ── GCS (gsutil subprocess) ─────────────────────────────────────

/// gcs_download(gs_url, path?) → {path, size}
fn builtin_gcs_download(args: Vec<Value>) -> Result<Value> {
    let gs_url = require_str(&args[0], "gcs_download")?;
    let local = if args.len() > 1 {
        require_str(&args[1], "gcs_download")?.to_string()
    } else {
        let dir = default_data_dir("downloads");
        let fname = filename_from_url(gs_url);
        format!("{dir}/{fname}")
    };

    eprintln!("  gcs: downloading {gs_url}");
    run_checked("gsutil", &["cp", gs_url, &local], "gcs_download")?;

    let size = std::fs::metadata(&local)
        .map(|m| m.len())
        .unwrap_or(0);

    let mut rec = HashMap::new();
    rec.insert("path".to_string(), Value::Str(local));
    rec.insert("size".to_string(), Value::Int(size as i64));
    rec.insert("protocol".to_string(), Value::Str("gcs".into()));
    Ok(Value::Record(rec))
}

/// gcs_upload(path, gs_url) → {size}
fn builtin_gcs_upload(args: Vec<Value>) -> Result<Value> {
    let local = require_str(&args[0], "gcs_upload")?;
    let gs_url = require_str(&args[1], "gcs_upload")?;

    let size = std::fs::metadata(local)
        .map(|m| m.len())
        .unwrap_or(0);

    eprintln!("  gcs: uploading {local} ({}) → {gs_url}", format_bytes(size));
    run_checked("gsutil", &["cp", local, gs_url], "gcs_upload")?;

    let mut rec = HashMap::new();
    rec.insert("size".to_string(), Value::Int(size as i64));
    rec.insert("protocol".to_string(), Value::Str("gcs".into()));
    Ok(Value::Record(rec))
}

// ── rsync (subprocess) ──────────────────────────────────────────

/// rsync(source, dest, opts?) → {exit_code}
/// opts Record: {compress: true, delete: false, exclude: "pattern"}
fn builtin_rsync(args: Vec<Value>) -> Result<Value> {
    let src = require_str(&args[0], "rsync")?;
    let dst = require_str(&args[1], "rsync")?;

    let mut cmd_args: Vec<String> = vec![
        "-av".to_string(),
        "--progress".to_string(),
    ];

    if args.len() > 2 {
        if let Value::Record(opts) = &args[2] {
            if matches!(opts.get("compress"), Some(Value::Bool(true))) {
                cmd_args.push("-z".to_string());
            }
            if matches!(opts.get("delete"), Some(Value::Bool(true))) {
                cmd_args.push("--delete".to_string());
            }
            if let Some(Value::Str(pattern)) = opts.get("exclude") {
                cmd_args.push(format!("--exclude={pattern}"));
            }
        }
    }

    cmd_args.push(src.to_string());
    cmd_args.push(dst.to_string());

    let refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
    run_checked("rsync", &refs, "rsync")?;

    let mut rec = HashMap::new();
    rec.insert("source".to_string(), Value::Str(src.to_string()));
    rec.insert("dest".to_string(), Value::Str(dst.to_string()));
    rec.insert("protocol".to_string(), Value::Str("rsync".into()));
    Ok(Value::Record(rec))
}

// ── Aspera (ascp subprocess) ────────────────────────────────────

/// aspera_download(url, path?) → {path}
/// Wraps ascp for high-speed transfers from EBI/NCBI.
/// URL: era-fasp@fasp.sra.ebi.ac.uk:/vol1/... or anonftp@ftp.ncbi.nlm.nih.gov:/...
fn builtin_aspera_download(args: Vec<Value>) -> Result<Value> {
    let remote = require_str(&args[0], "aspera_download")?;
    let local = if args.len() > 1 {
        require_str(&args[1], "aspera_download")?.to_string()
    } else {
        default_data_dir("downloads")
    };

    eprintln!("  aspera: downloading {remote}");

    // ascp needs the OpenSSH key path — check common locations
    let key_candidates = [
        std::env::var("ASPERA_SCP_PASS")
            .ok()
            .unwrap_or_default(),
        expand_home("~/.aspera/connect/etc/asperaweb_id_dsa.openssh"),
        expand_home("~/.aspera/cli/etc/asperaweb_id_dsa.openssh"),
    ];

    let key_path = key_candidates
        .iter()
        .find(|p| !p.is_empty() && std::path::Path::new(p).exists());

    let mut cmd_args = vec![
        "-QT",
        "-l", "300m",
        "-P", "33001",
    ];

    let key_str;
    if let Some(key) = key_path {
        key_str = key.clone();
        cmd_args.push("-i");
        cmd_args.push(&key_str);
    }

    cmd_args.push(remote);
    cmd_args.push(&local);

    run_checked("ascp", &cmd_args, "aspera_download")?;

    let mut rec = HashMap::new();
    rec.insert("path".to_string(), Value::Str(local));
    rec.insert("protocol".to_string(), Value::Str("aspera".into()));
    Ok(Value::Record(rec))
}

fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        format!("{home}/{rest}")
    } else {
        path.to_string()
    }
}

// ── SRA Toolkit (subprocess) ────────────────────────────────────

/// sra_prefetch(accession, path?) → {path, accession}
/// Downloads SRA data using prefetch.
fn builtin_sra_prefetch(args: Vec<Value>) -> Result<Value> {
    let accession = require_str(&args[0], "sra_prefetch")?;
    let output_dir = if args.len() > 1 {
        require_str(&args[1], "sra_prefetch")?.to_string()
    } else {
        default_data_dir("sra")
    };

    eprintln!("  sra: prefetching {accession} → {output_dir}");
    run_checked(
        "prefetch",
        &[accession, "--output-directory", &output_dir, "--progress"],
        "sra_prefetch",
    )?;

    let sra_path = format!("{output_dir}/{accession}/{accession}.sra");

    let mut rec = HashMap::new();
    rec.insert("path".to_string(), Value::Str(sra_path));
    rec.insert("accession".to_string(), Value::Str(accession.to_string()));
    rec.insert("output_dir".to_string(), Value::Str(output_dir));
    Ok(Value::Record(rec))
}

/// sra_fastq(accession, path?) → {path, accession}
/// Converts SRA to FASTQ using fasterq-dump.
fn builtin_sra_fastq(args: Vec<Value>) -> Result<Value> {
    let accession = require_str(&args[0], "sra_fastq")?;
    let output_dir = if args.len() > 1 {
        require_str(&args[1], "sra_fastq")?.to_string()
    } else {
        default_data_dir("fastq")
    };

    let _ = std::fs::create_dir_all(&output_dir);

    eprintln!("  sra: converting {accession} to FASTQ → {output_dir}");
    run_checked(
        "fasterq-dump",
        &[accession, "--outdir", &output_dir, "--progress", "--split-3"],
        "sra_fastq",
    )?;

    // Detect output files
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&output_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(accession) && name.ends_with(".fastq") {
                files.push(Value::Str(
                    entry.path().to_string_lossy().to_string(),
                ));
            }
        }
    }

    let mut rec = HashMap::new();
    rec.insert("accession".to_string(), Value::Str(accession.to_string()));
    rec.insert("output_dir".to_string(), Value::Str(output_dir));
    rec.insert("files".to_string(), Value::List(files));
    Ok(Value::Record(rec))
}

// Tests are inline because they exercise private helpers: parse_ftp_url, normalize_ssh_url,
// filename_from_url, format_bytes, run_with_progress, run_checked.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ftp_url_anonymous() {
        let (host, port, path, user, pass) =
            parse_ftp_url("ftp://ftp.ncbi.nlm.nih.gov/genomes/refseq").unwrap();
        assert_eq!(host, "ftp.ncbi.nlm.nih.gov");
        assert_eq!(port, 21);
        assert_eq!(path, "/genomes/refseq");
        assert_eq!(user, "anonymous");
    }

    #[test]
    fn test_parse_ftp_url_with_auth() {
        let (host, port, path, user, pass) =
            parse_ftp_url("ftp://user:secret@myhost.com:2121/data/file.gz").unwrap();
        assert_eq!(host, "myhost.com");
        assert_eq!(port, 2121);
        assert_eq!(path, "/data/file.gz");
        assert_eq!(user, "user");
        assert_eq!(pass, "secret");
    }

    #[test]
    fn test_normalize_ssh_url() {
        assert_eq!(
            normalize_ssh_url("sftp://user@host/path/file"),
            "user@host:/path/file"
        );
        assert_eq!(
            normalize_ssh_url("user@host:/path/file"),
            "user@host:/path/file"
        );
    }

    #[test]
    fn test_is_transfer_builtin() {
        assert!(is_transfer_builtin("ftp_download"));
        assert!(is_transfer_builtin("s3_download"));
        assert!(is_transfer_builtin("sra_fastq"));
        assert!(is_transfer_builtin("rsync"));
        assert!(!is_transfer_builtin("http_get"));
    }

    #[test]
    fn test_filename_from_url() {
        assert_eq!(
            filename_from_url("ftp://host/path/to/file.gz"),
            "file.gz"
        );
        assert_eq!(
            filename_from_url("s3://bucket/key/data.bam?version=1"),
            "data.bam"
        );
    }

    // Edge case: parse_ftp_url with IPv6 address
    #[test]
    fn test_parse_ftp_url_ipv6() {
        // IPv6 addresses in URLs are bracketed: ftp://[::1]/path
        // Our parser doesn't handle brackets specially, but should not panic
        let result = parse_ftp_url("ftp://[::1]/data/file.gz");
        // The parser will treat "[" as part of host — it won't crash
        assert!(result.is_ok());
    }

    // Edge case: parse_ftp_url with path containing spaces (percent-encoded)
    #[test]
    fn test_parse_ftp_url_path_with_spaces() {
        let (host, port, path, user, _pass) =
            parse_ftp_url("ftp://ftp.example.com/path%20with%20spaces/file.txt").unwrap();
        assert_eq!(host, "ftp.example.com");
        assert_eq!(port, 21);
        assert!(path.contains("path%20with%20spaces"));
        assert_eq!(user, "anonymous");
    }

    // Edge case: parse_ftp_url with no path
    #[test]
    fn test_parse_ftp_url_no_path() {
        let (host, port, path, _user, _pass) =
            parse_ftp_url("ftp://ftp.example.com").unwrap();
        assert_eq!(host, "ftp.example.com");
        assert_eq!(port, 21);
        assert_eq!(path, "/");
    }

    // Edge case: parse_ftp_url with ftps:// scheme
    #[test]
    fn test_parse_ftp_url_ftps() {
        let (host, _port, path, _user, _pass) =
            parse_ftp_url("ftps://secure.host.com/files/data.gz").unwrap();
        assert_eq!(host, "secure.host.com");
        assert_eq!(path, "/files/data.gz");
    }

    // Edge case: parse_ftp_url with invalid scheme
    #[test]
    fn test_parse_ftp_url_invalid_scheme() {
        let result = parse_ftp_url("http://example.com/file");
        assert!(result.is_err());
    }

    // Edge case: filename_from_url with query parameters
    #[test]
    fn test_filename_from_url_query_params() {
        assert_eq!(
            filename_from_url("https://example.com/path/genome.fa?auth=token&ver=2"),
            "genome.fa"
        );
    }

    // Edge case: filename_from_url with fragment
    #[test]
    fn test_filename_from_url_fragment() {
        // Fragment is after query, so the ?-split handles it
        assert_eq!(
            filename_from_url("https://example.com/data.vcf#section1"),
            "data.vcf#section1" // fragment not stripped (only ? is split)
        );
    }

    // Edge case: filename_from_url with no path (just host)
    #[test]
    fn test_filename_from_url_no_path() {
        // If URL has no path segments, should return "download" or the host
        let name = filename_from_url("https://example.com");
        assert!(!name.is_empty());
    }

    // Edge case: filename_from_url with trailing slash
    #[test]
    fn test_filename_from_url_trailing_slash() {
        let name = filename_from_url("https://example.com/dir/");
        // Last segment after '/' is empty string
        assert_eq!(name, "");
    }

    // Edge case: is_transfer_builtin for all known builtins
    #[test]
    fn test_is_transfer_builtin_all_known() {
        let known = [
            "ftp_download", "ftp_list", "ftp_upload",
            "sftp_download", "sftp_upload", "scp",
            "s3_download", "s3_upload", "s3_list",
            "gcs_download", "gcs_upload",
            "rsync",
            "aspera_download",
            "sra_prefetch", "sra_fastq",
        ];
        for name in &known {
            assert!(is_transfer_builtin(name), "{name} should be a transfer builtin");
        }
        // Negative cases
        assert!(!is_transfer_builtin("http_get"));
        assert!(!is_transfer_builtin("download"));
        assert!(!is_transfer_builtin(""));
    }

    // Edge case: normalize_ssh_url with ssh:// scheme
    #[test]
    fn test_normalize_ssh_url_ssh_scheme() {
        assert_eq!(
            normalize_ssh_url("ssh://user@host/remote/path"),
            "user@host:/remote/path"
        );
    }

    // Edge case: normalize_ssh_url with no path
    #[test]
    fn test_normalize_ssh_url_no_path() {
        assert_eq!(
            normalize_ssh_url("sftp://user@host"),
            "user@host"
        );
    }

    // Edge case: format_bytes boundary values
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert!(format_bytes(1024).contains("KB"));
        assert!(format_bytes(1_048_576).contains("MB"));
        assert!(format_bytes(1_073_741_824).contains("GB"));
    }
}
