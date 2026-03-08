use bl_core::error::{BioLangError, Result};
use bl_core::value::{Arity, Value};

use base64::{engine::general_purpose::STANDARD, Engine};
use crc32fast::Hasher as Crc32Hasher;
use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

type HmacSha256 = Hmac<Sha256>;

pub fn hash_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("md5", Arity::Exact(1)),
        ("sha1", Arity::Exact(1)),
        ("sha256", Arity::Exact(1)),
        ("sha512", Arity::Exact(1)),
        ("crc32", Arity::Exact(1)),
        ("hmac_sha256", Arity::Exact(2)),
        ("base64_encode", Arity::Exact(1)),
        ("base64_decode", Arity::Exact(1)),
        ("sketch", Arity::Range(1, 3)),
        ("sketch_dist", Arity::Exact(2)),
        #[cfg(feature = "native")]
        ("verify_checksum", Arity::Exact(2)),
    ]
}

pub fn is_hash_builtin(name: &str) -> bool {
    match name {
        "md5" | "sha1" | "sha256" | "sha512" | "crc32" | "hmac_sha256" | "base64_encode"
        | "base64_decode" | "sketch" | "sketch_dist" => true,
        #[cfg(feature = "native")]
        "verify_checksum" => true,
        _ => false,
    }
}

pub fn call_hash_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "md5" => {
            let s = require_str(&args[0], "md5")?;
            let digest = Md5::digest(s.as_bytes());
            Ok(Value::Str(hex::encode(digest)))
        }
        "sha1" => {
            let s = require_str(&args[0], "sha1")?;
            let digest = Sha1::digest(s.as_bytes());
            Ok(Value::Str(hex::encode(digest)))
        }
        "sha256" => {
            let s = require_str(&args[0], "sha256")?;
            let digest = Sha256::digest(s.as_bytes());
            Ok(Value::Str(hex::encode(digest)))
        }
        "sha512" => {
            let s = require_str(&args[0], "sha512")?;
            let digest = Sha512::digest(s.as_bytes());
            Ok(Value::Str(hex::encode(digest)))
        }
        "crc32" => {
            let s = require_str(&args[0], "crc32")?;
            let mut hasher = Crc32Hasher::new();
            hasher.update(s.as_bytes());
            Ok(Value::Int(hasher.finalize() as i64))
        }
        "hmac_sha256" => {
            let data = require_str(&args[0], "hmac_sha256")?;
            let key = require_str(&args[1], "hmac_sha256")?;
            let mut mac = HmacSha256::new_from_slice(key.as_bytes())
                .map_err(|e| BioLangError::runtime(bl_core::error::ErrorKind::TypeError, format!("hmac_sha256() key error: {e}"), None))?;
            mac.update(data.as_bytes());
            let result = mac.finalize();
            Ok(Value::Str(hex::encode(result.into_bytes())))
        }
        "base64_encode" => {
            let s = require_str(&args[0], "base64_encode")?;
            Ok(Value::Str(STANDARD.encode(s.as_bytes())))
        }
        "base64_decode" => {
            let s = require_str(&args[0], "base64_decode")?;
            let bytes = STANDARD.decode(s.as_bytes()).map_err(|e| {
                BioLangError::runtime(
                    bl_core::error::ErrorKind::TypeError,
                    format!("base64_decode() invalid input: {e}"),
                    None,
                )
            })?;
            let decoded = String::from_utf8(bytes).map_err(|e| {
                BioLangError::runtime(
                    bl_core::error::ErrorKind::TypeError,
                    format!("base64_decode() not valid UTF-8: {e}"),
                    None,
                )
            })?;
            Ok(Value::Str(decoded))
        }
        "sketch" => builtin_sketch(args),
        "sketch_dist" => builtin_sketch_dist(args),
        #[cfg(feature = "native")]
        "verify_checksum" => builtin_verify_checksum(args),
        _ => Err(BioLangError::runtime(
            bl_core::error::ErrorKind::TypeError,
            format!("unknown hash builtin: {name}"),
            None,
        )),
    }
}

// ── MinHash Sketch ──────────────────────────────────────────────

