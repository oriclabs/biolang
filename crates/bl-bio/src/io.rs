use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{BioSequence, StreamValue, Table, Value};

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

// noodles traits for BAM record field iteration
use noodles_sam::alignment::record::Cigar as CigarTrait;
use noodles_sam::alignment::record::Sequence as SequenceTrait;

/// Resolve a file path against `BIOLANG_DATA_DIR` if configured.
/// - Absolute paths are returned as-is.
/// - Relative paths that exist in CWD are returned as-is (CWD takes priority).
/// - Otherwise, if `BIOLANG_DATA_DIR` is set, the path is resolved relative to it.
/// - If nothing matches, the original path is returned (for error messages).
fn resolve_data_path(path: &str) -> String {
    let p = Path::new(path);
    // Absolute paths — use directly
    if p.is_absolute() {
        return path.to_string();
    }
    // If file exists in CWD, use it (backwards compatible default)
    if p.exists() {
        return path.to_string();
    }
    // Check BIOLANG_DATA_DIR
    if let Ok(data_dir) = std::env::var("BIOLANG_DATA_DIR") {
        if !data_dir.is_empty() {
            let resolved = PathBuf::from(&data_dir).join(path);
            if resolved.exists() {
                return resolved.to_string_lossy().to_string();
            }
        }
    }
    // Fallback: return original (will produce "file not found" at the call site)
    path.to_string()
}

/// Resolve a write path against `BIOLANG_DATA_DIR` if configured.
/// - Absolute paths are returned as-is.
/// - If `BIOLANG_DATA_DIR` is set and the path is relative, writes into data dir.
/// - Otherwise, writes to CWD (default behavior).
fn resolve_write_path(path: &str) -> String {
    let p = Path::new(path);
    if p.is_absolute() {
        return path.to_string();
    }
    if let Ok(data_dir) = std::env::var("BIOLANG_DATA_DIR") {
        if !data_dir.is_empty() {
            let resolved = PathBuf::from(&data_dir).join(path);
            // Ensure parent directory exists
            if let Some(parent) = resolved.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            return resolved.to_string_lossy().to_string();
        }
    }
    path.to_string()
}

/// Read a FASTA file, returning a Stream of records.
/// Each record is a Record { id: Str, description: Str, seq: DNA(...), length: Int }.
/// Supports .gz compressed files.
pub fn read_fasta(path: &str) -> Result<Value> {
    let path = resolve_data_path(path);
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err(BioLangError::runtime(
            bl_core::error::ErrorKind::IOError,
            format!("file not found: {path}"),
            None,
        ));
    }

    check_format_mismatch(&path, "fasta")?;

    let label = format!("fasta:{path}");
    let file = open_file(&path)?;

    if path.ends_with(".gz") {
        let decoder = flate2::read::GzDecoder::new(file);
        let buf = BufReader::new(decoder);
        let reader = noodles_fasta::io::Reader::new(buf);
        Ok(Value::Stream(StreamValue::new(
            label,
            Box::new(FastaIter::new(reader)),
        )))
    } else {
        let buf = BufReader::new(file);
        let reader = noodles_fasta::io::Reader::new(buf);
        Ok(Value::Stream(StreamValue::new(
            label,
            Box::new(FastaIter::new(reader)),
        )))
    }
}

/// Compute aggregate FASTA statistics in a single native pass.
/// Returns a Record with: count, total_bp, mean_length, median_length, min_length, max_length,
/// n50, mean_gc, lengths (List[Int]), gc_values (List[Float]).
pub fn fasta_stats(path: &str) -> Result<Value> {
    let stream = read_fasta(path)?;
    let s = match stream {
        Value::Stream(s) => s,
        _ => unreachable!(),
    };

    // Stream without materializing full records — collect only lengths and gc values
    let mut lengths: Vec<i64> = Vec::new();
    let mut gc_values: Vec<f64> = Vec::new();
    let mut total_bp: i64 = 0;
    let mut gc_sum: f64 = 0.0;

    while let Some(item) = s.next() {
        if let Value::Record(rec) = item {
            let len = match rec.get("length") {
                Some(Value::Int(n)) => *n,
                _ => 0,
            };
            let gc = match rec.get("gc") {
                Some(Value::Float(f)) => *f,
                _ => 0.0,
            };
            lengths.push(len);
            gc_values.push(gc);
            total_bp += len;
            gc_sum += gc;
        }
    }

    let count = lengths.len();

    if count == 0 {
        let mut map = HashMap::new();
        map.insert("count".to_string(), Value::Int(0));
        map.insert("total_bp".to_string(), Value::Int(0));
        map.insert("mean_length".to_string(), Value::Float(0.0));
        map.insert("median_length".to_string(), Value::Float(0.0));
        map.insert("min_length".to_string(), Value::Int(0));
        map.insert("max_length".to_string(), Value::Int(0));
        map.insert("n50".to_string(), Value::Int(0));
        map.insert("mean_gc".to_string(), Value::Float(0.0));
        map.insert("lengths".to_string(), Value::List(vec![]));
        map.insert("gc_values".to_string(), Value::List(vec![]));
        return Ok(Value::Record(map));
    }

    let mean_length = total_bp as f64 / count as f64;
    let mean_gc = gc_sum / count as f64;

    // Sort for median and N50
    let mut sorted = lengths.clone();
    sorted.sort_unstable();

    let median_length = if count % 2 == 0 {
        (sorted[count / 2 - 1] + sorted[count / 2]) as f64 / 2.0
    } else {
        sorted[count / 2] as f64
    };

    let min_length = sorted[0];
    let max_length = sorted[count - 1];

    // N50: sort descending, accumulate until >= half of total_bp
    let half = total_bp / 2;
    let mut cumulative: i64 = 0;
    let mut n50: i64 = 0;
    for &l in sorted.iter().rev() {
        cumulative += l;
        if cumulative >= half {
            n50 = l;
            break;
        }
    }

    let mut map = HashMap::new();
    map.insert("count".to_string(), Value::Int(count as i64));
    map.insert("total_bp".to_string(), Value::Int(total_bp));
    map.insert("mean_length".to_string(), Value::Float(mean_length));
    map.insert("median_length".to_string(), Value::Float(median_length));
    map.insert("min_length".to_string(), Value::Int(min_length));
    map.insert("max_length".to_string(), Value::Int(max_length));
    map.insert("n50".to_string(), Value::Int(n50));
    map.insert("mean_gc".to_string(), Value::Float(mean_gc));
    map.insert("lengths".to_string(), Value::List(lengths.into_iter().map(Value::Int).collect()));
    map.insert("gc_values".to_string(), Value::List(gc_values.into_iter().map(Value::Float).collect()));
    Ok(Value::Record(map))
}

/// Read a FASTA file eagerly, returning a Table.
/// Columns: id, description, seq, length.
/// For large files, prefer the streaming `read_fasta()` instead.
pub fn read_fasta_table(path: &str) -> Result<Value> {
    let stream = read_fasta(path)?;
    let items = match stream {
        Value::Stream(s) => s.collect_all(),
        _ => unreachable!(),
    };
    let columns = vec!["id".into(), "description".into(), "seq".into(), "length".into()];
    let mut rows = Vec::with_capacity(items.len());
    for item in items {
        if let Value::Record(map) = item {
            rows.push(vec![
                map.get("id").cloned().unwrap_or(Value::Nil),
                map.get("description").cloned().unwrap_or(Value::Nil),
                map.get("seq").cloned().unwrap_or(Value::Nil),
                map.get("length").cloned().unwrap_or(Value::Nil),
            ]);
        }
    }
    Ok(Value::Table(Table::new(columns, rows)))
}

/// Read a FASTQ file, returning a Stream of records.
/// Each record is a Record { id: Str, description: Str, seq: DNA(...), length: Int, quality: Str }.
/// Supports .gz compressed files.
pub fn read_fastq(path: &str) -> Result<Value> {
    let path = resolve_data_path(path);
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err(BioLangError::runtime(
            bl_core::error::ErrorKind::IOError,
            format!("file not found: {path}"),
            None,
        ));
    }

    check_format_mismatch(&path, "fastq")?;

    let label = format!("fastq:{path}");
    let file = open_file(&path)?;

    if path.ends_with(".gz") {
        let decoder = flate2::read::GzDecoder::new(file);
        let buf = BufReader::new(decoder);
        let reader = noodles_fastq::io::Reader::new(buf);
        Ok(Value::Stream(StreamValue::new(
            label,
            Box::new(FastqIter::new(reader)),
        )))
    } else {
        let buf = BufReader::new(file);
        let reader = noodles_fastq::io::Reader::new(buf);
        Ok(Value::Stream(StreamValue::new(
            label,
            Box::new(FastqIter::new(reader)),
        )))
    }
}

/// Peek at the first non-empty byte of a file to detect format mismatches.
/// Returns the first non-whitespace byte, or None if the file is empty.
fn peek_first_byte(path: &str) -> Option<u8> {
    use std::io::Read;
    let mut file = File::open(path).ok()?;
    // For .gz files, decompress first
    if path.ends_with(".gz") {
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut buf = [0u8; 1];
        loop {
            match decoder.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => {
                    if !buf[0].is_ascii_whitespace() {
                        return Some(buf[0]);
                    }
                }
                Err(_) => return None,
            }
        }
    } else {
        let mut buf = [0u8; 1];
        loop {
            match file.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => {
                    if !buf[0].is_ascii_whitespace() {
                        return Some(buf[0]);
                    }
                }
                Err(_) => return None,
            }
        }
    }
}

