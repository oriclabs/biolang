use bio_core::vcf_ops::*;

// -- parse_gt --

#[test]
fn test_parse_gt_diploid_het() {
    let result = parse_gt("GT", "0/1").unwrap();
    assert_eq!(result, vec![Some(0), Some(1)]);
}

#[test]
fn test_parse_gt_diploid_hom() {
    let result = parse_gt("GT", "1/1").unwrap();
    assert_eq!(result, vec![Some(1), Some(1)]);
}

#[test]
fn test_parse_gt_phased() {
    let result = parse_gt("GT", "0|1").unwrap();
    assert_eq!(result, vec![Some(0), Some(1)]);
}

#[test]
fn test_parse_gt_missing() {
    let result = parse_gt("GT", "./.").unwrap();
    assert_eq!(result, vec![None, None]);
}

#[test]
fn test_parse_gt_with_extra_fields() {
    let result = parse_gt("GT:DP:GQ", "0/1:30:99").unwrap();
    assert_eq!(result, vec![Some(0), Some(1)]);
}

#[test]
fn test_parse_gt_no_gt_field() {
    assert!(parse_gt("DP:GQ", "30:99").is_none());
}

#[test]
fn test_parse_gt_multiallelic() {
    let result = parse_gt("GT", "0/2").unwrap();
    assert_eq!(result, vec![Some(0), Some(2)]);
}

// -- is_transition --

#[test]
fn test_is_transition_ag() {
    assert!(is_transition("A", "G"));
    assert!(is_transition("G", "A"));
}

#[test]
fn test_is_transition_ct() {
    assert!(is_transition("C", "T"));
    assert!(is_transition("T", "C"));
}

#[test]
fn test_is_transversion() {
    assert!(!is_transition("A", "T"));
    assert!(!is_transition("A", "C"));
    assert!(!is_transition("G", "T"));
    assert!(!is_transition("G", "C"));
}

// -- classify_variant --

#[test]
fn test_classify_snp() {
    assert_eq!(classify_variant("A", "G"), VariantType::Snp);
}

#[test]
fn test_classify_indel() {
    assert_eq!(classify_variant("AC", "A"), VariantType::Indel);
    assert_eq!(classify_variant("A", "AT"), VariantType::Indel);
}

#[test]
fn test_classify_mnp() {
    assert_eq!(classify_variant("AC", "GT"), VariantType::Mnp);
}

#[test]
fn test_classify_star() {
    assert_eq!(classify_variant("A", "*"), VariantType::Other);
}

// -- summarize_variants --

#[test]
fn test_summarize_variants() {
    let variants: Vec<(&str, &[&str])> = vec![
        ("A", &["G"] as &[&str]),         // SNP, transition
        ("C", &["T"]),                     // SNP, transition
        ("A", &["T"]),                     // SNP, transversion
        ("AC", &["A"]),                    // Indel
        ("G", &["A", "T"] as &[&str]),     // multi: 2 SNPs (Ts + Tv)
    ];
    let summary = summarize_variants(&variants);
    assert_eq!(summary.snp, 5); // A>G, C>T, A>T, G>A, G>T
    assert_eq!(summary.indel, 1);
    assert_eq!(summary.transitions, 3); // A>G, C>T, G>A
    assert_eq!(summary.transversions, 2); // A>T, G>T
    assert_eq!(summary.multiallelic, 1);
    assert!((summary.ts_tv_ratio - 1.5).abs() < 0.01);
}

#[test]
fn test_summarize_empty() {
    let summary = summarize_variants(&[]);
    assert_eq!(summary.snp, 0);
    assert_eq!(summary.ts_tv_ratio, 0.0);
}
