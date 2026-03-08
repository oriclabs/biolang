use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;

/// Returns the list of (name, arity) for all filesystem builtins.
pub fn fs_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("file_exists", Arity::Exact(1)),
        ("read_text", Arity::Exact(1)),
        ("write_text", Arity::Exact(2)),
        ("read_lines", Arity::Exact(1)),
        ("write_lines", Arity::Exact(2)),
        ("append_text", Arity::Exact(2)),
        ("list_dir", Arity::Exact(1)),
        ("mkdir", Arity::Exact(1)),
        ("basename", Arity::Exact(1)),
        ("dirname", Arity::Exact(1)),
        ("extension", Arity::Exact(1)),
        ("path_join", Arity::Exact(2)),
        ("abs_path", Arity::Exact(1)),
        ("file_size", Arity::Exact(1)),
        ("is_dir", Arity::Exact(1)),
        ("is_file", Arity::Exact(1)),
        ("remove", Arity::Exact(1)),
        ("copy_file", Arity::Exact(2)),
        ("rename_file", Arity::Exact(2)),
        ("temp_file", Arity::Exact(0)),
        ("temp_dir", Arity::Exact(0)),
        ("glob", Arity::Exact(1)),
        ("remove_dir", Arity::Range(1, 2)),
    ]
}

/// Check if a name is a known filesystem builtin.
pub fn is_fs_builtin(name: &str) -> bool {
    matches!(
        name,
        "file_exists"
            | "read_text"
            | "write_text"
            | "read_lines"
            | "write_lines"
            | "append_text"
            | "list_dir"
            | "mkdir"
            | "basename"
            | "dirname"
            | "extension"
            | "path_join"
            | "abs_path"
            | "file_size"
            | "is_dir"
            | "is_file"
            | "remove"
            | "copy_file"
            | "rename_file"
            | "temp_file"
            | "temp_dir"
            | "glob"
            | "remove_dir"
    )
}

/// Execute a filesystem builtin by name.
pub fn call_fs_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "file_exists" => builtin_file_exists(args),
        "read_text" => builtin_read_text(args),
        "write_text" => builtin_write_text(args),
        "read_lines" => builtin_read_lines(args),
        "write_lines" => builtin_write_lines(args),
        "append_text" => builtin_append_text(args),
        "list_dir" => builtin_list_dir(args),
        "mkdir" => builtin_mkdir(args),
        "basename" => builtin_basename(args),
        "dirname" => builtin_dirname(args),
        "extension" => builtin_extension(args),
        "path_join" => builtin_path_join(args),
        "abs_path" => builtin_abs_path(args),
        "file_size" => builtin_file_size(args),
        "is_dir" => builtin_is_dir(args),
        "is_file" => builtin_is_file(args),
        "remove" => builtin_remove(args),
        "copy_file" => builtin_copy_file(args),
        "rename_file" => builtin_rename_file(args),
        "temp_file" => builtin_temp_file(),
        "temp_dir" => builtin_temp_dir(),
        "glob" => builtin_glob(args),
        "remove_dir" => builtin_remove_dir(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown fs builtin '{name}'"),
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

fn builtin_file_exists(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "file_exists")?;
    Ok(Value::Bool(std::path::Path::new(path).exists()))
}

fn builtin_read_text(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "read_text")?;
    let content = std::fs::read_to_string(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("read_text() failed: {e}"), None)
    })?;
    Ok(Value::Str(content))
}

fn builtin_write_text(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "write_text")?;
    let content = require_str(&args[1], "write_text")?;
    std::fs::write(path, content).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("write_text() failed: {e}"), None)
    })?;
    Ok(Value::Nil)
}

fn builtin_list_dir(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "list_dir")?;
    let entries = std::fs::read_dir(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("list_dir() failed: {e}"), None)
    })?;

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("list_dir() entry error: {e}"), None)
        })?;
        let meta = entry.metadata().map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("list_dir() metadata error: {e}"),
                None,
            )
        })?;
        let mut rec = HashMap::new();
        rec.insert(
            "name".to_string(),
            Value::Str(entry.file_name().to_string_lossy().to_string()),
        );
        rec.insert("size".to_string(), Value::Int(meta.len() as i64));
        rec.insert("is_dir".to_string(), Value::Bool(meta.is_dir()));
        result.push(Value::Record(rec));
    }
    Ok(Value::List(result))
}

fn builtin_mkdir(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "mkdir")?;
    std::fs::create_dir_all(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("mkdir() failed: {e}"), None)
    })?;
    Ok(Value::Nil)
}

fn builtin_basename(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "basename")?;
    let name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(Value::Str(name))
}

fn builtin_dirname(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "dirname")?;
    let parent = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(Value::Str(parent))
}

