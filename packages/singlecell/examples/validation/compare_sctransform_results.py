#!/usr/bin/env python3
"""Compare standalone sctransform oracle observations with BioLang output."""

from __future__ import annotations

import csv
import json
import math
import statistics
import sys
from pathlib import Path


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8-sig") as handle:
        return list(csv.DictReader(handle))


def read_json(path: Path) -> dict[str, object]:
    with path.open(encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object in {path}")
    return value


def finite_pairs(left: list[float], right: list[float]) -> tuple[list[float], list[float]]:
    pairs = [(a, b) for a, b in zip(left, right) if math.isfinite(a) and math.isfinite(b)]
    return [a for a, _ in pairs], [b for _, b in pairs]


def pearson(left: list[float], right: list[float]) -> float | None:
    left, right = finite_pairs(left, right)
    if len(left) < 2:
        return None
    mean_left = statistics.fmean(left)
    mean_right = statistics.fmean(right)
    numerator = sum((a - mean_left) * (b - mean_right) for a, b in zip(left, right))
    denominator = math.sqrt(
        sum((a - mean_left) ** 2 for a in left)
        * sum((b - mean_right) ** 2 for b in right)
    )
    return numerator / denominator if denominator > 0 else None


def ranks(values: list[float]) -> list[float]:
    order = sorted(range(len(values)), key=values.__getitem__)
    result = [0.0] * len(values)
    start = 0
    while start < len(order):
        end = start + 1
        while end < len(order) and values[order[end]] == values[order[start]]:
            end += 1
        rank = (start + end - 1) / 2.0 + 1.0
        for index in order[start:end]:
            result[index] = rank
        start = end
    return result


def spearman(left: list[float], right: list[float]) -> float | None:
    left, right = finite_pairs(left, right)
    return pearson(ranks(left), ranks(right)) if len(left) >= 2 else None


def rmse(left: list[float], right: list[float]) -> float | None:
    left, right = finite_pairs(left, right)
    if not left:
        return None
    return math.sqrt(statistics.fmean((a - b) ** 2 for a, b in zip(left, right)))


def linear_fit(reference: list[float], observed: list[float]) -> tuple[float | None, float | None]:
    """Return observed = intercept + slope * reference for finite pairs."""
    reference, observed = finite_pairs(reference, observed)
    if len(reference) < 2:
        return None, None
    mean_reference = statistics.fmean(reference)
    mean_observed = statistics.fmean(observed)
    denominator = sum((value - mean_reference) ** 2 for value in reference)
    if denominator == 0:
        return None, None
    slope = sum(
        (left - mean_reference) * (right - mean_observed)
        for left, right in zip(reference, observed)
    ) / denominator
    return slope, mean_observed - slope * mean_reference


def percentile(values: list[float], probability: float) -> float | None:
    """Linearly interpolated percentile, matching the usual type-7 definition."""
    finite = sorted(value for value in values if math.isfinite(value))
    if not finite:
        return None
    position = (len(finite) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return finite[lower]
    fraction = position - lower
    return finite[lower] * (1.0 - fraction) + finite[upper] * fraction


def relative_errors(
    reference: list[float], observed: list[float], *, min_abs_reference: float = 0.0
) -> list[float]:
    return [
        abs(right - left) / abs(left)
        for left, right in zip(reference, observed)
        if math.isfinite(left)
        and math.isfinite(right)
        and abs(left) > min_abs_reference
    ]


def sample_sd(values: list[float]) -> float | None:
    finite = [value for value in values if math.isfinite(value)]
    return statistics.stdev(finite) if len(finite) >= 2 else None


def main() -> int:
    if len(sys.argv) not in (4, 5):
        raise SystemExit(
            "usage: compare_sctransform_results.py ORACLE_DIR BIOLANG_DIR OUTPUT_JSON [TOP_N]"
        )
    oracle_dir, biolang_dir, output_path = map(Path, sys.argv[1:4])
    oracle_genes = {row["gene"]: row for row in read_rows(oracle_dir / "genes.csv")}
    biolang_genes = {row["gene"]: row for row in read_rows(biolang_dir / "genes.csv")}
    joined = sorted(oracle_genes.keys() & biolang_genes.keys())

    def values(source: dict[str, dict[str, str]], column: str) -> list[float]:
        return [float(source[gene][column]) for gene in joined]

    oracle_theta = values(oracle_genes, "theta")
    biolang_theta = values(biolang_genes, "theta")
    oracle_log_theta = [math.log10(value) if value > 0 else math.nan for value in oracle_theta]
    biolang_log_theta = [math.log10(value) if value > 0 else math.nan for value in biolang_theta]
    oracle_intercept = values(oracle_genes, "intercept")
    biolang_intercept = values(biolang_genes, "intercept")
    oracle_variance = values(oracle_genes, "residual_variance")
    biolang_variance = values(biolang_genes, "residual_variance")
    theta_slope, theta_offset = linear_fit(oracle_theta, biolang_theta)
    intercept_slope, intercept_offset = linear_fit(oracle_intercept, biolang_intercept)
    variance_slope, variance_offset = linear_fit(oracle_variance, biolang_variance)
    theta_relative_error = relative_errors(oracle_theta, biolang_theta)
    variance_relative_error = relative_errors(
        oracle_variance, biolang_variance, min_abs_reference=1e-12
    )

    oracle_ranking = read_rows(oracle_dir / "ranking.csv")
    biolang_ranking = read_rows(biolang_dir / "ranking.csv")
    oracle_rank_by_gene = {
        row["gene"]: float(row["rank"]) for row in oracle_ranking
    }
    biolang_rank_by_gene = {
        row["gene"]: float(row["rank"]) for row in biolang_ranking
    }
    joined_rank_genes = sorted(oracle_rank_by_gene.keys() & biolang_rank_by_gene.keys())
    requested_top_n = int(sys.argv[4]) if len(sys.argv) == 5 else 3000
    top_n = min(requested_top_n, len(oracle_ranking), len(biolang_ranking))
    oracle_top = {row["gene"] for row in oracle_ranking[:top_n]}
    biolang_top = {row["gene"] for row in biolang_ranking[:top_n]}
    top_intersection = len(oracle_top & biolang_top)
    joined_top_genes = sorted(oracle_top & biolang_top)

    oracle_residuals = {
        (row["gene"], row["cell"]): float(row["residual"])
        for row in read_rows(oracle_dir / "residuals.csv")
    }
    biolang_residuals = {
        (row["gene"], row["cell"]): float(row["residual"])
        for row in read_rows(biolang_dir / "residuals.csv")
    }
    residual_keys = sorted(oracle_residuals.keys() & biolang_residuals.keys())
    oracle_residual_values = [oracle_residuals[key] for key in residual_keys]
    biolang_residual_values = [biolang_residuals[key] for key in residual_keys]
    residual_slope, residual_offset = linear_fit(
        oracle_residual_values, biolang_residual_values
    )
    residual_rmse_value = rmse(oracle_residual_values, biolang_residual_values)
    oracle_residual_sd = sample_sd(oracle_residual_values)
    large_residual_relative_error = relative_errors(
        oracle_residual_values,
        biolang_residual_values,
        min_abs_reference=1.0,
    )
    per_gene_correlations = []
    residual_keys_by_gene: dict[str, list[tuple[str, str]]] = {}
    for key in residual_keys:
        residual_keys_by_gene.setdefault(key[0], []).append(key)
    for gene in sorted(residual_keys_by_gene):
        keys = residual_keys_by_gene[gene]
        correlation = pearson(
            [oracle_residuals[key] for key in keys],
            [biolang_residuals[key] for key in keys],
        )
        if correlation is not None:
            per_gene_correlations.append(correlation)

    # Compare the quantity SCTransform v2 actually regularizes.
    #
    # v2 smooths the overdispersion factor od = log10(1 + gmean/theta), not
    # theta, and recovers theta = gmean / (10^od - 1). That inversion has
    # d ln(theta)/d od = -(10^od * ln10)/(10^od - 1), which is about -25 at the
    # od values a UMI matrix produces, so a 0.003 difference in the smoothed od
    # arrives as a 7% difference in theta. Gating on theta therefore measures
    # the amplification rather than the fit. Both sides use the same abscissa -
    # the log10 geometric mean, verified identical between implementations to
    # 1e-13 - so od can be recomputed here and compared directly.
    od_differences: list[float] = []
    for gene in joined:
        try:
            log_gmean = float(biolang_genes[gene]["log_geometric_mean"])
            oracle_theta = float(oracle_genes[gene]["theta"])
            biolang_theta = float(biolang_genes[gene]["theta"])
        except (KeyError, ValueError):
            continue
        if not all(math.isfinite(v) for v in (log_gmean, oracle_theta, biolang_theta)):
            continue
        if oracle_theta <= 0 or biolang_theta <= 0:
            continue
        gmean = 10.0**log_gmean
        od_differences.append(
            math.log10(1.0 + gmean / biolang_theta)
            - math.log10(1.0 + gmean / oracle_theta)
        )
    od_absolute = [abs(value) for value in od_differences]

    # How much of the theta discrepancy survives into the residuals.
    #
    # Theta enters the residual only through the variance, r = (y - mu) /
    # sqrt(mu + mu^2/theta), so d(log r)/d(log theta) = mu / (2 * (theta + mu)).
    # That is bounded by 1/2 and small wherever mu << theta, which is most of a
    # UMI matrix - so a theta bias should arrive at the residuals heavily
    # attenuated. Reporting the attenuation measured per gene turns a failing
    # theta gate from an unexplained defect into a bounded, characterised
    # deviation, and the dose-response check confirms the two are actually
    # linked rather than coincidentally sized.
    theta_attenuation_pairs: list[tuple[float, float]] = []
    for gene in sorted(residual_keys_by_gene):
        if gene not in oracle_genes or gene not in biolang_genes:
            continue
        try:
            oracle_theta = float(oracle_genes[gene]["theta"])
            biolang_theta = float(biolang_genes[gene]["theta"])
        except (KeyError, ValueError):
            continue
        if not math.isfinite(oracle_theta) or not math.isfinite(biolang_theta):
            continue
        if oracle_theta == 0.0:
            continue
        keys = residual_keys_by_gene[gene]
        gene_residual_errors = relative_errors(
            [oracle_residuals[key] for key in keys],
            [biolang_residuals[key] for key in keys],
            min_abs_reference=1e-9,
        )
        if not gene_residual_errors:
            continue
        theta_attenuation_pairs.append(
            (
                abs(biolang_theta - oracle_theta) / abs(oracle_theta),
                percentile(gene_residual_errors, 0.5),
            )
        )
    theta_errors = [pair[0] for pair in theta_attenuation_pairs]
    gene_residual_medians = [pair[1] for pair in theta_attenuation_pairs]
    theta_error_median = percentile(theta_errors, 0.5)
    gene_residual_median = percentile(gene_residual_medians, 0.5)
    theta_to_residual_attenuation = (
        gene_residual_median / theta_error_median
        if theta_error_median not in (None, 0.0) and gene_residual_median is not None
        else None
    )

    oracle_probe_genes = {gene for gene, _ in oracle_residuals}
    biolang_probe_genes = {gene for gene, _ in biolang_residuals}
    joined_probe_genes = {gene for gene, _ in residual_keys}
    oracle_probe_variances = [
        float(oracle_genes[gene]["residual_variance"])
        for gene in oracle_probe_genes
        if gene in oracle_genes
    ]
    biolang_probe_variances = [
        float(biolang_genes[gene]["residual_variance"])
        for gene in biolang_probe_genes
        if gene in biolang_genes
    ]

    metrics = {
        "oracle_modelled_genes": len(oracle_genes),
        "biolang_modelled_genes": len(biolang_genes),
        "joined_modelled_genes": len(joined),
        "gene_set_intersection_over_oracle": len(joined) / max(1, len(oracle_genes)),
        "gene_set_jaccard": len(joined) / max(1, len(oracle_genes.keys() | biolang_genes.keys())),
        "log10_theta_pearson": pearson(oracle_log_theta, biolang_log_theta),
        "log10_theta_spearman": spearman(oracle_log_theta, biolang_log_theta),
        "theta_raw_regression_slope": theta_slope,
        "theta_raw_regression_offset": theta_offset,
        "theta_raw_relative_error_median": percentile(theta_relative_error, 0.5),
        "theta_raw_relative_error_p90": percentile(theta_relative_error, 0.9),
        "theta_raw_relative_error_max": max(theta_relative_error, default=None),
        "intercept_pearson": pearson(oracle_intercept, biolang_intercept),
        "intercept_rmse": rmse(oracle_intercept, biolang_intercept),
        "intercept_regression_slope": intercept_slope,
        "intercept_regression_offset": intercept_offset,
        "residual_variance_pearson": pearson(oracle_variance, biolang_variance),
        "residual_variance_spearman": spearman(oracle_variance, biolang_variance),
        "residual_variance_regression_slope": variance_slope,
        "residual_variance_regression_offset": variance_offset,
        "residual_variance_relative_error_median": percentile(
            variance_relative_error, 0.5
        ),
        "residual_variance_relative_error_p90": percentile(
            variance_relative_error, 0.9
        ),
        "top_feature_n": top_n,
        "top_feature_intersection": top_intersection,
        "top_feature_overlap": top_intersection / max(1, top_n),
        "top_feature_jaccard": top_intersection
        / max(1, len(oracle_top | biolang_top)),
        "feature_rank_compared_genes": len(joined_rank_genes),
        "feature_rank_spearman": spearman(
            [oracle_rank_by_gene[gene] for gene in joined_rank_genes],
            [biolang_rank_by_gene[gene] for gene in joined_rank_genes],
        ),
        "top_feature_rank_spearman_on_intersection": spearman(
            [oracle_rank_by_gene[gene] for gene in joined_top_genes],
            [biolang_rank_by_gene[gene] for gene in joined_top_genes],
        ),
        "joined_residual_observations": len(residual_keys),
        "oracle_probe_genes": len(oracle_probe_genes),
        "biolang_probe_genes": len(biolang_probe_genes),
        "joined_probe_genes": len(joined_probe_genes),
        "oracle_probe_residual_variance_min": min(oracle_probe_variances, default=None),
        "oracle_probe_residual_variance_max": max(oracle_probe_variances, default=None),
        "biolang_probe_residual_variance_min": min(biolang_probe_variances, default=None),
        "biolang_probe_residual_variance_max": max(biolang_probe_variances, default=None),
        "oracle_full_residual_variance_min": min(oracle_variance, default=None),
        "oracle_full_residual_variance_max": max(oracle_variance, default=None),
        "residual_pearson": pearson(oracle_residual_values, biolang_residual_values),
        "residual_rmse": residual_rmse_value,
        "oracle_residual_sd": oracle_residual_sd,
        "residual_rmse_over_oracle_sd": (
            residual_rmse_value / oracle_residual_sd
            if residual_rmse_value is not None
            and oracle_residual_sd is not None
            and oracle_residual_sd > 0
            else None
        ),
        "residual_regression_slope": residual_slope,
        "residual_regression_offset": residual_offset,
        "large_residual_relative_error_n": len(large_residual_relative_error),
        "large_residual_relative_error_median": percentile(
            large_residual_relative_error, 0.5
        ),
        "large_residual_relative_error_p90": percentile(
            large_residual_relative_error, 0.9
        ),
        "od_factor_compared_genes": len(od_differences),
        "od_factor_difference_median": percentile(od_differences, 0.5),
        "od_factor_absolute_difference_median": percentile(od_absolute, 0.5),
        "od_factor_absolute_difference_p90": percentile(od_absolute, 0.9),
        "theta_attenuation_probe_genes": len(theta_attenuation_pairs),
        "theta_relative_error_median_on_probe": theta_error_median,
        "per_gene_residual_relative_error_median": gene_residual_median,
        "theta_to_residual_attenuation": theta_to_residual_attenuation,
        "theta_to_residual_dose_response": pearson(
            theta_errors, gene_residual_medians
        ),
        "per_gene_residual_correlation_median": (
            statistics.median(per_gene_correlations) if per_gene_correlations else None
        ),
        "per_gene_residual_correlation_min": (
            min(per_gene_correlations) if per_gene_correlations else None
        ),
    }
    oracle_manifest_path = oracle_dir / "manifest.csv"
    biolang_manifest_path = biolang_dir / "manifest.csv"
    if oracle_manifest_path.exists() and biolang_manifest_path.exists():
        oracle_manifest = read_rows(oracle_manifest_path)[0]
        biolang_manifest = read_rows(biolang_manifest_path)[0]
        oracle_elapsed = float(oracle_manifest["elapsed_seconds"])
        biolang_elapsed = float(biolang_manifest["elapsed_seconds"])
        metrics.update(
            {
                "oracle_elapsed_seconds": oracle_elapsed,
                "biolang_elapsed_seconds": biolang_elapsed,
                "oracle_over_biolang_elapsed_ratio": (
                    oracle_elapsed / biolang_elapsed if biolang_elapsed > 0 else None
                ),
                "oracle_residual_probe_strategy": oracle_manifest.get(
                    "residual_probe_strategy"
                ),
                "biolang_residual_probe_strategy": biolang_manifest.get(
                    "residual_probe_strategy"
                ),
            }
        )
    oracle_resource_path = oracle_dir / "resources.json"
    biolang_resource_path = biolang_dir / "resources.json"
    if oracle_resource_path.exists() and biolang_resource_path.exists():
        oracle_resources = read_json(oracle_resource_path)
        biolang_resources = read_json(biolang_resource_path)
        oracle_wall = float(oracle_resources["wall_seconds"])
        biolang_wall = float(biolang_resources["wall_seconds"])
        oracle_peak = int(oracle_resources["peak_working_set_bytes"])
        biolang_peak = int(biolang_resources["peak_working_set_bytes"])
        metrics.update(
            {
                "oracle_process_wall_seconds": oracle_wall,
                "biolang_process_wall_seconds": biolang_wall,
                "oracle_over_biolang_process_wall_ratio": (
                    oracle_wall / biolang_wall if biolang_wall > 0 else None
                ),
                "oracle_peak_working_set_bytes": oracle_peak,
                "biolang_peak_working_set_bytes": biolang_peak,
                "oracle_peak_working_set_gib": oracle_peak / 1024**3,
                "biolang_peak_working_set_gib": biolang_peak / 1024**3,
                "oracle_over_biolang_peak_memory_ratio": (
                    oracle_peak / biolang_peak if biolang_peak > 0 else None
                ),
                "resource_measurement_platform": oracle_resources.get("platform"),
                "resource_poll_interval_seconds": oracle_resources.get(
                    "poll_interval_seconds"
                ),
            }
        )
    oracle_fit_gene_path = oracle_dir / "fit-genes.csv"
    biolang_fit_gene_path = biolang_dir / "fit-genes.csv"
    if oracle_fit_gene_path.exists() and biolang_fit_gene_path.exists():
        oracle_fit_rows = read_rows(oracle_fit_gene_path)
        biolang_fit_rows = read_rows(biolang_fit_gene_path)
        oracle_fit_by_gene = {row["gene"]: row for row in oracle_fit_rows}
        biolang_fit_by_gene = {row["gene"]: row for row in biolang_fit_rows}
        oracle_fit_genes = set(oracle_fit_by_gene)
        biolang_fit_genes = set(biolang_fit_by_gene)
        fit_gene_intersection = len(oracle_fit_genes & biolang_fit_genes)
        metrics.update(
            {
                "oracle_fit_genes": len(oracle_fit_genes),
                "biolang_fit_genes": len(biolang_fit_genes),
                "fit_gene_intersection": fit_gene_intersection,
                "fit_gene_overlap_min_set": fit_gene_intersection
                / max(1, min(len(oracle_fit_genes), len(biolang_fit_genes))),
            }
        )
        # Separate the unregularized per-gene estimator from the smoothing
        # stage. These values are aligned by gene name because the independently
        # sampled 2,000-gene fit sets are not identical.
        shared_raw_genes = sorted(oracle_fit_genes & biolang_fit_genes)
        unregularized_oracle_theta: list[float] = []
        unregularized_biolang_theta: list[float] = []
        unregularized_oracle_intercept: list[float] = []
        unregularized_biolang_intercept: list[float] = []
        for gene in shared_raw_genes:
            # The oracle keeps raw fit parameters in genes.csv while its
            # fit-genes.csv is the selected-name manifest. BioLang writes raw
            # parameters directly beside its selected fit genes.
            oracle_row = oracle_genes[gene]
            biolang_row = biolang_fit_by_gene[gene]
            if not all(
                column in oracle_row and column in biolang_row
                for column in ("raw_theta", "raw_intercept")
            ):
                continue
            try:
                left_theta = float(oracle_row["raw_theta"])
                right_theta = float(biolang_row["raw_theta"])
                left_intercept = float(oracle_row["raw_intercept"])
                right_intercept = float(biolang_row["raw_intercept"])
            except ValueError:
                continue
            if math.isfinite(left_theta) and math.isfinite(right_theta):
                unregularized_oracle_theta.append(left_theta)
                unregularized_biolang_theta.append(right_theta)
            if math.isfinite(left_intercept) and math.isfinite(right_intercept):
                unregularized_oracle_intercept.append(left_intercept)
                unregularized_biolang_intercept.append(right_intercept)

        raw_theta_ratios = [
            observed / reference
            for reference, observed in zip(
                unregularized_oracle_theta, unregularized_biolang_theta
            )
            if reference > 0.0 and observed > 0.0
        ]
        raw_theta_errors = relative_errors(
            unregularized_oracle_theta, unregularized_biolang_theta
        )
        raw_theta_slope, raw_theta_offset = linear_fit(
            unregularized_oracle_theta, unregularized_biolang_theta
        )
        raw_intercept_slope, raw_intercept_offset = linear_fit(
            unregularized_oracle_intercept, unregularized_biolang_intercept
        )
        metrics.update(
            {
                "unregularized_theta_compared_genes": len(
                    unregularized_oracle_theta
                ),
                "unregularized_theta_median_biolang_over_oracle": percentile(
                    raw_theta_ratios, 0.5
                ),
                "unregularized_theta_relative_error_median": percentile(
                    raw_theta_errors, 0.5
                ),
                "unregularized_theta_relative_error_p90": percentile(
                    raw_theta_errors, 0.9
                ),
                "unregularized_theta_regression_slope": raw_theta_slope,
                "unregularized_theta_regression_offset": raw_theta_offset,
                "unregularized_intercept_compared_genes": len(
                    unregularized_oracle_intercept
                ),
                "unregularized_intercept_regression_slope": raw_intercept_slope,
                "unregularized_intercept_regression_offset": raw_intercept_offset,
                "unregularized_intercept_rmse": rmse(
                    unregularized_oracle_intercept,
                    unregularized_biolang_intercept,
                ),
            }
        )

    oracle_fit_cell_path = oracle_dir / "fit-cells.csv"
    biolang_fit_cell_path = biolang_dir / "fit-cells.csv"
    if oracle_fit_cell_path.exists() and biolang_fit_cell_path.exists():
        oracle_fit_cells = {row["cell"] for row in read_rows(oracle_fit_cell_path)}
        biolang_fit_cells = {row["cell"] for row in read_rows(biolang_fit_cell_path)}
        fit_cell_intersection = len(oracle_fit_cells & biolang_fit_cells)
        metrics.update(
            {
                "oracle_fit_cells": len(oracle_fit_cells),
                "biolang_fit_cells": len(biolang_fit_cells),
                "fit_cell_intersection": fit_cell_intersection,
                "fit_cell_overlap_min_set": fit_cell_intersection
                / max(1, min(len(oracle_fit_cells), len(biolang_fit_cells))),
            }
        )
    acceptance = {
        "gene_set_at_least_0_99": metrics["gene_set_intersection_over_oracle"] >= 0.99,
        "theta_correlation_at_least_0_99": (
            metrics["log10_theta_pearson"] is not None
            and metrics["log10_theta_pearson"] >= 0.99
        ),
        "theta_raw_slope_0_98_to_1_02": (
            metrics["theta_raw_regression_slope"] is not None
            and 0.98 <= metrics["theta_raw_regression_slope"] <= 1.02
        ),
        "theta_raw_median_relative_error_at_most_0_05": (
            metrics["theta_raw_relative_error_median"] is not None
            and metrics["theta_raw_relative_error_median"] <= 0.05
        ),
        "theta_raw_p90_relative_error_at_most_0_10": (
            metrics["theta_raw_relative_error_p90"] is not None
            and metrics["theta_raw_relative_error_p90"] <= 0.10
        ),
        "intercept_correlation_at_least_0_99": (
            metrics["intercept_pearson"] is not None
            and metrics["intercept_pearson"] >= 0.99
        ),
        "intercept_slope_0_98_to_1_02": (
            metrics["intercept_regression_slope"] is not None
            and 0.98 <= metrics["intercept_regression_slope"] <= 1.02
        ),
        "intercept_rmse_at_most_0_10": (
            metrics["intercept_rmse"] is not None
            and metrics["intercept_rmse"] <= 0.10
        ),
        "variance_rank_correlation_at_least_0_95": (
            metrics["residual_variance_spearman"] is not None
            and metrics["residual_variance_spearman"] >= 0.95
        ),
        "variance_slope_0_98_to_1_02": (
            metrics["residual_variance_regression_slope"] is not None
            and 0.98 <= metrics["residual_variance_regression_slope"] <= 1.02
        ),
        "feature_overlap_at_least_0_95": metrics["top_feature_overlap"] >= 0.95,
        "feature_rank_correlation_at_least_0_95": (
            metrics["feature_rank_spearman"] is not None
            and metrics["feature_rank_spearman"] >= 0.95
        ),
        # A theta bias is tolerable only if it is demonstrably damped before it
        # reaches the residuals. Requiring attenuation below 0.25 keeps that a
        # measured claim rather than an assumption.
        # The smoothed quantity itself, on its own scale. The original-scale
        # theta gates remain visible and deliberately failing where calibration
        # differs; this additional gate measures the quantity v2 actually fits.
        # Reference-backend variation must be recorded as its own artifact
        # before it is used to alter any threshold here.
        "od_factor_median_absolute_difference_at_most_0_01": (
            metrics["od_factor_absolute_difference_median"] is not None
            and metrics["od_factor_absolute_difference_median"] <= 0.01
        ),
        "theta_bias_attenuated_below_0_25": (
            metrics["theta_to_residual_attenuation"] is not None
            and metrics["theta_to_residual_attenuation"] <= 0.25
        ),
        "residual_probe_covers_at_least_0_95_of_top_n": (
            metrics["joined_probe_genes"] / max(1, metrics["top_feature_n"]) >= 0.95
        ),
        "residual_correlation_at_least_0_99": (
            metrics["residual_pearson"] is not None and metrics["residual_pearson"] >= 0.99
        ),
        "residual_slope_0_98_to_1_02": (
            metrics["residual_regression_slope"] is not None
            and 0.98 <= metrics["residual_regression_slope"] <= 1.02
        ),
        "residual_rmse_over_sd_at_most_0_02": (
            metrics["residual_rmse_over_oracle_sd"] is not None
            and metrics["residual_rmse_over_oracle_sd"] <= 0.02
        ),
        "large_residual_median_relative_error_at_most_0_02": (
            metrics["large_residual_relative_error_median"] is not None
            and metrics["large_residual_relative_error_median"] <= 0.02
        ),
    }
    if metrics.get("unregularized_theta_median_biolang_over_oracle") is not None:
        acceptance["unregularized_theta_median_ratio_0_95_to_1_05"] = (
            0.95
            <= metrics["unregularized_theta_median_biolang_over_oracle"]
            <= 1.05
        )
        acceptance["unregularized_theta_median_relative_error_at_most_0_05"] = (
            metrics["unregularized_theta_relative_error_median"] is not None
            and metrics["unregularized_theta_relative_error_median"] <= 0.05
        )
    if metrics.get("oracle_over_biolang_process_wall_ratio") is not None:
        acceptance["biolang_process_wall_time_below_oracle"] = (
            metrics["oracle_over_biolang_process_wall_ratio"] > 1.0
        )
    if (
        metrics.get("oracle_peak_working_set_bytes", 0) > 0
        and metrics.get("biolang_peak_working_set_bytes", 0) > 0
    ):
        acceptance["biolang_peak_memory_at_most_oracle"] = (
            metrics["biolang_peak_working_set_bytes"]
            <= metrics["oracle_peak_working_set_bytes"]
        )
    result = {"metrics": metrics, "acceptance": acceptance, "passed": all(acceptance.values())}
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
