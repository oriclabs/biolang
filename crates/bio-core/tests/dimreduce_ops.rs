use bio_core::dimreduce_ops::*;

#[test]
fn test_tsne_basic() {
    let data = vec![
        vec![0.0, 0.0],
        vec![1.0, 1.0],
        vec![2.0, 2.0],
        vec![10.0, 10.0],
        vec![11.0, 11.0],
    ];
    let result = tsne(&data, 2, 2.0, 100, 10.0);
    assert_eq!(result.len(), 5);
    assert_eq!(result[0].len(), 2);
}

#[test]
fn test_umap_basic() {
    let data = vec![
        vec![0.0, 0.0],
        vec![1.0, 1.0],
        vec![2.0, 2.0],
        vec![10.0, 10.0],
        vec![11.0, 11.0],
    ];
    let result = umap(&data, 2, 3, 50, 0.1);
    assert_eq!(result.len(), 5);
    assert_eq!(result[0].len(), 2);
}
