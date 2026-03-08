use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, BioSequence, GenomicInterval, Strand, Value};
use std::collections::HashMap;

/// Register all bio-related builtin functions.
pub fn register_bio_builtins(env: &mut bl_core::value::Value) {
    // This is called differently — we register through the runtime's env
    // See register_bio_functions() below
    let _ = env;
}

/// Returns a list of (name, arity) for all bio builtins to register.
pub fn bio_builtin_list() -> Vec<(&'static str, Arity)> {
    let mut builtins = vec![
        ("dna", Arity::Exact(1)),
        ("rna", Arity::Exact(1)),
        ("protein", Arity::Exact(1)),
        ("transcribe", Arity::Exact(1)),
        ("translate", Arity::Exact(1)),
        ("reverse_complement", Arity::Exact(1)),
        ("complement", Arity::Exact(1)),
        ("gc_content", Arity::Exact(1)),
        ("subseq", Arity::Range(2, 3)),
        ("find_motif", Arity::Exact(2)),
        ("kmers", Arity::Exact(2)),
        ("find_orfs", Arity::Range(1, 2)),
        ("seq_len", Arity::Exact(1)),
        ("fasta", Arity::Range(1, 2)),
        ("fastq", Arity::Range(1, 2)),
        ("read_fasta", Arity::Exact(1)),
        ("read_fastq", Arity::Exact(1)),
        ("bed", Arity::Range(1, 2)),
        ("gff", Arity::Range(1, 2)),
        ("vcf", Arity::Range(1, 2)),
        ("read_vcf", Arity::Range(1, 2)),
        ("read_bed", Arity::Range(1, 2)),
        ("read_gff", Arity::Range(1, 2)),
        ("read_sam", Arity::Range(1, 2)),
        ("read_bam", Arity::Range(1, 2)),
        ("validate", Arity::Exact(1)),
        ("vcf_filter", Arity::Exact(2)),
        // New format readers
        ("sam", Arity::Range(1, 2)),
        ("bam", Arity::Range(1, 2)),
        ("sam_header", Arity::Exact(1)),
        ("maf", Arity::Range(1, 2)),
        ("bedgraph", Arity::Range(1, 2)),
        // Bio accessor builtins
        ("to_interval", Arity::Exact(1)),
        ("parse_info", Arity::Range(1, 2)),
        ("parse_attr", Arity::Range(1, 2)),
        ("parse_qual", Arity::Exact(1)),
        ("base_counts", Arity::Exact(1)),
        // Bio format writers
        ("write_fasta", Arity::Exact(2)),
        ("write_fastq", Arity::Exact(2)),
        ("write_bed", Arity::Exact(2)),
        ("write_vcf", Arity::Exact(2)),
        ("write_gff", Arity::Exact(2)),
        // FASTQ read trimming builtins
        ("trim_reads", Arity::Range(2, 3)),
        ("trim_adapters", Arity::Exact(3)),
        ("detect_adapters", Arity::Exact(1)),
        ("trim_quality", Arity::Exact(3)),
        ("filter_reads", Arity::Exact(3)),
        ("read_stats", Arity::Exact(1)),
        // Variant analysis builtins
        ("normalize_variant", Arity::Exact(1)),
        ("tstv_ratio", Arity::Exact(1)),
        ("het_hom_ratio", Arity::Exact(1)),
        ("variant_stats", Arity::Exact(1)),
        ("decompose_mnp", Arity::Exact(1)),
        ("filter_pass", Arity::Exact(1)),
        // RNA-seq normalization
        ("tpm", Arity::Exact(2)),
        ("fpkm", Arity::Exact(3)),
        ("cpm", Arity::Exact(1)),
        // Sequence analysis
        ("codon_usage", Arity::Exact(1)),
        ("tm", Arity::Exact(1)),
        ("restriction_sites", Arity::Exact(2)),
        // BAM analysis
        ("depth", Arity::Range(1, 2)),
        ("insert_size", Arity::Exact(1)),
        ("mapping_rate", Arity::Exact(1)),
        ("coverage_hist", Arity::Range(1, 2)),
        ("gc_bias", Arity::Exact(1)),
        ("on_target", Arity::Exact(2)),
    ];
    builtins.extend(crate::intervals::interval_builtin_list());
    builtins.extend(crate::alignment::alignment_builtin_list());
    builtins
}

/// Execute a bio builtin function by name.
pub fn call_bio_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "dna" => {
            let s = require_str(&args[0], "dna")?;
            validate_dna(&s)?;
            Ok(Value::DNA(BioSequence {
                data: s.to_uppercase(),
            }))
        }
        "rna" => {
            let s = require_str(&args[0], "rna")?;
            validate_rna(&s)?;
            Ok(Value::RNA(BioSequence {
                data: s.to_uppercase(),
            }))
        }
        "protein" => {
            let s = require_str(&args[0], "protein")?;
            Ok(Value::Protein(BioSequence {
                data: s.to_uppercase(),
            }))
        }
        "transcribe" => {
            let seq = require_dna(&args[0], "transcribe")?;
            Ok(Value::RNA(BioSequence {
                data: bio_core::seq_ops::transcribe(&seq.data),
            }))
        }
        "translate" => {
            require_rna_or_dna(&args[0], "translate")?;
            let seq_data = match &args[0] {
                Value::DNA(s) => &s.data,
                Value::RNA(s) => &s.data,
                _ => unreachable!(),
            };
            Ok(Value::Protein(BioSequence {
                data: bio_core::seq_ops::translate_to_stop(seq_data),
            }))
        }
        "reverse_complement" => match &args[0] {
            Value::DNA(seq) => Ok(Value::DNA(BioSequence {
                data: bio_core::seq_ops::reverse_complement_dna(&seq.data),
            })),
            Value::RNA(seq) => Ok(Value::RNA(BioSequence {
                data: bio_core::seq_ops::reverse_complement_rna(&seq.data),
            })),
            other => Err(BioLangError::type_error(
                format!("reverse_complement() requires DNA or RNA, got {}", other.type_of()),
                None,
            )),
        },
        "complement" => match &args[0] {
            Value::DNA(seq) => Ok(Value::DNA(BioSequence {
                data: bio_core::seq_ops::complement_dna(&seq.data),
            })),
            Value::RNA(seq) => Ok(Value::RNA(BioSequence {
                data: bio_core::seq_ops::complement_rna(&seq.data),
            })),
            other => Err(BioLangError::type_error(
                format!("complement() requires DNA or RNA, got {}", other.type_of()),
                None,
            )),
        },
        "gc_content" => {
            let seq = require_nucleic(&args[0], "gc_content")?;
            Ok(Value::Float(bio_core::seq_ops::gc_content(&seq.data)))
        }
        "subseq" => {
            let seq_val = &args[0];
            let start = require_int(&args[1], "subseq")? as usize;
            let data = get_seq_data(seq_val, "subseq")?;

            let end = if args.len() > 2 {
                require_int(&args[2], "subseq")? as usize
            } else {
                data.len()
            };

            if start > data.len() || end > data.len() || start > end {
                return Err(BioLangError::runtime(
                    ErrorKind::IndexOutOfBounds,
                    format!("subseq({start}, {end}) out of bounds for sequence of length {}", data.len()),
                    None,
                ));
            }

            let sub = &data[start..end];
            match seq_val {
                Value::DNA(_) => Ok(Value::DNA(BioSequence { data: sub.to_string() })),
                Value::RNA(_) => Ok(Value::RNA(BioSequence { data: sub.to_string() })),
                Value::Protein(_) => Ok(Value::Protein(BioSequence { data: sub.to_string() })),
                _ => unreachable!(),
            }
        }
        "find_motif" => {
            let seq = get_seq_data(&args[0], "find_motif")?;
            let motif = require_str(&args[1], "find_motif")?;
            let positions = bio_core::seq_ops::find_motif(&seq, &motif);
            Ok(Value::List(positions.into_iter().map(|p| Value::Int(p as i64)).collect()))
        }
        "kmers" => {
            let seq = get_seq_data(&args[0], "kmers")?;
            let k = require_int(&args[1], "kmers")? as usize;
            let result = bio_core::seq_ops::kmers(&seq, k);
            Ok(Value::List(result.into_iter().map(|s| Value::Str(s.to_string())).collect()))
        }
        "find_orfs" => {
            let seq = get_seq_data(&args[0], "find_orfs")?;
            let min_length = if args.len() > 1 {
                require_int(&args[1], "find_orfs")? as usize
            } else {
                100 // default minimum ORF length in bases
            };

            let orfs = bio_core::seq_ops::find_orfs(&seq, min_length);
            let values: Vec<Value> = orfs
                .into_iter()
                .map(|orf| {
                    let mut fields = std::collections::HashMap::new();
                    fields.insert("start".to_string(), Value::Int(orf.start as i64));
                    fields.insert("end".to_string(), Value::Int(orf.end as i64));
                    fields.insert("length".to_string(), Value::Int((orf.end - orf.start) as i64));
                    fields.insert("frame".to_string(), Value::Int(orf.frame as i64));
                    fields.insert(
                        "protein".to_string(),
                        Value::Protein(BioSequence {
                            data: orf.protein,
                        }),
                    );
                    Value::Record(fields)
                })
                .collect();
            Ok(Value::List(values))
        }
        "seq_len" => {
            let seq = get_seq_data(&args[0], "seq_len")?;
            Ok(Value::Int(seq.len() as i64))
        }
        "fasta" => {
            let path = require_str(&args[0], "fasta")?;
            if wants_no_stream(&args) {
                crate::io::read_fasta_table(&path)
            } else {
                crate::io::read_fasta(&path)
            }
        }
        "read_fasta" => {
            let path = require_str(&args[0], "read_fasta")?;
            crate::io::read_fasta_table(&path)
        }
        "fastq" => {
            let path = require_str(&args[0], "fastq")?;
            if wants_no_stream(&args) {
                crate::io::read_fastq_table(&path)
            } else {
                crate::io::read_fastq(&path)
            }
        }
        "read_fastq" => {
            let path = require_str(&args[0], "read_fastq")?;
            crate::io::read_fastq_table(&path)
        }
        "bed" | "read_bed" => {
            let path = require_str(&args[0], "bed")?;
            if wants_stream(&args) {
                crate::io::read_bed_stream(&path)
            } else {
                crate::io::read_bed(&path)
            }
        }
        "gff" | "read_gff" => {
            let path = require_str(&args[0], "gff")?;
            if wants_stream(&args) {
                crate::io::read_gff_stream(&path)
            } else {
                crate::io::read_gff(&path)
            }
        }
        "vcf" | "read_vcf" => {
            let path = require_str(&args[0], "vcf")?;
            if wants_stream(&args) {
                crate::io::read_vcf_stream(&path)
            } else {
                crate::io::read_vcf(&path)
            }
        }
        "validate" => {
            let path = require_str(&args[0], "validate")?;
            crate::io::validate_file(&path)
        }
        "vcf_filter" => {
            let path = require_str(&args[0], "vcf_filter")?;
            let expr = require_str(&args[1], "vcf_filter")?;
            crate::io::vcf_filter(&path, &expr)
        }
        "sam" | "read_sam" => {
            let path = require_str(&args[0], "sam")?;
            if wants_stream(&args) {
                crate::io::read_sam_stream(&path)
            } else {
                crate::io::read_sam(&path)
            }
        }
        "bam" | "read_bam" => {
            let path = require_str(&args[0], "bam")?;
            if wants_stream(&args) {
                crate::io::read_bam_stream(&path)
            } else {
                crate::io::read_bam(&path)
            }
        }
        "sam_header" => {
            let path = require_str(&args[0], "sam_header")?;
            crate::io::read_sam_header(&path)
        }
        "maf" => {
            let path = require_str(&args[0], "maf")?;
            if wants_stream(&args) {
                crate::io::read_maf_stream(&path)
            } else {
                crate::io::read_maf(&path)
            }
        }
        "bedgraph" => {
            let path = require_str(&args[0], "bedgraph")?;
            if wants_stream(&args) {
                crate::io::read_bedgraph_stream(&path)
            } else {
                crate::io::read_bedgraph(&path)
            }
        }
        // ── Bio accessor builtins ─────────────────────────────────
        "to_interval" => builtin_to_interval(&args[0]),
        "parse_info" => {
            let key = if args.len() > 1 {
                Some(require_str(&args[1], "parse_info")?)
            } else {
                None
            };
            builtin_parse_info(&args[0], key.as_deref())
        }
        "parse_attr" => {
            let key = if args.len() > 1 {
                Some(require_str(&args[1], "parse_attr")?)
            } else {
                None
            };
            builtin_parse_attr(&args[0], key.as_deref())
        }
        "parse_qual" => builtin_parse_qual(&args[0]),
        "base_counts" => builtin_base_counts(&args[0]),
        // ── Bio format writers ──────────────────────────────────────
        "write_fasta" => {
            let path = require_str(&args[1], "write_fasta")?;
            crate::io::write_fasta(&args[0], &path)
        }
        "write_fastq" => {
            let path = require_str(&args[1], "write_fastq")?;
            crate::io::write_fastq(&args[0], &path)
        }
        "write_bed" => {
            let path = require_str(&args[1], "write_bed")?;
            crate::io::write_bed(&args[0], &path)
        }
        "write_vcf" => {
            let path = require_str(&args[1], "write_vcf")?;
            crate::io::write_vcf(&args[0], &path)
        }
        "write_gff" => {
            let path = require_str(&args[1], "write_gff")?;
            crate::io::write_gff(&args[0], &path)
        }
        // ── FASTQ read trimming builtins ────────────────────────────
        "trim_reads" => {
            let path = require_str(&args[0], "trim_reads")?;
            let output = require_str(&args[1], "trim_reads")?;
            let opts = if args.len() > 2 {
                require_record(&args[2], "trim_reads")?
            } else {
                HashMap::new()
            };
            builtin_trim_reads(&path, &output, &opts)
        }
        "trim_adapters" => {
            let path = require_str(&args[0], "trim_adapters")?;
            let output = require_str(&args[1], "trim_adapters")?;
            let adapters = require_list(&args[2], "trim_adapters")?;
            let adapter_strs: Vec<String> = adapters
                .iter()
                .map(|v| require_str(v, "trim_adapters"))
                .collect::<Result<Vec<_>>>()?;
            builtin_trim_adapters(&path, &output, &adapter_strs)
        }
        "detect_adapters" => {
            let path = require_str(&args[0], "detect_adapters")?;
            builtin_detect_adapters(&path)
        }
        "trim_quality" => {
            let path = require_str(&args[0], "trim_quality")?;
            let output = require_str(&args[1], "trim_quality")?;
            let threshold = require_float_or_int(&args[2], "trim_quality")?;
            builtin_trim_quality(&path, &output, threshold)
        }
        "filter_reads" => {
            let path = require_str(&args[0], "filter_reads")?;
            let output = require_str(&args[1], "filter_reads")?;
            let opts = require_record(&args[2], "filter_reads")?;
            builtin_filter_reads(&path, &output, &opts)
        }
        "read_stats" => {
            let path = require_str(&args[0], "read_stats")?;
            builtin_read_stats(&path)
        }
        // Variant analysis
        "normalize_variant" => builtin_normalize_variant(args),
        "tstv_ratio" => builtin_tstv_ratio(args),
        "het_hom_ratio" => builtin_het_hom_ratio(args),
        "variant_stats" => builtin_variant_stats(args),
        "decompose_mnp" => builtin_decompose_mnp(args),
        "filter_pass" => builtin_filter_pass(args),
        // RNA-seq normalization
        "tpm" => builtin_tpm(args),
        "fpkm" => builtin_fpkm(args),
        "cpm" => builtin_cpm(args),
        // Sequence analysis
        "codon_usage" => builtin_codon_usage(args),
        "tm" => builtin_tm(args),
        "restriction_sites" => builtin_restriction_sites(args),
        // BAM analysis
        "depth" => builtin_depth(args),
        "insert_size" => builtin_insert_size(args),
        "mapping_rate" => builtin_mapping_rate(args),
        "coverage_hist" => builtin_coverage_hist(args),
        "gc_bias" => builtin_gc_bias(args),
        "on_target" => builtin_on_target(args),
        _ if crate::intervals::is_interval_builtin(name) => {
            crate::intervals::call_interval_builtin(name, args)
        }
        _ if crate::alignment::is_alignment_builtin(name) => {
            crate::alignment::call_alignment_builtin(name, args)
        }
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown bio builtin '{name}'"),
            None,
        )),
    }
}