/// Check format magic byte and return an error if it looks like the wrong format.
fn check_format_mismatch(path: &str, expected: &str) -> Result<()> {
    let byte = match peek_first_byte(path) {
        Some(b) => b,
        None => return Ok(()), // empty file — let the reader handle it
    };

    let hint = match (expected, byte) {
        // FASTA expects '>', got '@' (FASTQ)
        ("fasta", b'@') => Some("file appears to be FASTQ format (starts with @), not FASTA. Use fastq() instead"),
        // FASTQ expects '@', got '>' (FASTA)
        ("fastq", b'>') => Some("file appears to be FASTA format (starts with >), not FASTQ. Use fasta() instead"),
        // FASTA/FASTQ got tab-separated data (BED/VCF/GFF/SAM/TSV)
        ("fasta", b) if b != b'>' && b != b';' => {
            // Check if it looks like a tabular format
            if b == b'#' || b.is_ascii_alphanumeric() {
                // Could be many things — only warn for clearly wrong formats
                None
            } else {
                None
            }
        }
        ("fastq", b) if b != b'@' && b != b'#' => {
            if b == b'>' {
                Some("file appears to be FASTA format (starts with >), not FASTQ. Use fasta() instead")
            } else {
                None
            }
        }
        // BED/GFF/VCF/SAM: got '>' (FASTA) or '@' without tab (FASTQ)
        ("bed", b'>') => Some("file appears to be FASTA format, not BED. Use fasta() instead"),
        ("bed", b'@') => Some("file appears to be FASTQ or SAM format, not BED. Use fastq() or sam() instead"),
        ("gff", b'>') => Some("file appears to be FASTA format, not GFF. Use fasta() instead"),
        ("gff", b'@') => Some("file appears to be FASTQ or SAM format, not GFF. Use fastq() or sam() instead"),
        ("vcf", b'>') => Some("file appears to be FASTA format, not VCF. Use fasta() instead"),
        ("vcf", b'@') => Some("file appears to be FASTQ or SAM format, not VCF. Use fastq() or sam() instead"),
        ("sam", b'>') => Some("file appears to be FASTA format, not SAM. Use fasta() instead"),
        // BAM expects BAM magic (0x42='B')
        ("bam", b'>') => Some("file appears to be FASTA format, not BAM. Use fasta() instead"),
        ("bam", b'@') => Some("file appears to be SAM (text) format, not BAM (binary). Use sam() instead"),
        // SAM expects '@' header or tab-separated alignment
        ("sam", b'B') => Some("file appears to be BAM (binary) format, not SAM (text). Use bam() instead"),
        _ => None,
    };

    if let Some(msg) = hint {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{expected}(): {msg}"),
            None,
        ));
    }
    Ok(())
}

/// Read a FASTQ file eagerly, returning a Table.
/// Columns: id, description, seq, length, quality.
/// For large files, prefer the streaming `read_fastq()` instead.
pub fn read_fastq_table(path: &str) -> Result<Value> {
    let stream = read_fastq(path)?;
    let items = match stream {
        Value::Stream(s) => s.collect_all(),
        _ => unreachable!(),
    };
    let total = items.len();
    let show_progress = total >= 10000;
    if total >= 50000 {
        eprintln!("\x1b[33mWarning:\x1b[0m read_fastq() is loading {} reads into memory. \
            For large files, use fastq() for streaming: \
            fastq(\"file.fq.gz\") |> filter(...) |> kmer_count(k)", total);
    }
    let columns = vec!["id".into(), "description".into(), "seq".into(), "length".into(), "quality".into()];
    let mut rows = Vec::with_capacity(total);
    for (i, item) in items.into_iter().enumerate() {
        if show_progress && (i % 20000 == 0) {
            eprint!("\r\x1b[2Kread_fastq: loading {}/{} reads...", i, total);
        }
        if let Value::Record(map) = item {
            rows.push(vec![
                map.get("id").cloned().unwrap_or(Value::Nil),
                map.get("description").cloned().unwrap_or(Value::Nil),
                map.get("seq").cloned().unwrap_or(Value::Nil),
                map.get("length").cloned().unwrap_or(Value::Nil),
                map.get("quality").cloned().unwrap_or(Value::Nil),
            ]);
        }
    }
    if show_progress {
        eprint!("\r\x1b[2K"); // clear progress line
    }
    Ok(Value::Table(Table::new(columns, rows)))
}

fn open_file(path: &str) -> Result<File> {
    let path = resolve_data_path(path);
    File::open(&path).map_err(|e| {
        BioLangError::runtime(
            bl_core::error::ErrorKind::IOError,
            format!("cannot open file '{path}': {e}"),
            None,
        )
    })
}

/// Owns the FASTA reader and yields Value records one at a time.
/// Replicates the logic from noodles Records iterator but with ownership.
struct FastaIter<R: std::io::BufRead> {
    reader: noodles_fasta::io::Reader<R>,
    line_buf: String,
}

impl<R: std::io::BufRead> FastaIter<R> {
    fn new(reader: noodles_fasta::io::Reader<R>) -> Self {
        Self {
            reader,
            line_buf: String::new(),
        }
    }
}

impl<R: std::io::BufRead + Send> Iterator for FastaIter<R> {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        self.line_buf.clear();

        // Read definition line (>name description)
        match self.reader.read_definition(&mut self.line_buf) {
            Ok(0) => return None, // EOF
            Ok(_) => {}
            Err(_) => return None,
        }

        // Parse definition to extract name and description
        // The definition line starts with '>', which we strip
        let def_line = self.line_buf.trim().trim_start_matches('>');
        let (name, description) = if let Some(idx) = def_line.find(|c: char| c.is_whitespace()) {
            (def_line[..idx].to_string(), def_line[idx..].trim().to_string())
        } else {
            (def_line.to_string(), String::new())
        };

        // Read sequence
        let mut seq_buf = Vec::new();
        match self.reader.read_sequence(&mut seq_buf) {
            Ok(_) => {
                let seq_str = String::from_utf8_lossy(&seq_buf).to_uppercase();
                let seq_len = seq_str.len() as i64;
                let gc = bio_core::seq_ops::gc_content(&seq_str);

                let mut map = HashMap::with_capacity(5);
                map.insert("id".to_string(), Value::Str(name));
                map.insert("description".to_string(), Value::Str(description));
                map.insert(
                    "seq".to_string(),
                    Value::DNA(BioSequence { data: seq_str }),
                );
                map.insert("length".to_string(), Value::Int(seq_len));
                map.insert("gc".to_string(), Value::Float(gc));
                Some(Value::Record(map))
            }
            Err(_) => None,
        }
    }
}

/// Owns the FASTQ reader and yields Value records one at a time.
struct FastqIter<R: std::io::BufRead> {
    reader: noodles_fastq::io::Reader<R>,
    record_buf: noodles_fastq::Record,
}

impl<R: std::io::BufRead> FastqIter<R> {
    fn new(reader: noodles_fastq::io::Reader<R>) -> Self {
        Self {
            reader,
            record_buf: noodles_fastq::Record::default(),
        }
    }
}

impl<R: std::io::BufRead + Send> Iterator for FastqIter<R> {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        match self.reader.read_record(&mut self.record_buf) {
            Ok(0) => None, // EOF
            Ok(_) => {
                let name = self.record_buf.name().to_string();
                let description = self.record_buf.description().to_string();
                let seq_str = String::from_utf8_lossy(self.record_buf.sequence())
                    .to_uppercase();
                let quality = String::from_utf8_lossy(
                    self.record_buf.quality_scores(),
                )
                .to_string();
                let seq_len = seq_str.len() as i64;

                let mut map = HashMap::new();
                map.insert("id".to_string(), Value::Str(name));
                map.insert("description".to_string(), Value::Str(description));
                map.insert(
                    "seq".to_string(),
                    Value::DNA(BioSequence { data: seq_str }),
                );
                map.insert("length".to_string(), Value::Int(seq_len));
                map.insert("quality".to_string(), Value::Str(quality));
                Some(Value::Record(map))
            }
            Err(_) => None,
        }
    }
}

/// Read a BED file (BED3/BED6/BED12), returning a Table.
pub fn read_bed(path: &str) -> Result<Value> {
    use std::io::{BufRead, BufReader};

    let path = resolve_data_path(path);
    check_format_mismatch(&path, "bed")?;
    let file = std::fs::File::open(&path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("bed(): cannot open '{path}': {e}"), None)
    })?;
    let reader = BufReader::with_capacity(128 * 1024, file);
    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut num_fields = 0;
    let mut line_buf = String::new();

    let mut buf_reader = reader;
    loop {
        line_buf.clear();
        let bytes = buf_reader.read_line(&mut line_buf).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("bed(): read error: {e}"), None)
        })?;
        if bytes == 0 { break; }
        let line = line_buf.trim_end();
        if line.is_empty() || line.starts_with('#') || line.starts_with("track") || line.starts_with("browser") {
            continue;
        }
        // Manual tab-split avoids Vec<&str> allocation
        let mut tabs = line.splitn(7, '\t');
        let chrom = match tabs.next() { Some(s) => s, None => continue };
        let start = match tabs.next() { Some(s) => s, None => continue };
        let end = match tabs.next() { Some(s) => s, None => continue };
        let name = tabs.next();
        let score = tabs.next();
        let strand = tabs.next();

        let nf = 3 + name.is_some() as usize + score.is_some() as usize + strand.is_some() as usize;
        if num_fields == 0 {
            num_fields = nf;
        }
        let mut row = Vec::with_capacity(nf.min(6));
        row.push(Value::Str(chrom.to_string()));
        row.push(parse_int_field(start));
        row.push(parse_int_field(end));
        if let Some(n) = name {
            row.push(Value::Str(n.to_string()));
        }
        if let Some(s) = score {
            row.push(parse_numeric_field(s));
        }
        if let Some(s) = strand {
            row.push(Value::Str(s.to_string()));
        }
        rows.push(row);
    }

    let columns = match num_fields.min(6) {
        0..=3 => vec!["chrom", "start", "end"],
        4 => vec!["chrom", "start", "end", "name"],
        5 => vec!["chrom", "start", "end", "name", "score"],
        _ => vec!["chrom", "start", "end", "name", "score", "strand"],
    };
    let columns: Vec<String> = columns.iter().map(|s| s.to_string()).collect();

    Ok(Value::Table(Table::new(columns, rows)))
}

