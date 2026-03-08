use bio_core::gene::{QualityOps, Genome};

#[test]
fn test_quality_from_ascii() {
    let scores = QualityOps::from_ascii("IIIII");
    assert_eq!(scores, vec![40, 40, 40, 40, 40]);
}

#[test]
fn test_mean_phred() {
    let scores = vec![30, 30, 30, 30];
    assert!((QualityOps::mean_phred(&scores) - 30.0).abs() < 0.01);
}

#[test]
fn test_genome_registry() {
    let g = Genome::from_name("GRCh38").unwrap();
    assert_eq!(g.species, "Homo sapiens");
    assert!(!g.chromosomes.is_empty());
}
