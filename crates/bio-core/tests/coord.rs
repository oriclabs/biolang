use bio_core::CoordSystem;

#[test]
fn test_bed_to_vcf() {
    let (s, e) = CoordSystem::convert(CoordSystem::Bed, CoordSystem::Vcf, 100, 200);
    assert_eq!((s, e), (101, 200));
}

#[test]
fn test_vcf_to_bed() {
    let (s, e) = CoordSystem::convert(CoordSystem::Vcf, CoordSystem::Bed, 101, 200);
    assert_eq!((s, e), (100, 200));
}

#[test]
fn test_compatibility() {
    assert!(CoordSystem::Vcf.is_compatible(&CoordSystem::Gff));
    assert!(!CoordSystem::Bed.is_compatible(&CoordSystem::Vcf));
    assert!(CoordSystem::Unknown.is_compatible(&CoordSystem::Bed));
}