// ══════════════════════════════════════════════════════════════════════
// FASTQ Read Trimming Implementations
// ══════════════════════════════════════════════════════════════════════

/// Open a FASTQ file (plain or gzipped) and return a boxed BufRead.
fn open_fastq_reader(path: &str) -> Result<Box<dyn std::io::BufRead>> {
    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot open '{path}': {e}"), None)
    })?;
    if path.ends_with(".gz") {
        let decoder = flate2::read::GzDecoder::new(file);
        Ok(Box::new(std::io::BufReader::new(decoder)))
    } else {
        Ok(Box::new(std::io::BufReader::new(file)))
    }
}

/// Create a FASTQ writer to a file.
fn create_fastq_writer(path: &str) -> Result<noodles_fastq::io::Writer<std::io::BufWriter<std::fs::File>>> {
    let file = std::fs::File::create(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot create '{path}': {e}"), None)
    })?;
    Ok(noodles_fastq::io::Writer::new(std::io::BufWriter::new(file)))
}

/// Write a trimmed FASTQ record given name, sequence slice, and quality slice.
fn write_trimmed_record(
    writer: &mut noodles_fastq::io::Writer<std::io::BufWriter<std::fs::File>>,
    name: &[u8],
    seq: &[u8],
    qual: &[u8],
) -> Result<()> {
    use noodles_fastq as fq;
    let definition = fq::record::Definition::new(name, b"");
    let record = fq::Record::new(definition, seq, qual);
    writer.write_record(&record).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("write error: {e}"), None)
    })
}

// ── Sliding window quality trimmer ──────────────────────────────────

/// Trim using a sliding window approach. Returns the (start, end) positions to keep.
/// Quality scores are Phred+33 encoded.
fn trim_sliding_window(
    qual: &[u8],
    window_size: usize,
    threshold: f64,
    cut_front: bool,
    cut_tail: bool,
) -> (usize, usize) {
    let len = qual.len();
    if len == 0 {
        return (0, 0);
    }

    let mut start = 0usize;
    let mut end = len;

    // Trim from 3' end (cut_tail)
    if cut_tail {
        let mut cut_pos = len;
        if len >= window_size {
            for i in (0..=(len - window_size)).rev() {
                let window_end = i + window_size;
                let avg: f64 = qual[i..window_end]
                    .iter()
                    .map(|&q| (q as f64) - 33.0)
                    .sum::<f64>()
                    / window_size as f64;
                if avg >= threshold {
                    cut_pos = window_end;
                    break;
                }
                cut_pos = i;
            }
        } else {
            // Window larger than read — check entire read
            let avg: f64 = qual.iter().map(|&q| (q as f64) - 33.0).sum::<f64>() / len as f64;
            if avg < threshold {
                cut_pos = 0;
            }
        }
        end = cut_pos;
    }

    // Trim from 5' end (cut_front)
    if cut_front && end > 0 {
        let search_end = end;
        let mut found = false;
        if search_end >= window_size {
            for i in 0..=(search_end - window_size) {
                let window_end = i + window_size;
                let avg: f64 = qual[i..window_end]
                    .iter()
                    .map(|&q| (q as f64) - 33.0)
                    .sum::<f64>()
                    / window_size as f64;
                if avg >= threshold {
                    start = i;
                    found = true;
                    break;
                }
            }
            if !found {
                start = end; // discard entire read
            }
        } else if search_end > 0 {
            let avg: f64 = qual[..search_end]
                .iter()
                .map(|&q| (q as f64) - 33.0)
                .sum::<f64>()
                / search_end as f64;
            if avg < threshold {
                start = end;
            }
        }
    }

    (start, end)
}

/// Count N bases in a sequence slice.
fn count_n_bases(seq: &[u8]) -> usize {
    seq.iter().filter(|&&b| b == b'N' || b == b'n').count()
}

/// trim_reads(fastq_path, output_path) or trim_reads(fastq_path, output_path, options)
/// Quality-based read trimming with sliding window (fastp-like).
fn builtin_trim_reads(path: &str, output: &str, opts: &HashMap<String, Value>) -> Result<Value> {
    let quality_threshold = opt_float(opts, "quality", 20.0);
    let min_len = opt_int(opts, "min_len", 36) as usize;
    let window_size = opt_int(opts, "window", 4) as usize;
    let cut_front = opt_bool(opts, "cut_front", false);
    let cut_tail = opt_bool(opts, "cut_tail", true);
    let n_base_limit = opt_int(opts, "n_base_limit", 5) as usize;

    let reader_buf = open_fastq_reader(path)?;
    let mut reader = noodles_fastq::io::Reader::new(reader_buf);
    let mut writer = create_fastq_writer(output)?;

    let mut total_reads: i64 = 0;
    let mut passed_reads: i64 = 0;
    let mut failed_reads: i64 = 0;
    let mut total_bases_in: i64 = 0;
    let mut total_bases_out: i64 = 0;
    let mut total_len_before: i64 = 0;
    let mut total_len_after: i64 = 0;

    let mut record = noodles_fastq::Record::default();
    loop {
        match reader.read_record(&mut record) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("error reading FASTQ: {e}"),
                    None,
                ));
            }
        }

        let seq = record.sequence();
        let qual = record.quality_scores();
        let len_before = seq.len();
        total_reads += 1;
        total_bases_in += len_before as i64;
        total_len_before += len_before as i64;

        // Sliding window trim
        let (trim_start, trim_end) = trim_sliding_window(
            qual,
            window_size,
            quality_threshold,
            cut_front,
            cut_tail,
        );

        let trimmed_seq = &seq[trim_start..trim_end];
        let trimmed_qual = &qual[trim_start..trim_end];
        let len_after = trimmed_seq.len();

        // Check min length
        if len_after < min_len {
            failed_reads += 1;
            continue;
        }

        // Check N base limit
        if count_n_bases(trimmed_seq) > n_base_limit {
            failed_reads += 1;
            continue;
        }

        passed_reads += 1;
        total_bases_out += len_after as i64;
        total_len_after += len_after as i64;

        write_trimmed_record(&mut writer, record.name(), trimmed_seq, trimmed_qual)?;
    }

    let mean_before = if total_reads > 0 {
        total_len_before as f64 / total_reads as f64
    } else {
        0.0
    };
    let mean_after = if passed_reads > 0 {
        total_len_after as f64 / passed_reads as f64
    } else {
        0.0
    };

    let mut result = HashMap::new();
    result.insert("total_reads".to_string(), Value::Int(total_reads));
    result.insert("passed_reads".to_string(), Value::Int(passed_reads));
    result.insert("failed_reads".to_string(), Value::Int(failed_reads));
    result.insert("total_bases_in".to_string(), Value::Int(total_bases_in));
    result.insert("total_bases_out".to_string(), Value::Int(total_bases_out));
    result.insert("mean_length_before".to_string(), Value::Float(mean_before));
    result.insert("mean_length_after".to_string(), Value::Float(mean_after));
    result.insert("output".to_string(), Value::Str(output.to_string()));
    Ok(Value::Record(result))
}

