// Thin MIT bridge over Spotify Annoy 1.17.3 (Apache-2.0).
#include "annoylib.h"
#include "kissrandom.h"
#include <cstdint>
#include <vector>

using SeuratAnnoyIndex = Annoy::AnnoyIndex<
    int32_t, float, Annoy::Euclidean, Kiss64Random,
    Annoy::AnnoyIndexSingleThreadedBuildPolicy>;

extern "C" int bl_seurat_annoy_euclidean(
    const float* reference, size_t reference_rows,
    const float* query, size_t query_rows, size_t dimensions,
    size_t wanted, int trees, int32_t* indices, float* distances) {
  if (!reference || !query || !indices || !distances || dimensions == 0 ||
      wanted == 0 || reference_rows == 0 || trees <= 0) {
    return 0;
  }
  SeuratAnnoyIndex index(static_cast<int>(dimensions));
  char* error = nullptr;
  for (size_t row = 0; row < reference_rows; ++row) {
    if (!index.add_item(static_cast<int32_t>(row),
                        reference + row * dimensions, &error)) {
      return -1;
    }
  }
  if (!index.build(trees, -1, &error)) {
    return -2;
  }
  for (size_t row = 0; row < query_rows; ++row) {
    std::vector<int32_t> found;
    std::vector<float> found_distances;
    index.get_nns_by_vector(query + row * dimensions, wanted, -1,
                            &found, &found_distances);
    if (found.size() != wanted || found_distances.size() != wanted) {
      return -3;
    }
    for (size_t column = 0; column < wanted; ++column) {
      indices[row * wanted + column] = found[column];
      distances[row * wanted + column] = found_distances[column];
    }
  }
  return 1;
}

