"""Multi-species gene family analysis pipeline."""

import csv
import math
import os
from collections import Counter
from itertools import combinations


def read_fasta(path):
    sequences = []
    current_id = None
    current_seq = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line.startswith(">"):
                if current_id is not None:
                    sequences.append({"id": current_id, "sequence": "".join(current_seq)})
                current_id = line[1:].split()[0]
                current_seq = []
            else:
                current_seq.append(line)
    if current_id is not None:
        sequences.append({"id": current_id, "sequence": "".join(current_seq)})
    return sequences


def read_tsv(path):
    rows = []
    with open(path) as f:
        reader = csv.DictReader(f, delimiter="\t")
        for row in reader:
            rows.append(row)
    return rows


def write_tsv(rows, path):
    if not rows:
        return
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=rows[0].keys(), delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def get_kmers(seq, k):
    return [seq[i:i+k] for i in range(len(seq) - k + 1)]


def kmer_similarity(seq_a, seq_b, k):
    kmers_a = set(get_kmers(seq_a, k))
    kmers_b = set(get_kmers(seq_b, k))
    shared = len(kmers_a & kmers_b)
    total = len(kmers_a | kmers_b)
    if total == 0:
        return 0.0
    return round(shared / total, 4)


def kmer_distance(seq_a, seq_b, k):
    return round(1.0 - kmer_similarity(seq_a, seq_b, k), 4)


def window_identity(sequences, window_size):
    ref_seq = sequences[0]
    ref_len = len(ref_seq)
    n_seqs = len(sequences)
    results = []
    for start in range(ref_len - window_size + 1):
        end = start + window_size
        match_scores = []
        for pos in range(start, end):
            ref_char = ref_seq[pos]
            match_count = 0
            for si in range(1, n_seqs):
                other = sequences[si]
                if pos < len(other) and other[pos] == ref_char:
                    match_count += 1
            match_scores.append(match_count / (n_seqs - 1))
        avg = sum(match_scores) / len(match_scores)
        results.append({
            "position": start + window_size // 2,
            "conservation": round(avg, 4)
        })
    return results


def domain_divergence(sequences, species_info, orthologs, d_start, d_end):
    ref_seq = sequences[0]
    results = []
    for i in range(1, len(sequences)):
        other = sequences[i]
        end = min(d_end, len(ref_seq), len(other))
        positions = list(range(d_start, end))
        mismatches = sum(1 for p in positions if ref_seq[p] != other[p])
        total = len(positions)
        info = [s for s in species_info if s["seq_id"] == orthologs[i]["id"]][0]
        results.append({
            "species": info["common_name"],
            "divergence_mya": float(info["divergence_mya"]),
            "sub_rate": round(mismatches / total, 4) if total > 0 else 0.0
        })
    return results


