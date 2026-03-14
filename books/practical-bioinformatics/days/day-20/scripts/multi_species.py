#!/usr/bin/env python3
"""Day 20: Multi-Species Comparison — Python equivalent using requests + Biopython."""

import requests
import json
import os
import csv
from itertools import combinations
from collections import Counter

ENSEMBL_REST = "https://rest.ensembl.org"

def ensembl_symbol(species, symbol):
    """Look up gene by symbol via Ensembl REST API."""
    url = f"{ENSEMBL_REST}/lookup/symbol/{species}/{symbol}"
    resp = requests.get(url, headers={"Content-Type": "application/json"})
    resp.raise_for_status()
    return resp.json()

def ensembl_sequence(gene_id, seq_type="protein"):
    """Fetch sequence by Ensembl gene ID."""
    url = f"{ENSEMBL_REST}/sequence/id/{gene_id}"
    params = {"type": seq_type}
    resp = requests.get(url, headers={"Content-Type": "application/json"}, params=params)
    resp.raise_for_status()
    return resp.json()

def gc_content(seq):
    """Calculate GC content of a nucleotide sequence."""
    seq = seq.upper()
    gc = sum(1 for c in seq if c in "GC")
    return gc / len(seq) if len(seq) > 0 else 0.0

def kmers(seq, k):
    """Generate all k-mers from a sequence."""
    return [seq[i:i+k] for i in range(len(seq) - k + 1)]

def kmer_jaccard(seq1, seq2, k):
    """Compute Jaccard similarity of k-mer sets."""
    k1 = set(kmers(seq1, k))
    k2 = set(kmers(seq2, k))
    shared = len(k1 & k2)
    total = len(k1 | k2)
    return round(shared / total, 3) if total > 0 else 0.0

def aa_composition(seq):
    """Classify amino acids into hydrophobic, polar, charged."""
    hydrophobic = sum(1 for aa in seq if aa in "AVLIMFWP")
    polar = sum(1 for aa in seq if aa in "STNQYC")
    charged = sum(1 for aa in seq if aa in "DEKRH")
    total = len(seq)
    return {
        "hydrophobic": round(hydrophobic / total * 100, 1),
        "polar": round(polar / total * 100, 1),
        "charged": round(charged / total * 100, 1),
    }

def main():
    print("=" * 60)
    print("Day 20: Multi-Species Comparison (Python)")
    print("=" * 60)

    species = [
        {"name": "Human", "id": "homo_sapiens"},
        {"name": "Mouse", "id": "mus_musculus"},
        {"name": "Chicken", "id": "gallus_gallus"},
        {"name": "Zebrafish", "id": "danio_rerio"},
    ]

    # -- Fetch orthologs --
    print("\n-- Fetching BRCA1 Orthologs --\n")
    results = []
    for sp in species:
        try:
            gene = ensembl_symbol(sp["id"], "BRCA1")
            protein = ensembl_sequence(gene["id"], "protein")
            cds = ensembl_sequence(gene["id"], "cdna")
            prot_seq = protein["seq"]
            cds_seq = cds["seq"]
            results.append({
                "species": sp["name"],
                "gene_id": gene["id"],
                "protein_len": len(prot_seq),
                "protein_seq": prot_seq,
                "cds_len": len(cds_seq),
                "cds_seq": cds_seq,
                "gc": round(gc_content(cds_seq) * 100, 1),
            })
            print(f"  {sp['name']}: {gene['id']} ({len(prot_seq)} aa)")
        except Exception as e:
            print(f"  {sp['name']}: not found ({e})")

    # -- Comparison table --
    print("\n-- Cross-Species Comparison --\n")
    print(f"{'Species':<12} {'Protein':>8} {'CDS':>6} {'GC%':>6} {'Ratio':>6}")
    print("-" * 44)
    for r in results:
        ratio = round(r["cds_len"] / r["protein_len"], 1)
        print(f"{r['species']:<12} {r['protein_len']:>8} {r['cds_len']:>6} {r['gc']:>6} {ratio:>6}")

    # -- K-mer similarity --
    print("\n-- K-mer Jaccard Similarity (k=5) --\n")
    for i, j in combinations(range(len(results)), 2):
        sim = kmer_jaccard(results[i]["cds_seq"], results[j]["cds_seq"], 5)
        print(f"  {results[i]['species']} vs {results[j]['species']}: {sim}")

    # -- Amino acid composition --
    print("\n-- Amino Acid Composition --\n")
    for r in results:
        comp = aa_composition(r["protein_seq"])
        print(f"  {r['species']}: hydrophobic={comp['hydrophobic']}%, "
              f"polar={comp['polar']}%, charged={comp['charged']}%")

    # -- Multi-gene comparison --
    print("\n-- Multi-Gene Comparison --\n")
    genes = ["TP53", "BRCA1", "EGFR"]
    all_results = []
    for gene_symbol in genes:
        for sp in species:
            try:
                gene = ensembl_symbol(sp["id"], gene_symbol)
                prot = ensembl_sequence(gene["id"], "protein")
                all_results.append({
                    "gene": gene_symbol,
                    "species": sp["name"],
                    "length": len(prot["seq"]),
                })
            except Exception:
                pass

    print(f"{'Gene':<8} {'Species':<12} {'Length':>7}")
    print("-" * 30)
    for r in all_results:
        print(f"{r['gene']:<8} {r['species']:<12} {r['length']:>7}")

    # -- Export --
    os.makedirs("results", exist_ok=True)

    with open("results/species_comparison.csv", "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["species", "protein_len", "cds_len", "gc_percent", "cds_protein_ratio"])
        writer.writeheader()
        for r in results:
            writer.writerow({
                "species": r["species"],
                "protein_len": r["protein_len"],
                "cds_len": r["cds_len"],
                "gc_percent": r["gc"],
                "cds_protein_ratio": round(r["cds_len"] / r["protein_len"], 1),
            })

    with open("results/brca1_orthologs.fasta", "w") as f:
        for r in results:
            f.write(f">{r['species']}_BRCA1\n{r['protein_seq']}\n")

    print("\nExported results/species_comparison.csv")
    print("Exported results/brca1_orthologs.fasta")

    print("\n" + "=" * 60)
    print("Done!")
    print("=" * 60)

if __name__ == "__main__":
    main()