/// Read a GFF3 file, returning a Table.
pub fn read_gff(path: &str) -> Result<Value> {
    use std::io::{BufRead, BufReader};

    let path = resolve_data_path(path);
    check_format_mismatch(&path, "gff")?;
    let file = std::fs::File::open(&path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("gff(): cannot open '{path}': {e}"), None)
    })?;
    let reader = BufReader::with_capacity(128 * 1024, file);
    let columns: Vec<String> = vec![
        "seqid", "source", "type", "start", "end", "score", "strand", "phase", "attributes",
    ].into_iter().map(|s| s.to_string()).collect();

    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut line_buf = String::new();
    let mut buf_reader = reader;

    // String interning for repeated values (seqid, source, type, strand, phase)
    let mut intern: HashMap<Box<str>, Value> = HashMap::with_capacity(64);
    let mut intern_val = |s: &str| -> Value {
        if let Some(cached) = intern.get(s) {
            return cached.clone();
        }
        let val = Value::Str(s.to_string());
        intern.insert(s.into(), val.clone());
        val
    };

    loop {
        line_buf.clear();
        let bytes = buf_reader.read_line(&mut line_buf).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("gff(): read error: {e}"), None)
        })?;
        if bytes == 0 { break; }
        let line = line_buf.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Manual tab-split avoids Vec<&str> allocation per line
        let mut tabs = line.splitn(10, '\t');
        let seqid = match tabs.next() { Some(s) => s, None => continue };
        let source = match tabs.next() { Some(s) => s, None => continue };
        let ftype = match tabs.next() { Some(s) => s, None => continue };
        let start = match tabs.next() { Some(s) => s, None => continue };
        let end = match tabs.next() { Some(s) => s, None => continue };
        let score = match tabs.next() { Some(s) => s, None => continue };
        let strand = match tabs.next() { Some(s) => s, None => continue };
        let phase = match tabs.next() { Some(s) => s, None => continue };
        let attrs = match tabs.next() { Some(s) => s, None => continue };

        let mut row = Vec::with_capacity(9);
        row.push(intern_val(seqid));
        row.push(intern_val(source));
        row.push(intern_val(ftype));
        row.push(parse_int_field(start));
        row.push(parse_int_field(end));
        row.push(parse_score_field(score));
        row.push(intern_val(strand));
        row.push(intern_val(phase));
        // Attributes are unique per row — no interning
        row.push(Value::Str(attrs.to_string()));
        rows.push(row);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

/// Read a VCF file, returning a Table.
pub fn read_vcf(path: &str) -> Result<Value> {
    use std::io::{BufRead, BufReader};

    let path = resolve_data_path(path);
    check_format_mismatch(&path, "vcf")?;
    let file = std::fs::File::open(&path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("read_vcf(): cannot open '{path}': {e}"), None)
    })?;
    let reader = BufReader::with_capacity(128 * 1024, file);
    let mut variants: Vec<Value> = Vec::new();
    let mut line_buf = String::new();
    let mut buf_reader = reader;

    // String interning for repeated values (chrom, filter, id)
    let mut intern: HashMap<Box<str>, String> = HashMap::with_capacity(64);
    let mut intern_str = |s: &str| -> String {
        if let Some(cached) = intern.get(s) {
            return cached.clone();
        }
        let owned = s.to_string();
        intern.insert(s.into(), owned.clone());
        owned
    };

    loop {
        line_buf.clear();
        let bytes = buf_reader.read_line(&mut line_buf).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("read_vcf(): read error: {e}"), None)
        })?;
        if bytes == 0 { break; }
        let line = line_buf.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Manual tab-split: avoids Vec<&str> allocation per line
        let mut tabs = line.splitn(9, '\t');
        let chrom = match tabs.next() { Some(s) => s, None => continue };
        let pos_s = match tabs.next() { Some(s) => s, None => continue };
        let id = match tabs.next() { Some(s) => s, None => continue };
        let ref_a = match tabs.next() { Some(s) => s, None => continue };
        let alt_a = match tabs.next() { Some(s) => s, None => continue };
        let qual_s = match tabs.next() { Some(s) => s, None => continue };
        let filt = match tabs.next() { Some(s) => s, None => continue };
        let info_s = match tabs.next() { Some(s) => s, None => continue };

        let quality = match qual_s {
            "." => 0.0,
            s => s.parse::<f64>().unwrap_or(0.0),
        };
        // Store raw INFO string — parsed lazily on .info access
        // Skip storing _raw for "." (empty INFO) to reduce allocations
        let info = if info_s == "." {
            HashMap::new()
        } else {
            let mut m = HashMap::with_capacity(1);
            m.insert("_raw".into(), Value::Str(info_s.to_string()));
            m
        };
        variants.push(Value::Variant {
            chrom: intern_str(chrom),
            pos: pos_s.parse::<i64>().unwrap_or(0),
            id: intern_str(id),
            ref_allele: intern_str(ref_a),
            alt_allele: intern_str(alt_a),
            quality,
            filter: intern_str(filt),
            info,
        });
    }

    Ok(Value::List(variants))
}

fn read_file_content(path: &str, func: &str) -> Result<String> {
    let path = resolve_data_path(path);
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("file not found: {path}"),
            None,
        ));
    }
    // Guard against reading very large files into memory
    if let Ok(meta) = std::fs::metadata(&path) {
        let size = meta.len();
        if size > 500_000_000 {
            return Err(BioLangError::runtime(
                ErrorKind::IOError,
                format!(
                    "{func}(): file is too large to read eagerly ({} MB). \
                     Use a streaming reader instead (e.g. read_fasta(), read_vcf_stream()).",
                    size / 1_000_000
                ),
                None,
            ));
        }
    }
    check_format_mismatch(&path, func)?;
    std::fs::read_to_string(&path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("{func}(): cannot read file '{path}': {e}"),
            None,
        )
    })
}

fn parse_int_field(s: &str) -> Value {
    s.parse::<i64>().map(Value::Int).unwrap_or_else(|_| Value::Str(s.to_string()))
}

fn parse_numeric_field(s: &str) -> Value {
    if s == "." {
        return Value::Nil; // VCF missing value
    }
    if let Ok(n) = s.parse::<i64>() {
        Value::Int(n)
    } else if let Ok(f) = s.parse::<f64>() {
        Value::Float(f)
    } else {
        Value::Str(s.to_string())
    }
}

/// Parse a VCF INFO string (e.g. "DP=30;AF=0.5;MQ=60;DB") into a Record.
/// Key=Value pairs become {key: value}, bare flags become {key: true}.
/// Comma-separated values (e.g. "AF=0.45,0.12") become a List.
/// Values are parsed as Int, Float, or Str. "." means empty record.
fn parse_vcf_info_field(s: &str) -> Value {
    if s == "." || s.is_empty() {
        return Value::Record(HashMap::new());
    }
    let mut map = HashMap::new();
    for part in s.split(';') {
        if part.is_empty() {
            continue;
        }
        if let Some((key, val)) = part.split_once('=') {
            if val.contains(',') {
                // Multi-value field → parse each element, return as List
                let items: Vec<Value> = val.split(',').map(|v| parse_numeric_field(v)).collect();
                map.insert(key.to_string(), Value::List(items));
            } else {
                map.insert(key.to_string(), parse_numeric_field(val));
            }
        } else {
            // Bare flag (e.g. "DB", "PASS")
            map.insert(part.to_string(), Value::Bool(true));
        }
    }
    Value::Record(map)
}

fn parse_score_field(s: &str) -> Value {
    if s == "." {
        Value::Nil
    } else {
        parse_numeric_field(s)
    }
}

// ── validate() ──────────────────────────────────────────────────