// ── Adapter trimming ────────────────────────────────────────────────

/// Find the longest overlap between an adapter prefix and a read suffix.
/// Returns the position in the read where the adapter match starts, or None.
fn find_adapter_overlap(read: &[u8], adapter: &[u8], min_overlap: usize) -> Option<usize> {
    let max_overlap = read.len().min(adapter.len());
    if max_overlap < min_overlap {
        return None;
    }
    for overlap_len in (min_overlap..=max_overlap).rev() {
        let read_start = read.len() - overlap_len;
        let mismatches: usize = read[read_start..]
            .iter()
            .zip(&adapter[..overlap_len])
            .filter(|(a, b)| !a.eq_ignore_ascii_case(b))
            .count();
        if mismatches <= 1 {
            return Some(read_start);
        }
    }
    None
}

/// trim_adapters(fastq_path, output_path, adapters)
/// Adapter removal from reads.
fn builtin_trim_adapters(
    path: &str,
    output: &str,
    adapters: &[String],
) -> Result<Value> {
    let min_overlap: usize = 8;
    let min_len: usize = 36;

    let adapter_bytes: Vec<Vec<u8>> = adapters.iter().map(|a| a.as_bytes().to_vec()).collect();

    let reader_buf = open_fastq_reader(path)?;
    let mut reader = noodles_fastq::io::Reader::new(reader_buf);
    let mut writer = create_fastq_writer(output)?;

    let mut total_reads: i64 = 0;
    let mut trimmed_reads: i64 = 0;
    let mut untrimmed_reads: i64 = 0;
    let mut written_reads: i64 = 0;

    let mut record = noodles_fastq::Record::default();
    loop {
        match reader.read_record(&mut record) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("error reading FASTQ: {e}"),
                    None,
                ));
            }
        }

        total_reads += 1;
        let seq = record.sequence();
        let qual = record.quality_scores();

        // Try each adapter, keep the best (earliest) match
        let mut best_pos: Option<usize> = None;
        for adapter in &adapter_bytes {
            if let Some(pos) = find_adapter_overlap(seq, adapter, min_overlap) {
                best_pos = Some(match best_pos {
                    Some(prev) => prev.min(pos),
                    None => pos,
                });
            }
        }

        let (out_seq, out_qual, was_trimmed) = match best_pos {
            Some(pos) => (&seq[..pos], &qual[..pos], true),
            None => (seq, qual, false),
        };

        if out_seq.len() < min_len {
            // Discarded (too short after trim)
            if was_trimmed {
                trimmed_reads += 1;
            } else {
                untrimmed_reads += 1;
            }
            continue;
        }

        if was_trimmed {
            trimmed_reads += 1;
        } else {
            untrimmed_reads += 1;
        }
        written_reads += 1;

        write_trimmed_record(&mut writer, record.name(), out_seq, out_qual)?;
    }

    let adapter_rate = if total_reads > 0 {
        trimmed_reads as f64 / total_reads as f64
    } else {
        0.0
    };

    let mut result = HashMap::new();
    result.insert("total_reads".to_string(), Value::Int(total_reads));
    result.insert("trimmed_reads".to_string(), Value::Int(trimmed_reads));
    result.insert("untrimmed_reads".to_string(), Value::Int(untrimmed_reads));
    result.insert("written_reads".to_string(), Value::Int(written_reads));
    result.insert("adapter_rate".to_string(), Value::Float(adapter_rate));
    result.insert("output".to_string(), Value::Str(output.to_string()));
    Ok(Value::Record(result))
}

// ── Adapter detection ───────────────────────────────────────────────

/// Known adapter sequences for detection.
const KNOWN_ADAPTERS: &[(&str, &str)] = &[
    ("AGATCGGAAGAG", "Illumina Universal"),
    ("CTGTCTCTTATA", "Nextera"),
    ("TGGAATTCTCGG", "Small RNA"),
];

/// detect_adapters(fastq_path)
/// Auto-detect adapters from overrepresented suffixes.
fn builtin_detect_adapters(path: &str) -> Result<Value> {
    let max_reads: usize = 100_000;
    let suffix_len: usize = 12;
    let check_tail: usize = 20;

    let reader_buf = open_fastq_reader(path)?;
    let mut reader = noodles_fastq::io::Reader::new(reader_buf);

    let mut total_reads: usize = 0;
    // Count how many reads contain each known adapter in their last `check_tail` bases
    let mut adapter_counts: Vec<usize> = vec![0; KNOWN_ADAPTERS.len()];
    // Track 12-mer suffix frequencies
    let mut suffix_counts: HashMap<Vec<u8>, usize> = HashMap::new();

    let mut record = noodles_fastq::Record::default();
    loop {
        if total_reads >= max_reads {
            break;
        }
        match reader.read_record(&mut record) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let seq = record.sequence();
        total_reads += 1;

        // Extract last `suffix_len` bases
        if seq.len() >= suffix_len {
            let suffix = &seq[seq.len() - suffix_len..];
            *suffix_counts.entry(suffix.to_vec()).or_insert(0) += 1;
        }

        // Check known adapters in the tail of the read
        if seq.len() >= check_tail {
            let tail = &seq[seq.len() - check_tail..];
            for (i, (adapter_seq, _)) in KNOWN_ADAPTERS.iter().enumerate() {
                let adapter_bytes = adapter_seq.as_bytes();
                // Check if adapter prefix appears in tail
                let min_check = adapter_bytes.len().min(tail.len());
                for start in 0..tail.len() {
                    let remaining = tail.len() - start;
                    let check_len = min_check.min(remaining);
                    if check_len >= 8 {
                        let mismatches: usize = tail[start..start + check_len]
                            .iter()
                            .zip(&adapter_bytes[..check_len])
                            .filter(|(a, b)| !a.eq_ignore_ascii_case(b))
                            .count();
                        if mismatches <= 1 {
                            adapter_counts[i] += 1;
                            break;
                        }
                    }
                }
            }
        }
    }

    let mut results = Vec::new();
    for (i, (adapter_seq, adapter_name)) in KNOWN_ADAPTERS.iter().enumerate() {
        let count = adapter_counts[i];
        if count > 0 {
            let rate = if total_reads > 0 {
                count as f64 / total_reads as f64
            } else {
                0.0
            };
            let mut rec = HashMap::new();
            rec.insert("adapter".to_string(), Value::Str(adapter_seq.to_string()));
            rec.insert("name".to_string(), Value::Str(adapter_name.to_string()));
            rec.insert("count".to_string(), Value::Int(count as i64));
            rec.insert("rate".to_string(), Value::Float(rate));
            results.push(Value::Record(rec));
        }
    }

    // Sort by count descending
    results.sort_by(|a, b| {
        let ca = match a {
            Value::Record(r) => match r.get("count") {
                Some(Value::Int(n)) => *n,
                _ => 0,
            },
            _ => 0,
        };
        let cb = match b {
            Value::Record(r) => match r.get("count") {
                Some(Value::Int(n)) => *n,
                _ => 0,
            },
            _ => 0,
        };
        cb.cmp(&ca)
    });

    Ok(Value::List(results))
}

// ── Simple quality trimming ─────────────────────────────────────────

/// trim_quality(fastq_path, output_path, threshold)
/// Simple 3' quality trimming: trim from the right until quality >= threshold.
fn builtin_trim_quality(path: &str, output: &str, threshold: f64) -> Result<Value> {
    let reader_buf = open_fastq_reader(path)?;
    let mut reader = noodles_fastq::io::Reader::new(reader_buf);
    let mut writer = create_fastq_writer(output)?;

    let mut total_reads: i64 = 0;
    let mut passed_reads: i64 = 0;
    let mut bases_trimmed: i64 = 0;

    let mut record = noodles_fastq::Record::default();
    loop {
        match reader.read_record(&mut record) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("error reading FASTQ: {e}"),
                    None,
                ));
            }
        }

        let seq = record.sequence();
        let qual = record.quality_scores();
        let original_len = seq.len();
        total_reads += 1;

        // Find rightmost position where quality >= threshold
        let mut end = original_len;
        while end > 0 {
            let q = (qual[end - 1] as f64) - 33.0;
            if q >= threshold {
                break;
            }
            end -= 1;
        }

        if end == 0 {
            // Entire read trimmed away
            bases_trimmed += original_len as i64;
            continue;
        }

        bases_trimmed += (original_len - end) as i64;
        passed_reads += 1;
        write_trimmed_record(&mut writer, record.name(), &seq[..end], &qual[..end])?;
    }

    let mut result = HashMap::new();
    result.insert("total_reads".to_string(), Value::Int(total_reads));
    result.insert("passed_reads".to_string(), Value::Int(passed_reads));
    result.insert("bases_trimmed".to_string(), Value::Int(bases_trimmed));
    result.insert("output".to_string(), Value::Str(output.to_string()));
    Ok(Value::Record(result))
}

// ── Read filtering ──────────────────────────────────────────────────

/// Compute Shannon entropy of dinucleotide frequencies (0-1 scale).
fn dinucleotide_entropy(seq: &[u8]) -> f64 {
    if seq.len() < 2 {
        return 0.0;
    }
    let mut counts: HashMap<(u8, u8), usize> = HashMap::new();
    let total = seq.len() - 1;
    for i in 0..total {
        *counts.entry((seq[i], seq[i + 1])).or_insert(0) += 1;
    }
    let total_f = total as f64;
    let mut entropy = 0.0f64;
    for &count in counts.values() {
        if count > 0 {
            let p = count as f64 / total_f;
            entropy -= p * p.log2();
        }
    }
    // Normalize to 0-1: max entropy for 16 dinucleotides = log2(16) = 4
    let max_entropy = 4.0f64;
    (entropy / max_entropy).min(1.0)
}

/// filter_reads(fastq_path, output_path, options)
fn builtin_filter_reads(path: &str, output: &str, opts: &HashMap<String, Value>) -> Result<Value> {
    let min_len = opt_int(opts, "min_len", 36) as usize;
    let max_len = opt_int(opts, "max_len", 500) as usize;
    let min_quality = opt_float(opts, "min_quality", 20.0);
    let max_n = opt_int(opts, "max_n", 5) as usize;
    let complexity = opt_float(opts, "complexity", 0.3);

    let reader_buf = open_fastq_reader(path)?;
    let mut reader = noodles_fastq::io::Reader::new(reader_buf);
    let mut writer = create_fastq_writer(output)?;

    let mut total: i64 = 0;
    let mut passed: i64 = 0;
    let mut too_short: i64 = 0;
    let mut too_long: i64 = 0;
    let mut low_quality: i64 = 0;
    let mut low_complexity: i64 = 0;
    let mut too_many_n: i64 = 0;

    let mut record = noodles_fastq::Record::default();
    loop {
        match reader.read_record(&mut record) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("error reading FASTQ: {e}"),
                    None,
                ));
            }
        }

        let seq = record.sequence();
        let qual = record.quality_scores();
        total += 1;

        // Length filters
        if seq.len() < min_len {
            too_short += 1;
            continue;
        }
        if seq.len() > max_len {
            too_long += 1;
            continue;
        }

        // N count filter
        if count_n_bases(seq) > max_n {
            too_many_n += 1;
            continue;
        }

        // Mean quality filter
        let mean_q: f64 = if !qual.is_empty() {
            qual.iter().map(|&q| (q as f64) - 33.0).sum::<f64>() / qual.len() as f64
        } else {
            0.0
        };
        if mean_q < min_quality {
            low_quality += 1;
            continue;
        }

        // Complexity filter (Shannon entropy of dinucleotides)
        let ent = dinucleotide_entropy(seq);
        if ent < complexity {
            low_complexity += 1;
            continue;
        }

        passed += 1;
        write_trimmed_record(&mut writer, record.name(), seq, qual)?;
    }

    let mut result = HashMap::new();
    result.insert("total".to_string(), Value::Int(total));
    result.insert("passed".to_string(), Value::Int(passed));
    result.insert("too_short".to_string(), Value::Int(too_short));
    result.insert("too_long".to_string(), Value::Int(too_long));
    result.insert("low_quality".to_string(), Value::Int(low_quality));
    result.insert("low_complexity".to_string(), Value::Int(low_complexity));
    result.insert("too_many_n".to_string(), Value::Int(too_many_n));
    Ok(Value::Record(result))
}

