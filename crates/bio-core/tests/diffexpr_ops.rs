use bio_core::diffexpr_ops::*;

#[test]
fn test_diff_expr() {
    // 3 genes, 6 samples (3 per group)
    let counts = vec![
        vec![10.0, 12.0, 11.0, 50.0, 48.0, 52.0], // significantly different
        vec![20.0, 21.0, 19.0, 20.0, 22.0, 18.0],  // not different
        vec![5.0, 6.0, 4.0, 100.0, 95.0, 105.0],   // very different
    ];
    let groups = vec![0, 0, 0, 1, 1, 1];
    let names = vec!["GENE_A".into(), "GENE_B".into(), "GENE_C".into()];

    let results = diff_expr(&counts, &groups, Some(&names));
    assert_eq!(results.len(), 3);

    // GENE_A should have significant fold change
    assert!(results[0].log2fc.abs() > 1.0);
    // GENE_B should have small fold change
    assert!(results[1].log2fc.abs() < 0.5);
}

#[test]
fn test_normal_cdf() {
    // Access normal_cdf indirectly through diff_expr behavior
    // The module's normal_cdf is private, so we test it through diff_expr
    // Test that diff_expr returns reasonable p-values
    let counts = vec![
        vec![10.0, 10.0, 10.0, 10.0, 10.0, 10.0], // identical groups
    ];
    let groups = vec![0, 0, 0, 1, 1, 1];
    let results = diff_expr(&counts, &groups, None);
    assert_eq!(results.len(), 1);
    // p-value should be high (not significant) for identical groups
    assert!(results[0].pvalue > 0.5);
}
