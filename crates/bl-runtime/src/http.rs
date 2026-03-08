use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;
use std::io::Write;
use std::sync::OnceLock;

/// Shared proxy-aware HTTP agent for all bl-runtime HTTP operations.
/// Reads ALL_PROXY, HTTPS_PROXY, HTTP_PROXY env vars (in priority order).
/// Supports authenticated proxies: http://user:pass@host:port
pub(crate) fn shared_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let mut builder = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("biolang/0.1");
        if let Some(proxy) = proxy_from_env() {
            builder = builder.proxy(proxy);
        }
        builder.build()
    })
}

/// Read proxy settings from environment variables (ureq 2.x doesn't have try_from_env).
/// Checks ALL_PROXY, all_proxy, HTTPS_PROXY, https_proxy, HTTP_PROXY, http_proxy in order.
fn proxy_from_env() -> Option<ureq::Proxy> {
    for var in &[
        "ALL_PROXY", "all_proxy",
        "HTTPS_PROXY", "https_proxy",
        "HTTP_PROXY", "http_proxy",
    ] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                if let Ok(proxy) = ureq::Proxy::new(&val) {
                    return Some(proxy);
                }
            }
        }
    }
    None
}

/// Returns the list of (name, arity) for all HTTP builtins.
pub fn http_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("http_get", Arity::Range(1, 2)),
        ("http_post", Arity::Range(2, 3)),
        ("download", Arity::Range(1, 2)),
        ("upload", Arity::Range(2, 3)),
        ("ref_genome", Arity::Range(1, 2)),
        ("bio_fetch", Arity::Range(1, 2)),
        ("bio_sources", Arity::Range(0, 1)),
    ]
}

/// Check if a name is a known HTTP builtin.
pub fn is_http_builtin(name: &str) -> bool {
    matches!(
        name,
        "http_get" | "http_post" | "download" | "upload" | "ref_genome" | "bio_fetch" | "bio_sources"
    )
}

/// Execute an HTTP builtin by name.
pub fn call_http_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "http_get" => builtin_http_get(args),
        "http_post" => builtin_http_post(args),
        "download" => builtin_download(args),
        "upload" => builtin_upload(args),
        "ref_genome" => builtin_ref_genome(args),
        "bio_fetch" => builtin_bio_fetch(args),
        "bio_sources" => builtin_bio_sources(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown http builtin '{name}'"),
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

fn apply_headers(
    mut req: ureq::Request,
    headers: &HashMap<String, Value>,
) -> Result<ureq::Request> {
    for (key, val) in headers {
        match val {
            Value::Str(v) => {
                req = req.set(key, v);
            }
            other => {
                req = req.set(key, &format!("{other}"));
            }
        }
    }
    Ok(req)
}

fn extract_headers(val: &Value, func: &str) -> Result<HashMap<String, Value>> {
    match val {
        Value::Record(m) | Value::Map(m) => Ok(m.clone()),
        other => Err(BioLangError::type_error(
            format!("{func}() headers must be Record, got {}", other.type_of()),
            None,
        )),
    }
}

fn response_to_value(resp: ureq::Response) -> Result<Value> {
    let status = resp.status();
    let mut resp_headers = HashMap::new();
    for name in resp.headers_names() {
        if let Some(val) = resp.header(&name) {
            resp_headers.insert(name, Value::Str(val.to_string()));
        }
    }
    let body = resp.into_string().map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("failed to read response body: {e}"), None)
    })?;

    let mut rec = HashMap::new();
    rec.insert("status".to_string(), Value::Int(status as i64));
    rec.insert("body".to_string(), Value::Str(body));
    rec.insert("headers".to_string(), Value::Record(resp_headers));
    Ok(Value::Record(rec))
}

fn builtin_http_get(args: Vec<Value>) -> Result<Value> {
    let url = require_str(&args[0], "http_get")?;
    let mut req = shared_agent().get(url);

    if args.len() > 1 {
        let headers = extract_headers(&args[1], "http_get")?;
        req = apply_headers(req, &headers)?;
    }

    let resp = req.call().map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("http_get() failed: {e}"), None)
    })?;

    response_to_value(resp)
}