fn builtin_read_lines(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "read_lines")?;
    let content = std::fs::read_to_string(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("read_lines() failed: {e}"), None)
    })?;
    let lines: Vec<Value> = content.lines().map(|l| Value::Str(l.to_string())).collect();
    Ok(Value::List(lines))
}

fn builtin_write_lines(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "write_lines")?;
    let lines = match &args[1] {
        Value::List(l) => l,
        other => {
            return Err(BioLangError::type_error(
                format!("write_lines() requires List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let content: Vec<String> = lines.iter().map(|v| format!("{v}")).collect();
    std::fs::write(path, content.join("\n")).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("write_lines() failed: {e}"), None)
    })?;
    Ok(Value::Nil)
}

fn builtin_append_text(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "append_text")?;
    let text = require_str(&args[1], "append_text")?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("append_text() failed: {e}"),
                None,
            )
        })?;
    file.write_all(text.as_bytes()).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("append_text() write failed: {e}"),
            None,
        )
    })?;
    Ok(Value::Nil)
}

fn builtin_extension(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "extension")?;
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(Value::Str(ext))
}

fn builtin_path_join(args: Vec<Value>) -> Result<Value> {
    let base = require_str(&args[0], "path_join")?;
    let child = require_str(&args[1], "path_join")?;
    let joined = std::path::Path::new(base).join(child);
    Ok(Value::Str(joined.to_string_lossy().to_string()))
}

fn builtin_abs_path(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "abs_path")?;
    let abs = std::fs::canonicalize(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("abs_path() failed: {e}"), None)
    })?;
    Ok(Value::Str(abs.to_string_lossy().to_string()))
}

fn builtin_file_size(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "file_size")?;
    let meta = std::fs::metadata(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("file_size() failed: {e}"), None)
    })?;
    Ok(Value::Int(meta.len() as i64))
}

fn builtin_is_dir(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "is_dir")?;
    Ok(Value::Bool(std::path::Path::new(path).is_dir()))
}

fn builtin_is_file(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "is_file")?;
    Ok(Value::Bool(std::path::Path::new(path).is_file()))
}

fn builtin_remove(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "remove")?;
    let p = std::path::Path::new(path);
    if p.is_dir() {
        std::fs::remove_dir_all(p).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("remove() failed: {e}"), None)
        })?;
    } else {
        std::fs::remove_file(p).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("remove() failed: {e}"), None)
        })?;
    }
    Ok(Value::Nil)
}

fn builtin_copy_file(args: Vec<Value>) -> Result<Value> {
    let src = require_str(&args[0], "copy_file")?;
    let dst = require_str(&args[1], "copy_file")?;
    std::fs::copy(src, dst).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("copy_file() failed: {e}"), None)
    })?;
    Ok(Value::Str(dst.to_string()))
}

fn builtin_rename_file(args: Vec<Value>) -> Result<Value> {
    let src = require_str(&args[0], "rename_file")?;
    let dst = require_str(&args[1], "rename_file")?;
    std::fs::rename(src, dst).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("rename_file() failed: {e}"), None)
    })?;
    Ok(Value::Str(dst.to_string()))
}

fn temp_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("bl_{t}")
}

fn builtin_temp_file() -> Result<Value> {
    let tmp = std::env::temp_dir().join(temp_name());
    std::fs::write(&tmp, "").map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("temp_file() failed: {e}"), None)
    })?;
    Ok(Value::Str(tmp.to_string_lossy().to_string()))
}

fn builtin_temp_dir() -> Result<Value> {
    let tmp = std::env::temp_dir().join(temp_name());
    std::fs::create_dir_all(&tmp).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("temp_dir() failed: {e}"), None)
    })?;
    Ok(Value::Str(tmp.to_string_lossy().to_string()))
}

fn builtin_glob(args: Vec<Value>) -> Result<Value> {
    let pattern = require_str(&args[0], "glob")?;
    let paths = glob::glob(pattern).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("glob() invalid pattern: {e}"),
            None,
        )
    })?;
    let mut result = Vec::new();
    for entry in paths {
        match entry {
            Ok(path) => result.push(Value::Str(path.to_string_lossy().to_string())),
            Err(e) => {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("glob() error: {e}"),
                    None,
                ))
            }
        }
    }
    Ok(Value::List(result))
}

fn builtin_remove_dir(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "remove_dir")?;
    let recursive = if args.len() > 1 {
        match &args[1] {
            Value::Bool(b) => *b,
            _ => false,
        }
    } else {
        false
    };
    let p = std::path::Path::new(path);
    if !p.is_dir() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("remove_dir() path is not a directory: {path}"),
            None,
        ));
    }
    if recursive {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_dir(p)
    }
    .map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("remove_dir() failed: {e}"), None)
    })?;
    Ok(Value::Nil)
}