/// Auto-detect format from extension and validate a bio file.
/// Returns Record { valid, format, errors, lines_checked }.
pub fn validate_file(path: &str) -> Result<Value> {
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("file not found: {path}"),
            None,
        ));
    }

    let name = path.to_lowercase();
    let format = if name.ends_with(".fasta") || name.ends_with(".fa") || name.ends_with(".fna")
        || name.ends_with(".fasta.gz") || name.ends_with(".fa.gz")
    {
        "fasta"
    } else if name.ends_with(".fastq") || name.ends_with(".fq")
        || name.ends_with(".fastq.gz") || name.ends_with(".fq.gz")
    {
        "fastq"
    } else if name.ends_with(".vcf") || name.ends_with(".vcf.gz") {
        "vcf"
    } else if name.ends_with(".bed") {
        "bed"
    } else if name.ends_with(".gff") || name.ends_with(".gff3") || name.ends_with(".gtf") {
        "gff"
    } else if name.ends_with(".bam") {
        "bam"
    } else if name.ends_with(".sam") {
        "sam"
    } else if name.ends_with(".maf") || name.ends_with(".maf.gz") {
        "maf"
    } else if name.ends_with(".bedgraph") || name.ends_with(".bdg") {
        "bedgraph"
    } else {
        "unknown"
    };

    let mut errors: Vec<Value> = Vec::new();
    let mut lines_checked: i64 = 0;

    match format {
        "fasta" => {
            let content = read_file_content(path, "validate")?;
            let mut in_seq = false;
            for (i, line) in content.lines().enumerate() {
                lines_checked = (i + 1) as i64;
                if line.is_empty() {
                    continue;
                }
                if line.starts_with('>') {
                    if line.len() < 2 {
                        errors.push(Value::Str(format!("line {}: empty header", i + 1)));
                    }
                    in_seq = true;
                } else if in_seq {
                    if !line.chars().all(|c| "ACGTNacgtnRYSWKMBDHVryswkmbdhv.-".contains(c)) {
                        errors.push(Value::Str(format!(
                            "line {}: invalid character in sequence",
                            i + 1
                        )));
                    }
                } else {
                    errors.push(Value::Str(format!(
                        "line {}: sequence data before first header",
                        i + 1
                    )));
                }
                if errors.len() >= 20 {
                    break;
                }
            }
            if lines_checked == 0 {
                errors.push(Value::Str("empty file".into()));
            }
        }
        "fastq" => {
            let content = read_file_content(path, "validate")?;
            let lines: Vec<&str> = content.lines().collect();
            lines_checked = lines.len() as i64;
            if !lines.len().is_multiple_of(4) && !lines.is_empty() {
                errors.push(Value::Str(format!(
                    "line count {} not divisible by 4",
                    lines.len()
                )));
            }
            for (i, chunk) in lines.chunks(4).enumerate() {
                if chunk.len() < 4 {
                    break;
                }
                let line_num = i * 4 + 1;
                if !chunk[0].starts_with('@') {
                    errors.push(Value::Str(format!(
                        "line {line_num}: header should start with @"
                    )));
                }
                if !chunk[2].starts_with('+') {
                    errors.push(Value::Str(format!(
                        "line {}: separator should start with +",
                        line_num + 2
                    )));
                }
                if chunk[1].len() != chunk[3].len() {
                    errors.push(Value::Str(format!(
                        "line {}: sequence/quality length mismatch ({} vs {})",
                        line_num,
                        chunk[1].len(),
                        chunk[3].len()
                    )));
                }
                if errors.len() >= 20 {
                    break;
                }
            }
        }
        "vcf" => {
            let content = read_file_content(path, "validate")?;
            let mut found_header = false;
            for (i, line) in content.lines().enumerate() {
                lines_checked = (i + 1) as i64;
                if line.starts_with("##") {
                    continue;
                }
                if line.starts_with("#CHROM") {
                    found_header = true;
                    let cols: Vec<&str> = line.split('\t').collect();
                    if cols.len() < 8 {
                        errors.push(Value::Str(format!(
                            "line {}: #CHROM header has {} columns, expected ≥ 8",
                            i + 1,
                            cols.len()
                        )));
                    }
                    continue;
                }
                if !found_header && !line.starts_with('#') {
                    errors.push(Value::Str("missing #CHROM header line".into()));
                    found_header = true; // don't repeat
                }
                if !line.starts_with('#') {
                    let fields: Vec<&str> = line.split('\t').collect();
                    if fields.len() < 8 {
                        errors.push(Value::Str(format!(
                            "line {}: only {} fields, expected ≥ 8",
                            i + 1,
                            fields.len()
                        )));
                    }
                    if fields.len() >= 2 && fields[1].parse::<i64>().is_err() {
                        errors.push(Value::Str(format!(
                            "line {}: POS '{}' is not an integer",
                            i + 1,
                            fields[1]
                        )));
                    }
                }
                if errors.len() >= 20 {
                    break;
                }
            }
        }
        "bed" => {
            let content = read_file_content(path, "validate")?;
            for (i, line) in content.lines().enumerate() {
                lines_checked = (i + 1) as i64;
                if line.is_empty()
                    || line.starts_with('#')
                    || line.starts_with("track")
                    || line.starts_with("browser")
                {
                    continue;
                }
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() < 3 {
                    errors.push(Value::Str(format!(
                        "line {}: only {} fields, expected ≥ 3",
                        i + 1,
                        fields.len()
                    )));
                    continue;
                }
                if fields[1].parse::<i64>().is_err() {
                    errors.push(Value::Str(format!(
                        "line {}: start '{}' is not an integer",
                        i + 1,
                        fields[1]
                    )));
                }
                if fields[2].parse::<i64>().is_err() {
                    errors.push(Value::Str(format!(
                        "line {}: end '{}' is not an integer",
                        i + 1,
                        fields[2]
                    )));
                }
                if errors.len() >= 20 {
                    break;
                }
            }
        }
        "bam" => {
            // Check BAM magic bytes
            let file = open_file(path)?;
            let mut reader = BufReader::new(file);
            let mut magic = [0u8; 4];
            use std::io::Read;
            if reader.read_exact(&mut magic).is_err() {
                errors.push(Value::Str("file too small for BAM".into()));
            } else if &magic != b"BAM\x01" {
                errors.push(Value::Str(format!(
                    "invalid BAM magic: expected BAM\\1, got {:?}",
                    &magic
                )));
            }
            lines_checked = 1;
        }
        "sam" | "gff" => {
            let content = read_file_content(path, "validate")?;
            lines_checked = content.lines().count() as i64;
            // Basic check: non-empty
            if lines_checked == 0 {
                errors.push(Value::Str("empty file".into()));
            }
        }
        _ => {
            errors.push(Value::Str(format!("unknown format for '{path}'")));
        }
    }

    let mut rec = HashMap::new();
    rec.insert("valid".to_string(), Value::Bool(errors.is_empty()));
    rec.insert("format".to_string(), Value::Str(format.to_string()));
    rec.insert("errors".to_string(), Value::List(errors));
    rec.insert("lines_checked".to_string(), Value::Int(lines_checked));
    Ok(Value::Record(rec))
}

// ── vcf_filter() ────────────────────────────────────────────────

/// Filter VCF records by a simple expression.
/// Supports: QUAL > N, QUAL < N, QUAL >= N, QUAL <= N, FILTER == PASS, DP > N (from INFO).
/// Returns a Table of matching records.
pub fn vcf_filter(path: &str, expr: &str) -> Result<Value> {
    let content = read_file_content(path, "vcf_filter")?;

    let filters = parse_vcf_filters(expr)?;
    let mut variants: Vec<Value> = Vec::new();

    for line in content.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 8 {
            continue;
        }

        // Check all filter conditions
        let mut pass = true;
        for f in &filters {
            if !eval_vcf_condition(f, &fields) {
                pass = false;
                break;
            }
        }

        if pass {
            let quality = match parse_numeric_field(fields[5]) {
                Value::Float(f) => f,
                Value::Int(i) => i as f64,
                _ => 0.0,
            };
            let info = match parse_vcf_info_field(fields[7]) {
                Value::Record(m) => m,
                _ => HashMap::new(),
            };
            variants.push(Value::Variant {
                chrom: fields[0].to_string(),
                pos: fields[1].parse::<i64>().unwrap_or(0),
                id: fields[2].to_string(),
                ref_allele: fields[3].to_string(),
                alt_allele: fields[4].to_string(),
                quality,
                filter: fields[6].to_string(),
                info,
            });
        }
    }

    Ok(Value::List(variants))
}

enum VcfCondition {
    QualCmp(CmpOp, f64),
    FilterEq(String),
    InfoCmp(String, CmpOp, f64),
    InfoFlag(String),
}

#[derive(Clone, Copy)]
enum CmpOp {
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq,
}

fn parse_vcf_filters(expr: &str) -> Result<Vec<VcfCondition>> {
    let mut conditions = Vec::new();
    // Split on && or &
    let parts: Vec<&str> = expr.split("&&").flat_map(|s| s.split('&')).collect();

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Parse: FIELD OP VALUE
        let (field, op, value) = parse_condition_parts(part)?;

        match field.to_uppercase().as_str() {
            "QUAL" => {
                let num = value.parse::<f64>().map_err(|_| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!("vcf_filter: invalid number '{value}' in QUAL condition"),
                        None,
                    )
                })?;
                conditions.push(VcfCondition::QualCmp(op, num));
            }
            "FILTER" => {
                conditions.push(VcfCondition::FilterEq(value.to_string()));
            }
            _ => {
                // Treat as INFO field
                if value.is_empty() {
                    conditions.push(VcfCondition::InfoFlag(field.to_string()));
                } else if let Ok(num) = value.parse::<f64>() {
                    conditions.push(VcfCondition::InfoCmp(field.to_string(), op, num));
                } else {
                    conditions.push(VcfCondition::InfoFlag(field.to_string()));
                }
            }
        }
    }
    Ok(conditions)
}

fn parse_condition_parts(s: &str) -> Result<(&str, CmpOp, &str)> {
    for (tok, op) in &[
        (">=", CmpOp::Gte),
        ("<=", CmpOp::Lte),
        ("!=", CmpOp::Neq),
        ("==", CmpOp::Eq),
        (">", CmpOp::Gt),
        ("<", CmpOp::Lt),
        ("=", CmpOp::Eq),
    ] {
        if let Some(pos) = s.find(tok) {
            let field = s[..pos].trim();
            let value = s[pos + tok.len()..].trim();
            return Ok((field, *op, value));
        }
    }
    // No operator — treat as a flag check
    Ok((s.trim(), CmpOp::Eq, ""))
}

fn eval_vcf_condition(cond: &VcfCondition, fields: &[&str]) -> bool {
    match cond {
        VcfCondition::QualCmp(op, threshold) => {
            if let Ok(qual) = fields[5].parse::<f64>() {
                cmp_f64(qual, *op, *threshold)
            } else {
                false // QUAL is "." or non-numeric → fails
            }
        }
        VcfCondition::FilterEq(val) => fields[6] == val.as_str(),
        VcfCondition::InfoCmp(key, op, threshold) => {
            extract_info_number(fields[7], key)
                .map(|n| cmp_f64(n, *op, *threshold))
                .unwrap_or(false)
        }
        VcfCondition::InfoFlag(key) => fields[7].split(';').any(|part| {
            part == key.as_str() || part.starts_with(&format!("{key}="))
        }),
    }
}

