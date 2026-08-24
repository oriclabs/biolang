"""Independent NumPy oracle for left-closed histogram geometry."""

import csv
import json
import math
from pathlib import Path
from statistics import NormalDist

import numpy as np
import statsmodels.api as sm


def geometry(values, breaks):
    counts, edges = np.histogram(values, bins=np.asarray(breaks, dtype=float), density=False)
    widths = np.diff(edges)
    density = counts / (counts.sum() * widths)
    return {
        "left": edges[:-1].tolist(),
        "right": edges[1:].tolist(),
        "counts": counts.tolist(),
        "density": density.tolist(),
    }


def type7_box(values):
    values = np.sort(np.asarray(values, dtype=float))
    q1, median, q3 = np.quantile(values, [0.25, 0.5, 0.75], method="linear")
    iqr = q3 - q1
    inside = values[(values >= q1 - 1.5 * iqr) & (values <= q3 + 1.5 * iqr)]
    outliers = values[(values < inside[0]) | (values > inside[-1])]
    return {
        "summary": [inside[0], q1, median, q3, inside[-1]],
        "outliers": outliers.tolist(),
    }


def ecdf_geometry(values):
    x, counts = np.unique(np.asarray(values, dtype=float), return_counts=True)
    cumulative = np.cumsum(counts)
    return {
        "x": x.tolist(),
        "counts": counts.tolist(),
        "cumulative": cumulative.tolist(),
        "fraction": (cumulative / cumulative[-1]).tolist(),
    }


def normal_qq(values):
    sample = np.sort(np.asarray(values, dtype=float))
    n = len(sample)
    offset = 0.375 if n <= 10 else 0.5
    probabilities = (np.arange(n, dtype=float) + 1 - offset) / (n + 1 - 2 * offset)
    normal = NormalDist()
    theoretical = np.asarray([normal.inv_cdf(float(p)) for p in probabilities])
    sample_quartiles = np.quantile(sample, [0.25, 0.75], method="linear")
    theoretical_quartiles = np.asarray([normal.inv_cdf(0.25), normal.inv_cdf(0.75)])
    slope = np.diff(sample_quartiles)[0] / np.diff(theoretical_quartiles)[0]
    intercept = sample_quartiles[0] - slope * theoretical_quartiles[0]
    return {
        "theoretical": theoretical.tolist(),
        "sample": sample.tolist(),
        "line": [intercept, slope],
    }


def violin_geometry(values):
    values = np.sort(np.asarray(values, dtype=float))
    sd = np.std(values, ddof=1)
    q1, q3 = np.quantile(values, [0.25, 0.75], method="linear")
    spread = min(sd, (q3 - q1) / 1.34)
    if spread <= 0:
        spread = sd
    if spread <= 0:
        spread = abs(values[0])
    if spread <= 0:
        spread = 1.0
    bandwidth = 0.9 * spread * len(values) ** -0.2
    x = np.linspace(values[0] - 3 * bandwidth, values[-1] + 3 * bandwidth, 256)
    z = (x[:, None] - values[None, :]) / bandwidth
    density = np.exp(-0.5 * z * z).sum(axis=1) / (
        len(values) * bandwidth * math.sqrt(2 * math.pi)
    )
    return {
        "bandwidth": bandwidth,
        "x": x.tolist(),
        "density": density.tolist(),
        "scaled": (density / density.max()).tolist(),
    }


def linear_fit_geometry(x, y):
    x = np.asarray(x, dtype=float)
    y = np.asarray(y, dtype=float)
    at = np.unique(np.sort(x))
    model = sm.OLS(y, sm.add_constant(x)).fit()
    frame = model.get_prediction(sm.add_constant(at)).summary_frame(alpha=0.05)
    return {
        "slope": float(model.params[1]),
        "intercept": float(model.params[0]),
        "residual_mse": float(model.mse_resid),
        "x": at.tolist(),
        "fitted": frame["mean"].tolist(),
        "confidence_lower": frame["mean_ci_lower"].tolist(),
        "confidence_upper": frame["mean_ci_upper"].tolist(),
        "prediction_lower": frame["obs_ci_lower"].tolist(),
        "prediction_upper": frame["obs_ci_upper"].tolist(),
    }


results = Path("packages/statistics/validation/results")
with (results / "airquality.csv").open(newline="", encoding="utf-8-sig") as handle:
    air_rows = list(csv.DictReader(handle))
    air_ozone = [float(row["ozone"]) for row in air_rows]
    air_month = [float(row["month"]) for row in air_rows]

reference = {
    "edge_left": geometry([0, 1, 1.5, 2, 3], [0, 1, 2, 3]),
    "airquality_left": geometry(
        air_ozone,
        [0, 25, 50, 75, 100, 125, 150, 175],
    ),
    "box_type7": type7_box([1, 2, 2, 3, 4, 100]),
    "ecdf": ecdf_geometry([1, 2, 2, 3, 4, 100]),
    "normal_qq": normal_qq([1, 2, 2, 3, 4, 100]),
    "violin": violin_geometry([1, 2, 2, 3, 4, 100]),
    "air_box_type7": type7_box(air_ozone),
    "air_ecdf": ecdf_geometry(air_ozone),
    "air_normal_qq": normal_qq(air_ozone),
    "air_violin": violin_geometry(air_ozone),
    "linear_fit_air": linear_fit_geometry(air_month, air_ozone),
}
(results / "numpy-plot-reference.json").write_text(
    json.dumps(reference, indent=2),
    encoding="utf-8",
)