/// sketch(sequence, k?, n?) → List of Int hashes
/// Extract k-mers from a sequence, hash them, keep the smallest n.
/// Default: k=21, n=1000
fn builtin_sketch(args: Vec<Value>) -> Result<Value> {
    let seq = match &args[0] {
        Value::Str(s) => s.as_str(),
        Value::DNA(s) | Value::RNA(s) | Value::Protein(s) => s.data.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("sketch() requires Str/DNA/RNA/Protein, got {}", other.type_of()),
                None,
            ));
        }
    };

    let k = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) => *n as usize,
            _ => 21,
        }
    } else {
        21
    };

    let n = if args.len() > 2 {
        match &args[2] {
            Value::Int(n) => *n as usize,
            _ => 1000,
        }
    } else {
        1000
    };

    if seq.len() < k {
        return Ok(Value::List(vec![]));
    }

    // Collect hashed k-mers
    let seq_upper = seq.to_uppercase();
    let bytes = seq_upper.as_bytes();
    let mut hashes: Vec<u64> = Vec::with_capacity(bytes.len().saturating_sub(k) + 1);
    for window in bytes.windows(k) {
        // Simple hash: use crc32 extended to 64-bit via double-hash
        let mut hasher = Crc32Hasher::new();
        hasher.update(window);
        let h1 = hasher.finalize() as u64;
        // Second hash with reversed window for 64-bit spread
        let mut hasher2 = Crc32Hasher::new();
        let mut rev = window.to_vec();
        rev.reverse();
        hasher2.update(&rev);
        let h2 = hasher2.finalize() as u64;
        hashes.push((h1 << 32) | h2);
    }

    // Keep the smallest n hashes (MinHash)
    hashes.sort_unstable();
    hashes.dedup();
    hashes.truncate(n);

    Ok(Value::List(
        hashes.into_iter().map(|h| Value::Int(h as i64)).collect(),
    ))
}

/// sketch_dist(sketch_a, sketch_b) → Float (Jaccard distance, 0.0–1.0)
/// Compare two MinHash sketches. 0.0 = identical, 1.0 = completely different.
fn builtin_sketch_dist(args: Vec<Value>) -> Result<Value> {
    let a = match &args[0] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n),
                other => Err(BioLangError::type_error(
                    format!("sketch_dist() requires List of Int, got {}", other.type_of()),
                    None,
                )),
            })
            .collect::<Result<Vec<i64>>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!("sketch_dist() arg 1 requires List, got {}", other.type_of()),
                None,
            ));
        }
    };
    let b = match &args[1] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n),
                other => Err(BioLangError::type_error(
                    format!("sketch_dist() requires List of Int, got {}", other.type_of()),
                    None,
                )),
            })
            .collect::<Result<Vec<i64>>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!("sketch_dist() arg 2 requires List, got {}", other.type_of()),
                None,
            ));
        }
    };

    if a.is_empty() && b.is_empty() {
        return Ok(Value::Float(0.0));
    }
    if a.is_empty() || b.is_empty() {
        return Ok(Value::Float(1.0));
    }

    // Jaccard: |A ∩ B| / |A ∪ B|
    use std::collections::HashSet;
    let set_a: HashSet<i64> = a.into_iter().collect();
    let set_b: HashSet<i64> = b.into_iter().collect();
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;

    let jaccard = if union > 0.0 {
        intersection / union
    } else {
        1.0
    };
    // Distance = 1 - similarity
    Ok(Value::Float(1.0 - jaccard))
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

#[cfg(feature = "native")]
fn builtin_verify_checksum(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "verify_checksum")?;
    let expected = require_str(&args[1], "verify_checksum")?;
    let content = std::fs::read(path).map_err(|e| {
        BioLangError::runtime(
            bl_core::error::ErrorKind::IOError,
            format!("verify_checksum() failed to read file: {e}"),
            None,
        )
    })?;
    // Auto-detect algorithm from hash length
    let actual = match expected.len() {
        32 => hex::encode(Md5::digest(&content)),       // MD5
        40 => hex::encode(Sha1::digest(&content)),      // SHA-1
        64 => hex::encode(Sha256::digest(&content)),    // SHA-256
        128 => hex::encode(Sha512::digest(&content)),   // SHA-512
        _ => {
            return Err(BioLangError::runtime(
                bl_core::error::ErrorKind::TypeError,
                format!(
                    "verify_checksum() cannot detect algorithm from hash length {}; expected 32 (MD5), 40 (SHA-1), 64 (SHA-256), or 128 (SHA-512)",
                    expected.len()
                ),
                None,
            ))
        }
    };
    Ok(Value::Bool(actual.eq_ignore_ascii_case(expected)))
}