def main():
    os.makedirs("data/output", exist_ok=True)

    orthologs = read_fasta("data/orthologs.fasta")
    species_info = read_tsv("data/species_info.tsv")
    domain_annotations = read_tsv("data/domain_annotations.tsv")

    info_map = {s["seq_id"]: s for s in species_info}
    species_names = [info_map[o["id"]]["common_name"] for o in orthologs]
    sequences = [o["sequence"] for o in orthologs]
    n = len(sequences)

    seq_summary = []
    for o in orthologs:
        info = info_map[o["id"]]
        seq_summary.append({
            "species": info["common_name"],
            "length_aa": str(len(o["sequence"])),
            "divergence_mya": info["divergence_mya"]
        })
    write_tsv(seq_summary, "data/output/sequence_summary.tsv")

    human_protein = sequences[0]
    sim_table = []
    for o in orthologs:
        info = info_map[o["id"]]
        sim_table.append({
            "species": info["common_name"],
            "kmer3_sim": str(kmer_similarity(human_protein, o["sequence"], 3)),
            "kmer5_sim": str(kmer_similarity(human_protein, o["sequence"], 5))
        })
    sim_table.sort(key=lambda x: float(x["kmer5_sim"]), reverse=True)
    write_tsv(sim_table, "data/output/similarity_table.tsv")

    dist_rows = []
    for i in range(n):
        row = {"species": species_names[i]}
        for j in range(n):
            row[species_names[j]] = str(kmer_distance(sequences[i], sequences[j], 4))
        dist_rows.append(row)
    write_tsv(dist_rows, "data/output/distance_matrix.tsv")

    domain_regions = [
        {"name": "N-terminal_TAD", "start": 0, "end": 60},
        {"name": "Proline-rich", "start": 60, "end": 95},
        {"name": "DNA-binding", "start": 95, "end": 290},
        {"name": "Tetramerization", "start": 320, "end": 360},
        {"name": "C-terminal_reg", "start": 360, "end": 393},
    ]

    vertebrate_seqs = [
        o["sequence"] for o in orthologs
        if "fly" not in o["id"] and "worm" not in o["id"] and "yeast" not in o["id"]
    ]

    conservation = window_identity(vertebrate_seqs, 10)

    domain_cons = []
    for d in domain_regions:
        region = [w for w in conservation if d["start"] <= w["position"] < d["end"]]
        if region:
            avg = sum(w["conservation"] for w in region) / len(region)
        else:
            avg = 0.0
        domain_cons.append({
            "domain": d["name"],
            "start": str(d["start"]),
            "end_pos": str(d["end"]),
            "mean_conservation": str(round(avg, 4))
        })
    write_tsv(domain_cons, "data/output/domain_conservation.tsv")

    arch_table = []
    for sp in species_info:
        domains = [d for d in domain_annotations if d["seq_id"] == sp["seq_id"]]
        arch_table.append({
            "species": sp["common_name"],
            "n_domains": str(len(domains)),
            "domains": ", ".join(d["domain_name"] for d in domains),
            "seq_length": sp["seq_length"]
        })
    write_tsv(arch_table, "data/output/domain_architecture.tsv")

    dbd = domain_divergence(sequences, species_info, orthologs, 95, 290)
    tad = domain_divergence(sequences, species_info, orthologs, 0, 60)

    evo_rates = []
    for i in range(len(dbd)):
        ratio = round(tad[i]["sub_rate"] / (dbd[i]["sub_rate"] + 0.001), 2)
        evo_rates.append({
            "species": dbd[i]["species"],
            "divergence_mya": str(dbd[i]["divergence_mya"]),
            "dbd_rate": str(dbd[i]["sub_rate"]),
            "tad_rate": str(tad[i]["sub_rate"]),
            "ratio": str(ratio)
        })
    write_tsv(evo_rates, "data/output/evolutionary_rates.tsv")

    dbd_cons = [d for d in domain_cons if d["domain"] == "DNA-binding"][0]
    tet_cons = [d for d in domain_cons if d["domain"] == "Tetramerization"][0]
    tad_cons = [d for d in domain_cons if d["domain"] == "N-terminal_TAD"][0]
    mean_ratio = sum(float(r["ratio"]) for r in evo_rates) / len(evo_rates)

    lengths = [int(s["length_aa"]) for s in seq_summary]

    summary_lines = [
        "=== Multi-Species TP53 Gene Family Analysis ===",
        "",
        f"Species analyzed: {n}",
        f"Vertebrate orthologs: {len(vertebrate_seqs)}",
        "",
        "Sequence lengths (aa):",
        f"  Min: {min(lengths)}",
        f"  Max: {max(lengths)}",
        f"  Mean: {round(sum(lengths) / len(lengths), 1)}",
        "",
        "Domain conservation (vertebrates):",
        f"  DNA-binding domain: {dbd_cons['mean_conservation']}",
        f"  Tetramerization: {tet_cons['mean_conservation']}",
        f"  N-terminal TAD: {tad_cons['mean_conservation']}",
        "",
        "Evolutionary rate ratio (TAD/DBD):",
        f"  Mean: {round(mean_ratio, 2)}",
        "  (>1.0 means TAD evolves faster than DBD)",
        "",
        "Output files:",
        "  data/output/sequence_summary.tsv",
        "  data/output/similarity_table.tsv",
        "  data/output/distance_matrix.tsv",
        "  data/output/domain_conservation.tsv",
        "  data/output/domain_architecture.tsv",
        "  data/output/evolutionary_rates.tsv",
        "  data/output/summary.txt",
    ]

    with open("data/output/summary.txt", "w") as f:
        f.write("\n".join(summary_lines) + "\n")


if __name__ == "__main__":
    main()