fn builtin_http_post(args: Vec<Value>) -> Result<Value> {
    let url = require_str(&args[0], "http_post")?;
    let body = require_str(&args[1], "http_post")?;
    let mut req = shared_agent().post(url);

    if args.len() > 2 {
        let headers = extract_headers(&args[2], "http_post")?;
        req = apply_headers(req, &headers)?;
    }

    let resp = req.send_string(body).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("http_post() failed: {e}"), None)
    })?;

    response_to_value(resp)
}

fn builtin_download(args: Vec<Value>) -> Result<Value> {
    let url = require_str(&args[0], "download")?;
    // If no path given, save to ~/.biolang/downloads/<filename>
    let path_owned: String;
    let path = if args.len() > 1 {
        require_str(&args[1], "download")?
    } else {
        let url_path = url.split('?').next().unwrap_or(url);
        let filename = url_path.rsplit('/').next().unwrap_or("download");
        let dir = default_data_dir("downloads");
        path_owned = format!("{dir}/{filename}");
        &path_owned
    };

    download_with_progress(url, path, "download")
}

/// Default data directory under ~/.biolang/<subdir>/
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

fn download_with_progress(url: &str, path: &str, func: &str) -> Result<Value> {
    let resp = shared_agent().get(url).call().map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("{func}() failed: {e}"), None)
    })?;

    // Get content-length for progress
    let total: Option<u64> = resp
        .header("content-length")
        .and_then(|s| s.parse().ok());

    let mut file = std::fs::File::create(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("{func}() cannot create file: {e}"),
            None,
        )
    })?;

    let mut reader = resp.into_reader();
    let mut buf = [0u8; 65536];
    let mut downloaded: u64 = 0;
    let mut last_print: u64 = 0;

    loop {
        let n = std::io::Read::read(&mut reader, &mut buf).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("{func}() read error: {e}"), None)
        })?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("{func}() write error: {e}"), None)
        })?;
        downloaded += n as u64;

        // Print progress every 1MB
        if downloaded - last_print >= 1_048_576 {
            last_print = downloaded;
            if let Some(total) = total {
                let pct = (downloaded as f64 / total as f64 * 100.0) as u32;
                let mb_done = downloaded as f64 / 1_048_576.0;
                let mb_total = total as f64 / 1_048_576.0;
                eprint!("\r  {func}: {mb_done:.1} / {mb_total:.1} MB ({pct}%)  ");
            } else {
                let mb = downloaded as f64 / 1_048_576.0;
                eprint!("\r  {func}: {mb:.1} MB downloaded  ");
            }
        }
    }

    // Clear progress line
    if last_print > 0 {
        let size = format_bytes(downloaded);
        eprintln!("\r  {func}: {path} ({size})                    ");
    }

    let mut rec = HashMap::new();
    rec.insert("path".to_string(), Value::Str(path.to_string()));
    rec.insert("size".to_string(), Value::Int(downloaded as i64));
    rec.insert("url".to_string(), Value::Str(url.to_string()));
    Ok(Value::Record(rec))
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

fn builtin_upload(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "upload")?;
    let url = require_str(&args[1], "upload")?;

    let content = std::fs::read(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("upload() cannot read file: {e}"),
            None,
        )
    })?;
    let size = content.len();

    let mut req = shared_agent().put(url);
    if args.len() > 2 {
        let headers = extract_headers(&args[2], "upload")?;
        req = apply_headers(req, &headers)?;
    }

    let resp = req
        .set("content-type", "application/octet-stream")
        .send_bytes(&content)
        .map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("upload() failed: {e}"), None)
        })?;

    let status = resp.status();
    let body = resp.into_string().unwrap_or_default();

    let mut rec = HashMap::new();
    rec.insert("status".to_string(), Value::Int(status as i64));
    rec.insert("size".to_string(), Value::Int(size as i64));
    rec.insert("body".to_string(), Value::Str(body));
    Ok(Value::Record(rec))
}

