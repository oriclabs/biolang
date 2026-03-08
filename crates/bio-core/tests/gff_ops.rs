use bio_core::gff_ops::*;
use std::collections::HashMap;

#[test]
fn test_detect_format_gff3() {
    assert_eq!(detect_format("ID=gene1;Name=BRCA1"), GffFormat::Gff3);
}

#[test]
fn test_detect_format_gtf() {
    assert_eq!(
        detect_format("gene_id \"ENSG00000012048\"; gene_name \"BRCA1\""),
        GffFormat::Gtf
    );
}

#[test]
fn test_url_decode() {
    assert_eq!(url_decode("hello%20world"), "hello world");
    assert_eq!(url_decode("foo+bar"), "foo bar");
    assert_eq!(url_decode("no%encoding"), "no%encoding"); // invalid hex
    assert_eq!(url_decode("plain"), "plain");
}

#[test]
fn test_parse_gff3_attributes() {
    let attrs = parse_gff3_attributes("ID=gene1;Name=BRCA1;Dbxref=GeneID%3A672");
    assert_eq!(attrs.get("ID").unwrap(), "gene1");
    assert_eq!(attrs.get("Name").unwrap(), "BRCA1");
    assert_eq!(attrs.get("Dbxref").unwrap(), "GeneID:672");
}

#[test]
fn test_parse_gff3_attributes_empty() {
    let attrs = parse_gff3_attributes("");
    assert!(attrs.is_empty());
}

#[test]
fn test_parse_gtf_attributes() {
    let attrs =
        parse_gtf_attributes("gene_id \"ENSG00000012048\"; gene_name \"BRCA1\";");
    assert_eq!(attrs.get("gene_id").unwrap(), "ENSG00000012048");
    assert_eq!(attrs.get("gene_name").unwrap(), "BRCA1");
}

#[test]
fn test_parse_gff_line_gff3() {
    let line = "chr1\t.\tgene\t1000\t9000\t.\t+\t.\tID=gene1;Name=BRCA1";
    let rec = parse_gff_line(line, GffFormat::Gff3).unwrap();
    assert_eq!(rec.seqid, "chr1");
    assert_eq!(rec.source, ".");
    assert_eq!(rec.feature_type, "gene");
    assert_eq!(rec.start, 1000);
    assert_eq!(rec.end, 9000);
    assert_eq!(rec.score, None);
    assert_eq!(rec.strand, '+');
    assert_eq!(rec.phase, None);
    assert_eq!(rec.attributes.get("ID").unwrap(), "gene1");
    assert_eq!(rec.attributes.get("Name").unwrap(), "BRCA1");
}

#[test]
fn test_parse_gff_line_gtf() {
    let line = "chr1\tensembl\texon\t1000\t1500\t.\t+\t.\tgene_id \"ENSG001\"; transcript_id \"ENST001\"";
    let rec = parse_gff_line(line, GffFormat::Gtf).unwrap();
    assert_eq!(rec.feature_type, "exon");
    assert_eq!(rec.start, 1000);
    assert_eq!(rec.end, 1500);
    assert_eq!(rec.attributes.get("gene_id").unwrap(), "ENSG001");
    assert_eq!(rec.attributes.get("transcript_id").unwrap(), "ENST001");
}

#[test]
fn test_parse_gff_line_with_score_and_phase() {
    let line = "chr1\t.\tCDS\t1200\t1500\t100.5\t+\t0\tID=cds1";
    let rec = parse_gff_line(line, GffFormat::Gff3).unwrap();
    assert_eq!(rec.score, Some(100.5));
    assert_eq!(rec.phase, Some(0));
}

#[test]
fn test_parse_gff_line_too_few_columns() {
    let line = "chr1\t.\tgene\t1000";
    assert!(parse_gff_line(line, GffFormat::Gff3).is_err());
}

#[test]
fn test_format_gff_line_gff3() {
    let rec = GffRecord {
        seqid: "chr1".into(),
        source: ".".into(),
        feature_type: "gene".into(),
        start: 1000,
        end: 9000,
        score: None,
        strand: '+',
        phase: None,
        attributes: {
            let mut m = HashMap::new();
            m.insert("ID".into(), "gene1".into());
            m
        },
    };
    let line = format_gff_line(&rec, GffFormat::Gff3);
    assert!(line.starts_with("chr1\t.\tgene\t1000\t9000\t.\t+\t.\t"));
    assert!(line.contains("ID=gene1"));
}

#[test]
fn test_format_gff_line_gtf() {
    let rec = GffRecord {
        seqid: "chr1".into(),
        source: "ensembl".into(),
        feature_type: "exon".into(),
        start: 100,
        end: 200,
        score: Some(50.0),
        strand: '-',
        phase: None,
        attributes: {
            let mut m = HashMap::new();
            m.insert("gene_id".into(), "ENSG001".into());
            m
        },
    };
    let line = format_gff_line(&rec, GffFormat::Gtf);
    assert!(line.starts_with("chr1\tensembl\texon\t100\t200\t50\t-\t.\t"));
    assert!(line.contains("gene_id \"ENSG001\""));
}

#[test]
fn test_parse_region() {
    let (chr, start, end) = parse_region("chr1:1000-2000").unwrap();
    assert_eq!(chr, "chr1");
    assert_eq!(start, 1000);
    assert_eq!(end, 2000);
}

#[test]
fn test_parse_region_with_commas() {
    let (chr, start, end) = parse_region("chr1:1,000-2,000").unwrap();
    assert_eq!(chr, "chr1");
    assert_eq!(start, 1000);
    assert_eq!(end, 2000);
}

#[test]
fn test_parse_region_invalid() {
    assert!(parse_region("chr1").is_err());
    assert!(parse_region("chr1:1000").is_err());
    assert!(parse_region("chr1:abc-def").is_err());
}

#[test]
fn test_roundtrip_gff3() {
    let line = "chr1\t.\tgene\t1000\t9000\t.\t+\t.\tID=gene1";
    let rec = parse_gff_line(line, GffFormat::Gff3).unwrap();
    let formatted = format_gff_line(&rec, GffFormat::Gff3);
    let rec2 = parse_gff_line(&formatted, GffFormat::Gff3).unwrap();
    assert_eq!(rec.seqid, rec2.seqid);
    assert_eq!(rec.start, rec2.start);
    assert_eq!(rec.end, rec2.end);
    assert_eq!(rec.feature_type, rec2.feature_type);
    assert_eq!(rec.attributes, rec2.attributes);
}
