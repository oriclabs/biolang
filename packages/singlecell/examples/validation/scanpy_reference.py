"""Run the package fixture through a seeded Scanpy reference workflow."""

from __future__ import annotations

import argparse
import importlib.metadata
import json
from pathlib import Path

import numpy as np
import pandas as pd
import scanpy as sc


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--metrics", type=Path, default=Path("validation_scanpy_metrics.json"))
    args = parser.parse_args()

    np.random.seed(0)
    adata = sc.read_10x_mtx(
        args.input,
        var_names="gene_symbols",
        make_unique=True,
        cache=False,
    )
    adata.var["mt"] = adata.var_names.str.startswith("MT-")
    sc.pp.calculate_qc_metrics(
        adata,
        qc_vars=["mt"],
        percent_top=None,
        log1p=False,
        inplace=True,
    )
    sc.pp.filter_genes(adata, min_cells=3)
    keep = (
        (adata.obs["n_genes_by_counts"] >= 20)
        & (adata.obs["n_genes_by_counts"] <= 5000)
        & (adata.obs["pct_counts_mt"] <= 25.0)
    )
    adata = adata[keep].copy()
    adata.layers["counts"] = adata.X.copy()

    sc.pp.normalize_total(adata, target_sum=10_000)
    sc.pp.log1p(adata)
    sc.pp.highly_variable_genes(
        adata,
        n_top_genes=min(2000, adata.n_vars),
        flavor="seurat",
    )
    adata = adata[:, adata.var["highly_variable"]].copy()
    n_pcs = min(30, adata.n_obs - 1, adata.n_vars - 1)
    sc.pp.pca(adata, n_comps=n_pcs, random_state=0)
    sc.pp.neighbors(adata, n_neighbors=15, n_pcs=n_pcs, random_state=0)
    sc.tl.leiden(
        adata,
        resolution=0.5,
        random_state=0,
        key_added="cluster",
        flavor="igraph",
        n_iterations=2,
        directed=False,
    )

    pd.DataFrame(
        {"barcode": adata.obs_names, "cluster": adata.obs["cluster"].astype(str)}
    ).to_csv(args.output, index=False)
    args.metrics.write_text(
        json.dumps(
            {
                "cells": int(adata.n_obs),
                "genes": int(adata.n_vars),
                "clusters": int(adata.obs["cluster"].nunique()),
                "variance_ratio": adata.uns["pca"]["variance_ratio"].tolist(),
                "scanpy_version": importlib.metadata.version("scanpy"),
            },
            indent=2,
        ),
        encoding="utf-8",
    )
    print(
        f"SCANPY_VALIDATION_OK cells={adata.n_obs} "
        f"clusters={adata.obs['cluster'].nunique()}"
    )


if __name__ == "__main__":
    main()
