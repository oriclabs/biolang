"""Compare independently generated HBC Seurat and BioLang artifacts.

This script is validation tooling, not a BioLang runtime dependency. Cell IDs
are joined by sample plus original 10x barcode; numeric cluster labels are never
assumed to have the same meaning across implementations.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import pandas as pd
from pynndescent import NNDescent
from scipy.optimize import linear_sum_assignment
from sklearn.metrics import adjusted_mutual_info_score, adjusted_rand_score
from sklearn.neighbors import NearestNeighbors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("seurat_dir", type=Path)
    parser.add_argument("biolang_dir", type=Path)
    parser.add_argument("output_json", type=Path)
    return parser.parse_args()


def cell_key(frame: pd.DataFrame) -> pd.Series:
    return frame["sample"].astype(str) + "::" + frame["barcode"].astype(str)


def remove_self(neighbors: np.ndarray, wanted: int) -> np.ndarray:
    rows = np.arange(neighbors.shape[0])[:, None]
    result = np.empty((neighbors.shape[0], wanted), dtype=np.int32)
    for row in range(neighbors.shape[0]):
        values = neighbors[row][neighbors[row] != rows[row, 0]]
        if len(values) < wanted:
            raise RuntimeError(f"neighbor row {row} has only {len(values)} non-self entries")
        result[row] = values[:wanted]
    return result


def mean_neighbor_jaccard(left: np.ndarray, right: np.ndarray) -> float:
    if left.shape != right.shape:
        raise ValueError(f"neighbor shapes differ: {left.shape} vs {right.shape}")
    scores = np.empty(left.shape[0], dtype=np.float64)
    for row in range(left.shape[0]):
        intersection = len(set(left[row]).intersection(right[row]))
        scores[row] = intersection / (2 * left.shape[1] - intersection)
    return float(scores.mean())


def approximate_pc_neighbors(values: np.ndarray, seed: int, k: int) -> np.ndarray:
    index = NNDescent(
        values,
        n_neighbors=max(30, k + 1),
        metric="euclidean",
        random_state=seed,
        n_jobs=-1,
    )
    neighbors, _ = index.neighbor_graph
    return remove_self(neighbors, k)


def exact_2d_neighbors(values: np.ndarray, k: int) -> np.ndarray:
    model = NearestNeighbors(n_neighbors=k + 1, algorithm="kd_tree")
    neighbors = model.fit(values).kneighbors(return_distance=False)
    return remove_self(neighbors, k)


def main() -> None:
    args = parse_args()
    seurat_cells = pd.read_csv(args.seurat_dir / "cells.csv.gz")
    biolang_cells = pd.read_csv(args.biolang_dir / "cells.csv")
    seurat_cells["key"] = cell_key(seurat_cells)
    biolang_cells["key"] = cell_key(biolang_cells)

    if seurat_cells["key"].duplicated().any() or biolang_cells["key"].duplicated().any():
        raise RuntimeError("sample-plus-barcode keys are not unique")

    joined = seurat_cells.merge(
        biolang_cells,
        on="key",
        how="inner",
        suffixes=("_seurat", "_biolang"),
        validate="one_to_one",
    ).sort_values("key", kind="stable")
    if len(joined) != len(seurat_cells) or len(joined) != len(biolang_cells):
        raise RuntimeError(
            "cell identity mismatch: "
            f"joined={len(joined)} seurat={len(seurat_cells)} biolang={len(biolang_cells)}"
        )

    seurat_labels = joined["cluster_seurat"].astype(str)
    biolang_labels = joined["cluster_biolang"].astype(str)
    contingency = pd.crosstab(biolang_labels, seurat_labels)
    row_indices, column_indices = linear_sum_assignment(-contingency.to_numpy())
    mapped_correct = int(contingency.to_numpy()[row_indices, column_indices].sum())
    mapping = {
        str(contingency.index[row]): str(contingency.columns[column])
        for row, column in zip(row_indices, column_indices, strict=True)
    }

    seurat_features = set(
        pd.read_csv(args.seurat_dir / "integration-features.csv")["gene"].astype(str)
    )
    biolang_features = set(
        pd.read_csv(args.biolang_dir / "features.csv")["gene"].astype(str)
    )
    feature_intersection = len(seurat_features & biolang_features)
    feature_union = len(seurat_features | biolang_features)

    # Re-index both PC matrices to the already-verified joined cell order.
    seurat_pcs = pd.read_csv(args.seurat_dir / "pcs.csv.gz")
    biolang_pcs = pd.read_csv(args.biolang_dir / "pcs.csv")
    seurat_pcs["key"] = cell_key(seurat_pcs)
    biolang_pcs["key"] = cell_key(biolang_pcs)
    ordered_keys = joined["key"]
    seurat_pcs = seurat_pcs.set_index("key").loc[ordered_keys]
    biolang_pcs = biolang_pcs.set_index("key").loc[ordered_keys]
    seurat_pc_values = seurat_pcs.filter(regex=r"^PC_[0-9]+$").iloc[:, :40].to_numpy()
    biolang_pc_values = biolang_pcs.filter(regex=r"^pc_[0-9]+$").iloc[:, :40].to_numpy()
    if seurat_pc_values.shape != biolang_pc_values.shape or seurat_pc_values.shape[1] != 40:
        raise RuntimeError(
            f"expected matching 40-PC matrices, got {seurat_pc_values.shape} and {biolang_pc_values.shape}"
        )

    k = 15
    seurat_pc_neighbors = approximate_pc_neighbors(seurat_pc_values, 123456, k)
    biolang_pc_neighbors = approximate_pc_neighbors(biolang_pc_values, 123456, k)
    seurat_umap_neighbors = exact_2d_neighbors(
        joined[["umap_1_seurat", "umap_2_seurat"]].to_numpy(), k
    )
    biolang_umap_neighbors = exact_2d_neighbors(
        joined[["umap_1_biolang", "umap_2_biolang"]].to_numpy(), k
    )

    results = {
        "joined_cells": int(len(joined)),
        "seurat_cells": int(len(seurat_cells)),
        "biolang_cells": int(len(biolang_cells)),
        "seurat_clusters": int(seurat_labels.nunique()),
        "biolang_clusters": int(biolang_labels.nunique()),
        "adjusted_rand_index": float(adjusted_rand_score(seurat_labels, biolang_labels)),
        "adjusted_mutual_information": float(
            adjusted_mutual_info_score(seurat_labels, biolang_labels)
        ),
        "one_to_one_mapped_accuracy": mapped_correct / len(joined),
        "one_to_one_cluster_mapping": mapping,
        "seurat_integration_features": len(seurat_features),
        "biolang_integration_features": len(biolang_features),
        "feature_intersection": feature_intersection,
        "feature_overlap_min_set": feature_intersection
        / min(len(seurat_features), len(biolang_features)),
        "feature_jaccard": feature_intersection / feature_union,
        "pc_15nn_mean_jaccard": mean_neighbor_jaccard(
            seurat_pc_neighbors, biolang_pc_neighbors
        ),
        "pc_neighbor_method": "NNDescent(seed=123456,n_neighbors=30)",
        "umap_15nn_mean_jaccard": mean_neighbor_jaccard(
            seurat_umap_neighbors, biolang_umap_neighbors
        ),
        "umap_neighbor_method": "exact-kd-tree",
    }

    seurat_marker_path = args.seurat_dir / "markers.csv"
    biolang_marker_path = args.biolang_dir / "markers.csv"
    if seurat_marker_path.exists() and biolang_marker_path.exists():
        seurat_markers = pd.read_csv(seurat_marker_path)
        biolang_markers = pd.read_csv(biolang_marker_path)
        seurat_markers["cluster"] = seurat_markers["cluster"].astype(str)
        biolang_markers["cluster"] = biolang_markers["cluster"].astype(str)
        biolang_markers["mapped_cluster"] = biolang_markers["cluster"].map(mapping)

        seurat_pairs = set(zip(seurat_markers["cluster"], seurat_markers["gene"]))
        biolang_pairs = set(
            zip(biolang_markers["mapped_cluster"], biolang_markers["gene"])
        )
        marker_intersection = len(seurat_pairs & biolang_pairs)
        marker_union = len(seurat_pairs | biolang_pairs)

        top_n = 50
        seurat_top_pairs: set[tuple[str, str]] = set()
        biolang_top_pairs: set[tuple[str, str]] = set()
        cluster_top50_overlaps: dict[str, int] = {}
        for biolang_cluster, seurat_cluster in mapping.items():
            seurat_top = (
                seurat_markers[seurat_markers["cluster"] == seurat_cluster]
                .sort_values(["avg_log2FC", "gene"], ascending=[False, True])
                .head(top_n)
            )
            biolang_top = (
                biolang_markers[biolang_markers["cluster"] == biolang_cluster]
                .sort_values(["avg_log2fc", "gene"], ascending=[False, True])
                .head(top_n)
            )
            seurat_genes = set(seurat_top["gene"].astype(str))
            biolang_genes = set(biolang_top["gene"].astype(str))
            cluster_top50_overlaps[biolang_cluster] = len(seurat_genes & biolang_genes)
            seurat_top_pairs.update((seurat_cluster, gene) for gene in seurat_genes)
            biolang_top_pairs.update((seurat_cluster, gene) for gene in biolang_genes)

        canonical_panel = [
            "CD3D", "IL7R", "CD8A", "GNLY", "NKG7", "MS4A1", "CD79A",
            "CD14", "LYZ", "S100A8", "FCGR3A", "MS4A7", "FCER1A", "CST3", "PPBP",
        ]
        canonical_peak_matches: dict[str, bool] = {}
        for gene in canonical_panel:
            seurat_gene = seurat_markers[seurat_markers["gene"] == gene]
            biolang_gene = biolang_markers[biolang_markers["gene"] == gene]
            if seurat_gene.empty or biolang_gene.empty:
                canonical_peak_matches[gene] = False
                continue
            seurat_peak = str(
                seurat_gene.sort_values("avg_log2FC", ascending=False).iloc[0]["cluster"]
            )
            biolang_peak = str(
                biolang_gene.sort_values("avg_log2fc", ascending=False).iloc[0]["cluster"]
            )
            canonical_peak_matches[gene] = mapping.get(biolang_peak) == seurat_peak

        results.update(
            {
                "seurat_positive_marker_rows": int(len(seurat_markers)),
                "biolang_positive_marker_rows": int(len(biolang_markers)),
                "mapped_marker_pair_intersection": marker_intersection,
                "mapped_marker_pair_recall_min_set": marker_intersection
                / min(len(seurat_pairs), len(biolang_pairs)),
                "mapped_marker_pair_jaccard": marker_intersection / marker_union,
                "mapped_top50_marker_pair_intersection": len(
                    seurat_top_pairs & biolang_top_pairs
                ),
                "mapped_top50_marker_pair_recall": len(
                    seurat_top_pairs & biolang_top_pairs
                )
                / len(biolang_top_pairs),
                "mapped_top50_overlap_by_biolang_cluster": cluster_top50_overlaps,
                "canonical_panel_peak_cluster_matches": canonical_peak_matches,
                "canonical_panel_peak_match_count": sum(canonical_peak_matches.values()),
                "canonical_panel_size": len(canonical_panel),
            }
        )

    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
