use bl_convert::{convert, validate_file, ConvertOptions, Format};
use flate2::read::GzDecoder;
use std::fs;
use std::io::Read;
use tempfile::tempdir;

fn defaults() -> ConvertOptions {
    ConvertOptions::default()
}

#[test]
fn detects_formats_beneath_compression_suffixes() {
    assert_eq!(
        Format::detect("sample.vcf.gz".as_ref()).unwrap(),
        Format::Vcf
    );
    assert_eq!(
        Format::detect("reads.FASTQ.GZ".as_ref()).unwrap(),
        Format::Fastq
    );
}

#[test]
fn csv_to_tsv_preserves_quoted_delimiters_and_newlines() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.csv");
    let output = directory.path().join("output.tsv");
    fs::write(&input, "id,note\n001,\"alpha,beta\"\n002,\"two\nlines\"\n").unwrap();

    let report = convert(&input, &output, &defaults()).unwrap();

    assert_eq!(report.records_written, 2);
    assert!(!report.lossy);
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_path(&output)
        .unwrap();
    let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(&rows[0][0], "001");
    assert_eq!(&rows[0][1], "alpha,beta");
    assert_eq!(&rows[1][1], "two\nlines");
}

#[test]
fn existing_output_is_never_replaced_without_force() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.csv");
    let output = directory.path().join("output.tsv");
    fs::write(&input, "id\n1\n").unwrap();
    fs::write(&output, "keep me").unwrap();

    let error = convert(&input, &output, &defaults()).unwrap_err();

    assert!(error.to_string().contains("--force"));
    assert_eq!(fs::read_to_string(output).unwrap(), "keep me");
}

#[test]
fn dry_run_parses_everything_but_creates_nothing() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.csv");
    let output = directory.path().join("output.json");
    fs::write(&input, "id\n1\n2\n").unwrap();
    let mut options = defaults();
    options.dry_run = true;

    let report = convert(&input, &output, &options).unwrap();

    assert_eq!(report.records_written, 2);
    assert!(report.output_bytes.is_none());
    assert!(!report.output_validated);
    assert!(!output.exists());
}

#[test]
fn vcf_to_bed_uses_pos_ref_length_and_symbolic_end() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.vcf");
    let output = directory.path().join("output.bed");
    fs::write(
        &input,
        "##fileformat=VCFv4.3\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\nchr1\t10\trs1\tAT\tA\t42\tPASS\t.\nchr2\t100\t.\tN\t<DEL>\t.\tPASS\tEND=125;SVTYPE=DEL\n",
    )
    .unwrap();

    let report = convert(&input, &output, &defaults()).unwrap();

    assert!(report.lossy);
    assert_eq!(report.records_written, 2);
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "chr1\t9\t11\trs1\t0\t.\nchr2\t99\t125\tN><DEL>\t0\t.\n"
    );
}

#[test]
fn gff_to_bed_filters_features_and_uses_named_attribute() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.gff3");
    let output = directory.path().join("output.bed");
    fs::write(
        &input,
        "##gff-version 3\nchr1\tsrc\tgene\t5\t20\t.\t+\t.\tID=g1;Name=TP53\nchr1\tsrc\texon\t5\t9\t.\t+\t.\tParent=g1\n",
    )
    .unwrap();
    let mut options = defaults();
    options.feature = Some("gene".into());
    options.name_attribute = Some("Name".into());

    let report = convert(&input, &output, &options).unwrap();

    assert_eq!(report.records_read, 2);
    assert_eq!(report.records_written, 1);
    assert_eq!(report.records_skipped, 1);
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "chr1\t4\t20\tTP53\t0\t+\n"
    );
}

#[test]
fn multiline_fastq_to_fasta_discards_quality_explicitly() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.fastq");
    let output = directory.path().join("output.fasta");
    fs::write(&input, "@read1 description\nACGT\nTG\n+\nIIII\nII\n").unwrap();

    let report = convert(&input, &output, &defaults()).unwrap();

    assert!(report.lossy);
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("quality")));
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        ">read1 description\nACGTTG\n"
    );
}

#[test]
fn invalid_input_leaves_no_partial_output() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("broken.fastq");
    let output = directory.path().join("output.fastq");
    fs::write(&input, "@read1\nACGT\n+\nIII\n").unwrap();

    let error = convert(&input, &output, &defaults()).unwrap_err();

    assert!(error.to_string().contains("quality"));
    assert!(!output.exists());
}