fn cmp_f64(a: f64, op: CmpOp, b: f64) -> bool {
    match op {
        CmpOp::Gt => a > b,
        CmpOp::Lt => a < b,
        CmpOp::Gte => a >= b,
        CmpOp::Lte => a <= b,
        CmpOp::Eq => (a - b).abs() < f64::EPSILON,
        CmpOp::Neq => (a - b).abs() >= f64::EPSILON,
    }
}

fn extract_info_number(info: &str, key: &str) -> Option<f64> {
    let prefix = format!("{key}=");
    for part in info.split(';') {
        if let Some(val) = part.strip_prefix(&prefix) {
            return val.parse().ok();
        }
    }
    None
}

// ── SAM reader ──────────────────────────────────────────────────

/// Read a SAM file, returning a Table.
/// Columns: qname, flag, rname, pos, mapq, cigar, rnext, pnext, tlen, seq, qual
/// Header lines (@) are skipped.
pub fn read_sam(path: &str) -> Result<Value> {
    let content = read_file_content(path, "sam")?;
    let columns: Vec<String> = [
        "qname", "flag", "rname", "pos", "mapq", "cigar", "rnext", "pnext", "tlen", "seq", "qual",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let mut rows: Vec<Vec<Value>> = Vec::new();
    for line in content.lines() {
        if line.is_empty() || line.starts_with('@') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 11 {
            continue;
        }
        let row = vec![
            Value::Str(fields[0].to_string()),  // qname
            parse_int_field(fields[1]),           // flag (Int)
            Value::Str(fields[2].to_string()),  // rname
            parse_int_field(fields[3]),           // pos (Int)
            parse_int_field(fields[4]),           // mapq (Int)
            Value::Str(fields[5].to_string()),  // cigar
            Value::Str(fields[6].to_string()),  // rnext
            parse_int_field(fields[7]),           // pnext (Int)
            parse_int_field(fields[8]),           // tlen (Int)
            Value::Str(fields[9].to_string()),  // seq
            Value::Str(fields[10].to_string()), // qual
        ];
        rows.push(row);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

/// Read SAM header lines, returning a List of Records.
/// Each record: {tag: "HD"|"SQ"|"RG"|"PG"|..., fields: Record of key-value pairs}
pub fn read_sam_header(path: &str) -> Result<Value> {
    let content = read_file_content(path, "sam_header")?;
    let mut headers: Vec<Value> = Vec::new();

    for line in content.lines() {
        if !line.starts_with('@') {
            break; // headers are at the top
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let tag = parts[0].trim_start_matches('@').to_string();
        let mut fields = HashMap::new();
        for part in &parts[1..] {
            if let Some((k, v)) = part.split_once(':') {
                fields.insert(k.to_string(), Value::Str(v.to_string()));
            }
        }
        let mut rec = HashMap::new();
        rec.insert("tag".to_string(), Value::Str(tag));
        rec.insert("fields".to_string(), Value::Record(fields));
        headers.push(Value::Record(rec));
    }

    Ok(Value::List(headers))
}

// ── BAM reader ──────────────────────────────────────────────────

/// Read a BAM file, returning a Table.
/// Same columns as SAM: qname, flag, rname, pos, mapq, cigar, rnext, pnext, tlen, seq, qual
pub fn read_bam(path: &str) -> Result<Value> {
    let path = resolve_data_path(path);
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("file not found: {path}"),
            None,
        ));
    }
    check_format_mismatch(&path, "bam")?;

    let columns: Vec<String> = [
        "qname", "flag", "rname", "pos", "mapq", "cigar", "rnext", "pnext", "tlen", "seq", "qual",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let file = open_file(&path)?;
    let mut reader = noodles_bam::io::Reader::new(BufReader::new(file));

    // Read header
    let header = reader.read_header().map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("bam(): cannot read header: {e}"), None)
    })?;

    // Build reference name lookup from header
    let ref_names: Vec<String> = header
        .reference_sequences()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();

    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut record = noodles_sam::alignment::RecordBuf::default();
    loop {
        match reader.read_record_buf(&header, &mut record) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let qname = record
                    .name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "*".to_string());
                let flags = record.flags().bits();
                let rname = record
                    .reference_sequence_id()
                    .and_then(|id| ref_names.get(id))
                    .cloned()
                    .unwrap_or_else(|| "*".to_string());
                let pos = record
                    .alignment_start()
                    .map(|p| usize::from(p) as i64)
                    .unwrap_or(0);
                let mapq = record
                    .mapping_quality()
                    .map(|q| u8::from(q) as i64)
                    .unwrap_or(255);
                let cigar = {
                    let ops = record.cigar();
                    if ops.is_empty() {
                        "*".to_string()
                    } else {
                        use noodles_sam::alignment::record::cigar::op::Kind;
                        use noodles_sam::alignment::record::cigar::Op;
                        ops.iter()
                            .filter_map(|r| r.ok())
                            .map(|op: Op| {
                                let ch = match op.kind() {
                                    Kind::Match => 'M',
                                    Kind::Insertion => 'I',
                                    Kind::Deletion => 'D',
                                    Kind::Skip => 'N',
                                    Kind::SoftClip => 'S',
                                    Kind::HardClip => 'H',
                                    Kind::Pad => 'P',
                                    Kind::SequenceMatch => '=',
                                    Kind::SequenceMismatch => 'X',
                                };
                                format!("{}{ch}", op.len())
                            })
                            .collect::<String>()
                    }
                };
                let rnext = record
                    .mate_reference_sequence_id()
                    .and_then(|id| ref_names.get(id))
                    .cloned()
                    .unwrap_or_else(|| "*".to_string());
                let pnext = record
                    .mate_alignment_start()
                    .map(|p| usize::from(p) as i64)
                    .unwrap_or(0);
                let tlen = record.template_length();
                let seq = {
                    let seq_buf = record.sequence();
                    if seq_buf.is_empty() {
                        "*".to_string()
                    } else {
                        seq_buf.iter().map(char::from).collect::<String>()
                    }
                };
                let qual = {
                    let q = record.quality_scores();
                    if q.is_empty() {
                        "*".to_string()
                    } else {
                        q.iter().map(|s| (s + 33) as char).collect()
                    }
                };

                let row = vec![
                    Value::Str(qname),
                    Value::Int(flags as i64),
                    Value::Str(rname),
                    Value::Int(pos),
                    Value::Int(mapq),
                    Value::Str(cigar),
                    Value::Str(rnext),
                    Value::Int(pnext),
                    Value::Int(tlen as i64),
                    Value::Str(seq),
                    Value::Str(qual),
                ];
                rows.push(row);
            }
            Err(e) => {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("bam(): read error: {e}"),
                    None,
                ));
            }
        }
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── MAF reader ──────────────────────────────────────────────────

/// Read a MAF (Mutation Annotation Format) file, returning a Table.
/// Auto-detects columns from header line (Hugo_Symbol line).
/// Standard columns: Hugo_Symbol, Entrez_Gene_Id, Center, NCBI_Build, Chromosome,
///   Start_Position, End_Position, Strand, Variant_Classification, Variant_Type,
///   Reference_Allele, Tumor_Seq_Allele1, Tumor_Seq_Allele2, ...
pub fn read_maf(path: &str) -> Result<Value> {
    let content = read_file_content(path, "maf")?;
    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for line in content.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();

        // First non-comment line is the header
        if columns.is_empty() {
            columns = fields.iter().map(|s| s.to_string()).collect();
            continue;
        }

        let row: Vec<Value> = fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                // Parse numeric columns by known names
                let col_name = columns.get(i).map(|s| s.as_str()).unwrap_or("");
                match col_name {
                    "Start_Position" | "End_Position" | "Entrez_Gene_Id" | "t_depth"
                    | "t_ref_count" | "t_alt_count" | "n_depth" | "n_ref_count"
                    | "n_alt_count" => parse_int_field(f),
                    _ => Value::Str(f.to_string()),
                }
            })
            .collect();
        rows.push(row);
    }

    if columns.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            "maf(): no header line found".to_string(),
            None,
        ));
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── bedGraph reader ─────────────────────────────────────────────