// ── Reference Genome Downloads ──────────────────────────────────

const REF_GENOMES: &[(&str, &str, &str)] = &[
    // (name, url, description)
    ("hg38", "https://hgdownload.soe.ucsc.edu/goldenPath/hg38/bigZips/hg38.fa.gz",
     "Human GRCh38/hg38"),
    ("grch38", "https://hgdownload.soe.ucsc.edu/goldenPath/hg38/bigZips/hg38.fa.gz",
     "Human GRCh38/hg38"),
    ("hg19", "https://hgdownload.soe.ucsc.edu/goldenPath/hg19/bigZips/hg19.fa.gz",
     "Human GRCh37/hg19"),
    ("grch37", "https://hgdownload.soe.ucsc.edu/goldenPath/hg19/bigZips/hg19.fa.gz",
     "Human GRCh37/hg19"),
    ("t2t", "https://s3-us-west-2.amazonaws.com/human-pangenomics/T2T/CHM13/assemblies/analysis_set/chm13v2.0.fa.gz",
     "T2T-CHM13 v2.0"),
    ("chm13", "https://s3-us-west-2.amazonaws.com/human-pangenomics/T2T/CHM13/assemblies/analysis_set/chm13v2.0.fa.gz",
     "T2T-CHM13 v2.0"),
    ("mm39", "https://hgdownload.soe.ucsc.edu/goldenPath/mm39/bigZips/mm39.fa.gz",
     "Mouse GRCm39/mm39"),
    ("mm10", "https://hgdownload.soe.ucsc.edu/goldenPath/mm10/bigZips/mm10.fa.gz",
     "Mouse GRCm38/mm10"),
    ("dm6", "https://hgdownload.soe.ucsc.edu/goldenPath/dm6/bigZips/dm6.fa.gz",
     "Drosophila dm6"),
    ("ce11", "https://hgdownload.soe.ucsc.edu/goldenPath/ce11/bigZips/ce11.fa.gz",
     "C. elegans ce11"),
    ("danrer11", "https://hgdownload.soe.ucsc.edu/goldenPath/danRer11/bigZips/danRer11.fa.gz",
     "Zebrafish GRCz11"),
    ("saccer3", "https://hgdownload.soe.ucsc.edu/goldenPath/sacCer3/bigZips/sacCer3.fa.gz",
     "S. cerevisiae sacCer3"),
];

fn builtin_ref_genome(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "ref_genome")?;
    let name_lower = name.to_lowercase();

    // If name is "list", show available genomes
    if name_lower == "list" {
        let items: Vec<Value> = REF_GENOMES
            .iter()
            .map(|(name, _url, desc)| {
                let mut rec = HashMap::new();
                rec.insert("name".to_string(), Value::Str(name.to_string()));
                rec.insert("description".to_string(), Value::Str(desc.to_string()));
                Value::Record(rec)
            })
            .collect();
        // Deduplicate by name
        let mut seen = std::collections::HashSet::new();
        let items: Vec<Value> = items
            .into_iter()
            .filter(|v| {
                if let Value::Record(r) = v {
                    if let Some(Value::Str(n)) = r.get("name") {
                        return seen.insert(n.clone());
                    }
                }
                true
            })
            .collect();
        return Ok(Value::List(items));
    }

    // Find the genome
    let entry = REF_GENOMES
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(&name_lower));

    let (_, url, desc) = match entry {
        Some(e) => e,
        None => {
            let available: Vec<&str> = REF_GENOMES.iter().map(|(n, _, _)| *n).collect();
            return Err(BioLangError::runtime(
                ErrorKind::NameError,
                format!(
                    "unknown reference genome '{}'. Available: {}. Use ref_genome(\"list\") to see all.",
                    name,
                    available.join(", ")
                ),
                None,
            ));
        }
    };

    // Determine output path
    let dest = if args.len() > 1 {
        require_str(&args[1], "ref_genome")?.to_string()
    } else {
        let dir = default_data_dir("genomes");
        format!("{dir}/{name_lower}.fa.gz")
    };

    // Check if already downloaded
    if std::path::Path::new(&dest).exists() {
        let meta = std::fs::metadata(&dest).ok();
        let size = meta.map(|m| m.len()).unwrap_or(0);
        if size > 0 {
            eprintln!("  ref_genome: {name_lower} already exists at {dest} ({})", format_bytes(size));
            let mut rec = HashMap::new();
            rec.insert("path".to_string(), Value::Str(dest));
            rec.insert("name".to_string(), Value::Str(name_lower));
            rec.insert("description".to_string(), Value::Str(desc.to_string()));
            rec.insert("cached".to_string(), Value::Bool(true));
            return Ok(Value::Record(rec));
        }
    }

    eprintln!("  ref_genome: downloading {desc} → {dest}");
    let result = download_with_progress(url, &dest, "ref_genome")?;

    // Enrich with genome info
    if let Value::Record(mut rec) = result {
        rec.insert("name".to_string(), Value::Str(name_lower));
        rec.insert("description".to_string(), Value::Str(desc.to_string()));
        rec.insert("cached".to_string(), Value::Bool(false));
        Ok(Value::Record(rec))
    } else {
        Ok(result)
    }
}

