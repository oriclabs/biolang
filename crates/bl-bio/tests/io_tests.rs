use bl_bio::io::*;
use bl_core::value::{BioSequence, Table, Value};
use std::collections::HashMap;

fn test_data_dir() -> std::path::PathBuf {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // crates
    dir.pop(); // project root
    dir.push("tests");
    dir.push("data");
    dir
}

fn tmp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("biolang_test_{name}"))
}

// ════════════════════════════════════════════════════════════════
// FASTA
// ════════════════════════════════════════════════════════════════

#[test]
fn test_fasta_stream_basic() {
    let path = test_data_dir().join("test.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let r1 = s.next().unwrap();
        if let Value::Record(map) = r1 {
            assert_eq!(map.get("id"), Some(&Value::Str("seq1".into())));
            assert_eq!(
                map.get("seq"),
                Some(&Value::DNA(BioSequence { data: "ATGATCGATCG".into() }))
            );
            assert_eq!(map.get("length"), Some(&Value::Int(11)));
            assert_eq!(map.get("description"), Some(&Value::Str("first sequence".into())));
        } else {
            panic!("expected Record, got {r1:?}");
        }
        let r2 = s.next().unwrap();
        if let Value::Record(map) = r2 {
            assert_eq!(map.get("id"), Some(&Value::Str("seq2".into())));
            assert_eq!(map.get("length"), Some(&Value::Int(12)));
        } else {
            panic!("expected Record");
        }
        let r3 = s.next().unwrap();
        if let Value::Record(map) = r3 {
            assert_eq!(map.get("id"), Some(&Value::Str("seq3".into())));
        } else {
            panic!("expected Record");
        }
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fasta_table_basic() {
    let path = test_data_dir().join("test.fa");
    let result = read_fasta_table(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 3);
        assert_eq!(t.columns, vec!["id", "description", "seq", "length"]);
        assert_eq!(t.rows[0][0], Value::Str("seq1".into()));
        assert_eq!(t.rows[0][3], Value::Int(11));
        // Table is reusable — access multiple times
        assert_eq!(t.rows[1][0], Value::Str("seq2".into()));
        assert_eq!(t.rows[2][0], Value::Str("seq3".into()));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_fasta_file_not_found() {
    let result = read_fasta("nonexistent.fa");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found") || err.contains("cannot open"), "err: {err}");
}

#[test]
fn test_fasta_absolute_path() {
    let path = test_data_dir().join("test.fa");
    let abs = std::fs::canonicalize(&path).unwrap();
    let result = read_fasta(abs.to_str().unwrap()).unwrap();
    assert!(matches!(result, Value::Stream(_)));
}

#[test]
fn test_fasta_single_record() {
    let path = test_data_dir().join("single.fa");
    let result = read_fasta_table(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][0], Value::Str("only_seq".into()));
        assert_eq!(t.rows[0][1], Value::Str("test single record".into()));
        assert_eq!(t.rows[0][3], Value::Int(11));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_fasta_empty_file() {
    let path = test_data_dir().join("empty.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        assert!(s.next().is_none(), "empty FASTA should yield no records");
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fasta_empty_file_table() {
    let path = test_data_dir().join("empty.fa");
    let result = read_fasta_table(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 0);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_fasta_multiline_sequence() {
    let path = test_data_dir().join("multiline.fa");
    let result = read_fasta_table(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 2);
        // First sequence should have all 3 lines concatenated
        assert_eq!(t.rows[0][3], Value::Int(35)); // 11 + 12 + 12
        assert_eq!(
            t.rows[0][2],
            Value::DNA(BioSequence { data: "ATGATCGATCGGCGCATATGCGCAAACCCGGGTTT".into() })
        );
        // Second sequence is short
        assert_eq!(t.rows[1][3], Value::Int(4));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_fasta_empty_sequence() {
    let path = test_data_dir().join("empty_seq.fa");
    let result = read_fasta_table(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 2);
        assert_eq!(t.rows[0][0], Value::Str("empty_seq".into()));
        assert_eq!(t.rows[0][3], Value::Int(0));
        assert_eq!(t.rows[1][0], Value::Str("seq2".into()));
        assert_eq!(t.rows[1][3], Value::Int(4));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_fasta_gzipped() {
    let path = test_data_dir().join("test.fa.gz");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items.len(), 3);
        if let Value::Record(map) = &items[0] {
            assert_eq!(map.get("id"), Some(&Value::Str("seq1".into())));
        }
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fasta_stream_exhaustion() {
    let path = test_data_dir().join("test.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let _ = s.collect_all();
        // After exhaustion, next() returns None
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fasta_rejects_fastq() {
    let path = test_data_dir().join("test.fq");
    let result = read_fasta(path.to_str().unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("FASTQ"), "error should mention FASTQ: {err}");
}

#[test]
fn test_fasta_write_roundtrip() {
    let path = test_data_dir().join("test.fa");
    let table = read_fasta_table(path.to_str().unwrap()).unwrap();
    let records = if let Value::Table(t) = &table {
        (0..t.num_rows())
            .map(|i| Value::Record(t.row_to_record(i)))
            .collect::<Vec<_>>()
    } else {
        panic!("expected Table");
    };

    let out_path = tmp_path("roundtrip.fa");
    let count = write_fasta(&Value::List(records), out_path.to_str().unwrap()).unwrap();
    assert_eq!(count, Value::Int(3));

    // Read back and verify
    let table2 = read_fasta_table(out_path.to_str().unwrap()).unwrap();
    if let (Value::Table(t1), Value::Table(t2)) = (&table, &table2) {
        assert_eq!(t1.num_rows(), t2.num_rows());
        for i in 0..t1.num_rows() {
            assert_eq!(t1.rows[i][0], t2.rows[i][0], "id mismatch at row {i}");
            assert_eq!(t1.rows[i][3], t2.rows[i][3], "length mismatch at row {i}");
        }
    }
    let _ = std::fs::remove_file(&out_path);
}

// ════════════════════════════════════════════════════════════════
// FASTQ
// ════════════════════════════════════════════════════════════════

#[test]
fn test_fastq_stream_basic() {
    let path = test_data_dir().join("test.fq");
    let result = read_fastq(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let r1 = s.next().unwrap();
        if let Value::Record(map) = r1 {
            assert_eq!(map.get("id"), Some(&Value::Str("read1".into())));
            assert_eq!(
                map.get("seq"),
                Some(&Value::DNA(BioSequence { data: "ATGATCGATCG".into() }))
            );
            assert_eq!(map.get("length"), Some(&Value::Int(11)));
            assert_eq!(map.get("quality"), Some(&Value::Str("IIIIIIIIIII".into())));
        } else {
            panic!("expected Record");
        }
        assert!(s.next().is_some()); // read2
        assert!(s.next().is_some()); // read3
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fastq_table_basic() {
    let path = test_data_dir().join("test.fq");
    let result = read_fastq_table(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 3);
        assert_eq!(t.columns, vec!["id", "description", "seq", "length", "quality"]);
        assert_eq!(t.rows[0][0], Value::Str("read1".into()));
        assert_eq!(t.rows[0][3], Value::Int(11));
        assert_eq!(t.rows[0][4], Value::Str("IIIIIIIIIII".into()));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_fastq_file_not_found() {
    let result = read_fastq("nonexistent.fq");
    assert!(result.is_err());
}

#[test]
fn test_fastq_absolute_path() {
    let path = test_data_dir().join("test.fq");
    let abs = std::fs::canonicalize(&path).unwrap();
    let result = read_fastq(abs.to_str().unwrap()).unwrap();
    assert!(matches!(result, Value::Stream(_)));
}

#[test]
fn test_fastq_single_record() {
    let path = test_data_dir().join("single.fq");
    let result = read_fastq_table(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][0], Value::Str("only_read".into()));
        assert_eq!(t.rows[0][1], Value::Str("description".into()));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_fastq_empty_file() {
    let path = test_data_dir().join("empty.fq");
    let result = read_fastq(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fastq_gzipped() {
    let path = test_data_dir().join("test.fq.gz");
    let result = read_fastq(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items.len(), 3);
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fastq_rejects_fasta() {
    let path = test_data_dir().join("test.fa");
    let result = read_fastq(path.to_str().unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("FASTA"), "error should mention FASTA: {err}");
}

#[test]
fn test_fastq_stream_exhaustion() {
    let path = test_data_dir().join("test.fq");
    let result = read_fastq(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let _ = s.collect_all();
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fastq_write_roundtrip() {
    let path = test_data_dir().join("test.fq");
    let table = read_fastq_table(path.to_str().unwrap()).unwrap();
    let records = if let Value::Table(t) = &table {
        (0..t.num_rows())
            .map(|i| Value::Record(t.row_to_record(i)))
            .collect::<Vec<_>>()
    } else {
        panic!("expected Table");
    };

    let out_path = tmp_path("roundtrip.fq");
    let count = write_fastq(&Value::List(records), out_path.to_str().unwrap()).unwrap();
    assert_eq!(count, Value::Int(3));

    let table2 = read_fastq_table(out_path.to_str().unwrap()).unwrap();
    if let (Value::Table(t1), Value::Table(t2)) = (&table, &table2) {
        assert_eq!(t1.num_rows(), t2.num_rows());
        for i in 0..t1.num_rows() {
            assert_eq!(t1.rows[i][0], t2.rows[i][0], "id mismatch at row {i}");
            assert_eq!(t1.rows[i][3], t2.rows[i][3], "length mismatch at row {i}");
        }
    }
    let _ = std::fs::remove_file(&out_path);
}

// ════════════════════════════════════════════════════════════════
// BED
// ════════════════════════════════════════════════════════════════

#[test]
fn test_bed_table_basic() {
    let path = test_data_dir().join("test.bed");
    let result = read_bed(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 4);
        assert_eq!(t.num_cols(), 6);
        assert_eq!(t.columns[0], "chrom");
        assert_eq!(t.columns[1], "start");
        assert_eq!(t.columns[2], "end");
        assert_eq!(t.rows[0][0], Value::Str("chr1".into()));
        assert_eq!(t.rows[0][1], Value::Int(10000));
        assert_eq!(t.rows[0][2], Value::Int(10500));
        assert_eq!(t.rows[0][3], Value::Str("peak1".into()));
    } else {
        panic!("expected Table, got {result:?}");
    }
}

#[test]
fn test_bed_stream_basic() {
    let path = test_data_dir().join("test.bed");
    let result = read_bed_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items.len(), 4);
        if let Value::Record(map) = &items[0] {
            assert_eq!(map.get("chrom"), Some(&Value::Str("chr1".into())));
            assert_eq!(map.get("start"), Some(&Value::Int(10000)));
            assert_eq!(map.get("end"), Some(&Value::Int(10500)));
            assert_eq!(map.get("name"), Some(&Value::Str("peak1".into())));
        } else {
            panic!("expected Record");
        }
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_bed_file_not_found() {
    assert!(read_bed("nonexistent.bed").is_err());
}

#[test]
fn test_bed_absolute_path() {
    let path = test_data_dir().join("test.bed");
    let abs = std::fs::canonicalize(&path).unwrap();
    let result = read_bed(abs.to_str().unwrap()).unwrap();
    assert!(matches!(result, Value::Table(_)));
}

#[test]
fn test_bed_single_record() {
    let path = test_data_dir().join("single.bed");
    let result = read_bed(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][0], Value::Str("chr1".into()));
        assert_eq!(t.rows[0][1], Value::Int(100));
        assert_eq!(t.rows[0][2], Value::Int(200));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_bed_empty_file() {
    let path = test_data_dir().join("empty.bed");
    let result = read_bed(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 0);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_bed3_minimal() {
    let path = test_data_dir().join("bed3.bed");
    let result = read_bed(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 3);
        assert_eq!(t.num_cols(), 3);
        assert_eq!(t.columns, vec!["chrom", "start", "end"]);
        assert_eq!(t.rows[0][0], Value::Str("chr1".into()));
        assert_eq!(t.rows[0][1], Value::Int(100));
        assert_eq!(t.rows[2][0], Value::Str("chr2".into()));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_bed_stream_exhaustion() {
    let path = test_data_dir().join("test.bed");
    let result = read_bed_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let _ = s.collect_all();
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_bed_write_roundtrip() {
    let path = test_data_dir().join("test.bed");
    let table = read_bed(path.to_str().unwrap()).unwrap();
    let records = if let Value::Table(t) = &table {
        (0..t.num_rows())
            .map(|i| Value::Record(t.row_to_record(i)))
            .collect::<Vec<_>>()
    } else {
        panic!("expected Table");
    };

    let out_path = tmp_path("roundtrip.bed");
    let count = write_bed(&Value::List(records), out_path.to_str().unwrap()).unwrap();
    assert_eq!(count, Value::Int(4));

    let table2 = read_bed(out_path.to_str().unwrap()).unwrap();
    if let (Value::Table(t1), Value::Table(t2)) = (&table, &table2) {
        assert_eq!(t1.num_rows(), t2.num_rows());
        for i in 0..t1.num_rows() {
            assert_eq!(t1.rows[i][0], t2.rows[i][0], "chrom mismatch at row {i}");
            assert_eq!(t1.rows[i][1], t2.rows[i][1], "start mismatch at row {i}");
            assert_eq!(t1.rows[i][2], t2.rows[i][2], "end mismatch at row {i}");
        }
    }
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn test_bed_write_table_input() {
    let path = test_data_dir().join("test.bed");
    let table = read_bed(path.to_str().unwrap()).unwrap();
    let out_path = tmp_path("roundtrip_table.bed");
    let count = write_bed(&table, out_path.to_str().unwrap()).unwrap();
    assert_eq!(count, Value::Int(4));
    let _ = std::fs::remove_file(&out_path);
}

// ════════════════════════════════════════════════════════════════
// GFF
// ════════════════════════════════════════════════════════════════

#[test]
fn test_gff_table_basic() {
    let path = test_data_dir().join("test.gff");
    let result = read_gff(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 3);
        assert_eq!(t.num_cols(), 9);
        assert_eq!(t.columns[0], "seqid");
        assert_eq!(t.columns[2], "type");
        assert_eq!(t.rows[0][0], Value::Str("chr1".into()));
        assert_eq!(t.rows[0][2], Value::Str("gene".into()));
        assert_eq!(t.rows[0][3], Value::Int(11869));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_gff_stream_basic() {
    let path = test_data_dir().join("test.gff");
    let result = read_gff_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items.len(), 3);
        if let Value::Record(map) = &items[0] {
            assert_eq!(map.get("seqid"), Some(&Value::Str("chr1".into())));
            assert_eq!(map.get("type"), Some(&Value::Str("gene".into())));
            assert_eq!(map.get("start"), Some(&Value::Int(11869)));
        } else {
            panic!("expected Record");
        }
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_gff_file_not_found() {
    assert!(read_gff("nonexistent.gff").is_err());
}

#[test]
fn test_gff_absolute_path() {
    let path = test_data_dir().join("test.gff");
    let abs = std::fs::canonicalize(&path).unwrap();
    let result = read_gff(abs.to_str().unwrap()).unwrap();
    assert!(matches!(result, Value::Table(_)));
}

#[test]
fn test_gff_single_record() {
    let path = test_data_dir().join("single.gff");
    let result = read_gff(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][0], Value::Str("chr1".into()));
        assert_eq!(t.rows[0][2], Value::Str("gene".into()));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_gff_empty_file() {
    let path = test_data_dir().join("empty.gff");
    let result = read_gff(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 0);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_gff_stream_exhaustion() {
    let path = test_data_dir().join("test.gff");
    let result = read_gff_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let _ = s.collect_all();
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_gff_attributes_field() {
    let path = test_data_dir().join("test.gff");
    let result = read_gff(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        // attributes column should contain the raw attributes string
        let attr = &t.rows[0][8];
        if let Value::Str(s) = attr {
            assert!(s.contains("Name=DDX11L1"), "attributes should contain Name: {s}");
        }
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_gff_write_roundtrip() {
    let path = test_data_dir().join("test.gff");
    let table = read_gff(path.to_str().unwrap()).unwrap();
    let records = if let Value::Table(t) = &table {
        (0..t.num_rows())
            .map(|i| Value::Record(t.row_to_record(i)))
            .collect::<Vec<_>>()
    } else {
        panic!("expected Table");
    };

    let out_path = tmp_path("roundtrip.gff");
    let count = write_gff(&Value::List(records), out_path.to_str().unwrap()).unwrap();
    assert_eq!(count, Value::Int(3));

    let table2 = read_gff(out_path.to_str().unwrap()).unwrap();
    if let (Value::Table(t1), Value::Table(t2)) = (&table, &table2) {
        assert_eq!(t1.num_rows(), t2.num_rows());
        for i in 0..t1.num_rows() {
            assert_eq!(t1.rows[i][0], t2.rows[i][0], "seqid mismatch at row {i}");
            assert_eq!(t1.rows[i][3], t2.rows[i][3], "start mismatch at row {i}");
        }
    }
    let _ = std::fs::remove_file(&out_path);
}

// ════════════════════════════════════════════════════════════════
// VCF
// ════════════════════════════════════════════════════════════════

#[test]
fn test_vcf_table_basic() {
    let path = test_data_dir().join("test.vcf");
    let result = read_vcf(path.to_str().unwrap()).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
        if let Value::Variant { chrom, pos, ref_allele, .. } = &items[0] {
            assert_eq!(chrom, "chr1");
            assert_eq!(*pos, 10177);
            assert_eq!(ref_allele, "A");
        } else {
            panic!("expected Variant");
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_vcf_stream_basic() {
    let path = test_data_dir().join("test.vcf");
    let result = read_vcf_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items.len(), 3);
        if let Value::Variant { chrom, pos, ref_allele, .. } = &items[0] {
            assert_eq!(chrom, "chr1");
            assert_eq!(*pos, 10177);
            assert_eq!(ref_allele, "A");
        } else {
            panic!("expected Variant");
        }
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_vcf_file_not_found() {
    assert!(read_vcf("nonexistent.vcf").is_err());
}

#[test]
fn test_vcf_absolute_path() {
    let path = test_data_dir().join("test.vcf");
    let abs = std::fs::canonicalize(&path).unwrap();
    let result = read_vcf(abs.to_str().unwrap()).unwrap();
    assert!(matches!(result, Value::List(_)));
}

#[test]
fn test_vcf_single_record() {
    let path = test_data_dir().join("single.vcf");
    let result = read_vcf(path.to_str().unwrap()).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 1);
        if let Value::Variant { chrom, pos, .. } = &items[0] {
            assert_eq!(chrom, "chr1");
            assert_eq!(*pos, 10177);
        } else {
            panic!("expected Variant");
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_vcf_empty_file() {
    let path = test_data_dir().join("empty.vcf");
    let result = read_vcf(path.to_str().unwrap()).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 0);
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_vcf_stream_exhaustion() {
    let path = test_data_dir().join("test.vcf");
    let result = read_vcf_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let _ = s.collect_all();
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_vcf_info_field() {
    let path = test_data_dir().join("test.vcf");
    let result = read_vcf(path.to_str().unwrap()).unwrap();
    if let Value::List(items) = result {
        if let Value::Variant { info, .. } = &items[0] {
            // INFO is now stored as raw string for lazy parsing
            assert!(info.contains_key("_raw"), "INFO should contain _raw key");
            if let Some(Value::Str(raw)) = info.get("_raw") {
                assert!(raw.contains("AF="), "raw INFO should contain AF=");
                assert!(raw.contains("DP="), "raw INFO should contain DP=");
            } else {
                panic!("_raw should be a string");
            }
        } else {
            panic!("expected Variant");
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_vcf_indel() {
    let path = test_data_dir().join("test.vcf");
    let result = read_vcf(path.to_str().unwrap()).unwrap();
    if let Value::List(items) = result {
        // Third row is a deletion (22bp REF -> 1bp ALT)
        if let Value::Variant { chrom, ref_allele, .. } = &items[2] {
            assert_eq!(chrom, "chr2");
            assert!(ref_allele.len() > 1, "indel REF should be multi-base: {ref_allele}");
        } else {
            panic!("expected Variant");
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_vcf_write_roundtrip() {
    let path = test_data_dir().join("test.vcf");
    let variants = read_vcf(path.to_str().unwrap()).unwrap();
    let items1 = if let Value::List(ref l) = variants { l.clone() } else { panic!("expected List") };

    let out_path = tmp_path("roundtrip.vcf");
    let count = write_vcf(&variants, out_path.to_str().unwrap()).unwrap();
    assert_eq!(count, Value::Int(3));

    let variants2 = read_vcf(out_path.to_str().unwrap()).unwrap();
    let items2 = if let Value::List(ref l) = variants2 { l.clone() } else { panic!("expected List") };
    assert_eq!(items1.len(), items2.len());
    for i in 0..items1.len() {
        if let (Value::Variant { chrom: c1, pos: p1, ref_allele: r1, .. },
                Value::Variant { chrom: c2, pos: p2, ref_allele: r2, .. }) = (&items1[i], &items2[i]) {
            assert_eq!(c1, c2, "chrom mismatch at row {i}");
            assert_eq!(p1, p2, "pos mismatch at row {i}");
            assert_eq!(r1, r2, "ref mismatch at row {i}");
        }
    }
    let _ = std::fs::remove_file(&out_path);
}

// ════════════════════════════════════════════════════════════════
// SAM
// ════════════════════════════════════════════════════════════════

#[test]
fn test_sam_table_basic() {
    let path = test_data_dir().join("test.sam");
    let result = read_sam(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 3);
        assert_eq!(t.num_cols(), 11);
        assert_eq!(t.columns[0], "qname");
        assert_eq!(t.columns[1], "flag");
        assert_eq!(t.columns[2], "rname");
        assert_eq!(t.columns[3], "pos");
        assert_eq!(t.rows[0][0], Value::Str("read1".into()));
        assert_eq!(t.rows[0][1], Value::Int(99));
        assert_eq!(t.rows[0][2], Value::Str("chr1".into()));
        assert_eq!(t.rows[0][3], Value::Int(10000));
        assert_eq!(t.rows[0][4], Value::Int(60));  // MAPQ
        assert_eq!(t.rows[0][5], Value::Str("50M".into())); // CIGAR
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_sam_stream_basic() {
    let path = test_data_dir().join("test.sam");
    let result = read_sam_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items.len(), 3);
        if let Value::Record(map) = &items[0] {
            assert_eq!(map.get("qname"), Some(&Value::Str("read1".into())));
            assert_eq!(map.get("flag"), Some(&Value::Int(99)));
            assert_eq!(map.get("rname"), Some(&Value::Str("chr1".into())));
            assert_eq!(map.get("pos"), Some(&Value::Int(10000)));
            assert_eq!(map.get("mapq"), Some(&Value::Int(60)));
        } else {
            panic!("expected Record");
        }
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_sam_header() {
    let path = test_data_dir().join("test.sam");
    let result = read_sam_header(path.to_str().unwrap()).unwrap();
    if let Value::List(headers) = result {
        assert_eq!(headers.len(), 4);
        if let Value::Record(h) = &headers[0] {
            assert_eq!(h["tag"], Value::Str("HD".into()));
            if let Value::Record(fields) = &h["fields"] {
                assert_eq!(fields["VN"], Value::Str("1.6".into()));
                assert_eq!(fields["SO"], Value::Str("coordinate".into()));
            }
        }
        if let Value::Record(h) = &headers[1] {
            assert_eq!(h["tag"], Value::Str("SQ".into()));
            if let Value::Record(fields) = &h["fields"] {
                assert_eq!(fields["SN"], Value::Str("chr1".into()));
            }
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_sam_file_not_found() {
    assert!(read_sam("nonexistent.sam").is_err());
}

#[test]
fn test_sam_absolute_path() {
    let path = test_data_dir().join("test.sam");
    let abs = std::fs::canonicalize(&path).unwrap();
    let result = read_sam(abs.to_str().unwrap()).unwrap();
    assert!(matches!(result, Value::Table(_)));
}

#[test]
fn test_sam_single_record() {
    let path = test_data_dir().join("single.sam");
    let result = read_sam(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][0], Value::Str("read1".into()));
        assert_eq!(t.rows[0][1], Value::Int(99));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_sam_empty_file() {
    let path = test_data_dir().join("empty.sam");
    let result = read_sam(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 0);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_sam_unmapped_read() {
    let path = test_data_dir().join("test.sam");
    let result = read_sam(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        // Third read is unmapped (flag 4)
        assert_eq!(t.rows[2][1], Value::Int(4));
        assert_eq!(t.rows[2][2], Value::Str("*".into())); // rname
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_sam_stream_exhaustion() {
    let path = test_data_dir().join("test.sam");
    let result = read_sam_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let _ = s.collect_all();
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_sam_cigar_complex() {
    let path = test_data_dir().join("test.sam");
    let result = read_sam(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        // Second read has a complex CIGAR: 30M5I15M
        assert_eq!(t.rows[1][5], Value::Str("30M5I15M".into()));
    } else {
        panic!("expected Table");
    }
}

// ════════════════════════════════════════════════════════════════
// MAF
// ════════════════════════════════════════════════════════════════

#[test]
fn test_maf_table_basic() {
    let path = test_data_dir().join("test.maf");
    let result = read_maf(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 3);
        assert!(t.num_cols() >= 13);
        assert_eq!(t.columns[0], "Hugo_Symbol");
        assert_eq!(t.columns[4], "Chromosome");
        assert_eq!(t.columns[5], "Start_Position");
        assert_eq!(t.rows[0][0], Value::Str("TP53".into()));
        assert_eq!(t.rows[0][1], Value::Int(7157));
        assert_eq!(t.rows[0][4], Value::Str("chr17".into()));
        assert_eq!(t.rows[0][5], Value::Int(7675088));
        assert_eq!(t.rows[0][8], Value::Str("Missense_Mutation".into()));
        assert_eq!(t.rows[1][0], Value::Str("BRAF".into()));
        assert_eq!(t.rows[1][1], Value::Int(673));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_maf_stream_basic() {
    let path = test_data_dir().join("test.maf");
    let result = read_maf_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items.len(), 3);
        if let Value::Record(map) = &items[0] {
            assert_eq!(map.get("Hugo_Symbol"), Some(&Value::Str("TP53".into())));
            assert_eq!(map.get("Entrez_Gene_Id"), Some(&Value::Int(7157)));
            assert_eq!(map.get("Chromosome"), Some(&Value::Str("chr17".into())));
            assert_eq!(map.get("Start_Position"), Some(&Value::Int(7675088)));
        } else {
            panic!("expected Record");
        }
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_maf_file_not_found() {
    assert!(read_maf("nonexistent.maf").is_err());
}

#[test]
fn test_maf_absolute_path() {
    let path = test_data_dir().join("test.maf");
    let abs = std::fs::canonicalize(&path).unwrap();
    let result = read_maf(abs.to_str().unwrap()).unwrap();
    assert!(matches!(result, Value::Table(_)));
}

#[test]
fn test_maf_single_record() {
    let path = test_data_dir().join("single.maf");
    let result = read_maf(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][0], Value::Str("TP53".into()));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_maf_empty_file() {
    let path = test_data_dir().join("empty.maf");
    let result = read_maf(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 0);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_maf_stream_exhaustion() {
    let path = test_data_dir().join("test.maf");
    let result = read_maf_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let _ = s.collect_all();
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_maf_variant_types() {
    let path = test_data_dir().join("test.maf");
    let result = read_maf(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        // Check variant classification column
        assert_eq!(t.rows[0][8], Value::Str("Missense_Mutation".into()));
        assert_eq!(t.rows[2][8], Value::Str("In_Frame_Del".into()));
        // Check variant type column
        assert_eq!(t.rows[0][9], Value::Str("SNP".into()));
        assert_eq!(t.rows[2][9], Value::Str("DEL".into()));
    } else {
        panic!("expected Table");
    }
}

// ════════════════════════════════════════════════════════════════
// bedGraph
// ════════════════════════════════════════════════════════════════

#[test]
fn test_bedgraph_table_basic() {
    let path = test_data_dir().join("test.bedgraph");
    let result = read_bedgraph(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 4);
        assert_eq!(t.num_cols(), 4);
        assert_eq!(t.columns[0], "chrom");
        assert_eq!(t.columns[3], "value");
        assert_eq!(t.rows[0][0], Value::Str("chr1".into()));
        assert_eq!(t.rows[0][1], Value::Int(10000));
        assert_eq!(t.rows[0][2], Value::Int(10100));
        assert_eq!(t.rows[0][3], Value::Float(25.5));
        assert_eq!(t.rows[3][3], Value::Int(7));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_bedgraph_stream_basic() {
    let path = test_data_dir().join("test.bedgraph");
    let result = read_bedgraph_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let items = s.collect_all();
        assert_eq!(items.len(), 4);
        if let Value::Record(map) = &items[0] {
            assert_eq!(map.get("chrom"), Some(&Value::Str("chr1".into())));
            assert_eq!(map.get("start"), Some(&Value::Int(10000)));
            assert_eq!(map.get("end"), Some(&Value::Int(10100)));
            assert_eq!(map.get("value"), Some(&Value::Float(25.5)));
        } else {
            panic!("expected Record");
        }
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_bedgraph_file_not_found() {
    assert!(read_bedgraph("nonexistent.bedgraph").is_err());
}

#[test]
fn test_bedgraph_absolute_path() {
    let path = test_data_dir().join("test.bedgraph");
    let abs = std::fs::canonicalize(&path).unwrap();
    let result = read_bedgraph(abs.to_str().unwrap()).unwrap();
    assert!(matches!(result, Value::Table(_)));
}

#[test]
fn test_bedgraph_single_record() {
    let path = test_data_dir().join("single.bedgraph");
    let result = read_bedgraph(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][0], Value::Str("chr1".into()));
        assert_eq!(t.rows[0][3], Value::Float(25.5));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_bedgraph_empty_file() {
    let path = test_data_dir().join("empty.bedgraph");
    let result = read_bedgraph(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 0);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_bedgraph_stream_exhaustion() {
    let path = test_data_dir().join("test.bedgraph");
    let result = read_bedgraph_stream(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let _ = s.collect_all();
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_bedgraph_integer_value() {
    let path = test_data_dir().join("test.bedgraph");
    let result = read_bedgraph(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        // Last row has integer value 7 (no decimal point)
        assert_eq!(t.rows[3][3], Value::Int(7));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_bedgraph_float_value() {
    let path = test_data_dir().join("test.bedgraph");
    let result = read_bedgraph(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = result {
        // First row has float value 25.5
        assert_eq!(t.rows[0][3], Value::Float(25.5));
        assert_eq!(t.rows[1][3], Value::Float(42.0));
        assert_eq!(t.rows[2][3], Value::Float(18.3));
    } else {
        panic!("expected Table");
    }
}

// ════════════════════════════════════════════════════════════════
// Cross-format: file not found (all stream readers)
// ════════════════════════════════════════════════════════════════

#[test]
fn test_all_stream_readers_file_not_found() {
    assert!(read_bed_stream("nonexistent.bed").is_err());
    assert!(read_gff_stream("nonexistent.gff").is_err());
    assert!(read_vcf_stream("nonexistent.vcf").is_err());
    assert!(read_sam_stream("nonexistent.sam").is_err());
    assert!(read_bam_stream("nonexistent.bam").is_err());
    assert!(read_maf_stream("nonexistent.maf").is_err());
    assert!(read_bedgraph_stream("nonexistent.bedgraph").is_err());
}

#[test]
fn test_all_table_readers_file_not_found() {
    assert!(read_bed("nonexistent.bed").is_err());
    assert!(read_gff("nonexistent.gff").is_err());
    assert!(read_vcf("nonexistent.vcf").is_err());
    assert!(read_sam("nonexistent.sam").is_err());
    assert!(read_maf("nonexistent.maf").is_err());
    assert!(read_bedgraph("nonexistent.bedgraph").is_err());
}

// ════════════════════════════════════════════════════════════════
// Format mismatch detection
// ════════════════════════════════════════════════════════════════

#[test]
fn test_fasta_rejects_fastq_format() {
    let path = test_data_dir().join("test.fq");
    let result = read_fasta(path.to_str().unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("FASTQ"), "error should mention FASTQ: {err}");
}

#[test]
fn test_fastq_rejects_fasta_format() {
    let path = test_data_dir().join("test.fa");
    let result = read_fastq(path.to_str().unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("FASTA"), "error should mention FASTA: {err}");
}

// ════════════════════════════════════════════════════════════════
// BIOLANG_DATA_DIR resolution
// ════════════════════════════════════════════════════════════════

#[test]
fn test_data_dir_env_resolution() {
    let data_dir = test_data_dir();
    // Set BIOLANG_DATA_DIR and try reading with just filename
    std::env::set_var("BIOLANG_DATA_DIR", data_dir.to_str().unwrap());
    let result = read_fasta("test.fa");
    std::env::remove_var("BIOLANG_DATA_DIR");
    // This should succeed if DATA_DIR resolution works
    assert!(result.is_ok(), "BIOLANG_DATA_DIR resolution failed: {:?}", result.err());
}

// ════════════════════════════════════════════════════════════════
// Write error handling
// ════════════════════════════════════════════════════════════════

#[test]
fn test_write_fasta_requires_list() {
    let result = write_fasta(&Value::Str("not a list".into()), "out.fa");
    assert!(result.is_err());
}

#[test]
fn test_write_fastq_requires_list() {
    let result = write_fastq(&Value::Str("not a list".into()), "out.fq");
    assert!(result.is_err());
}

#[test]
fn test_write_bed_requires_list_or_table() {
    let result = write_bed(&Value::Str("not a list".into()), "out.bed");
    assert!(result.is_err());
}

#[test]
fn test_write_vcf_requires_list() {
    let result = write_vcf(&Value::Str("not a list".into()), "out.vcf");
    assert!(result.is_err());
}

#[test]
fn test_write_gff_requires_list() {
    let result = write_gff(&Value::Str("not a list".into()), "out.gff");
    assert!(result.is_err());
}

#[test]
fn test_write_fasta_empty_list() {
    let out_path = tmp_path("empty_write.fa");
    let count = write_fasta(&Value::List(vec![]), out_path.to_str().unwrap()).unwrap();
    assert_eq!(count, Value::Int(0));
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn test_write_fastq_empty_list() {
    let out_path = tmp_path("empty_write.fq");
    let count = write_fastq(&Value::List(vec![]), out_path.to_str().unwrap()).unwrap();
    assert_eq!(count, Value::Int(0));
    let _ = std::fs::remove_file(&out_path);
}

// ════════════════════════════════════════════════════════════════
// BufRead-based GFF/BED parsing tests (regression for OOM fix)
// ════════════════════════════════════════════════════════════════

#[test]
fn test_read_gff_bufread_basic() {
    let path = test_data_dir().join("test.gff");
    let result = read_gff(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = &result {
        assert_eq!(t.columns.len(), 9);
        assert_eq!(t.columns[0], "seqid");
        assert_eq!(t.columns[2], "type");
        assert_eq!(t.columns[8], "attributes");
        assert!(t.rows.len() >= 3, "expected at least 3 GFF rows");
        // Check first row
        assert_eq!(t.rows[0][0], Value::Str("chr1".to_string()));
        assert_eq!(t.rows[0][2], Value::Str("gene".to_string()));
        assert_eq!(t.rows[0][3], Value::Int(11869)); // start
        assert_eq!(t.rows[0][4], Value::Int(14409)); // end
    } else {
        panic!("read_gff should return Table");
    }
}

#[test]
fn test_read_gff_skips_comments() {
    let path = test_data_dir().join("test.gff");
    let result = read_gff(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = &result {
        // The test.gff has 1 comment line + 3 data lines
        assert_eq!(t.rows.len(), 3);
    } else {
        panic!("read_gff should return Table");
    }
}

#[test]
fn test_read_gff_empty() {
    let path = test_data_dir().join("empty.gff");
    let result = read_gff(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = &result {
        assert_eq!(t.rows.len(), 0);
        assert_eq!(t.columns.len(), 9);
    } else {
        panic!("read_gff should return Table");
    }
}

#[test]
fn test_read_gff_single() {
    let path = test_data_dir().join("single.gff");
    let result = read_gff(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = &result {
        assert_eq!(t.rows.len(), 1);
    } else {
        panic!("read_gff should return Table");
    }
}

#[test]
fn test_read_bed_bufread_basic() {
    let path = test_data_dir().join("test.bed");
    let result = read_bed(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = &result {
        assert!(t.columns.len() >= 3);
        assert_eq!(t.columns[0], "chrom");
        assert_eq!(t.columns[1], "start");
        assert_eq!(t.columns[2], "end");
        assert_eq!(t.rows.len(), 4);
        // Check first row
        assert_eq!(t.rows[0][0], Value::Str("chr1".to_string()));
        assert_eq!(t.rows[0][1], Value::Int(10000));
        assert_eq!(t.rows[0][2], Value::Int(10500));
    } else {
        panic!("read_bed should return Table");
    }
}

#[test]
fn test_read_bed_bufread_empty() {
    let path = test_data_dir().join("empty.bed");
    let result = read_bed(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = &result {
        assert_eq!(t.rows.len(), 0);
    } else {
        panic!("read_bed should return Table");
    }
}

#[test]
fn test_read_bed_bufread_bed3() {
    let path = test_data_dir().join("bed3.bed");
    let result = read_bed(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = &result {
        assert_eq!(t.columns.len(), 3); // BED3 only has chrom, start, end
        assert!(t.rows.len() >= 1);
    } else {
        panic!("read_bed should return Table");
    }
}

#[test]
fn test_read_bed_bufread_multi() {
    let path = test_data_dir().join("multi.bed");
    let result = read_bed(path.to_str().unwrap()).unwrap();
    if let Value::Table(t) = &result {
        assert!(t.rows.len() >= 2, "multi.bed should have multiple rows");
    } else {
        panic!("read_bed should return Table");
    }
}

// ════════════════════════════════════════════════════════════════
// STREAMING & LARGE FILE HANDLING TESTS
// ════════════════════════════════════════════════════════════════

#[test]
fn test_fasta_stream_constant_memory_pattern() {
    // Verify streaming: consume records one at a time without collecting
    // This pattern should work for files of any size
    let path = test_data_dir().join("test.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let mut count = 0;
        let mut total_len = 0i64;
        while let Some(item) = s.next() {
            if let Value::Record(rec) = item {
                if let Some(Value::Int(len)) = rec.get("length") {
                    total_len += len;
                }
                count += 1;
            }
        }
        assert!(count > 0, "should have read at least one record");
        assert!(total_len > 0, "total length should be positive");
        // Stream should now be exhausted
        assert!(s.next().is_none());
        assert!(s.is_exhausted());
    } else {
        panic!("read_fasta should return Stream");
    }
}

#[test]
fn test_fastq_stream_constant_memory_pattern() {
    let path = test_data_dir().join("test.fq");
    let result = read_fastq(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let mut count = 0;
        while let Some(item) = s.next() {
            if let Value::Record(rec) = item {
                // Each FASTQ record should have seq and quality
                assert!(rec.contains_key("seq"), "record should have seq field");
                assert!(rec.contains_key("quality"), "record should have quality field");
                count += 1;
            }
        }
        assert!(count > 0, "should have read at least one FASTQ record");
        assert!(s.is_exhausted());
    } else {
        panic!("read_fastq should return Stream");
    }
}

#[test]
fn test_fasta_stream_early_termination() {
    // Reading only first N records from a stream (simulates constant-memory
    // processing of huge files by stopping early)
    let path = test_data_dir().join("test.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        // Take only the first record
        let first = s.next();
        assert!(first.is_some(), "should have at least one record");
        // Drop the stream — should not panic or leak
        drop(s);
    } else {
        panic!("read_fasta should return Stream");
    }
}

#[test]
fn test_fasta_gz_stream() {
    let path = test_data_dir().join("test.fa.gz");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let mut count = 0;
        while let Some(_) = s.next() {
            count += 1;
        }
        assert!(count > 0, "gzipped FASTA should have records");
    } else {
        panic!("read_fasta should return Stream for .gz");
    }
}

#[test]
fn test_fastq_gz_stream() {
    let path = test_data_dir().join("test.fq.gz");
    let result = read_fastq(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let mut count = 0;
        while let Some(_) = s.next() {
            count += 1;
        }
        assert!(count > 0, "gzipped FASTQ should have records");
    } else {
        panic!("read_fastq should return Stream for .gz");
    }
}

#[test]
fn test_fasta_stream_double_consume_exhausted() {
    let path = test_data_dir().join("test.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        // Consume all
        while s.next().is_some() {}
        assert!(s.is_exhausted());
        // Calling next again should return None (not panic)
        assert!(s.next().is_none());
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fasta_empty_file_stream() {
    let path = test_data_dir().join("empty.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        assert!(s.next().is_none(), "empty file should produce no records");
    } else {
        panic!("read_fasta should return Stream");
    }
}

#[test]
fn test_fasta_single_record_stream() {
    let path = test_data_dir().join("single.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let first = s.next();
        assert!(first.is_some(), "single.fa should have one record");
        assert!(s.next().is_none(), "single.fa should have only one record");
    } else {
        panic!("read_fasta should return Stream");
    }
}

#[test]
fn test_fasta_multiline_stream() {
    // Multiline FASTA (sequence split across lines)
    let path = test_data_dir().join("multiline.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let first = s.next();
        assert!(first.is_some(), "multiline.fa should have records");
        if let Some(Value::Record(rec)) = first {
            // The sequence should be concatenated from multiple lines
            if let Some(Value::Int(len)) = rec.get("length") {
                assert!(*len > 0, "sequence length should be positive");
            }
        }
    } else {
        panic!("read_fasta should return Stream");
    }
}

#[test]
fn test_generated_large_fasta_stream() {
    // Generate a FASTA file with many records and stream it
    use std::io::Write;
    let tmp = std::env::temp_dir().join("biolang_test_large.fa");
    {
        let mut f = std::fs::File::create(&tmp).unwrap();
        for i in 0..10000 {
            writeln!(f, ">seq_{i} test sequence").unwrap();
            writeln!(f, "ATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCG").unwrap();
        }
    }

    let result = read_fasta(tmp.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let mut count = 0u64;
        while let Some(_) = s.next() {
            count += 1;
        }
        assert_eq!(count, 10000, "should stream all 10000 records");
    } else {
        panic!("expected Stream");
    }
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_generated_large_fastq_stream() {
    use std::io::Write;
    let tmp = std::env::temp_dir().join("biolang_test_large.fq");
    {
        let mut f = std::fs::File::create(&tmp).unwrap();
        for i in 0..5000 {
            writeln!(f, "@read_{i}").unwrap();
            writeln!(f, "ATCGATCGATCGATCGATCGATCGATCG").unwrap();
            writeln!(f, "+").unwrap();
            writeln!(f, "IIIIIIIIIIIIIIIIIIIIIIIIIIIII").unwrap();
        }
    }

    let result = read_fastq(tmp.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let mut count = 0u64;
        while let Some(_) = s.next() {
            count += 1;
        }
        assert_eq!(count, 5000, "should stream all 5000 reads");
    } else {
        panic!("expected Stream");
    }
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_generated_large_vcf_stream() {
    use std::io::Write;
    let tmp = std::env::temp_dir().join("biolang_test_large.vcf");
    {
        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(f, "##fileformat=VCFv4.3").unwrap();
        writeln!(f, "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO").unwrap();
        for i in 1..=5000 {
            writeln!(f, "chr1\t{i}\t.\tA\tT\t30\tPASS\t.").unwrap();
        }
    }

    let result = read_vcf(tmp.to_str().unwrap()).unwrap();
    // read_vcf returns List of Variant values
    match result {
        Value::List(items) => {
            assert_eq!(items.len(), 5000, "should have all 5000 variants");
        }
        _ => panic!("expected List from read_vcf, got {:?}", result.type_of()),
    }
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_fasta_stream_gc_computation() {
    // Simulate a real-world pattern: stream FASTA, compute GC for each
    let path = test_data_dir().join("test.fa");
    let result = read_fasta(path.to_str().unwrap()).unwrap();
    if let Value::Stream(s) = result {
        let mut gc_values = Vec::new();
        while let Some(item) = s.next() {
            if let Value::Record(rec) = item {
                if let Some(Value::Float(gc)) = rec.get("gc") {
                    gc_values.push(*gc);
                }
            }
        }
        assert!(!gc_values.is_empty(), "should have computed GC values");
        for gc in &gc_values {
            assert!(*gc >= 0.0 && *gc <= 1.0, "GC should be in [0, 1]");
        }
    } else {
        panic!("expected Stream");
    }
}

#[test]
fn test_fasta_stats_returns_record() {
    let path = test_data_dir().join("test.fa");
    let result = fasta_stats(path.to_str().unwrap()).unwrap();
    if let Value::Record(rec) = result {
        assert!(rec.contains_key("count"));
        assert!(rec.contains_key("total_bp"));
        assert!(rec.contains_key("mean_length"));
        assert!(rec.contains_key("n50"));
        assert!(rec.contains_key("mean_gc"));
        if let Some(Value::Int(count)) = rec.get("count") {
            assert!(*count > 0);
        }
    } else {
        panic!("fasta_stats should return Record");
    }
}

#[test]
fn test_fasta_stats_empty() {
    let path = test_data_dir().join("empty.fa");
    let result = fasta_stats(path.to_str().unwrap()).unwrap();
    if let Value::Record(rec) = result {
        assert_eq!(rec.get("count"), Some(&Value::Int(0)));
    } else {
        panic!("fasta_stats should return Record");
    }
}

#[test]
fn test_read_nonexistent_file() {
    let result = read_fasta("/nonexistent/path/file.fa");
    assert!(result.is_err(), "should error on nonexistent file");
}

#[test]
fn test_format_mismatch_fasta_vs_fastq() {
    // Trying to read a FASTQ file as FASTA should give a helpful error
    let path = test_data_dir().join("test.fq");
    let result = read_fasta(path.to_str().unwrap());
    assert!(result.is_err(), "reading FASTQ as FASTA should error");
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("FASTQ") || msg.contains("fastq"),
        "error should mention FASTQ format mismatch"
    );
}