#[test]
fn fastq_rejects_non_phred_quality_symbols() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("broken.fastq");
    let output = directory.path().join("output.fasta");
    fs::write(&input, "@read1\nACGT\n+\nIII \n").unwrap();

    let error = convert(&input, &output, &defaults()).unwrap_err();

    assert!(error.to_string().contains("printable Phred+33"));
    assert!(!output.exists());
}

#[test]
fn invalid_vcf_end_is_not_silently_ignored() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("broken.vcf");
    let output = directory.path().join("output.bed");
    fs::write(
        &input,
        "##fileformat=VCFv4.3\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\nchr1\t10\t.\tA\t<DEL>\t.\tPASS\tEND=unknown\n",
    )
    .unwrap();

    let error = convert(&input, &output, &defaults()).unwrap_err();

    assert!(error.to_string().contains("invalid INFO/END"));
    assert!(!output.exists());
}

#[test]
fn uneven_delimited_rows_are_rejected_without_partial_output() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("broken.csv");
    let output = directory.path().join("output.json");
    fs::write(&input, "id,value\na,1\nb,2,unexpected\n").unwrap();

    let error = convert(&input, &output, &defaults()).unwrap_err();

    assert!(error.to_string().contains("record 2"));
    assert!(!output.exists());
}

#[test]
fn duplicate_delimited_headers_cannot_silently_overwrite_json_keys() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("duplicate.csv");
    let output = directory.path().join("output.json");
    fs::write(&input, "gene,gene\nTP53,BRCA1\n").unwrap();

    let error = convert(&input, &output, &defaults()).unwrap_err();

    assert!(error.to_string().contains("duplicate header 'gene'"));
    assert!(!output.exists());
}

#[test]
fn gzip_output_is_compressed_and_validated() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.csv");
    let output = directory.path().join("output.tsv.gz");
    fs::write(&input, "id,value\na,1\nb,2\n").unwrap();

    let report = convert(&input, &output, &defaults()).unwrap();

    assert!(report.output_validated);
    assert_eq!(validate_file(&output, Format::Tsv).unwrap(), 2);
    let mut text = String::new();
    GzDecoder::new(fs::File::open(output).unwrap())
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(text, "id\tvalue\na\t1\nb\t2\n");
}

#[test]
fn nested_json_to_csv_reports_the_tabular_type_loss() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.json");
    let output = directory.path().join("output.csv");
    fs::write(&input, r#"[{"id":"a","tags":["x","y"]}]"#).unwrap();

    let report = convert(&input, &output, &defaults()).unwrap();

    assert!(report.lossy);
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("Nested")));
    assert_eq!(validate_file(&output, Format::Csv).unwrap(), 1);
}

#[test]
fn force_replaces_an_existing_output_after_successful_validation() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.csv");
    let output = directory.path().join("output.tsv");
    fs::write(&input, "id\nnew\n").unwrap();
    fs::write(&output, "old\n").unwrap();
    let mut options = defaults();
    options.force = true;

    let report = convert(&input, &output, &options).unwrap();

    assert!(report.output_validated);
    assert_eq!(fs::read_to_string(output).unwrap(), "id\nnew\n");
}

/// Sample input for each source format, sized so a conversion produces records
/// rather than an empty file.
fn fixture(format: Format) -> &'static str {
    match format {
        Format::Csv => "id,value\na,1\nb,2\n",
        Format::Tsv => "id\tvalue\na\t1\nb\t2\n",
        Format::Json => "[{\"id\":\"a\",\"value\":\"1\"},{\"id\":\"b\",\"value\":\"2\"}]\n",
        Format::Bed => "track name=demo\nchr1\t5\t10\tfeat\t0\t+\n",
        Format::Vcf => concat!(
            "##fileformat=VCFv4.3\n",
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
            "chr1\t10\trs1\tAT\tA\t42\tPASS\t.\n"
        ),
        Format::Gff => "##gff-version 3\nchr1\tsrc\tgene\t5\t20\t.\t+\t.\tID=g1;Name=TP53\n",
        Format::Gtf => "chr1\tsrc\tgene\t5\t20\t.\t+\t.\tgene_id \"g1\"; gene_name \"TP53\";\n",
        Format::Fasta => ">r1 description\nACGTACGT\n",
        Format::Fastq => "@r1 description\nACGT\n+r1 description\nIIII\n",
    }
}

