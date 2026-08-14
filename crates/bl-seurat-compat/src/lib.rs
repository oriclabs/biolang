//! Compatibility kernels derived from permissively licensed upstream code.

unsafe extern "C" {
    fn bl_seurat_annoy_euclidean(
        reference: *const f32,
        reference_rows: usize,
        query: *const f32,
        query_rows: usize,
        dimensions: usize,
        wanted: usize,
        trees: i32,
        indices: *mut i32,
        distances: *mut f32,
    ) -> i32;
}

/// Run the same Spotify Annoy 1.17.3 Euclidean index/query contract used by
/// Seurat 5.5.1 through RcppAnnoy 0.0.23.
pub fn annoy_euclidean(
    reference: &[Vec<f64>],
    query: &[Vec<f64>],
    wanted: usize,
    trees: usize,
) -> Result<Vec<Vec<(usize, f64)>>, String> {
    let dimensions = reference.first().map(Vec::len).unwrap_or(0);
    if dimensions == 0 || reference.iter().any(|row| row.len() != dimensions) {
        return Err("Annoy reference matrix is empty or ragged".to_string());
    }
    if query.iter().any(|row| row.len() != dimensions) {
        return Err("Annoy query dimensions do not match the reference".to_string());
    }
    let wanted = wanted.min(reference.len());
    if wanted == 0 || query.is_empty() {
        return Ok(vec![Vec::new(); query.len()]);
    }
    let reference_flat: Vec<f32> = reference
        .iter()
        .flat_map(|row| row.iter().map(|&value| value as f32))
        .collect();
    let query_flat: Vec<f32> = query
        .iter()
        .flat_map(|row| row.iter().map(|&value| value as f32))
        .collect();
    let mut indices = vec![-1_i32; query.len() * wanted];
    let mut distances = vec![0.0_f32; query.len() * wanted];
    // The bridge owns all pointed-to buffers for the duration of the call and
    // writes exactly query.len() * wanted output entries on success.
    let status = unsafe {
        bl_seurat_annoy_euclidean(
            reference_flat.as_ptr(),
            reference.len(),
            query_flat.as_ptr(),
            query.len(),
            dimensions,
            wanted,
            trees as i32,
            indices.as_mut_ptr(),
            distances.as_mut_ptr(),
        )
    };
    if status != 1 {
        return Err(format!("Spotify Annoy bridge failed with status {status}"));
    }
    Ok(indices
        .chunks_exact(wanted)
        .zip(distances.chunks_exact(wanted))
        .map(|(row_indices, row_distances)| {
            row_indices
                .iter()
                .zip(row_distances)
                .map(|(&index, &distance)| (index as usize, distance as f64))
                .collect()
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::annoy_euclidean;

    #[test]
    fn euclidean_index_finds_the_expected_small_neighbours() {
        let data = vec![vec![2.0, 2.0], vec![3.0, 2.0], vec![3.0, 3.0]];
        let found = annoy_euclidean(&data, &[vec![4.0, 4.0]], 3, 50).unwrap();
        assert_eq!(
            found[0].iter().map(|pair| pair.0).collect::<Vec<_>>(),
            vec![2, 1, 0]
        );
    }
}