/// Read a bedGraph file, returning a Table.
/// Columns: chrom, start, end, value
/// Supports track/browser header lines.
pub fn read_bedgraph(path: &str) -> Result<Value> {
    let content = read_file_content(path, "bedgraph")?;
    let columns: Vec<String> = ["chrom", "start", "end", "value"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut rows: Vec<Vec<Value>> = Vec::new();
    for line in content.lines() {
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("track")
            || line.starts_with("browser")
        {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        // Also try space-separated (some tools output this)
        let fields: Vec<&str> = if fields.len() < 4 {
            line.split_whitespace().collect()
        } else {
            fields
        };
        if fields.len() < 4 {
            continue;
        }
        let row = vec![
            Value::Str(fields[0].to_string()), // chrom
            parse_int_field(fields[1]),          // start
            parse_int_field(fields[2]),          // end
            parse_numeric_field(fields[3]),       // value (float or int)
        ];
        rows.push(row);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── Streaming iterators ─────────────────────────────────────────
//
// Each yields Value::Record with the same field names as the
// corresponding Table reader, so `collect() |> table()` round-trips.

/// Helper: open a file for streaming — supports .gz via flate2.
pub fn open_buf_reader(path: &str) -> Result<Box<dyn std::io::BufRead + Send>> {
    let file = open_file(path)?;
    if path.ends_with(".gz") {
        let decoder = flate2::read::GzDecoder::new(file);
        Ok(Box::new(BufReader::new(decoder)))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
}

// ── BED streaming ───────────────────────────────────────────────

struct BedIter<R: std::io::BufRead> {
    lines: std::io::Lines<R>,
}

impl<R: std::io::BufRead + Send> Iterator for BedIter<R> {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        loop {
            let line = self.lines.next()?.ok()?;
            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with("track")
                || line.starts_with("browser")
            {
                continue;
            }
            let mut tabs = line.splitn(7, '\t');
            let chrom = match tabs.next() { Some(s) => s, None => continue };
            let start = match tabs.next() { Some(s) => s, None => continue };
            let end = match tabs.next() { Some(s) => s, None => continue };
            let name = tabs.next();
            let score = tabs.next();
            let strand = tabs.next();

            let cap = 3 + name.is_some() as usize + score.is_some() as usize + strand.is_some() as usize;
            let mut map = HashMap::with_capacity(cap);
            map.insert("chrom".into(), Value::Str(chrom.to_string()));
            map.insert("start".into(), parse_int_field(start));
            map.insert("end".into(), parse_int_field(end));
            if let Some(n) = name {
                map.insert("name".into(), Value::Str(n.to_string()));
            }
            if let Some(s) = score {
                map.insert("score".into(), parse_numeric_field(s));
            }
            if let Some(s) = strand {
                map.insert("strand".into(), Value::Str(s.to_string()));
            }
            return Some(Value::Record(map));
        }
    }
}

pub fn read_bed_stream(path: &str) -> Result<Value> {
    let path = &check_file_exists(path)?;
    check_format_mismatch(path, "bed")?;
    use std::io::BufRead;
    let reader = open_buf_reader(path)?;
    let label = format!("bed:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(BedIter { lines: reader.lines() }),
    )))
}

// ── GFF streaming ───────────────────────────────────────────────

struct GffIter<R: std::io::BufRead> {
    lines: std::io::Lines<R>,
}

impl<R: std::io::BufRead + Send> Iterator for GffIter<R> {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        loop {
            let line = self.lines.next()?.ok()?;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut tabs = line.splitn(10, '\t');
            let seqid = match tabs.next() { Some(s) => s, None => continue };
            let source = match tabs.next() { Some(s) => s, None => continue };
            let ftype = match tabs.next() { Some(s) => s, None => continue };
            let start = match tabs.next() { Some(s) => s, None => continue };
            let end = match tabs.next() { Some(s) => s, None => continue };
            let score = match tabs.next() { Some(s) => s, None => continue };
            let strand = match tabs.next() { Some(s) => s, None => continue };
            let phase = match tabs.next() { Some(s) => s, None => continue };
            let attrs = match tabs.next() { Some(s) => s, None => continue };

            let mut map = HashMap::with_capacity(9);
            map.insert("seqid".into(), Value::Str(seqid.to_string()));
            map.insert("source".into(), Value::Str(source.to_string()));
            map.insert("type".into(), Value::Str(ftype.to_string()));
            map.insert("start".into(), parse_int_field(start));
            map.insert("end".into(), parse_int_field(end));
            map.insert("score".into(), parse_score_field(score));
            map.insert("strand".into(), Value::Str(strand.to_string()));
            map.insert("phase".into(), Value::Str(phase.to_string()));
            map.insert("attributes".into(), Value::Str(attrs.to_string()));
            return Some(Value::Record(map));
        }
    }
}

pub fn read_gff_stream(path: &str) -> Result<Value> {
    let path = &check_file_exists(path)?;
    check_format_mismatch(path, "gff")?;
    use std::io::BufRead;
    let reader = open_buf_reader(path)?;
    let label = format!("gff:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(GffIter { lines: reader.lines() }),
    )))
}

// ── VCF streaming ───────────────────────────────────────────────

struct VcfIter<R: std::io::BufRead> {
    lines: std::io::Lines<R>,
    intern: HashMap<Box<str>, String>,
}

impl<R: std::io::BufRead + Send> VcfIter<R> {
    fn intern_str(&mut self, s: &str) -> String {
        if let Some(cached) = self.intern.get(s) {
            return cached.clone();
        }
        let owned = s.to_string();
        self.intern.insert(s.into(), owned.clone());
        owned
    }
}

impl<R: std::io::BufRead + Send> Iterator for VcfIter<R> {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        loop {
            let line = self.lines.next()?.ok()?;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut tabs = line.splitn(9, '\t');
            let chrom = match tabs.next() { Some(s) => s, None => continue };
            let pos_s = match tabs.next() { Some(s) => s, None => continue };
            let id = match tabs.next() { Some(s) => s, None => continue };
            let ref_a = match tabs.next() { Some(s) => s, None => continue };
            let alt_a = match tabs.next() { Some(s) => s, None => continue };
            let qual_s = match tabs.next() { Some(s) => s, None => continue };
            let filt = match tabs.next() { Some(s) => s, None => continue };
            let info_s = match tabs.next() { Some(s) => s, None => continue };

            let quality = match qual_s {
                "." => 0.0,
                s => s.parse::<f64>().unwrap_or(0.0),
            };
            // Store raw INFO string — parsed lazily on .info access
            let info = if info_s == "." {
                HashMap::new()
            } else {
                let mut m = HashMap::with_capacity(1);
                m.insert("_raw".into(), Value::Str(info_s.to_string()));
                m
            };
            let chrom_s = self.intern_str(chrom);
            let id_s = self.intern_str(id);
            let filt_s = self.intern_str(filt);
            let ref_s = self.intern_str(ref_a);
            let alt_s = self.intern_str(alt_a);
            return Some(Value::Variant {
                chrom: chrom_s,
                pos: pos_s.parse::<i64>().unwrap_or(0),
                id: id_s,
                ref_allele: ref_s,
                alt_allele: alt_s,
                quality,
                filter: filt_s,
                info,
            });
        }
    }
}

pub fn read_vcf_stream(path: &str) -> Result<Value> {
    let path = &check_file_exists(path)?;
    check_format_mismatch(path, "vcf")?;
    use std::io::BufRead;
    let reader = open_buf_reader(path)?;
    let label = format!("vcf:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(VcfIter { lines: reader.lines(), intern: HashMap::with_capacity(64) }),
    )))
}

// ── SAM streaming ───────────────────────────────────────────────

struct SamIter<R: std::io::BufRead> {
    lines: std::io::Lines<R>,
}

impl<R: std::io::BufRead + Send> Iterator for SamIter<R> {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        loop {
            let line = self.lines.next()?.ok()?;
            if line.is_empty() || line.starts_with('@') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 11 {
                continue;
            }
            let mut map = HashMap::new();
            map.insert("qname".into(), Value::Str(fields[0].to_string()));
            map.insert("flag".into(), parse_int_field(fields[1]));
            map.insert("rname".into(), Value::Str(fields[2].to_string()));
            map.insert("pos".into(), parse_int_field(fields[3]));
            map.insert("mapq".into(), parse_int_field(fields[4]));
            map.insert("cigar".into(), Value::Str(fields[5].to_string()));
            map.insert("rnext".into(), Value::Str(fields[6].to_string()));
            map.insert("pnext".into(), parse_int_field(fields[7]));
            map.insert("tlen".into(), parse_int_field(fields[8]));
            map.insert("seq".into(), Value::Str(fields[9].to_string()));
            map.insert("qual".into(), Value::Str(fields[10].to_string()));
            return Some(Value::Record(map));
        }
    }
}

pub fn read_sam_stream(path: &str) -> Result<Value> {
    let path = &check_file_exists(path)?;
    check_format_mismatch(path, "sam")?;
    use std::io::BufRead;
    let reader = open_buf_reader(path)?;
    let label = format!("sam:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(SamIter { lines: reader.lines() }),
    )))
}

// ── BAM streaming ───────────────────────────────────────────────

struct BamIter {
    reader: noodles_bam::io::Reader<noodles_bgzf::io::reader::Reader<BufReader<File>>>,
    header: noodles_sam::Header,
    ref_names: Vec<String>,
    record_buf: noodles_sam::alignment::RecordBuf,
}

impl Iterator for BamIter {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        use noodles_sam::alignment::record::cigar::op::Kind;
        use noodles_sam::alignment::record::cigar::Op;

        match self.reader.read_record_buf(&self.header, &mut self.record_buf) {
            Ok(0) | Err(_) => None,
            Ok(_) => {
                let qname = self.record_buf
                    .name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "*".to_string());
                let flags = self.record_buf.flags().bits();
                let rname = self.record_buf
                    .reference_sequence_id()
                    .and_then(|id| self.ref_names.get(id))
                    .cloned()
                    .unwrap_or_else(|| "*".to_string());
                let pos = self.record_buf
                    .alignment_start()
                    .map(|p| usize::from(p) as i64)
                    .unwrap_or(0);
                let mapq = self.record_buf
                    .mapping_quality()
                    .map(|q| u8::from(q) as i64)
                    .unwrap_or(255);
                let cigar = {
                    let ops = self.record_buf.cigar();
                    if ops.is_empty() {
                        "*".to_string()
                    } else {
                        ops.iter()
                            .filter_map(|r| r.ok())
                            .map(|op: Op| {
                                let ch = match op.kind() {
                                    Kind::Match => 'M',
                                    Kind::Insertion => 'I',
                                    Kind::Deletion => 'D',
                                    Kind::Skip => 'N',
                                    Kind::SoftClip => 'S',
                                    Kind::HardClip => 'H',
                                    Kind::Pad => 'P',
                                    Kind::SequenceMatch => '=',
                                    Kind::SequenceMismatch => 'X',
                                };
                                format!("{}{ch}", op.len())
                            })
                            .collect::<String>()
                    }
                };
                let rnext = self.record_buf
                    .mate_reference_sequence_id()
                    .and_then(|id| self.ref_names.get(id))
                    .cloned()
                    .unwrap_or_else(|| "*".to_string());
                let pnext = self.record_buf
                    .mate_alignment_start()
                    .map(|p| usize::from(p) as i64)
                    .unwrap_or(0);
                let tlen = self.record_buf.template_length();
                let seq = {
                    let seq_buf = self.record_buf.sequence();
                    if seq_buf.is_empty() {
                        "*".to_string()
                    } else {
                        seq_buf.iter().map(char::from).collect::<String>()
                    }
                };
                let qual = {
                    let q = self.record_buf.quality_scores();
                    if q.is_empty() {
                        "*".to_string()
                    } else {
                        q.iter().map(|s| (s + 33) as char).collect()
                    }
                };

                let mut map = HashMap::new();
                map.insert("qname".into(), Value::Str(qname));
                map.insert("flag".into(), Value::Int(flags as i64));
                map.insert("rname".into(), Value::Str(rname));
                map.insert("pos".into(), Value::Int(pos));
                map.insert("mapq".into(), Value::Int(mapq));
                map.insert("cigar".into(), Value::Str(cigar));
                map.insert("rnext".into(), Value::Str(rnext));
                map.insert("pnext".into(), Value::Int(pnext));
                map.insert("tlen".into(), Value::Int(tlen as i64));
                map.insert("seq".into(), Value::Str(seq));
                map.insert("qual".into(), Value::Str(qual));
                Some(Value::Record(map))
            }
        }
    }
}

