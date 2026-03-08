use bl_bio::call_bio_builtin;
use bl_core::value::{BioSequence, GenomicInterval, Strand, Table, Value};
use std::collections::HashMap;

// ── Sequence operations ─────────────────────────────────────────

#[test]
fn test_transcribe_dispatch() {
    let result = call_bio_builtin(
        "transcribe",
        vec![Value::DNA(BioSequence { data: "ATGATCG".into() })],
    ).unwrap();
    assert_eq!(result, Value::RNA(BioSequence { data: "AUGAUCG".into() }));
}

#[test]
fn test_complement_dispatch() {
    let result = call_bio_builtin(
        "complement",
        vec![Value::DNA(BioSequence { data: "ATCG".into() })],
    ).unwrap();
    assert_eq!(result, Value::DNA(BioSequence { data: "TAGC".into() }));
}

#[test]
fn test_reverse_complement_dispatch() {
    let result = call_bio_builtin(
        "reverse_complement",
        vec![Value::DNA(BioSequence { data: "ATCG".into() })],
    ).unwrap();
    assert_eq!(result, Value::DNA(BioSequence { data: "CGAT".into() }));
}

#[test]
fn test_translate_dispatch() {
    let result = call_bio_builtin(
        "translate",
        vec![Value::RNA(BioSequence { data: "AUGAUCUAA".into() })],
    ).unwrap();
    assert_eq!(result, Value::Protein(BioSequence { data: "MI".into() }));
}

#[test]
fn test_translate_dna_dispatch() {
    let result = call_bio_builtin(
        "translate",
        vec![Value::DNA(BioSequence { data: "ATGATCGATCG".into() })],
    ).unwrap();
    assert_eq!(result, Value::Protein(BioSequence { data: "MID".into() }));
}

#[test]
fn test_gc_content() {
    let result = call_bio_builtin(
        "gc_content",
        vec![Value::DNA(BioSequence { data: "ATGC".into() })],
    ).unwrap();
    assert_eq!(result, Value::Float(0.5));
}

#[test]
fn test_find_motif() {
    let result = call_bio_builtin(
        "find_motif",
        vec![
            Value::DNA(BioSequence { data: "ATGATGATG".into() }),
            Value::Str("ATG".into()),
        ],
    ).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(0), Value::Int(3), Value::Int(6)])
    );
}

#[test]
fn test_kmers() {
    let result = call_bio_builtin(
        "kmers",
        vec![
            Value::DNA(BioSequence { data: "ATCG".into() }),
            Value::Int(2),
        ],
    ).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("AT".into()),
            Value::Str("TC".into()),
            Value::Str("CG".into()),
        ])
    );
}

#[test]
fn test_subseq() {
    let result = call_bio_builtin(
        "subseq",
        vec![
            Value::DNA(BioSequence { data: "ATGATCG".into() }),
            Value::Int(2),
            Value::Int(5),
        ],
    ).unwrap();
    assert_eq!(
        result,
        Value::DNA(BioSequence { data: "GAT".into() })
    );
}

// ── Bio accessor tests ──────────────────────────────────────────

#[test]
fn test_to_interval_record() {
    let mut fields = HashMap::new();
    fields.insert("chrom".to_string(), Value::Str("chr1".into()));
    fields.insert("start".to_string(), Value::Int(1000));
    fields.insert("end".to_string(), Value::Int(2000));
    fields.insert("strand".to_string(), Value::Str("+".into()));

    let result = call_bio_builtin("to_interval", vec![Value::Record(fields)]).unwrap();
    assert_eq!(
        result,
        Value::Interval(GenomicInterval {
            chrom: "chr1".into(),
            start: 1000,
            end: 2000,
            strand: Strand::Plus,
        })
    );
}

#[test]
fn test_to_interval_gff_record() {
    let mut fields = HashMap::new();
    fields.insert("seqid".to_string(), Value::Str("chrX".into()));
    fields.insert("start".to_string(), Value::Int(500));
    fields.insert("end".to_string(), Value::Int(600));

    let result = call_bio_builtin("to_interval", vec![Value::Record(fields)]).unwrap();
    assert_eq!(
        result,
        Value::Interval(GenomicInterval {
            chrom: "chrX".into(),
            start: 500,
            end: 600,
            strand: Strand::Unknown,
        })
    );
}

#[test]
fn test_to_interval_table() {
    let t = Table::new(
        vec!["chrom".into(), "start".into(), "end".into(), "strand".into()],
        vec![
            vec![Value::Str("chr1".into()), Value::Int(100), Value::Int(200), Value::Str("+".into())],
            vec![Value::Str("chr2".into()), Value::Int(300), Value::Int(400), Value::Str("-".into())],
        ],
    );
    let result = call_bio_builtin("to_interval", vec![Value::Table(t)]).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Interval(GenomicInterval { chrom: "chr1".into(), start: 100, end: 200, strand: Strand::Plus }),
            Value::Interval(GenomicInterval { chrom: "chr2".into(), start: 300, end: 400, strand: Strand::Minus }),
        ])
    );
}

