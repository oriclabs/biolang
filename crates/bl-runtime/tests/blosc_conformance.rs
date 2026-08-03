//! Conformance tests for the Blosc1 decompressor.
//!
//! The vectors in `tests/data/blosc/` are ground truth produced by numcodecs
//! (the exact encoder scanpy/anndata use), covering the full grid of
//! codec x shuffle x typesize x block layout. Regenerate with
//! `tests/data/blosc/generate_vectors.py` — see the header there for the
//! container invocation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bl_runtime::blosc;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
struct Case {
    name: String,
    cname: String,
    shuffle: String,
    typesize: usize,
    nelem: usize,
    nblocks: usize,
    memcpyed: bool,
    c_off: usize,
    c_len: usize,
    r_off: usize,
    r_len: usize,
}

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/blosc")
}

fn load() -> (Vec<Case>, Vec<u8>) {
    let dir = data_dir();
    let json = std::fs::read_to_string(dir.join("vectors.json")).expect("vectors.json");
    let bin = std::fs::read(dir.join("vectors.bin")).expect("vectors.bin");
    (
        serde_json::from_str(&json).expect("parse vectors.json"),
        bin,
    )
}

#[test]
fn decompresses_every_conformance_vector() {
    let (cases, blob) = load();
    assert!(!cases.is_empty(), "no vectors loaded");

    let mut failures: Vec<String> = Vec::new();
    // coverage tally, so a silently-shrunk vector set can't make this pass trivially
    let mut covered: BTreeMap<(String, String), usize> = BTreeMap::new();

    for c in &cases {
        let comp = &blob[c.c_off..c.c_off + c.c_len];
        let expect = &blob[c.r_off..c.r_off + c.r_len];

        assert!(
            blosc::is_blosc(comp),
            "{}: not recognised as a blosc buffer",
            c.name
        );

        match blosc::decompress(comp) {
            Ok(got) if got == expect => {
                if !c.memcpyed {
                    *covered
                        .entry((c.cname.clone(), c.shuffle.clone()))
                        .or_default() += 1;
                }
            }
            Ok(got) => {
                let at = got
                    .iter()
                    .zip(expect)
                    .position(|(a, b)| a != b)
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "length only".into());
                failures.push(format!(
                    "{}: wrong output (got {} bytes, want {}, first diff at {})",
                    c.name,
                    got.len(),
                    expect.len(),
                    at
                ));
            }
            Err(e) => failures.push(format!("{}: {e}", c.name)),
        }
    }

    // Every codec must have genuinely-compressed (not memcpyed) coverage under
    // each shuffle mode, otherwise a green run proves very little.
    for cname in ["blosclz", "lz4", "lz4hc", "zlib", "zstd"] {
        for sh in ["noshuffle", "shuffle", "bitshuffle"] {
            // lz4hc shares lz4's codec id in the header; numcodecs still emits
            // it as a distinct cname, so only require coverage where vectors exist.
            let key = (cname.to_string(), sh.to_string());
            if cases
                .iter()
                .any(|c| !c.memcpyed && c.cname == cname && c.shuffle == sh)
            {
                assert!(
                    covered.get(&key).copied().unwrap_or(0) > 0,
                    "no compressed vector passed for {cname}/{sh}"
                );
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} vectors failed:\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(25)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn covers_multi_block_and_split_layouts() {
    let (cases, _) = load();
    let multi = cases.iter().filter(|c| c.nblocks > 1).count();
    let split = cases
        .iter()
        .filter(|c| !c.memcpyed && c.typesize > 1 && c.nelem >= 128)
        .count();
    assert!(
        multi >= 50,
        "vector set lost multi-block coverage ({multi})"
    );
    assert!(
        split >= 20,
        "vector set lost split-block coverage ({split})"
    );
}

/// End-to-end: `read_anndata` on a store written by anndata/zarr with default
/// settings (blosc + lz4 + byte shuffle). Before blosc support this store was
/// rejected outright, which is the failure this work exists to fix.
#[test]
fn reads_a_default_compressed_anndata_store() {
    use bl_core::value::Value;

    let dir = data_dir();
    let store = dir.join("store.zarr");
    assert!(store.exists(), "missing store.zarr fixture");

    // guard: the fixture must actually be blosc-compressed, or this proves nothing
    let zarray = std::fs::read_to_string(store.join("X/.zarray")).expect("X/.zarray");
    assert!(
        zarray.contains("\"blosc\""),
        "store.zarr fixture is not blosc-compressed"
    );

    let out = bl_runtime::anndata_zarr::call_anndata_builtin(
        "read_anndata",
        vec![Value::Str(store.to_string_lossy().into_owned())],
    )
    .expect("read_anndata failed on a blosc-compressed store");

    let Value::Record(rec) = out else {
        panic!("read_anndata did not return a record");
    };
    let n_cells = match rec.get("n_cells") {
        Some(Value::Int(i)) => *i as usize,
        other => panic!("bad n_cells: {other:?}"),
    };
    let n_genes = match rec.get("n_genes") {
        Some(Value::Int(i)) => *i as usize,
        other => panic!("bad n_genes: {other:?}"),
    };
    assert_eq!((n_cells, n_genes), (137, 59));

    let expect_bytes = std::fs::read(dir.join("store_X.f32")).expect("store_X.f32");
    let expect: Vec<f32> = expect_bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    assert_eq!(expect.len(), n_cells * n_genes);

    let Some(Value::List(rows)) = rec.get("matrix") else {
        panic!("no matrix in record");
    };
    assert_eq!(rows.len(), n_cells);

    let mut compared = 0usize;
    for (i, row) in rows.iter().enumerate() {
        let Value::List(cols) = row else {
            panic!("row {i} is not a list")
        };
        assert_eq!(cols.len(), n_genes, "row {i} wrong width");
        for (j, v) in cols.iter().enumerate() {
            let got = match v {
                Value::Float(f) => *f,
                Value::Int(n) => *n as f64,
                other => panic!("cell ({i},{j}) is {other:?}"),
            };
            let want = expect[i * n_genes + j] as f64;
            assert!(
                (got - want).abs() < 1e-6,
                "cell ({i},{j}): got {got}, want {want}"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 137 * 59);
}

#[test]
fn rejects_malformed_buffers() {
    assert!(blosc::decompress(&[]).is_err());
    assert!(blosc::decompress(&[0u8; 8]).is_err());
    assert!(!blosc::is_blosc(&[0u8; 4]));

    // header promising far more compressed bytes than are present
    let mut hdr = vec![2u8, 1, 0x21, 8];
    hdr.extend_from_slice(&1024u32.to_le_bytes()); // nbytes
    hdr.extend_from_slice(&512u32.to_le_bytes()); // blocksize
    hdr.extend_from_slice(&99_999u32.to_le_bytes()); // cbytes
    assert!(blosc::decompress(&hdr).is_err());
}