// SAFETY: BamIter owns its reader/header exclusively and is only accessed
// through the Arc<Mutex<>> inside StreamValue.
unsafe impl Send for BamIter {}

pub fn read_bam_stream(path: &str) -> Result<Value> {
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("file not found: {path}"),
            None,
        ));
    }
    check_format_mismatch(path, "bam")?;

    let file = open_file(path)?;
    let mut reader = noodles_bam::io::Reader::new(BufReader::new(file));
    let header = reader.read_header().map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("bam(): cannot read header: {e}"), None)
    })?;
    let ref_names: Vec<String> = header
        .reference_sequences()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();
    let label = format!("bam:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(BamIter {
            reader,
            header,
            ref_names,
            record_buf: noodles_sam::alignment::RecordBuf::default(),
        }),
    )))
}

// ── SAM/BAM -> AlignedRead streaming ────────────────────────────

struct SamReadIter<R: std::io::BufRead> {
    lines: std::io::Lines<R>,
}

impl<R: std::io::BufRead + Send> Iterator for SamReadIter<R> {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        loop {
            let line = self.lines.next()?.ok()?;
            if line.is_empty() || line.starts_with('@') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 11 {
                continue;
            }
            return Some(Value::AlignedRead(bio_core::AlignedRead {
                qname: fields[0].to_string(),
                flag: fields[1].parse().unwrap_or(0),
                rname: fields[2].to_string(),
                pos: fields[3].parse().unwrap_or(0),
                mapq: fields[4].parse().unwrap_or(0),
                cigar: fields[5].to_string(),
                rnext: fields[6].to_string(),
                pnext: fields[7].parse().unwrap_or(0),
                tlen: fields[8].parse().unwrap_or(0),
                seq: fields[9].to_string(),
                qual: fields[10].to_string(),
            }));
        }
    }
}

/// Read SAM file as a Stream of AlignedRead values.
pub fn read_sam_records(path: &str) -> Result<Value> {
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("file not found: {path}"),
            None,
        ));
    }
    let file = open_file(path)?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;
    let label = format!("sam_records:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(SamReadIter { lines: reader.lines() }),
    )))
}

struct BamReadIter {
    reader: noodles_bam::io::Reader<noodles_bgzf::io::reader::Reader<BufReader<File>>>,
    header: noodles_sam::Header,
    ref_names: Vec<String>,
    record_buf: noodles_sam::alignment::RecordBuf,
}

impl Iterator for BamReadIter {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        use noodles_sam::alignment::record::cigar::op::Kind;
        use noodles_sam::alignment::record::cigar::Op;

        match self.reader.read_record_buf(&self.header, &mut self.record_buf) {
            Ok(0) | Err(_) => None,
            Ok(_) => {
                let rec = &self.record_buf;
                let qname = rec.name().map(|n| n.to_string()).unwrap_or_else(|| "*".to_string());
                let flags = rec.flags().bits();
                let rname = rec.reference_sequence_id()
                    .and_then(|id| self.ref_names.get(id))
                    .cloned()
                    .unwrap_or_else(|| "*".to_string());
                let pos = rec.alignment_start().map(|p| usize::from(p) as i64).unwrap_or(0);
                let mapq = rec.mapping_quality().map(|q| u8::from(q)).unwrap_or(255);
                let cigar = {
                    let ops = rec.cigar();
                    if ops.is_empty() {
                        "*".to_string()
                    } else {
                        ops.iter()
                            .filter_map(|r| r.ok())
                            .map(|op: Op| {
                                let ch = match op.kind() {
                                    Kind::Match => 'M',
                                    Kind::Insertion => 'I',
                                    Kind::Deletion => 'D',
                                    Kind::Skip => 'N',
                                    Kind::SoftClip => 'S',
                                    Kind::HardClip => 'H',
                                    Kind::Pad => 'P',
                                    Kind::SequenceMatch => '=',
                                    Kind::SequenceMismatch => 'X',
                                };
                                format!("{}{ch}", op.len())
                            })
                            .collect::<String>()
                    }
                };
                let rnext = rec.mate_reference_sequence_id()
                    .and_then(|id| self.ref_names.get(id))
                    .cloned()
                    .unwrap_or_else(|| "*".to_string());
                let pnext = rec.mate_alignment_start().map(|p| usize::from(p) as i64).unwrap_or(0);
                let tlen = rec.template_length();
                let seq = {
                    let seq_buf = rec.sequence();
                    if seq_buf.is_empty() { "*".to_string() } else { seq_buf.iter().map(char::from).collect() }
                };
                let qual = {
                    let q = rec.quality_scores();
                    if q.is_empty() { "*".to_string() } else { q.iter().map(|s| (s + 33) as char).collect() }
                };
                Some(Value::AlignedRead(bio_core::AlignedRead {
                    qname,
                    flag: flags,
                    rname,
                    pos,
                    mapq,
                    cigar,
                    rnext,
                    pnext,
                    tlen: tlen as i64,
                    seq,
                    qual,
                }))
            }
        }
    }
}

// SAFETY: BamReadIter owns its reader/header exclusively and is only accessed
// through the Arc<Mutex<>> inside StreamValue.
unsafe impl Send for BamReadIter {}

/// Read BAM file as a Stream of AlignedRead values.
pub fn read_bam_records(path: &str) -> Result<Value> {
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("file not found: {path}"),
            None,
        ));
    }
    let file = open_file(path)?;
    let mut reader = noodles_bam::io::Reader::new(BufReader::new(file));
    let header = reader.read_header().map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("read_bam_records(): cannot read header: {e}"), None)
    })?;
    let ref_names: Vec<String> = header.reference_sequences().iter()
        .map(|(name, _)| name.to_string()).collect();
    let label = format!("bam_records:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(BamReadIter {
            reader,
            header,
            ref_names,
            record_buf: noodles_sam::alignment::RecordBuf::default(),
        }),
    )))
}

// ── MAF streaming ───────────────────────────────────────────────

struct MafIter<R: std::io::BufRead> {
    lines: std::io::Lines<R>,
    columns: Option<Vec<String>>,
}

impl<R: std::io::BufRead + Send> Iterator for MafIter<R> {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        loop {
            let line = self.lines.next()?.ok()?;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();

            // First non-comment line is the header
            if self.columns.is_none() {
                self.columns = Some(fields.iter().map(|s| s.to_string()).collect());
                continue;
            }

            let cols = self.columns.as_ref().unwrap();
            let mut map = HashMap::new();
            for (i, f) in fields.iter().enumerate() {
                let col_name = cols.get(i).map(|s| s.as_str()).unwrap_or("");
                let val = match col_name {
                    "Start_Position" | "End_Position" | "Entrez_Gene_Id" | "t_depth"
                    | "t_ref_count" | "t_alt_count" | "n_depth" | "n_ref_count"
                    | "n_alt_count" => parse_int_field(f),
                    _ => Value::Str(f.to_string()),
                };
                if !col_name.is_empty() {
                    map.insert(col_name.to_string(), val);
                }
            }
            return Some(Value::Record(map));
        }
    }
}

pub fn read_maf_stream(path: &str) -> Result<Value> {
    let path = &check_file_exists(path)?;
    check_format_mismatch(path, "maf")?;
    use std::io::BufRead;
    let reader = open_buf_reader(path)?;
    let label = format!("maf:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(MafIter { lines: reader.lines(), columns: None }),
    )))
}

// ── BedGraph streaming ──────────────────────────────────────────

struct BedGraphIter<R: std::io::BufRead> {
    lines: std::io::Lines<R>,
}

impl<R: std::io::BufRead + Send> Iterator for BedGraphIter<R> {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        loop {
            let line = self.lines.next()?.ok()?;
            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with("track")
                || line.starts_with("browser")
            {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let fields: Vec<&str> = if fields.len() < 4 {
                line.split_whitespace().collect()
            } else {
                fields
            };
            if fields.len() < 4 {
                continue;
            }
            let mut map = HashMap::new();
            map.insert("chrom".into(), Value::Str(fields[0].to_string()));
            map.insert("start".into(), parse_int_field(fields[1]));
            map.insert("end".into(), parse_int_field(fields[2]));
            map.insert("value".into(), parse_numeric_field(fields[3]));
            return Some(Value::Record(map));
        }
    }
}

pub fn read_bedgraph_stream(path: &str) -> Result<Value> {
    let path = &check_file_exists(path)?;
    check_format_mismatch(path, "bedgraph")?;
    use std::io::BufRead;
    let reader = open_buf_reader(path)?;
    let label = format!("bedgraph:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(BedGraphIter { lines: reader.lines() }),
    )))
}

fn check_file_exists(path: &str) -> Result<String> {
    let path = resolve_data_path(path);
    if !Path::new(&path).exists() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("file not found: {path}"),
            None,
        ));
    }
    Ok(path)
}