// ── bio_fetch / bio_sources ─────────────────────────────────────

/// Curated registry of common bioinformatics data sources.
/// (shortcut_name, url, category, description, filename_override)
const BIO_SOURCES: &[(&str, &str, &str, &str, &str)] = &[
    // ── Reference Genomes ───────────────────────────────────────
    ("hg38",       "https://hgdownload.soe.ucsc.edu/goldenPath/hg38/bigZips/hg38.fa.gz",
     "genome", "Human GRCh38/hg38 reference", "hg38.fa.gz"),
    ("hg19",       "https://hgdownload.soe.ucsc.edu/goldenPath/hg19/bigZips/hg19.fa.gz",
     "genome", "Human GRCh37/hg19 reference", "hg19.fa.gz"),
    ("t2t",        "https://s3-us-west-2.amazonaws.com/human-pangenomics/T2T/CHM13/assemblies/analysis_set/chm13v2.0.fa.gz",
     "genome", "T2T-CHM13 v2.0 telomere-to-telomere", "chm13v2.0.fa.gz"),
    ("mm39",       "https://hgdownload.soe.ucsc.edu/goldenPath/mm39/bigZips/mm39.fa.gz",
     "genome", "Mouse GRCm39/mm39 reference", "mm39.fa.gz"),
    ("mm10",       "https://hgdownload.soe.ucsc.edu/goldenPath/mm10/bigZips/mm10.fa.gz",
     "genome", "Mouse GRCm38/mm10 reference", "mm10.fa.gz"),
    ("dm6",        "https://hgdownload.soe.ucsc.edu/goldenPath/dm6/bigZips/dm6.fa.gz",
     "genome", "Drosophila melanogaster dm6", "dm6.fa.gz"),
    ("ce11",       "https://hgdownload.soe.ucsc.edu/goldenPath/ce11/bigZips/ce11.fa.gz",
     "genome", "C. elegans ce11", "ce11.fa.gz"),
    ("danrer11",   "https://hgdownload.soe.ucsc.edu/goldenPath/danRer11/bigZips/danRer11.fa.gz",
     "genome", "Zebrafish GRCz11", "danRer11.fa.gz"),
    ("saccer3",    "https://hgdownload.soe.ucsc.edu/goldenPath/sacCer3/bigZips/sacCer3.fa.gz",
     "genome", "S. cerevisiae sacCer3", "sacCer3.fa.gz"),
    ("e_coli",     "https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/000/005/845/GCF_000005845.2_ASM584v2/GCF_000005845.2_ASM584v2_genomic.fna.gz",
     "genome", "E. coli K-12 MG1655", "e_coli_k12.fa.gz"),

    // ── Gene Annotations ────────────────────────────────────────
    ("gencode_human", "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/release_46/gencode.v46.annotation.gtf.gz",
     "annotation", "GENCODE v46 human gene annotation (GTF)", "gencode.v46.gtf.gz"),
    ("gencode_mouse", "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_mouse/release_M35/gencode.vM35.annotation.gtf.gz",
     "annotation", "GENCODE vM35 mouse gene annotation (GTF)", "gencode.vM35.gtf.gz"),
    ("refseq_hg38", "https://ftp.ncbi.nlm.nih.gov/genomes/refseq/vertebrate_mammalian/Homo_sapiens/annotation_releases/GCF_000001405.40-RS_2024_08/GCF_000001405.40_GRCh38.p14_genomic.gff.gz",
     "annotation", "NCBI RefSeq GRCh38 annotation (GFF3)", "refseq_hg38.gff.gz"),

    // ── Transcriptomes (for Salmon/Kallisto) ────────────────────
    ("gencode_transcripts", "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/release_46/gencode.v46.transcripts.fa.gz",
     "transcriptome", "GENCODE v46 human transcripts (cDNA)", "gencode.v46.transcripts.fa.gz"),
    ("gencode_mouse_transcripts", "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_mouse/release_M35/gencode.vM35.transcripts.fa.gz",
     "transcriptome", "GENCODE vM35 mouse transcripts (cDNA)", "gencode.vM35.transcripts.fa.gz"),

    // ── Variant Databases ───────────────────────────────────────
    ("dbsnp",      "https://ftp.ncbi.nih.gov/snp/latest_release/VCF/GCF_000001405.40.gz",
     "variants", "dbSNP latest (GRCh38) VCF", "dbsnp_latest.vcf.gz"),
    ("dbsnp_index", "https://ftp.ncbi.nih.gov/snp/latest_release/VCF/GCF_000001405.40.gz.tbi",
     "variants", "dbSNP latest tabix index", "dbsnp_latest.vcf.gz.tbi"),
    ("clinvar",    "https://ftp.ncbi.nlm.nih.gov/pub/clinvar/vcf_GRCh38/clinvar.vcf.gz",
     "variants", "ClinVar GRCh38 VCF (latest)", "clinvar_grch38.vcf.gz"),
    ("clinvar_index", "https://ftp.ncbi.nlm.nih.gov/pub/clinvar/vcf_GRCh38/clinvar.vcf.gz.tbi",
     "variants", "ClinVar GRCh38 tabix index", "clinvar_grch38.vcf.gz.tbi"),
    ("gnomad_sites", "https://storage.googleapis.com/gcp-public-data--gnomad/release/4.1/vcf/exomes/gnomad.exomes.v4.1.sites.chr22.vcf.bgz",
     "variants", "gnomAD v4.1 exome sites chr22 (example)", "gnomad_exomes_chr22.vcf.bgz"),
    ("cosmic_coding", "https://cancer.sanger.ac.uk/cosmic/file_download/GRCh38/cosmic/v99/VCF/CosmicCodingMuts.vcf.gz",
     "variants", "COSMIC v99 coding mutations (requires key)", "cosmic_coding.vcf.gz"),

    // ── Blacklists & Regions ────────────────────────────────────
    ("blacklist_hg38", "https://github.com/Boyle-Lab/Blacklist/raw/master/lists/hg38-blacklist.v2.bed.gz",
     "regions", "ENCODE blacklist v2 (hg38)", "hg38_blacklist_v2.bed.gz"),
    ("blacklist_hg19", "https://github.com/Boyle-Lab/Blacklist/raw/master/lists/hg19-blacklist.v2.bed.gz",
     "regions", "ENCODE blacklist v2 (hg19)", "hg19_blacklist_v2.bed.gz"),
    ("blacklist_mm10", "https://github.com/Boyle-Lab/Blacklist/raw/master/lists/mm10-blacklist.v2.bed.gz",
     "regions", "ENCODE blacklist v2 (mm10)", "mm10_blacklist_v2.bed.gz"),
    ("centromeres_hg38", "https://hgdownload.soe.ucsc.edu/goldenPath/hg38/database/centromeres.txt.gz",
     "regions", "Centromere locations (hg38)", "hg38_centromeres.txt.gz"),

    // ── Adapter Sequences ───────────────────────────────────────
    ("adapters_illumina", "https://raw.githubusercontent.com/timflutre/trimmomatic/master/adapters/TruSeq3-PE-2.fa",
     "adapters", "Illumina TruSeq3 PE adapters (FASTA)", "TruSeq3-PE-2.fa"),
    ("adapters_nextera", "https://raw.githubusercontent.com/timflutre/trimmomatic/master/adapters/NexteraPE-PE.fa",
     "adapters", "Nextera PE adapters (FASTA)", "NexteraPE-PE.fa"),
    ("adapters_truseq", "https://raw.githubusercontent.com/timflutre/trimmomatic/master/adapters/TruSeq3-SE.fa",
     "adapters", "Illumina TruSeq3 SE adapters (FASTA)", "TruSeq3-SE.fa"),

    // ── Pre-built Indexes ───────────────────────────────────────
    ("bwa_hg38",   "https://genome-idx.s3.amazonaws.com/bt2/GRCh38_noalt_as.zip",
     "index", "BWA/Bowtie2 hg38 no-alt index (zipped)", "GRCh38_noalt_as.zip"),
    ("hisat2_hg38", "https://genome-idx.s3.amazonaws.com/hisat/grch38_snptran.tar.gz",
     "index", "HISAT2 GRCh38 + SNP + transcripts index", "hisat2_grch38_snptran.tar.gz"),

    // ── Test / Example Data ─────────────────────────────────────
    ("test_fastq",  "https://raw.githubusercontent.com/BioJulia/BioSequences.jl/master/test/data/seqs.fastq",
     "test", "Small test FASTQ (< 1KB)", "test.fastq"),
    ("test_bam",    "https://raw.githubusercontent.com/samtools/samtools/develop/examples/ex1.bam",
     "test", "Small test BAM from samtools (ex1)", "test_ex1.bam"),
    ("test_vcf",    "https://raw.githubusercontent.com/samtools/htslib/develop/test/tabix/vcf_file.vcf",
     "test", "Small test VCF from htslib", "test.vcf"),

    // ── Phylogenetics / Taxonomy ────────────────────────────────
    ("taxdump",     "https://ftp.ncbi.nlm.nih.gov/pub/taxonomy/taxdump.tar.gz",
     "taxonomy", "NCBI taxonomy dump (names, nodes, etc.)", "taxdump.tar.gz"),

    // ── Protein ─────────────────────────────────────────────────
    ("uniprot_human", "https://ftp.uniprot.org/pub/databases/uniprot/current_release/knowledgebase/reference_proteomes/Eukaryota/UP000005640/UP000005640_9606.fasta.gz",
     "protein", "UniProt human reference proteome", "uniprot_human.fasta.gz"),
    ("pfam",        "https://ftp.ebi.ac.uk/pub/databases/Pfam/current_release/Pfam-A.hmm.gz",
     "protein", "Pfam-A HMM profiles (latest)", "Pfam-A.hmm.gz"),
];