// ── Comprehensive read statistics ───────────────────────────────────

/// read_stats(fastq_path)
/// Comprehensive FASTQ statistics.
fn builtin_read_stats(path: &str) -> Result<Value> {
    let max_dup_reads: usize = 10_000;
    let dup_prefix_len: usize = 50;

    let reader_buf = open_fastq_reader(path)?;
    let mut reader = noodles_fastq::io::Reader::new(reader_buf);

    let mut total_reads: i64 = 0;
    let mut total_bases: i64 = 0;
    let mut lengths: Vec<usize> = Vec::new();
    let mut gc_count: i64 = 0;
    let mut q20_bases: i64 = 0;
    let mut q30_bases: i64 = 0;
    let mut qual_sum: f64 = 0.0;
    let mut total_qual_bases: i64 = 0;

    // Per-position quality sums and counts (up to 300bp)
    let max_pos: usize = 300;
    let mut pos_qual_sum: Vec<f64> = vec![0.0; max_pos];
    let mut pos_qual_count: Vec<i64> = vec![0; max_pos];
    let mut pos_n_count: Vec<i64> = vec![0; max_pos];

    // Adapter detection counts
    let mut adapter_hit_counts: Vec<i64> = vec![0; KNOWN_ADAPTERS.len()];

    // Duplication estimation (hash first N read prefixes)
    let mut dup_set: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut dup_sampled: usize = 0;

    let mut record = noodles_fastq::Record::default();
    loop {
        match reader.read_record(&mut record) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let seq = record.sequence();
        let qual = record.quality_scores();
        let len = seq.len();

        total_reads += 1;
        total_bases += len as i64;
        lengths.push(len);

        // GC and N content
        for &b in seq {
            match b {
                b'G' | b'g' | b'C' | b'c' => gc_count += 1,
                b'N' | b'n' => {}
                _ => {}
            }
        }

        // Quality stats
        for (i, &q) in qual.iter().enumerate() {
            let phred = (q as f64) - 33.0;
            qual_sum += phred;
            total_qual_bases += 1;
            if phred >= 20.0 {
                q20_bases += 1;
            }
            if phred >= 30.0 {
                q30_bases += 1;
            }
            if i < max_pos {
                pos_qual_sum[i] += phred;
                pos_qual_count[i] += 1;
            }
        }

        // Per-position N count
        for (i, &b) in seq.iter().enumerate() {
            if i >= max_pos {
                break;
            }
            if b == b'N' || b == b'n' {
                pos_n_count[i] += 1;
            }
        }

        // Adapter contamination check (in tail of read)
        if seq.len() >= 20 {
            let tail = &seq[seq.len() - 20..];
            for (ai, (adapter_seq, _)) in KNOWN_ADAPTERS.iter().enumerate() {
                let ab = adapter_seq.as_bytes();
                let check_len = ab.len().min(tail.len());
                if check_len >= 8 {
                    for start in 0..tail.len() {
                        let remaining = tail.len() - start;
                        let cl = check_len.min(remaining);
                        if cl >= 8 {
                            let mm: usize = tail[start..start + cl]
                                .iter()
                                .zip(&ab[..cl])
                                .filter(|(a, b)| !a.eq_ignore_ascii_case(b))
                                .count();
                            if mm <= 1 {
                                adapter_hit_counts[ai] += 1;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Duplication sampling
        if dup_sampled < max_dup_reads && seq.len() >= dup_prefix_len {
            let prefix = seq[..dup_prefix_len].to_vec();
            *dup_set.entry(prefix).or_insert(0) += 1;
            dup_sampled += 1;
        }
    }

    if total_reads == 0 {
        let mut result = HashMap::new();
        result.insert("total_reads".to_string(), Value::Int(0));
        result.insert("total_bases".to_string(), Value::Int(0));
        return Ok(Value::Record(result));
    }

    // Length distribution
    lengths.sort_unstable();
    let min_len = *lengths.first().unwrap() as i64;
    let max_len_val = *lengths.last().unwrap() as i64;
    let mean_len = total_bases as f64 / total_reads as f64;
    let median_len = lengths[lengths.len() / 2] as i64;

    // N50 calculation
    let half_total: i64 = total_bases / 2;
    let mut cumulative: i64 = 0;
    let mut n50: i64 = 0;
    for &l in lengths.iter().rev() {
        cumulative += l as i64;
        if cumulative >= half_total {
            n50 = l as i64;
            break;
        }
    }

    // GC content
    let gc_fraction = if total_bases > 0 {
        gc_count as f64 / total_bases as f64
    } else {
        0.0
    };

    // Quality stats
    let mean_quality = if total_qual_bases > 0 {
        qual_sum / total_qual_bases as f64
    } else {
        0.0
    };
    let q20_pct = if total_qual_bases > 0 {
        q20_bases as f64 / total_qual_bases as f64 * 100.0
    } else {
        0.0
    };
    let q30_pct = if total_qual_bases > 0 {
        q30_bases as f64 / total_qual_bases as f64 * 100.0
    } else {
        0.0
    };

    // Per-position mean quality (trim to actual max read length)
    let actual_max_pos = lengths.last().copied().unwrap_or(0).min(max_pos);
    let per_pos_quality: Vec<Value> = (0..actual_max_pos)
        .map(|i| {
            if pos_qual_count[i] > 0 {
                Value::Float(pos_qual_sum[i] / pos_qual_count[i] as f64)
            } else {
                Value::Float(0.0)
            }
        })
        .collect();

    // Per-position N fraction
    let per_pos_n: Vec<Value> = (0..actual_max_pos)
        .map(|i| {
            if pos_qual_count[i] > 0 {
                Value::Float(pos_n_count[i] as f64 / pos_qual_count[i] as f64)
            } else {
                Value::Float(0.0)
            }
        })
        .collect();

    // Adapter contamination rates
    let mut adapter_results = Vec::new();
    for (i, (adapter_seq, adapter_name)) in KNOWN_ADAPTERS.iter().enumerate() {
        let rate = adapter_hit_counts[i] as f64 / total_reads as f64;
        let mut rec = HashMap::new();
        rec.insert("adapter".to_string(), Value::Str(adapter_seq.to_string()));
        rec.insert("name".to_string(), Value::Str(adapter_name.to_string()));
        rec.insert("count".to_string(), Value::Int(adapter_hit_counts[i]));
        rec.insert("rate".to_string(), Value::Float(rate));
        adapter_results.push(Value::Record(rec));
    }

    // Duplication rate
    let dup_rate = if dup_sampled > 0 {
        let unique = dup_set.len();
        1.0 - (unique as f64 / dup_sampled as f64)
    } else {
        0.0
    };

    // Build result record
    let mut result = HashMap::new();
    result.insert("total_reads".to_string(), Value::Int(total_reads));
    result.insert("total_bases".to_string(), Value::Int(total_bases));

    // Length distribution sub-record
    let mut len_dist = HashMap::new();
    len_dist.insert("min".to_string(), Value::Int(min_len));
    len_dist.insert("max".to_string(), Value::Int(max_len_val));
    len_dist.insert("mean".to_string(), Value::Float(mean_len));
    len_dist.insert("median".to_string(), Value::Int(median_len));
    len_dist.insert("n50".to_string(), Value::Int(n50));
    result.insert("length".to_string(), Value::Record(len_dist));

    // Quality distribution sub-record
    let mut qual_dist = HashMap::new();
    qual_dist.insert("mean".to_string(), Value::Float(mean_quality));
    qual_dist.insert("q20_pct".to_string(), Value::Float(q20_pct));
    qual_dist.insert("q30_pct".to_string(), Value::Float(q30_pct));
    qual_dist.insert("per_position".to_string(), Value::List(per_pos_quality));
    result.insert("quality".to_string(), Value::Record(qual_dist));

    result.insert("gc_content".to_string(), Value::Float(gc_fraction));
    result.insert("n_content".to_string(), Value::List(per_pos_n));
    result.insert("adapters".to_string(), Value::List(adapter_results));
    result.insert("duplication_rate".to_string(), Value::Float(dup_rate));

    Ok(Value::Record(result))
}

// ── Bio accessor implementations ───────────────────────────────────

/// Convert a Record with {chrom, start, end} or a Table with those columns -> Interval(s).
/// Accepts both BED-style (chrom/start/end) and GFF-style (seqid/start/end).
fn builtin_to_interval(val: &Value) -> Result<Value> {
    match val {
        Value::Record(fields) => record_to_interval(fields).map(Value::Interval),
        Value::Table(t) => {
            let mut intervals = Vec::with_capacity(t.rows.len());
            // Find column indices once
            let chrom_ci = t.col_index("chrom").or_else(|| t.col_index("seqid"));
            let start_ci = t.col_index("start");
            let end_ci = t.col_index("end");
            let strand_ci = t.col_index("strand");

            let chrom_ci = chrom_ci.ok_or_else(|| {
                BioLangError::type_error("to_interval() table needs 'chrom' or 'seqid' column", None)
            })?;
            let start_ci = start_ci.ok_or_else(|| {
                BioLangError::type_error("to_interval() table needs 'start' column", None)
            })?;
            let end_ci = end_ci.ok_or_else(|| {
                BioLangError::type_error("to_interval() table needs 'end' column", None)
            })?;

            for row in &t.rows {
                let chrom = match &row[chrom_ci] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("to_interval() chrom must be Str, got {}", other.type_of()),
                            None,
                        ))
                    }
                };
                let start = val_to_i64(&row[start_ci], "to_interval", "start")?;
                let end = val_to_i64(&row[end_ci], "to_interval", "end")?;
                let strand = strand_ci
                    .and_then(|ci| match &row[ci] {
                        Value::Str(s) => Some(Strand::from_str_lossy(s)),
                        _ => None,
                    })
                    .unwrap_or(Strand::Unknown);
                intervals.push(Value::Interval(GenomicInterval { chrom, start, end, strand }));
            }
            Ok(Value::List(intervals))
        }
        Value::Interval(_) => Ok(val.clone()),
        other => Err(BioLangError::type_error(
            format!("to_interval() requires Record or Table, got {}", other.type_of()),
            None,
        )),
    }
}

fn record_to_interval(fields: &HashMap<String, Value>) -> Result<GenomicInterval> {
    let chrom = fields
        .get("chrom")
        .or_else(|| fields.get("seqid"))
        .ok_or_else(|| {
            BioLangError::type_error("to_interval() record needs 'chrom' or 'seqid' field", None)
        })?;
    let chrom = match chrom {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("to_interval() chrom must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let start = fields
        .get("start")
        .ok_or_else(|| BioLangError::type_error("to_interval() record needs 'start' field", None))?;
    let end = fields
        .get("end")
        .ok_or_else(|| BioLangError::type_error("to_interval() record needs 'end' field", None))?;
    let start = val_to_i64(start, "to_interval", "start")?;
    let end = val_to_i64(end, "to_interval", "end")?;
    let strand = fields
        .get("strand")
        .and_then(|v| match v {
            Value::Str(s) => Some(Strand::from_str_lossy(s)),
            _ => None,
        })
        .unwrap_or(Strand::Unknown);
    Ok(GenomicInterval { chrom, start, end, strand })
}

/// Parse VCF INFO field: "DP=100;AF=0.5;DB" -> Record {DP: 100, AF: 0.5, DB: true}
/// If key is given, return just that value.
/// Accepts a Str (raw INFO) or a Record (extracts .info field).
fn builtin_parse_info(val: &Value, key: Option<&str>) -> Result<Value> {
    let info_str = match val {
        Value::Str(s) => s.clone(),
        Value::Record(fields) => match fields.get("info") {
            Some(Value::Str(s)) => s.clone(),
            Some(Value::Nil) | None => return Ok(if key.is_some() { Value::Nil } else { Value::Record(HashMap::new()) }),
            Some(other) => {
                return Err(BioLangError::type_error(
                    format!("parse_info() info field must be Str, got {}", other.type_of()),
                    None,
                ))
            }
        },
        other => {
            return Err(BioLangError::type_error(
                format!("parse_info() requires Str or Record, got {}", other.type_of()),
                None,
            ))
        }
    };

    if info_str == "." || info_str.is_empty() {
        return Ok(if key.is_some() { Value::Nil } else { Value::Record(HashMap::new()) });
    }

    let parsed = parse_vcf_info_str(&info_str);

    if let Some(k) = key {
        Ok(parsed.get(k).cloned().unwrap_or(Value::Nil))
    } else {
        Ok(Value::Record(parsed))
    }
}

fn parse_vcf_info_str(info: &str) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    for field in info.split(';') {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((k, v)) = field.split_once('=') {
            result.insert(k.to_string(), parse_info_value(v));
        } else {
            // Flag field (no value) — treat as true
            result.insert(field.to_string(), Value::Bool(true));
        }
    }
    result
}

fn parse_info_value(v: &str) -> Value {
    // Try int first
    if let Ok(n) = v.parse::<i64>() {
        return Value::Int(n);
    }
    // Try float
    if let Ok(f) = v.parse::<f64>() {
        return Value::Float(f);
    }
    // Multi-value (comma-separated)
    if v.contains(',') {
        let items: Vec<Value> = v.split(',').map(|s| parse_info_value(s.trim())).collect();
        return Value::List(items);
    }
    Value::Str(v.to_string())
}

/// Parse GFF/GTF attributes: "ID=gene1;Name=TP53;biotype=protein_coding" -> Record
/// GTF format: 'gene_id "ENSG00000141510"; gene_name "TP53"' also supported.
/// If key is given, return just that value.
fn builtin_parse_attr(val: &Value, key: Option<&str>) -> Result<Value> {
    let attr_str = match val {
        Value::Str(s) => s.clone(),
        Value::Record(fields) => match fields.get("attributes") {
            Some(Value::Str(s)) => s.clone(),
            Some(Value::Nil) | None => return Ok(if key.is_some() { Value::Nil } else { Value::Record(HashMap::new()) }),
            Some(other) => {
                return Err(BioLangError::type_error(
                    format!("parse_attr() attributes field must be Str, got {}", other.type_of()),
                    None,
                ))
            }
        },
        other => {
            return Err(BioLangError::type_error(
                format!("parse_attr() requires Str or Record, got {}", other.type_of()),
                None,
            ))
        }
    };

    if attr_str == "." || attr_str.is_empty() {
        return Ok(if key.is_some() { Value::Nil } else { Value::Record(HashMap::new()) });
    }

    let parsed = parse_gff_attr_str(&attr_str);

    if let Some(k) = key {
        Ok(parsed.get(k).cloned().unwrap_or(Value::Nil))
    } else {
        Ok(Value::Record(parsed))
    }
}

fn parse_gff_attr_str(attr: &str) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    // Detect format: GTF uses space + quotes, GFF3 uses = and ;
    let is_gtf = attr.contains("\" ") || attr.contains("\";");
    if is_gtf {
        // GTF: gene_id "ENSG..."; gene_name "TP53";
        for field in attr.split(';') {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            if let Some((k, v)) = field.split_once(char::is_whitespace) {
                let k = k.trim();
                let v = v.trim().trim_matches('"');
                result.insert(k.to_string(), Value::Str(v.to_string()));
            }
        }
    } else {
        // GFF3: ID=gene1;Name=TP53;Dbxref=GeneID:1234,HGNC:5678
        for field in attr.split(';') {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            if let Some((k, v)) = field.split_once('=') {
                let v = v.trim();
                if v.contains(',') {
                    let items: Vec<Value> =
                        v.split(',').map(|s| Value::Str(s.trim().to_string())).collect();
                    result.insert(k.to_string(), Value::List(items));
                } else {
                    result.insert(k.to_string(), Value::Str(v.to_string()));
                }
            }
        }
    }
    result
}

/// Parse FASTQ quality string -> List of Phred scores (Int).
/// Phred+33 encoding (standard Illumina 1.8+).
fn builtin_parse_qual(val: &Value) -> Result<Value> {
    let qual_str = match val {
        Value::Str(s) => s.clone(),
        Value::Record(fields) => match fields.get("quality") {
            Some(Value::Str(s)) => s.clone(),
            Some(Value::Nil) | None => return Ok(Value::List(Vec::new())),
            Some(other) => {
                return Err(BioLangError::type_error(
                    format!("parse_qual() quality field must be Str, got {}", other.type_of()),
                    None,
                ))
            }
        },
        other => {
            return Err(BioLangError::type_error(
                format!("parse_qual() requires Str or Record, got {}", other.type_of()),
                None,
            ))
        }
    };

    let scores: Vec<Value> = qual_str
        .bytes()
        .map(|b| Value::Int((b as i64) - 33))
        .collect();
    Ok(Value::List(scores))
}

/// Base composition of a DNA/RNA sequence -> Record {A: n, T: n, G: n, C: n, N: n, GC: 0.xx}
fn builtin_base_counts(val: &Value) -> Result<Value> {
    let data = get_seq_data(val, "base_counts")?;
    let mut counts: HashMap<char, i64> = HashMap::new();
    for c in data.chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    let total = data.len() as f64;
    let gc = (*counts.get(&'G').unwrap_or(&0) + *counts.get(&'C').unwrap_or(&0)) as f64;

    let mut result = HashMap::new();
    // Standard bases
    let bases = match val {
        Value::DNA(_) => vec!['A', 'T', 'G', 'C', 'N'],
        Value::RNA(_) => vec!['A', 'U', 'G', 'C', 'N'],
        _ => vec!['A', 'T', 'G', 'C', 'N'],
    };
    for b in bases {
        result.insert(b.to_string(), Value::Int(*counts.get(&b).unwrap_or(&0)));
    }
    result.insert(
        "GC".to_string(),
        Value::Float(if total > 0.0 { gc / total } else { 0.0 }),
    );
    result.insert("total".to_string(), Value::Int(data.len() as i64));
    Ok(Value::Record(result))
}

fn val_to_i64(val: &Value, func: &str, field: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        Value::Float(f) => Ok(*f as i64),
        other => Err(BioLangError::type_error(
            format!("{func}() {field} must be numeric, got {}", other.type_of()),
            None,
        )),
    }
}

// ── Helpers ────────────────────────────────────────────────────────

/// Check if second arg is a Record with `stream: true`.
/// For fasta/fastq: check if user explicitly passed {stream: false} to get table mode.
fn wants_no_stream(args: &[Value]) -> bool {
    if args.len() > 1 {
        if let Value::Record(opts) = &args[1] {
            if let Some(v) = opts.get("stream") {
                return !v.is_truthy();
            }
            // Also support {table: true}
            if let Some(v) = opts.get("table") {
                return v.is_truthy();
            }
        }
    }
    false
}

fn wants_stream(args: &[Value]) -> bool {
    if args.len() > 1 {
        if let Value::Record(opts) = &args[1] {
            return opts
                .get("stream")
                .map(|v| v.is_truthy())
                .unwrap_or(false);
        }
    }
    false
}

fn require_str(val: &Value, func: &str) -> Result<String> {
    match val {
        Value::Str(s) => Ok(s.clone()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Int, got {}", other.type_of()),
            None,
        )),
    }
}