#[test]
fn test_to_interval_passthrough() {
    let iv = Value::Interval(GenomicInterval {
        chrom: "chr1".into(), start: 0, end: 100, strand: Strand::Unknown,
    });
    let result = call_bio_builtin("to_interval", vec![iv.clone()]).unwrap();
    assert_eq!(result, iv);
}

#[test]
fn test_parse_info_full() {
    let info = Value::Str("DP=100;AF=0.5;DB;AC=1,2".into());
    let result = call_bio_builtin("parse_info", vec![info]).unwrap();
    if let Value::Record(fields) = result {
        assert_eq!(fields["DP"], Value::Int(100));
        assert_eq!(fields["AF"], Value::Float(0.5));
        assert_eq!(fields["DB"], Value::Bool(true));
        assert_eq!(fields["AC"], Value::List(vec![Value::Int(1), Value::Int(2)]));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_parse_info_key() {
    let info = Value::Str("DP=100;AF=0.5;DB".into());
    let result = call_bio_builtin("parse_info", vec![info.clone(), Value::Str("DP".into())]).unwrap();
    assert_eq!(result, Value::Int(100));

    let result2 = call_bio_builtin("parse_info", vec![info.clone(), Value::Str("DB".into())]).unwrap();
    assert_eq!(result2, Value::Bool(true));

    let result3 = call_bio_builtin("parse_info", vec![info, Value::Str("MISSING".into())]).unwrap();
    assert_eq!(result3, Value::Nil);
}

#[test]
fn test_parse_info_from_record() {
    let mut fields = HashMap::new();
    fields.insert("chrom".to_string(), Value::Str("chr1".into()));
    fields.insert("info".to_string(), Value::Str("DP=50;AF=0.1".into()));

    let result = call_bio_builtin("parse_info", vec![Value::Record(fields), Value::Str("DP".into())]).unwrap();
    assert_eq!(result, Value::Int(50));
}

#[test]
fn test_parse_attr_gff3() {
    let attr = Value::Str("ID=gene1;Name=TP53;Dbxref=GeneID:1234,HGNC:5678".into());
    let result = call_bio_builtin("parse_attr", vec![attr]).unwrap();
    if let Value::Record(fields) = result {
        assert_eq!(fields["ID"], Value::Str("gene1".into()));
        assert_eq!(fields["Name"], Value::Str("TP53".into()));
        assert_eq!(
            fields["Dbxref"],
            Value::List(vec![Value::Str("GeneID:1234".into()), Value::Str("HGNC:5678".into())])
        );
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_parse_attr_gtf() {
    let attr = Value::Str(r#"gene_id "ENSG00000141510"; gene_name "TP53"; biotype "protein_coding""#.into());
    let result = call_bio_builtin("parse_attr", vec![attr]).unwrap();
    if let Value::Record(fields) = result {
        assert_eq!(fields["gene_id"], Value::Str("ENSG00000141510".into()));
        assert_eq!(fields["gene_name"], Value::Str("TP53".into()));
        assert_eq!(fields["biotype"], Value::Str("protein_coding".into()));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_parse_attr_key() {
    let attr = Value::Str("ID=gene1;Name=TP53".into());
    let result = call_bio_builtin("parse_attr", vec![attr, Value::Str("Name".into())]).unwrap();
    assert_eq!(result, Value::Str("TP53".into()));
}

#[test]
fn test_parse_qual() {
    let result = call_bio_builtin("parse_qual", vec![Value::Str("!I~".into())]).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(0), Value::Int(40), Value::Int(93)]));
}

#[test]
fn test_parse_qual_from_record() {
    let mut fields = HashMap::new();
    fields.insert("quality".to_string(), Value::Str("II".into()));
    let result = call_bio_builtin("parse_qual", vec![Value::Record(fields)]).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(40), Value::Int(40)]));
}

#[test]
fn test_base_counts() {
    let result = call_bio_builtin(
        "base_counts",
        vec![Value::DNA(BioSequence { data: "AATGCC".into() })],
    ).unwrap();
    if let Value::Record(fields) = result {
        assert_eq!(fields["A"], Value::Int(2));
        assert_eq!(fields["T"], Value::Int(1));
        assert_eq!(fields["G"], Value::Int(1));
        assert_eq!(fields["C"], Value::Int(2));
        assert_eq!(fields["N"], Value::Int(0));
        assert_eq!(fields["total"], Value::Int(6));
        assert_eq!(fields["GC"], Value::Float(0.5));
    } else {
        panic!("expected Record");
    }
}