/// bio_fetch(name, path?) → {path, name, description, cached}
fn builtin_bio_fetch(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "bio_fetch")?;
    let name_lower = name.to_lowercase().replace('-', "_");

    // Special: "list" shows all sources
    if name_lower == "list" {
        return builtin_bio_sources(vec![]);
    }

    // Find in registry
    let entry = BIO_SOURCES
        .iter()
        .find(|(n, _, _, _, _)| n.eq_ignore_ascii_case(&name_lower));

    let (_, url, category, desc, default_filename) = match entry {
        Some(e) => e,
        None => {
            // Suggest close matches
            let mut suggestions: Vec<&str> = BIO_SOURCES
                .iter()
                .filter(|(n, _, _, _, _)| {
                    n.contains(&name_lower) || name_lower.contains(*n)
                })
                .map(|(n, _, _, _, _)| *n)
                .collect();
            suggestions.truncate(5);

            let hint = if suggestions.is_empty() {
                "Use bio_fetch(\"list\") or bio_sources() to see all available.".to_string()
            } else {
                format!("Did you mean: {}?", suggestions.join(", "))
            };

            return Err(BioLangError::runtime(
                ErrorKind::NameError,
                format!("unknown data source '{name}'. {hint}"),
                None,
            ));
        }
    };

    // Determine output path
    let dest = if args.len() > 1 {
        require_str(&args[1], "bio_fetch")?.to_string()
    } else {
        let dir = default_data_dir(category);
        format!("{dir}/{default_filename}")
    };

    // Check cache
    if std::path::Path::new(&dest).exists() {
        let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        if size > 0 {
            eprintln!(
                "  bio_fetch: {} already cached at {} ({})",
                name_lower,
                dest,
                format_bytes(size)
            );
            let mut rec = HashMap::new();
            rec.insert("path".to_string(), Value::Str(dest));
            rec.insert("name".to_string(), Value::Str(name_lower));
            rec.insert("category".to_string(), Value::Str(category.to_string()));
            rec.insert("description".to_string(), Value::Str(desc.to_string()));
            rec.insert("cached".to_string(), Value::Bool(true));
            return Ok(Value::Record(rec));
        }
    }

    eprintln!("  bio_fetch: {desc}");
    let result = download_with_progress(url, &dest, "bio_fetch")?;

    if let Value::Record(mut rec) = result {
        rec.insert("name".to_string(), Value::Str(name_lower));
        rec.insert("category".to_string(), Value::Str(category.to_string()));
        rec.insert("description".to_string(), Value::Str(desc.to_string()));
        rec.insert("cached".to_string(), Value::Bool(false));
        Ok(Value::Record(rec))
    } else {
        Ok(result)
    }
}