#[allow(dead_code)]
fn require_float(val: &Value, func: &str) -> Result<f64> {
    match val {
        Value::Float(f) => Ok(*f),
        Value::Int(n) => Ok(*n as f64),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Float, got {}", other.type_of()),
            None,
        )),
    }
}

/// Accept either Float or Int, returning f64.
fn require_float_or_int(val: &Value, func: &str) -> Result<f64> {
    match val {
        Value::Float(f) => Ok(*f),
        Value::Int(n) => Ok(*n as f64),
        other => Err(BioLangError::type_error(
            format!("{func}() requires a number, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_record(val: &Value, func: &str) -> Result<HashMap<String, Value>> {
    match val {
        Value::Record(r) => Ok(r.clone()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Record, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_list(val: &Value, func: &str) -> Result<Vec<Value>> {
    match val {
        Value::List(l) => Ok(l.clone()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires List, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_dna<'a>(val: &'a Value, func: &str) -> Result<&'a BioSequence> {
    match val {
        Value::DNA(seq) => Ok(seq),
        other => Err(BioLangError::type_error(
            format!("{func}() requires DNA, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_rna_or_dna<'a>(val: &'a Value, func: &str) -> Result<&'a BioSequence> {
    match val {
        Value::DNA(seq) | Value::RNA(seq) => Ok(seq),
        other => Err(BioLangError::type_error(
            format!("{func}() requires DNA or RNA, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_nucleic<'a>(val: &'a Value, func: &str) -> Result<&'a BioSequence> {
    match val {
        Value::DNA(seq) | Value::RNA(seq) => Ok(seq),
        other => Err(BioLangError::type_error(
            format!("{func}() requires DNA or RNA, got {}", other.type_of()),
            None,
        )),
    }
}

fn get_seq_data(val: &Value, func: &str) -> Result<String> {
    match val {
        Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => Ok(seq.data.clone()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires a sequence type, got {}", other.type_of()),
            None,
        )),
    }
}

fn validate_dna(s: &str) -> Result<()> {
    if !bio_core::seq_ops::is_valid_dna(s) {
        for (i, c) in s.chars().enumerate() {
            if !matches!(c.to_ascii_uppercase(), 'A' | 'T' | 'G' | 'C' | 'N') {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("invalid DNA base '{c}' at position {i}"),
                    None,
                ));
            }
        }
    }
    Ok(())
}

fn validate_rna(s: &str) -> Result<()> {
    if !bio_core::seq_ops::is_valid_rna(s) {
        for (i, c) in s.chars().enumerate() {
            if !matches!(c.to_ascii_uppercase(), 'A' | 'U' | 'G' | 'C' | 'N') {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("invalid RNA base '{c}' at position {i}"),
                    None,
                ));
            }
        }
    }
    Ok(())
}

// ── Option extraction helpers for Record-based options ──────────────

/// Extract an optional integer from a Record, with a default.
fn opt_int(opts: &HashMap<String, Value>, key: &str, default: i64) -> i64 {
    match opts.get(key) {
        Some(Value::Int(n)) => *n,
        Some(Value::Float(f)) => *f as i64,
        _ => default,
    }
}

/// Extract an optional float from a Record, with a default.
fn opt_float(opts: &HashMap<String, Value>, key: &str, default: f64) -> f64 {
    match opts.get(key) {
        Some(Value::Float(f)) => *f,
        Some(Value::Int(n)) => *n as f64,
        _ => default,
    }
}

/// Extract an optional bool from a Record, with a default.
fn opt_bool(opts: &HashMap<String, Value>, key: &str, default: bool) -> bool {
    match opts.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(v) => v.is_truthy(),
        None => default,
    }
}

// ══════════════════════════════════════════════════════════════════════
// Variant Analysis Builtins
// ══════════════════════════════════════════════════════════════════════

/// Helper: extract a VCF Table and return (ref_col_idx, alt_col_idx, filter_col_idx, table_ref)
fn require_vcf_table<'a>(val: &'a Value, func: &str) -> Result<&'a bl_core::value::Table> {
    match val {
        Value::Table(t) => {
            if t.col_index("ref").is_none() && t.col_index("REF").is_none() {
                return Err(BioLangError::type_error(
                    format!("{func}() table needs 'ref' or 'REF' column"),
                    None,
                ));
            }
            Ok(t)
        }
        other => Err(BioLangError::type_error(
            format!("{func}() requires a VCF Table, got {}", other.type_of()),
            None,
        )),
    }
}

fn vcf_ref_alt_cols(t: &bl_core::value::Table) -> (usize, usize) {
    let ref_ci = t.col_index("ref").or_else(|| t.col_index("REF")).unwrap();
    let alt_ci = t.col_index("alt").or_else(|| t.col_index("ALT")).unwrap_or(ref_ci + 1);
    (ref_ci, alt_ci)
}

fn val_as_str(v: &Value) -> &str {
    match v {
        Value::Str(s) => s.as_str(),
        _ => ".",
    }
}

/// normalize_variant(table) -> Table
/// Left-align and trim variant alleles: remove common suffixes, then common prefixes.
fn builtin_normalize_variant(args: Vec<Value>) -> Result<Value> {
    let t = require_vcf_table(&args[0], "normalize_variant")?;
    let (ref_ci, alt_ci) = vcf_ref_alt_cols(t);
    let pos_ci = t.col_index("pos").or_else(|| t.col_index("POS"));

    let mut new_rows: Vec<Vec<Value>> = Vec::with_capacity(t.rows.len());
    for row in &t.rows {
        let mut row: Vec<Value> = row.clone();
        let ref_allele = val_as_str(&row[ref_ci]).to_string();
        let alt_allele = val_as_str(&row[alt_ci]).to_string();

        let ref_bytes = ref_allele.as_bytes();
        let alt_bytes = alt_allele.as_bytes();

        // Trim common suffix
        let mut r_end = ref_bytes.len();
        let mut a_end = alt_bytes.len();
        while r_end > 1 && a_end > 1 && ref_bytes[r_end - 1] == alt_bytes[a_end - 1] {
            r_end -= 1;
            a_end -= 1;
        }

        // Trim common prefix (keep at least 1 base)
        let mut prefix_trim = 0;
        while prefix_trim + 1 < r_end && prefix_trim + 1 < a_end
            && ref_bytes[prefix_trim] == alt_bytes[prefix_trim]
        {
            prefix_trim += 1;
        }

        let new_ref = &ref_allele[prefix_trim..r_end];
        let new_alt = &alt_allele[prefix_trim..a_end];
        row[ref_ci] = Value::Str(new_ref.to_string());
        row[alt_ci] = Value::Str(new_alt.to_string());

        // Adjust position if we trimmed prefix
        if prefix_trim > 0 {
            if let Some(pi) = pos_ci {
                if let Value::Int(p) = &row[pi] {
                    row[pi] = Value::Int(p + prefix_trim as i64);
                }
            }
        }

        new_rows.push(row);
    }

    Ok(Value::Table(bl_core::value::Table::new(t.columns.clone(), new_rows)))
}

/// tstv_ratio(table) -> Float
/// Compute transition/transversion ratio from a VCF table.
fn builtin_tstv_ratio(args: Vec<Value>) -> Result<Value> {
    let t = require_vcf_table(&args[0], "tstv_ratio")?;
    let (ref_ci, alt_ci) = vcf_ref_alt_cols(t);

    let mut transitions: u64 = 0;
    let mut transversions: u64 = 0;

    for row in &t.rows {
        let ref_a = val_as_str(&row[ref_ci]);
        let alt_str = val_as_str(&row[alt_ci]);
        for alt in alt_str.split(',') {
            let alt = alt.trim();
            if ref_a.len() == 1 && alt.len() == 1 {
                if bio_core::vcf_ops::is_transition(ref_a, alt) {
                    transitions += 1;
                } else {
                    transversions += 1;
                }
            }
        }
    }

    let ratio = if transversions > 0 {
        transitions as f64 / transversions as f64
    } else {
        0.0
    };
    Ok(Value::Float(ratio))
}

/// het_hom_ratio(table) -> Float
/// Compute heterozygous/homozygous ratio. Looks for FORMAT and first sample columns.
fn builtin_het_hom_ratio(args: Vec<Value>) -> Result<Value> {
    let t = require_vcf_table(&args[0], "het_hom_ratio")?;
    let fmt_ci = t.col_index("format").or_else(|| t.col_index("FORMAT"));
    // Find first sample column (column after FORMAT, or last column)
    let sample_ci = fmt_ci.map(|f| f + 1).filter(|&s| s < t.columns.len());

    let (fmt_ci, sample_ci) = match (fmt_ci, sample_ci) {
        (Some(f), Some(s)) => (f, s),
        _ => return Ok(Value::Float(0.0)),
    };

    let mut het: u64 = 0;
    let mut hom: u64 = 0;

    for row in &t.rows {
        let format = val_as_str(&row[fmt_ci]);
        let sample = val_as_str(&row[sample_ci]);
        if let Some(gt) = bio_core::vcf_ops::parse_gt(format, sample) {
            let non_missing: Vec<u8> = gt.iter().filter_map(|a| *a).collect();
            if non_missing.len() >= 2 {
                if non_missing.iter().all(|&a| a == non_missing[0]) {
                    hom += 1;
                } else {
                    het += 1;
                }
            }
        }
    }

    let ratio = if hom > 0 {
        het as f64 / hom as f64
    } else {
        0.0
    };
    Ok(Value::Float(ratio))
}

/// variant_stats(table) -> Record
/// Comprehensive variant summary: SNP/indel/MNP counts, Ts/Tv, multiallelic count.
fn builtin_variant_stats(args: Vec<Value>) -> Result<Value> {
    let t = require_vcf_table(&args[0], "variant_stats")?;
    let (ref_ci, alt_ci) = vcf_ref_alt_cols(t);

    let variant_data: Vec<(String, Vec<String>)> = t
        .rows
        .iter()
        .map(|row| {
            let r = val_as_str(&row[ref_ci]).to_string();
            let a: Vec<String> = val_as_str(&row[alt_ci]).split(',').map(|s| s.trim().to_string()).collect();
            (r, a)
        })
        .collect();

    let variant_refs: Vec<(&str, Vec<&str>)> = variant_data
        .iter()
        .map(|(r, a)| (r.as_str(), a.iter().map(|s| s.as_str()).collect::<Vec<&str>>()))
        .collect();
    let pairs: Vec<(&str, &[&str])> = variant_refs.iter().map(|(r, a)| (*r, a.as_slice())).collect();
    let summary = bio_core::vcf_ops::summarize_variants(&pairs);

    let mut result = HashMap::new();
    result.insert("total".to_string(), Value::Int(t.rows.len() as i64));
    result.insert("snp".to_string(), Value::Int(summary.snp as i64));
    result.insert("indel".to_string(), Value::Int(summary.indel as i64));
    result.insert("mnp".to_string(), Value::Int(summary.mnp as i64));
    result.insert("other".to_string(), Value::Int(summary.other as i64));
    result.insert("transitions".to_string(), Value::Int(summary.transitions as i64));
    result.insert("transversions".to_string(), Value::Int(summary.transversions as i64));
    result.insert("ts_tv_ratio".to_string(), Value::Float(summary.ts_tv_ratio));
    result.insert("multiallelic".to_string(), Value::Int(summary.multiallelic as i64));
    Ok(Value::Record(result))
}

/// decompose_mnp(table) -> Table
/// Decompose multi-nucleotide polymorphisms into individual SNPs.
fn builtin_decompose_mnp(args: Vec<Value>) -> Result<Value> {
    let t = require_vcf_table(&args[0], "decompose_mnp")?;
    let (ref_ci, alt_ci) = vcf_ref_alt_cols(t);
    let pos_ci = t.col_index("pos").or_else(|| t.col_index("POS"));

    let mut new_rows = Vec::new();
    for row in &t.rows {
        let ref_a = val_as_str(&row[ref_ci]);
        let alt_a = val_as_str(&row[alt_ci]);

        if ref_a.len() == alt_a.len() && ref_a.len() > 1 {
            // Decompose: create one SNP per differing position
            let base_pos: Option<i64> = pos_ci.and_then(|pi| match &row[pi] {
                Value::Int(p) => Some(*p),
                _ => None,
            });

            for (i, (rb, ab)) in ref_a.bytes().zip(alt_a.bytes()).enumerate() {
                if rb != ab {
                    let mut new_row = row.clone();
                    new_row[ref_ci] = Value::Str(String::from(rb as char));
                    new_row[alt_ci] = Value::Str(String::from(ab as char));
                    if let (Some(pi), Some(bp)) = (pos_ci, base_pos) {
                        new_row[pi] = Value::Int(bp + i as i64);
                    }
                    new_rows.push(new_row);
                }
            }
        } else {
            new_rows.push(row.clone());
        }
    }

    Ok(Value::Table(bl_core::value::Table::new(t.columns.clone(), new_rows)))
}

/// filter_pass(table) -> Table
/// Keep only rows where FILTER column is "PASS" or ".".
fn builtin_filter_pass(args: Vec<Value>) -> Result<Value> {
    let t = require_vcf_table(&args[0], "filter_pass")?;
    let filter_ci = t.col_index("filter").or_else(|| t.col_index("FILTER"));

    let new_rows: Vec<Vec<Value>> = match filter_ci {
        Some(fi) => t
            .rows
            .iter()
            .filter(|row| {
                let f = val_as_str(&row[fi]);
                f == "PASS" || f == "." || f.is_empty()
            })
            .cloned()
            .collect(),
        None => t.rows.clone(), // No FILTER column — keep all
    };

    Ok(Value::Table(bl_core::value::Table::new(t.columns.clone(), new_rows)))
}

// ══════════════════════════════════════════════════════════════════════
// RNA-seq Normalization Builtins
// ══════════════════════════════════════════════════════════════════════

fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a bl_core::value::Table> {
    match val {
        Value::Table(t) => Ok(t),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table, got {}", other.type_of()),
            None,
        )),
    }
}

fn val_as_f64(v: &Value) -> f64 {
    match v {
        Value::Int(n) => *n as f64,
        Value::Float(f) => *f,
        _ => 0.0,
    }
}

/// tpm(counts_table, lengths_table) -> Table
/// Transcripts Per Million normalization.
/// counts_table: columns [gene, count], lengths_table: columns [gene, length]
fn builtin_tpm(args: Vec<Value>) -> Result<Value> {
    let counts = require_table(&args[0], "tpm")?;
    let lengths = require_table(&args[1], "tpm")?;

    let gene_ci = counts.col_index("gene").unwrap_or(0);
    let count_ci = counts.col_index("count").unwrap_or(1);
    let lg_ci = lengths.col_index("gene").unwrap_or(0);
    let len_ci = lengths.col_index("length").unwrap_or(1);

    // Build gene -> length map
    let mut length_map: HashMap<String, f64> = HashMap::new();
    for row in &lengths.rows {
        let gene = val_as_str(&row[lg_ci]).to_string();
        let len = val_as_f64(&row[len_ci]);
        if len > 0.0 {
            length_map.insert(gene, len);
        }
    }

    // Step 1: RPK (reads per kilobase)
    let mut rpks: Vec<(String, f64)> = Vec::new();
    let mut rpk_sum = 0.0;
    for row in &counts.rows {
        let gene = val_as_str(&row[gene_ci]).to_string();
        let count = val_as_f64(&row[count_ci]);
        let len = length_map.get(&gene).copied().unwrap_or(1000.0);
        let rpk = count / (len / 1000.0);
        rpk_sum += rpk;
        rpks.push((gene, rpk));
    }

    // Step 2: Scale to per million
    let scale = rpk_sum / 1_000_000.0;
    let mut rows = Vec::with_capacity(rpks.len());
    for (gene, rpk) in rpks {
        let tpm = if scale > 0.0 { rpk / scale } else { 0.0 };
        rows.push(vec![Value::Str(gene), Value::Float(tpm)]);
    }

    Ok(Value::Table(bl_core::value::Table::new(
        vec!["gene".to_string(), "tpm".to_string()],
        rows,
    )))
}

/// fpkm(counts_table, lengths_table, total_reads) -> Table
/// Fragments Per Kilobase per Million mapped reads.
fn builtin_fpkm(args: Vec<Value>) -> Result<Value> {
    let counts = require_table(&args[0], "fpkm")?;
    let lengths = require_table(&args[1], "fpkm")?;
    let total_reads = require_float_or_int(&args[2], "fpkm")?;

    let gene_ci = counts.col_index("gene").unwrap_or(0);
    let count_ci = counts.col_index("count").unwrap_or(1);
    let lg_ci = lengths.col_index("gene").unwrap_or(0);
    let len_ci = lengths.col_index("length").unwrap_or(1);

    let mut length_map: HashMap<String, f64> = HashMap::new();
    for row in &lengths.rows {
        let gene = val_as_str(&row[lg_ci]).to_string();
        let len = val_as_f64(&row[len_ci]);
        if len > 0.0 {
            length_map.insert(gene, len);
        }
    }

    let scale = total_reads / 1_000_000.0;
    let mut rows = Vec::new();
    for row in &counts.rows {
        let gene = val_as_str(&row[gene_ci]).to_string();
        let count = val_as_f64(&row[count_ci]);
        let len = length_map.get(&gene).copied().unwrap_or(1000.0);
        let fpkm = if scale > 0.0 && len > 0.0 {
            count / (scale * (len / 1000.0))
        } else {
            0.0
        };
        rows.push(vec![Value::Str(gene), Value::Float(fpkm)]);
    }

    Ok(Value::Table(bl_core::value::Table::new(
        vec!["gene".to_string(), "fpkm".to_string()],
        rows,
    )))
}

/// cpm(counts_table) -> Table
/// Counts Per Million normalization.
fn builtin_cpm(args: Vec<Value>) -> Result<Value> {
    let counts = require_table(&args[0], "cpm")?;
    let gene_ci = counts.col_index("gene").unwrap_or(0);
    let count_ci = counts.col_index("count").unwrap_or(1);

    let total: f64 = counts.rows.iter().map(|r| val_as_f64(&r[count_ci])).sum();
    let scale = total / 1_000_000.0;

    let mut rows = Vec::new();
    for row in &counts.rows {
        let gene = val_as_str(&row[gene_ci]).to_string();
        let count = val_as_f64(&row[count_ci]);
        let cpm = if scale > 0.0 { count / scale } else { 0.0 };
        rows.push(vec![Value::Str(gene), Value::Float(cpm)]);
    }

    Ok(Value::Table(bl_core::value::Table::new(
        vec!["gene".to_string(), "cpm".to_string()],
        rows,
    )))
}

// ══════════════════════════════════════════════════════════════════════
// Sequence Analysis Builtins
// ══════════════════════════════════════════════════════════════════════

/// codon_usage(dna_or_rna) -> Record
/// Count occurrences of each codon in a coding sequence.
fn builtin_codon_usage(args: Vec<Value>) -> Result<Value> {
    let data = get_seq_data(&args[0], "codon_usage")?;
    let seq = data.to_uppercase();
    let mut counts: HashMap<String, i64> = HashMap::new();

    let bases = seq.as_bytes();
    let n_codons = bases.len() / 3;
    for i in 0..n_codons {
        let codon = std::str::from_utf8(&bases[i * 3..i * 3 + 3]).unwrap_or("???");
        *counts.entry(codon.to_string()).or_insert(0) += 1;
    }

    let total = n_codons as f64;
    let mut result = HashMap::new();
    for (codon, count) in &counts {
        let mut rec = HashMap::new();
        rec.insert("count".to_string(), Value::Int(*count));
        rec.insert(
            "frequency".to_string(),
            Value::Float(if total > 0.0 { *count as f64 / total } else { 0.0 }),
        );
        result.insert(codon.clone(), Value::Record(rec));
    }

    Ok(Value::Record(result))
}

/// tm(dna) -> Float
/// Calculate melting temperature using the Wallace rule (short oligos) or nearest-neighbor
/// approximation. For sequences <= 14bp: Tm = 2*(A+T) + 4*(G+C).
/// For longer: basic salt-adjusted formula.
fn builtin_tm(args: Vec<Value>) -> Result<Value> {
    let seq = require_dna(&args[0], "tm")?;
    let data = seq.data.to_uppercase();
    let len = data.len();
    if len == 0 {
        return Ok(Value::Float(0.0));
    }

    let gc = data.chars().filter(|&c| c == 'G' || c == 'C').count();
    let at = data.chars().filter(|&c| c == 'A' || c == 'T').count();

    let tm = if len <= 14 {
        // Wallace rule
        (2 * at + 4 * gc) as f64
    } else {
        // Basic salt-adjusted: Tm = 64.9 + 41 * (gc - 16.4) / len
        64.9 + 41.0 * (gc as f64 - 16.4) / len as f64
    };

    Ok(Value::Float(tm))
}

/// Common restriction enzyme recognition sites.
const RESTRICTION_ENZYMES: &[(&str, &str)] = &[
    ("EcoRI", "GAATTC"),
    ("BamHI", "GGATCC"),
    ("HindIII", "AAGCTT"),
    ("NotI", "GCGGCCGC"),
    ("XhoI", "CTCGAG"),
    ("SalI", "GTCGAC"),
    ("NdeI", "CATATG"),
    ("NcoI", "CCATGG"),
    ("XbaI", "TCTAGA"),
    ("SpeI", "ACTAGT"),
    ("BglII", "AGATCT"),
    ("KpnI", "GGTACC"),
    ("SacI", "GAGCTC"),
    ("PstI", "CTGCAG"),
    ("SmaI", "CCCGGG"),
    ("ApaI", "GGGCCC"),
    ("ClaI", "ATCGAT"),
    ("EcoRV", "GATATC"),
    ("ScaI", "AGTACT"),
    ("SphI", "GCATGC"),
];

/// restriction_sites(dna, enzyme_or_list) -> List of Records
/// Find restriction enzyme cut sites in a DNA sequence.
/// Second arg: enzyme name string or list of enzyme names.
fn builtin_restriction_sites(args: Vec<Value>) -> Result<Value> {
    let seq = require_dna(&args[0], "restriction_sites")?;
    let data = seq.data.to_uppercase();

    // Parse enzyme list
    let enzyme_names: Vec<String> = match &args[1] {
        Value::Str(s) => {
            if s == "*" || s == "all" {
                RESTRICTION_ENZYMES.iter().map(|(n, _)| n.to_string()).collect()
            } else {
                s.split(',').map(|e| e.trim().to_string()).collect()
            }
        }
        Value::List(list) => list
            .iter()
            .filter_map(|v| match v {
                Value::Str(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        other => {
            return Err(BioLangError::type_error(
                format!("restriction_sites() enzyme arg must be Str or List, got {}", other.type_of()),
                None,
            ))
        }
    };

    let enzyme_map: HashMap<&str, &str> = RESTRICTION_ENZYMES.iter().copied().collect();
    let mut results = Vec::new();

    for name in &enzyme_names {
        let site = match enzyme_map.get(name.as_str()) {
            Some(s) => *s,
            None => {
                // Treat the name as a literal sequence
                name.as_str()
            }
        };
        let site_upper = site.to_uppercase();
        let positions = bio_core::seq_ops::find_motif(&data, &site_upper);
        if !positions.is_empty() {
            let mut rec = HashMap::new();
            rec.insert("enzyme".to_string(), Value::Str(name.clone()));
            rec.insert("site".to_string(), Value::Str(site_upper));
            rec.insert("cuts".to_string(), Value::Int(positions.len() as i64));
            rec.insert(
                "positions".to_string(),
                Value::List(positions.into_iter().map(|p| Value::Int(p as i64)).collect()),
            );
            results.push(Value::Record(rec));
        }
    }

    Ok(Value::List(results))
}

// ══════════════════════════════════════════════════════════════════════
// BAM Analysis Builtins
// ══════════════════════════════════════════════════════════════════════

/// Open a BAM file and return (header, reader).
fn open_bam_reader(
    path: &str,
) -> Result<(
    noodles_sam::Header,
    noodles_bam::io::Reader<noodles_bgzf::io::Reader<std::io::BufReader<std::fs::File>>>,
)> {
    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot open '{path}': {e}"), None)
    })?;
    let buf = std::io::BufReader::new(file);
    let mut reader = noodles_bam::io::Reader::new(buf);
    let header = reader.read_header().map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot read BAM header: {e}"), None)
    })?;
    Ok((header, reader))
}

/// Read next BAM record; returns false on EOF.
fn read_next_bam(
    reader: &mut noodles_bam::io::Reader<noodles_bgzf::io::Reader<std::io::BufReader<std::fs::File>>>,
    header: &noodles_sam::Header,
    record: &mut noodles_sam::alignment::RecordBuf,
) -> Result<bool> {
    match reader.read_record_buf(header, record) {
        Ok(0) => Ok(false),
        Ok(_) => Ok(true),
        Err(e) => Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("BAM read error: {e}"),
            None,
        )),
    }
}

/// Compute alignment span from a RecordBuf's CIGAR.
fn cigar_alignment_span(record: &noodles_sam::alignment::RecordBuf) -> usize {
    use noodles_sam::alignment::record::Cigar;
    use noodles_sam::alignment::record::cigar::op::Kind;
    use noodles_sam::alignment::record::cigar::Op;
    record
        .cigar()
        .iter()
        .filter_map(|r: std::result::Result<Op, _>| r.ok())
        .map(|op: Op| match op.kind() {
            Kind::Match | Kind::Deletion | Kind::Skip
            | Kind::SequenceMatch | Kind::SequenceMismatch => op.len(),
            _ => 0,
        })
        .sum()
}

/// depth(bam_path) or depth(bam_path, region) -> Record
/// Calculate summary depth from a BAM file.
fn builtin_depth(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "depth")?;
    let (header, mut reader) = open_bam_reader(&path)?;

    let mut total_bases: i64 = 0;
    let mut mapped_reads: i64 = 0;
    let mut total_reads: i64 = 0;

    let mut record = noodles_sam::alignment::RecordBuf::default();
    while read_next_bam(&mut reader, &header, &mut record)? {
        total_reads += 1;

        let flags = record.flags();
        if flags.is_unmapped() || flags.is_secondary() || flags.is_supplementary() {
            continue;
        }
        mapped_reads += 1;
        total_bases += cigar_alignment_span(&record) as i64;
    }

    let genome_size: i64 = header
        .reference_sequences()
        .values()
        .map(|rs| rs.length().get() as i64)
        .sum();

    let mean_depth = if genome_size > 0 {
        total_bases as f64 / genome_size as f64
    } else {
        0.0
    };

    let mut result = HashMap::new();
    result.insert("total_reads".to_string(), Value::Int(total_reads));
    result.insert("mapped_reads".to_string(), Value::Int(mapped_reads));
    result.insert("total_aligned_bases".to_string(), Value::Int(total_bases));
    result.insert("genome_size".to_string(), Value::Int(genome_size));
    result.insert("mean_depth".to_string(), Value::Float(mean_depth));
    Ok(Value::Record(result))
}

/// insert_size(bam_path) -> Record
/// Calculate insert size distribution statistics from paired-end BAM.
fn builtin_insert_size(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "insert_size")?;
    let (header, mut reader) = open_bam_reader(&path)?;

    let mut sizes: Vec<i64> = Vec::new();

    let mut record = noodles_sam::alignment::RecordBuf::default();
    while read_next_bam(&mut reader, &header, &mut record)? {
        let flags = record.flags();
        if !flags.is_segmented()
            || flags.is_unmapped()
            || flags.is_mate_unmapped()
            || flags.is_secondary()
            || flags.is_supplementary()
        {
            continue;
        }

        let tlen = record.template_length();
        if tlen > 0 {
            sizes.push(tlen as i64);
        }
    }

    if sizes.is_empty() {
        let mut result = HashMap::new();
        result.insert("count".to_string(), Value::Int(0));
        result.insert("mean".to_string(), Value::Float(0.0));
        result.insert("median".to_string(), Value::Int(0));
        result.insert("std_dev".to_string(), Value::Float(0.0));
        return Ok(Value::Record(result));
    }

    sizes.sort_unstable();
    let count = sizes.len();
    let sum: i64 = sizes.iter().sum();
    let mean = sum as f64 / count as f64;
    let median = sizes[count / 2];
    let variance: f64 = sizes.iter().map(|&s| (s as f64 - mean).powi(2)).sum::<f64>() / count as f64;
    let std_dev = variance.sqrt();

    let p25 = sizes[count / 4];
    let p75 = sizes[3 * count / 4];

    let mut result = HashMap::new();
    result.insert("count".to_string(), Value::Int(count as i64));
    result.insert("mean".to_string(), Value::Float(mean));
    result.insert("median".to_string(), Value::Int(median));
    result.insert("std_dev".to_string(), Value::Float(std_dev));
    result.insert("p25".to_string(), Value::Int(p25));
    result.insert("p75".to_string(), Value::Int(p75));
    result.insert("min".to_string(), Value::Int(sizes[0]));
    result.insert("max".to_string(), Value::Int(sizes[count - 1]));
    Ok(Value::Record(result))
}

/// mapping_rate(bam_path) -> Record
/// Calculate mapping statistics from a BAM file.
fn builtin_mapping_rate(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "mapping_rate")?;
    let (header, mut reader) = open_bam_reader(&path)?;

    let mut total: i64 = 0;
    let mut mapped: i64 = 0;
    let mut paired: i64 = 0;
    let mut proper_pair: i64 = 0;
    let mut duplicates: i64 = 0;
    let mut secondary: i64 = 0;
    let mut supplementary: i64 = 0;
    let mut primary_mapped: i64 = 0;

    let mut record = noodles_sam::alignment::RecordBuf::default();
    while read_next_bam(&mut reader, &header, &mut record)? {
        let flags = record.flags();
        total += 1;

        if flags.is_secondary() {
            secondary += 1;
            continue;
        }
        if flags.is_supplementary() {
            supplementary += 1;
            continue;
        }
        if flags.is_duplicate() {
            duplicates += 1;
        }
        if flags.is_segmented() {
            paired += 1;
        }
        if !flags.is_unmapped() {
            mapped += 1;
            primary_mapped += 1;
        }
        if flags.is_properly_segmented() {
            proper_pair += 1;
        }
    }

    let mapping_rate = if total > 0 {
        mapped as f64 / total as f64
    } else {
        0.0
    };

    let mut result = HashMap::new();
    result.insert("total".to_string(), Value::Int(total));
    result.insert("mapped".to_string(), Value::Int(mapped));
    result.insert("primary_mapped".to_string(), Value::Int(primary_mapped));
    result.insert("paired".to_string(), Value::Int(paired));
    result.insert("proper_pair".to_string(), Value::Int(proper_pair));
    result.insert("duplicates".to_string(), Value::Int(duplicates));
    result.insert("secondary".to_string(), Value::Int(secondary));
    result.insert("supplementary".to_string(), Value::Int(supplementary));
    result.insert("mapping_rate".to_string(), Value::Float(mapping_rate));
    Ok(Value::Record(result))
}