// ── Bio Format Writers ─────────────────────────────────────────

/// Write records to a FASTA file.
/// Takes a List of Records with `id` and `seq` fields, and a path.
pub fn write_fasta(records: &Value, path: &str) -> Result<Value> {
    use std::io::{BufWriter, Write};
    let path = resolve_write_path(path);
    let table_records;
    let items = match records {
        Value::List(l) => l,
        Value::Table(t) => {
            table_records = (0..t.num_rows())
                .map(|i| Value::Record(t.row_to_record(i)))
                .collect::<Vec<_>>();
            &table_records
        }
        _ => return Err(BioLangError::type_error("write_fasta() requires List or Table of records", None)),
    };
    let file = File::create(&path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot create '{path}': {e}"), None)
    })?;
    let mut writer = BufWriter::with_capacity(256 * 1024, file);
    let mut count = 0i64;
    for item in items {
        if let Value::Record(map) = item {
            let id = map.get("id").map(|v| format!("{v}")).unwrap_or_default();
            let desc = map.get("description").map(|v| format!("{v}")).unwrap_or_default();
            let seq_bytes: &[u8] = match map.get("seq") {
                Some(Value::DNA(s)) | Some(Value::RNA(s)) | Some(Value::Protein(s)) => s.data.as_bytes(),
                Some(Value::Str(s)) => s.as_bytes(),
                _ => b"",
            };
            if desc.is_empty() {
                writeln!(writer, ">{id}").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
            } else {
                writeln!(writer, ">{id} {desc}").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
            }
            // Write sequence in 80-char lines
            for chunk in seq_bytes.chunks(80) {
                writer.write_all(chunk).map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
                writer.write_all(b"\n").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
            }
            count += 1;
        }
    }
    writer.flush().map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
    Ok(Value::Int(count))
}

/// Write records to a FASTQ file.
/// Takes a List of Records with `id`, `seq`, `quality` fields, and a path.
pub fn write_fastq(records: &Value, path: &str) -> Result<Value> {
    use std::io::{BufWriter, Write};
    let path = resolve_write_path(path);
    let table_records;
    let items = match records {
        Value::List(l) => l,
        Value::Table(t) => {
            table_records = (0..t.num_rows())
                .map(|i| Value::Record(t.row_to_record(i)))
                .collect::<Vec<_>>();
            &table_records
        }
        _ => return Err(BioLangError::type_error("write_fastq() requires List or Table of records", None)),
    };
    let file = File::create(&path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot create '{path}': {e}"), None)
    })?;
    let mut writer = BufWriter::with_capacity(256 * 1024, file);
    let mut count = 0i64;
    for item in items {
        if let Value::Record(map) = item {
            let id = map.get("id").map(|v| format!("{v}")).unwrap_or_default();
            let seq = match map.get("seq") {
                Some(Value::DNA(s)) | Some(Value::RNA(s)) | Some(Value::Protein(s)) => &s.data,
                Some(Value::Str(s)) => s,
                _ => "",
            };
            let quality_owned;
            let quality = match map.get("quality") {
                Some(v) => { quality_owned = format!("{v}"); &quality_owned }
                None => { quality_owned = "I".repeat(seq.len()); &quality_owned }
            };
            writeln!(writer, "@{id}").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
            writeln!(writer, "{seq}").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
            writeln!(writer, "+").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
            writeln!(writer, "{quality}").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
            count += 1;
        }
    }
    writer.flush().map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
    Ok(Value::Int(count))
}

/// Write records to a BED file.
/// Takes a List of Records with `chrom`, `start`, `end` (and optional `name`, `score`, `strand`).
pub fn write_bed(records: &Value, path: &str) -> Result<Value> {
    use std::io::Write;
    let path = resolve_write_path(path);
    let items = match records {
        Value::List(l) => l,
        Value::Table(t) => {
            // Convert table rows to records
            let rows: Vec<Value> = (0..t.num_rows()).map(|i| Value::Record(t.row_to_record(i))).collect();
            return write_bed(&Value::List(rows), &path);
        }
        _ => return Err(BioLangError::type_error("write_bed() requires List or Table", None)),
    };
    let mut file = File::create(&path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot create '{path}': {e}"), None)
    })?;
    let mut count = 0i64;
    for item in items {
        if let Value::Record(map) = item {
            let chrom = map.get("chrom").map(|v| format!("{v}")).unwrap_or_default();
            let start = map.get("start").map(|v| format!("{v}")).unwrap_or("0".into());
            let end = map.get("end").map(|v| format!("{v}")).unwrap_or("0".into());
            let name = map.get("name").map(|v| format!("{v}"));
            let score = map.get("score").map(|v| format!("{v}"));
            let strand = map.get("strand").map(|v| format!("{v}"));
            if let (Some(name), Some(score), Some(strand)) = (&name, &score, &strand) {
                writeln!(file, "{chrom}\t{start}\t{end}\t{name}\t{score}\t{strand}")
            } else if let Some(name) = &name {
                writeln!(file, "{chrom}\t{start}\t{end}\t{name}")
            } else {
                writeln!(file, "{chrom}\t{start}\t{end}")
            }.map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
            count += 1;
        }
    }
    Ok(Value::Int(count))
}

/// Write records to a VCF file.
/// Takes a List of Records with standard VCF fields and a path.
pub fn write_vcf(records: &Value, path: &str) -> Result<Value> {
    use std::io::Write;
    let path = resolve_write_path(path);
    let table_records;
    let items = match records {
        Value::List(l) => l,
        Value::Table(t) => {
            table_records = (0..t.num_rows())
                .map(|i| Value::Record(t.row_to_record(i)))
                .collect::<Vec<_>>();
            &table_records
        }
        _ => return Err(BioLangError::type_error("write_vcf() requires List or Table of records", None)),
    };
    let mut file = File::create(&path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot create '{path}': {e}"), None)
    })?;
    // Write minimal VCF header
    writeln!(file, "##fileformat=VCFv4.2").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
    writeln!(file, "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
    let mut count = 0i64;
    for item in items {
        let (chrom, pos, id, ref_a, alt, qual, filt, info_str) = match item {
            Value::Variant { chrom, pos, id, ref_allele, alt_allele, quality, filter, info } => {
                let info_s = if info.is_empty() {
                    ".".to_string()
                } else {
                    info.iter()
                        .map(|(k, v)| match v {
                            Value::Bool(true) => k.clone(),
                            _ => format!("{k}={v}"),
                        })
                        .collect::<Vec<_>>()
                        .join(";")
                };
                (chrom.clone(), format!("{pos}"), id.clone(), ref_allele.clone(), alt_allele.clone(), format!("{quality}"), filter.clone(), info_s)
            }
            Value::Record(map) => {
                let chrom = map.get("chrom").map(|v| format!("{v}")).unwrap_or(".".into());
                let pos = map.get("pos").map(|v| format!("{v}")).unwrap_or("0".into());
                let id = map.get("id").map(|v| format!("{v}")).unwrap_or(".".into());
                let ref_a = map.get("ref").map(|v| format!("{v}")).unwrap_or(".".into());
                let alt = map.get("alt").map(|v| format!("{v}")).unwrap_or(".".into());
                let qual = map.get("qual").or_else(|| map.get("quality")).map(|v| format!("{v}")).unwrap_or(".".into());
                let filt = map.get("filter").map(|v| format!("{v}")).unwrap_or(".".into());
                let info = map.get("info").map(|v| format!("{v}")).unwrap_or(".".into());
                (chrom, pos, id, ref_a, alt, qual, filt, info)
            }
            _ => continue,
        };
        writeln!(file, "{chrom}\t{pos}\t{id}\t{ref_a}\t{alt}\t{qual}\t{filt}\t{info_str}")
            .map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
        count += 1;
    }
    Ok(Value::Int(count))
}

/// Write records to a GFF file.
/// Takes a List of Records and a path.
pub fn write_gff(records: &Value, path: &str) -> Result<Value> {
    use std::io::Write;
    let path = resolve_write_path(path);
    let table_records;
    let items = match records {
        Value::List(l) => l,
        Value::Table(t) => {
            table_records = (0..t.num_rows())
                .map(|i| Value::Record(t.row_to_record(i)))
                .collect::<Vec<_>>();
            &table_records
        }
        _ => return Err(BioLangError::type_error("write_gff() requires List or Table of records", None)),
    };
    let mut file = File::create(&path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot create '{path}': {e}"), None)
    })?;
    writeln!(file, "##gff-version 3").map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
    let mut count = 0i64;
    for item in items {
        if let Value::Record(map) = item {
            let seqid = map.get("seqid").or_else(|| map.get("chrom")).map(|v| format!("{v}")).unwrap_or(".".into());
            let source = map.get("source").map(|v| format!("{v}")).unwrap_or(".".into());
            let feature = map.get("type").or_else(|| map.get("feature")).map(|v| format!("{v}")).unwrap_or(".".into());
            let start = map.get("start").map(|v| format!("{v}")).unwrap_or("0".into());
            let end = map.get("end").map(|v| format!("{v}")).unwrap_or("0".into());
            let score = map.get("score").map(|v| format!("{v}")).unwrap_or(".".into());
            let strand = map.get("strand").map(|v| format!("{v}")).unwrap_or(".".into());
            let phase = map.get("phase").map(|v| format!("{v}")).unwrap_or(".".into());
            let attrs = map.get("attributes").map(|v| format!("{v}")).unwrap_or(".".into());
            writeln!(file, "{seqid}\t{source}\t{feature}\t{start}\t{end}\t{score}\t{strand}\t{phase}\t{attrs}")
                .map_err(|e| BioLangError::runtime(ErrorKind::IOError, e.to_string(), None))?;
            count += 1;
        }
    }
    Ok(Value::Int(count))
}
