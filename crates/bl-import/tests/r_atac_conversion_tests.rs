use bl_import::convert;

#[test]
fn signac_preprocessing_maps_to_exact_atac_operations() {
    let source = r#"
library(Signac)
object <- RunTFIDF(object)
object <- FindTopFeatures(object, min.cutoff = 20)
object <- RunSVD(object)
DepthCor(object)
"#;
    let converted = convert(source, "r", "analysis.R");
    assert!(converted.contains("import \"atac\" as atac"), "{converted}");
    assert!(converted.contains("atac.tfidf(object)"), "{converted}");
    assert!(
        converted.contains("atac.top_features(object, 20)"),
        "{converted}"
    );
    assert!(converted.contains("atac.lsi(object)"), "{converted}");
    assert!(converted.contains("atac.depth_cor(object)"), "{converted}");
    assert!(
        !converted.contains("normalize_total(object)"),
        "{converted}"
    );
    assert!(!converted.contains("pca(object)"), "{converted}");
}