/// coverage_hist(bam_path) or coverage_hist(bam_path, bin_size) -> Table
/// Per-chromosome coverage summary.
fn builtin_coverage_hist(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "coverage_hist")?;
    let (header, mut reader) = open_bam_reader(&path)?;

    let ref_seqs: Vec<(String, i64)> = header
        .reference_sequences()
        .iter()
        .map(|(name, rs)| (name.to_string(), rs.length().get() as i64))
        .collect();

    let mut chrom_bases: HashMap<usize, i64> = HashMap::new();
    let mut chrom_reads: HashMap<usize, i64> = HashMap::new();

    let mut record = noodles_sam::alignment::RecordBuf::default();
    while read_next_bam(&mut reader, &header, &mut record)? {
        let flags = record.flags();
        if flags.is_unmapped() || flags.is_secondary() || flags.is_supplementary() {
            continue;
        }

        if let Some(ref_id) = record.reference_sequence_id() {
            *chrom_reads.entry(ref_id).or_insert(0) += 1;
            let aligned_len = cigar_alignment_span(&record);
            *chrom_bases.entry(ref_id).or_insert(0) += aligned_len as i64;
        }
    }

    let mut rows = Vec::new();
    for (idx, (name, length)) in ref_seqs.iter().enumerate() {
        let bases = chrom_bases.get(&idx).copied().unwrap_or(0);
        let reads = chrom_reads.get(&idx).copied().unwrap_or(0);
        let mean_cov = if *length > 0 {
            bases as f64 / *length as f64
        } else {
            0.0
        };
        rows.push(vec![
            Value::Str(name.clone()),
            Value::Int(*length),
            Value::Int(reads),
            Value::Int(bases),
            Value::Float(mean_cov),
        ]);
    }

    Ok(Value::Table(bl_core::value::Table::new(
        vec![
            "chrom".to_string(),
            "length".to_string(),
            "reads".to_string(),
            "bases".to_string(),
            "mean_coverage".to_string(),
        ],
        rows,
    )))
}

