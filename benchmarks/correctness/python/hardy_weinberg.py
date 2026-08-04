"""Hardy-Weinberg expectations from observed genotype counts. Output JSON."""
import json
import sys

cohorts = [
    {"name": "cohort_1", "aa": 1469, "ab": 138, "bb": 5},
    {"name": "cohort_2", "aa": 900,  "ab": 850, "bb": 250},
    {"name": "cohort_3", "aa": 320,  "ab": 480, "bb": 200},
]

results = []
for c in cohorts:
    n = c["aa"] + c["ab"] + c["bb"]
    p = (2 * c["aa"] + c["ab"]) / (2 * n)
    q = 1 - p
    results.append({
        "name": c["name"], "n": n,
        "p": round(p, 9), "q": round(q, 9),
        "exp_aa": round(p * p * n, 6),
        "exp_ab": round(2 * p * q * n, 6),
        "exp_bb": round(q * q * n, 6),
    })

json.dump({"cohorts": results}, sys.stdout, indent=2)
print()
