# Day 27: Reproducible Analysis
# Structured, seed-controlled, parameterized analysis
import argparse
import numpy as np
import pandas as pd
from scipy import stats
import json
from datetime import datetime

def load_config(path="config.yaml"):
    """Simple YAML-like config loader."""
    config = {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            key, val = line.split(":", 1)
            val = val.strip()
            try:
                val = int(val)
            except ValueError:
                try:
                    val = float(val)
                except ValueError:
                    pass
            config[key.strip()] = val
    return config

def main():
    parser = argparse.ArgumentParser(description="Reproducible analysis")
    parser.add_argument("--config", default="config.yaml")
    parser.add_argument("--seed", type=int, default=None)
    args = parser.parse_args()

    config = load_config(args.config)
    seed = args.seed or config.get("seed", 42)
    np.random.seed(seed)

    data = pd.read_csv("reproducible_data.csv")
    trt = data[data.group == "Treatment"]["value"]
    ctrl = data[data.group == "Control"]["value"]

    t_stat, p_val = stats.ttest_ind(trt, ctrl)
    d = (trt.mean() - ctrl.mean()) / np.sqrt(
        ((len(trt)-1)*trt.std()**2 + (len(ctrl)-1)*ctrl.std()**2) /
        (len(trt)+len(ctrl)-2))

    results = {
        "timestamp": datetime.now().isoformat(),
        "seed": seed,
        "config": config,
        "n_treatment": len(trt),
        "n_control": len(ctrl),
        "mean_diff": round(trt.mean() - ctrl.mean(), 4),
        "t_statistic": round(t_stat, 4),
        "p_value": round(p_val, 4),
        "cohens_d": round(d, 4)
    }

    print(json.dumps(results, indent=2))
    with open("results.json", "w") as f:
        json.dump(results, f, indent=2)
    print("\nResults saved to results.json")

if __name__ == "__main__":
    main()