/// gc_bias(bam_path) -> Table
/// Compute GC content vs coverage bias from a BAM file.
/// Groups reads into GC% bins (0-100) and reports normalized coverage.
fn builtin_gc_bias(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "gc_bias")?;
    let (header, mut reader) = open_bam_reader(&path)?;

    let mut bin_counts = vec![0i64; 101];
    let mut total_reads: i64 = 0;

    let mut record = noodles_sam::alignment::RecordBuf::default();
    while read_next_bam(&mut reader, &header, &mut record)? {
        let flags = record.flags();
        if flags.is_unmapped() || flags.is_secondary() || flags.is_supplementary() {
            continue;
        }

        // RecordBuf sequence is a Vec<u8> of ASCII bases
        let seq = record.sequence().as_ref();
        let len = seq.len();
        if len == 0 {
            continue;
        }

        let gc_count = seq
            .iter()
            .filter(|&&b| b == b'G' || b == b'g' || b == b'C' || b == b'c')
            .count();

        let gc_pct = (gc_count as f64 / len as f64 * 100.0).round() as usize;
        let gc_pct = gc_pct.min(100);
        bin_counts[gc_pct] += 1;
        total_reads += 1;
    }

    let expected = if total_reads > 0 {
        total_reads as f64 / 101.0
    } else {
        1.0
    };

    let mut rows = Vec::new();
    for (gc_pct, &count) in bin_counts.iter().enumerate() {
        if count > 0 {
            let normalized = count as f64 / expected;
            rows.push(vec![
                Value::Int(gc_pct as i64),
                Value::Int(count),
                Value::Float(normalized),
            ]);
        }
    }

    Ok(Value::Table(bl_core::value::Table::new(
        vec![
            "gc_pct".to_string(),
            "read_count".to_string(),
            "normalized".to_string(),
        ],
        rows,
    )))
}

