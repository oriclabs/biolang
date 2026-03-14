#!/usr/bin/env python3
"""Day 4: Coding Crash Course for Biologists — Python equivalent.

Uses standard library and pandas for data operations.
Install: pip install pandas
"""

import pandas as pd

print("=" * 60)
print("Day 4: Coding Crash Course for Biologists")
print("=" * 60)

# ── Section 1: Variables ──────────────────────────────────────

print()
print("--- Variables: Labeling Your Tubes ---")

sample_name = "Patient_042"
concentration = 23.5
is_contaminated = False
bases_sequenced = 3200000

print(f"Sample: {sample_name}")
print(f"Concentration: {concentration} ng/uL")
print(f"Clean: {not is_contaminated}")
print(f"Bases: {bases_sequenced}")

# ── Section 2: Lists ─────────────────────────────────────────

print()
print("--- Lists: Your Sample Rack ---")

samples = ["Control_1", "Control_2", "Treated_1", "Treated_2"]
print(f"Number of samples: {len(samples)}")
print(f"First sample: {samples[0]}")

updated = samples + ["Treated_3"]
print(f"After adding: {len(updated)} samples")
print(f"Contains Control_1: {'Control_1' in samples}")
print(f"Contains Control_9: {'Control_9' in samples}")

# ── Section 3: Records (Dictionaries) ────────────────────────

print()
print("--- Records: Lab Notebook Entries ---")

experiment = {
    "date": "2024-03-15",
    "investigator": "Dr. Chen",
    "cell_line": "HeLa",
    "treatment": "Doxorubicin",
    "concentration_uM": 0.5,
    "viability_percent": 72.3,
}

print(f"Cell line: {experiment['cell_line']}")
print(f"Viability: {experiment['viability_percent']}%")

# ── Section 4: Loops ─────────────────────────────────────────

print()
print("--- Loops: Processing Every Sample ---")

genes = ["BRCA1", "TP53", "EGFR", "KRAS", "MYC"]
for gene in genes:
    print(f"  Analyzing {gene}...")

print()
print("GC content per sequence:")
sequences = ["ATCGATCG", "GCGCGCGC", "AATTAATT"]


def gc_content(seq: str) -> float:
    """Calculate GC content of a DNA sequence."""
    gc = sum(1 for b in seq.upper() if b in "GC")
    return gc / len(seq) if len(seq) > 0 else 0.0


for seq in sequences:
    gc = gc_content(seq)
    gc_pct = round(gc * 100.0, 1)
    print(f"  {seq} -> GC: {gc_pct}%")

# ── Section 5: Conditions ────────────────────────────────────

print()
print("--- Conditions: QC Decisions ---")

qc_samples = [
    {"name": "S001", "reads": 25000000, "quality": 35.2},
    {"name": "S002", "reads": 500000, "quality": 28.7},
    {"name": "S003", "reads": 18000000, "quality": 33.1},
    {"name": "S004", "reads": 12000000, "quality": 22.0},
]

for s in qc_samples:
    if s["reads"] < 1000000:
        print(f"  {s['name']}: FAIL (too few reads: {s['reads']})")
    elif s["quality"] < 25.0:
        print(f"  {s['name']}: FAIL (low quality: {s['quality']})")
    else:
        print(f"  {s['name']}: PASS")

# ── Section 6: Functions ─────────────────────────────────────

print()
print("--- Functions: Reusable Protocols ---")


def qc_check(reads: int, min_reads: int) -> str:
    """Check if a sample passes read-count QC."""
    return "FAIL" if reads < min_reads else "PASS"


def fold_change(control: float, treated: float) -> float:
    """Calculate fold change between conditions."""
    return round(treated / control, 2)


print(f"QC 25M reads: {qc_check(25000000, 1000000)}")
print(f"QC 500K reads: {qc_check(500000, 1000000)}")
print(f"Fold change 5.2 -> 12.8: {fold_change(5.2, 12.8)}")
print(f"Fold change 8.1 -> 7.9: {fold_change(8.1, 7.9)}")

# ── Section 7: Pipes (list comprehensions in Python) ─────────

print()
print("--- Pipes: Connecting Steps ---")

dna_seq = "ATGCGATCGATCGATCGATCGATCG"
gc_result = round(gc_content(dna_seq), 3)
print(f"GC content (piped): {gc_result}")

dna_seqs = [
    "ATCGATCGATCG",
    "GCGCGCGCGCGC",
    "ATATATATATATAT",
    "GCGCATATAGCGC",
    "TTTTTAAAAACCCCC",
]

gc_with_values = [{"seq": s, "gc": gc_content(s)} for s in dna_seqs]
gc_rich = [r for r in gc_with_values if r["gc"] > 0.5]
gc_rich_count = len(gc_rich)

print(f"{gc_rich_count} out of {len(dna_seqs)} sequences are GC-rich")

# ── Section 8: Complete Analysis ──────────────────────────────

print()
print("--- Complete Analysis: Gene Expression ---")

expr_data = pd.DataFrame(
    [
        {"gene": "BRCA1", "control": 5.2, "treated": 12.8},
        {"gene": "TP53", "control": 8.1, "treated": 7.9},
        {"gene": "EGFR", "control": 3.4, "treated": 15.2},
        {"gene": "MYC", "control": 6.7, "treated": 6.5},
        {"gene": "KRAS", "control": 4.1, "treated": 11.3},
    ]
)

expr_data["fold_change"] = round(expr_data["treated"] / expr_data["control"], 2)
expr_data["direction"] = expr_data.apply(
    lambda row: "UP" if row["treated"] > row["control"] else "DOWN", axis=1
)

upregulated = (
    expr_data[expr_data["fold_change"] > 2.0]
    .sort_values("fold_change", ascending=False)
    .reset_index(drop=True)
)

print("=== Upregulated Genes (FC > 2.0) ===")
for _, row in upregulated.iterrows():
    print(f"  {row['gene']}: {row['fold_change']}x {row['direction']}")
print(f"\nTotal: {len(upregulated)} of {len(expr_data)} genes upregulated")

# ── Summary ───────────────────────────────────────────────────

print()
print("=" * 60)
print("Day 4 complete! You now know:")
print("  - Variables, lists, records")
print("  - Loops, conditions, functions")
print("  - Pipes for chaining analysis steps")
print("=" * 60)