/// bio_sources(category?) → Table of all available data sources
fn builtin_bio_sources(args: Vec<Value>) -> Result<Value> {
    let filter = if !args.is_empty() {
        match &args[0] {
            Value::Str(s) => Some(s.to_lowercase()),
            _ => None,
        }
    } else {
        None
    };

    use bl_core::value::Table;

    let columns = vec![
        "name".to_string(),
        "category".to_string(),
        "description".to_string(),
    ];

    let rows: Vec<Vec<Value>> = BIO_SOURCES
        .iter()
        .filter(|(_, _, cat, _, _)| {
            filter
                .as_ref()
                .map(|f| cat.contains(f.as_str()))
                .unwrap_or(true)
        })
        .map(|(name, _, cat, desc, _)| {
            vec![
                Value::Str(name.to_string()),
                Value::Str(cat.to_string()),
                Value::Str(desc.to_string()),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(columns, rows)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_http_builtin() {
        assert!(is_http_builtin("http_get"));
        assert!(is_http_builtin("http_post"));
        assert!(is_http_builtin("download"));
        assert!(!is_http_builtin("http_put"));
    }

    #[test]
    fn test_http_get_bad_url() {
        let result = call_http_builtin("http_get", vec![Value::Str("not-a-url".into())]);
        assert!(result.is_err());
    }

    #[test]
    fn test_http_post_bad_url() {
        let result = call_http_builtin(
            "http_post",
            vec![Value::Str("not-a-url".into()), Value::Str("body".into())],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_download_bad_url() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt").to_string_lossy().to_string();
        let result = call_http_builtin(
            "download",
            vec![Value::Str("not-a-url".into()), Value::Str(path)],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_headers() {
        let mut map = HashMap::new();
        map.insert("Content-Type".to_string(), Value::Str("application/json".into()));
        let headers = extract_headers(&Value::Record(map), "test").unwrap();
        assert_eq!(
            headers.get("Content-Type"),
            Some(&Value::Str("application/json".into()))
        );
    }

    #[test]
    fn test_extract_headers_wrong_type() {
        assert!(extract_headers(&Value::Int(42), "test").is_err());
    }
}