/// on_target(bam_path, bed_path) -> Record
/// Calculate on-target rate: fraction of reads overlapping BED regions.
fn builtin_on_target(args: Vec<Value>) -> Result<Value> {
    let bam_path = require_str(&args[0], "on_target")?;
    let bed_path = require_str(&args[1], "on_target")?;

    // Load BED regions
    let bed_content = std::fs::read_to_string(&bed_path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("cannot read '{bed_path}': {e}"), None)
    })?;
    let mut regions: HashMap<String, Vec<(i64, i64)>> = HashMap::new();
    for line in bed_content.lines() {
        if line.starts_with('#') || line.starts_with("track") || line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() >= 3 {
            let chrom = fields[0].to_string();
            let start: i64 = fields[1].parse().unwrap_or(0);
            let end: i64 = fields[2].parse().unwrap_or(0);
            regions.entry(chrom).or_default().push((start, end));
        }
    }
    for intervals in regions.values_mut() {
        intervals.sort_unstable();
    }

    let (header, mut reader) = open_bam_reader(&bam_path)?;
    let ref_names: Vec<String> = header
        .reference_sequences()
        .keys()
        .map(|n| n.to_string())
        .collect();

    let mut total: i64 = 0;
    let mut on_target: i64 = 0;
    let mut off_target: i64 = 0;

    let mut record = noodles_sam::alignment::RecordBuf::default();
    while read_next_bam(&mut reader, &header, &mut record)? {
        let flags = record.flags();
        if flags.is_unmapped() || flags.is_secondary() || flags.is_supplementary() {
            continue;
        }
        total += 1;

        let ref_id = match record.reference_sequence_id() {
            Some(id) => id,
            None => {
                off_target += 1;
                continue;
            }
        };

        let chrom = match ref_names.get(ref_id) {
            Some(name) => name.as_str(),
            None => {
                off_target += 1;
                continue;
            }
        };

        let pos = record
            .alignment_start()
            .map(|p| usize::from(p) as i64 - 1) // 0-based
            .unwrap_or(0);
        let aligned_len = cigar_alignment_span(&record) as i64;
        let end = pos + aligned_len;

        // Binary search for overlapping intervals
        if let Some(intervals) = regions.get(chrom) {
            let idx = intervals.partition_point(|&(_, e)| e <= pos);
            let mut found = false;
            for &(istart, iend) in &intervals[idx..] {
                if istart >= end {
                    break;
                }
                if iend > pos && istart < end {
                    found = true;
                    break;
                }
            }
            if found {
                on_target += 1;
            } else {
                off_target += 1;
            }
        } else {
            off_target += 1;
        }
    }

    let on_target_rate = if total > 0 {
        on_target as f64 / total as f64
    } else {
        0.0
    };

    let mut result = HashMap::new();
    result.insert("total_reads".to_string(), Value::Int(total));
    result.insert("on_target".to_string(), Value::Int(on_target));
    result.insert("off_target".to_string(), Value::Int(off_target));
    result.insert("on_target_rate".to_string(), Value::Float(on_target_rate));
    Ok(Value::Record(result))
}
