"""Cross-check BioLang's paired pseudobulk example with SciPy."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

import numpy as np
from scipy.stats import ttest_rel


GENES = ["IFIT1", "ACTB", "MS4A1", "CD3D", "MT-ND1"]


def profiles() -> tuple[np.ndarray, list[str]]:
    rows: list[np.ndarray] = []
    names: list[str] = []
    for donor in range(4):
        for condition in ("control", "treated"):
            cells = []
            for replicate in range(8):
                interferon = (
                    10.0 + donor * 2.0 + replicate % 3
                    if condition == "treated"
                    else 0.0
                )
                cells.append(
                    [
                        5.0 + interferon,
                        60.0 + replicate,
                        2.0,
                        18.0,
                        2.0 + replicate % 2,
                    ]
                )
            counts = np.asarray(cells).sum(axis=0)
            rows.append(np.log2(1.0 + 1_000_000.0 * counts / counts.sum()))
            names.append(f"D{donor + 1}@@{condition}")
    return np.asarray(rows), names


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--biolang",
        type=Path,
        default=Path("singlecell-results/paired-de.csv"),
    )
    args = parser.parse_args()

    matrix, names = profiles()
    control = matrix[[i for i, name in enumerate(names) if name.endswith("@@control")]]
    treated = matrix[[i for i, name in enumerate(names) if name.endswith("@@treated")]]
    effects = treated.mean(axis=0) - control.mean(axis=0)
    tests = [ttest_rel(control[:, i], treated[:, i]) for i in range(len(GENES))]

    with args.biolang.open(newline="", encoding="utf-8") as handle:
        biolang = {row["gene"]: row for row in csv.DictReader(handle)}

    for i, gene in enumerate(GENES):
        observed = float(biolang[gene]["log2fc"])
        if not np.isclose(observed, effects[i], atol=1e-9):
            raise SystemExit(
                f"{gene}: BioLang log2fc={observed}, SciPy log2fc={effects[i]}"
            )

    if not np.isclose(
        float(biolang["IFIT1"]["pvalue"]), tests[0].pvalue, atol=1e-10
    ):
        raise SystemExit("IFIT1 paired p-value differs from SciPy")

    print(
        "advanced validation: BioLang and SciPy agree "
        f"(IFIT1 log2fc={effects[0]:.6f}, p={tests[0].pvalue:.6g})"
    )


if __name__ == "__main__":
    main()