/// The extension `Format::detect` recognizes, so these fixtures route through
/// the same detection path a user's filenames would.
fn extension(format: Format) -> &'static str {
    match format {
        Format::Csv => "csv",
        Format::Tsv => "tsv",
        Format::Json => "json",
        Format::Bed => "bed",
        Format::Vcf => "vcf",
        Format::Gff => "gff3",
        Format::Gtf => "gtf",
        Format::Fasta => "fasta",
        Format::Fastq => "fastq",
    }
}

/// `bl-convert formats` prints `supported_pairs()` as the tool's public
/// contract, including a LOSSY column. Nothing previously connected either
/// half of that table to what a conversion does, so a pair could be advertised
/// without being implemented, and a pair could be advertised as lossless while
/// dropping a field. The second is what happened: FASTQ -> FASTQ discarded the
/// description on the `+` line and still reported `"lossy": false`.
#[test]
fn every_advertised_pair_is_implemented_and_reports_the_loss_it_declares() {
    for (from, to, declared_lossy) in bl_convert::supported_pairs() {
        let directory = tempdir().unwrap();
        let input = directory.path().join(format!("input.{}", extension(from)));
        let output = directory.path().join(format!("output.{}", extension(to)));
        fs::write(&input, fixture(from)).unwrap();

        let report = convert(&input, &output, &defaults()).unwrap_or_else(|error| {
            panic!("{from} -> {to} is advertised by `bl-convert formats` but failed: {error}")
        });

        assert_eq!(
            report.lossy, declared_lossy,
            "`bl-convert formats` advertises {from} -> {to} as lossy={declared_lossy}, \
             but the conversion reported lossy={}",
            report.lossy
        );
        assert!(
            report.records_written > 0,
            "{from} -> {to} produced no records from a non-empty fixture"
        );
        if report.lossy {
            assert!(
                !report.warnings.is_empty(),
                "{from} -> {to} is lossy but names nothing that was lost"
            );
        }
    }
}

/// `track1` is a legal contig name, and BED columns are tab-separated, so only
/// a following space marks a real header line. Matching the bare prefix let a
/// broken interval past every coordinate check.
#[test]
fn a_broken_interval_on_a_track_prefixed_contig_is_rejected() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("scaffolds.bed");
    let output = directory.path().join("out.bed");
    fs::write(&input, "chr1\t5\t10\ntrack1\tBOGUS\tNOTANUMBER\tjunk\n").unwrap();

    let error = convert(&input, &output, &defaults()).unwrap_err();

    assert!(
        error.to_string().contains("invalid start"),
        "expected a coordinate error, got: {error}"
    );
    assert!(!output.exists());
}

#[test]
fn real_track_and_browser_headers_still_pass_through_beside_track_named_contigs() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("scaffolds.bed");
    let output = directory.path().join("out.bed");
    fs::write(
        &input,
        "track name=demo\nbrowser position chr1:1-100\ntrack1\t5\t10\ntrackless\t7\t9\n",
    )
    .unwrap();

    let report = convert(&input, &output, &defaults()).unwrap();

    assert_eq!(
        report.records_written, 2,
        "both track-prefixed contigs are records, not headers"
    );
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "track name=demo\nbrowser position chr1:1-100\ntrack1\t5\t10\ntrackless\t7\t9\n"
    );
}

/// FASTQ -> FASTQ is advertised as lossless, so it has to be.
#[test]
fn fastq_normalization_keeps_the_plus_line_description() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("reads.fastq");
    let output = directory.path().join("out.fastq");
    // Folded across lines so the record still has to be reassembled; the `+`
    // description is the part that used to vanish.
    fs::write(&input, "@r1 desc\nAC\nGT\n+r1 desc\nII\nII\n").unwrap();

    let report = convert(&input, &output, &defaults()).unwrap();

    assert!(!report.lossy);
    assert!(report.warnings.is_empty());
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "@r1 desc\nACGT\n+r1 desc\nIIII\n"
    );
}

/// A bare `+` must stay bare rather than gaining the header back.
#[test]
fn fastq_normalization_leaves_a_bare_plus_line_bare() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("reads.fastq");
    let output = directory.path().join("out.fastq");
    fs::write(&input, "@r1 desc\nACGT\n+\nIIII\n").unwrap();

    convert(&input, &output, &defaults()).unwrap();

    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "@r1 desc\nACGT\n+\nIIII\n"
    );
}
