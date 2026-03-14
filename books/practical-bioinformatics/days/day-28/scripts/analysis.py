"""Clinical variant report pipeline."""

import csv
import os
import sys
from pathlib import Path


def read_vcf(path):
    variants = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line.startswith("##"):
                continue
            if line.startswith("#CHROM"):
                headers = line.lstrip("#").split("\t")
                continue
            fields = line.split("\t")
            row = {}
            for i, h in enumerate(headers):
                row[h.lower()] = fields[i] if i < len(fields) else ""
            variants.append(row)
    return variants


def extract_info_field(info_str, field_name):
    for part in info_str.split(";"):
        if part.startswith(f"{field_name}="):
            return part.split("=", 1)[1]
    return ""


def validate_vcf(variants):
    if not variants:
        raise ValueError("VCF file contains no variants")
    required = ["chrom", "pos", "ref", "alt", "qual"]
    for col in required:
        if col not in variants[0]:
            raise ValueError(f"Missing required VCF column: {col}")
    return {"variant_count": len(variants), "status": "valid"}


def quality_filter(variants, min_qual, min_dp):
    result = []
    for v in variants:
        q = float(v.get("qual", "0"))
        dp_str = extract_info_field(v.get("info", ""), "DP")
        d = int(dp_str) if dp_str else 0
        if q >= min_qual and d >= min_dp:
            result.append(v)
    return result


def make_variant_key(row):
    return f"{row['chrom']}:{row['pos']}:{row['ref']}:{row['alt']}"


def frequency_filter(variants, max_af):
    result = []
    for v in variants:
        af_str = extract_info_field(v.get("info", ""), "AF")
        af = float(af_str) if af_str else 0.0
        if af <= max_af:
            result.append(v)
    return result


def read_tsv(path):
    rows = []
    with open(path) as f:
        reader = csv.DictReader(f, delimiter="\t")
        for row in reader:
            rows.append(dict(row))
    return rows


def panel_filter(variants, panel):
    panel_genes = set(row["gene"] for row in panel)
    return [v for v in variants if v.get("gene", "") in panel_genes]


def clinvar_score(cls):
    scores = {
        "pathogenic": 3,
        "likely_pathogenic": 2,
        "uncertain": 0,
        "likely_benign": -1,
        "benign": -2,
    }
    return scores.get(cls, 0)


def frequency_score(af):
    if af == 0.0:
        return 1
    if af < 0.001:
        return 0
    return -1


def impact_score(impact):
    scores = {
        "frameshift": 2,
        "nonsense": 2,
        "splice": 2,
        "missense": 1,
        "synonymous": -1,
    }
    return scores.get(impact, 0)


def gene_disease_score(strength):
    return 1 if strength == "definitive" else 0


def classify_variant(score):
    if score >= 4:
        return "Pathogenic"
    if score == 3:
        return "Likely Pathogenic"
    if score >= 1:
        return "VUS"
    if score == 0:
        return "Likely Benign"
    return "Benign"


def score_variant(row):
    cv = row.get("clinvar_class", "unknown")
    af_str = extract_info_field(row.get("info", ""), "AF")
    af = float(af_str) if af_str else 0.0
    imp = row.get("impact", extract_info_field(row.get("info", ""), "IMPACT"))
    gd = row.get("gene_disease", "unknown")
    s1 = clinvar_score(cv)
    s2 = frequency_score(af)
    s3 = impact_score(imp)
    s4 = gene_disease_score(gd)
    total = s1 + s2 + s3 + s4
    return {
        "variant_key": row.get("variant_key", ""),
        "gene": row.get("gene", ""),
        "chrom": row["chrom"],
        "pos": row["pos"],
        "ref_allele": row["ref"],
        "alt_allele": row["alt"],
        "impact": imp,
        "clinvar": cv,
        "af": af,
        "score": total,
        "classification": classify_variant(total),
    }


def annotate_variants(variants, gene_db, clinvar_db):
    gene_lookup = {g["gene"]: g for g in gene_db}
    clinvar_lookup = {c["variant_key"]: c for c in clinvar_db}
    result = []
    for v in variants:
        v["variant_key"] = make_variant_key(v)
        v["gene"] = extract_info_field(v.get("info", ""), "GENE")
        v["impact"] = extract_info_field(v.get("info", ""), "IMPACT")
        gene_info = gene_lookup.get(v["gene"], {})
        for k, val in gene_info.items():
            if k != "gene":
                v[k] = val
        clinvar_info = clinvar_lookup.get(v["variant_key"], {})
        for k, val in clinvar_info.items():
            if k != "variant_key":
                v[k] = val
        result.append(v)
    return result


def compute_qc_stats(all_variants, qc_passed, rare, panel_matched):
    quals = [float(v.get("qual", "0")) for v in all_variants]
    depths = []
    for v in all_variants:
        dp_str = extract_info_field(v.get("info", ""), "DP")
        depths.append(int(dp_str) if dp_str else 0)
    return {
        "total_input": len(all_variants),
        "passed_qc": len(qc_passed),
        "rare_count": len(rare),
        "panel_count": len(panel_matched),
        "mean_qual": sum(quals) / len(quals) if quals else 0,
        "mean_dp": sum(depths) / len(depths) if depths else 0,
    }


def format_variant_line(v):
    return (
        f"  {v['gene']} | {v['chrom']}:{v['pos']} | "
        f"{v['ref_allele']}>{v['alt_allele']} | {v['impact']} | "
        f"{v['clinvar']} | Score:{v['score']}"
    )


def build_report(patient_info, classified, qc_stats):
    report = [
        "================================================================",
        "       CLINICAL VARIANT ANALYSIS REPORT (EDUCATIONAL ONLY)",
        "================================================================",
        "",
        "DISCLAIMER: This report is generated by an educational pipeline.",
        "It must NOT be used for clinical decision-making.",
        "",
        "--- PATIENT INFORMATION ---",
        f"Patient ID: {patient_info['patient_id']}",
        f"Sample ID: {patient_info['sample_id']}",
        f"Indication: {patient_info['indication']}",
        f"Report Date: {patient_info['report_date']}",
        "",
    ]
    path_v = [v for v in classified if v["classification"] == "Pathogenic"]
    lp_v = [v for v in classified if v["classification"] == "Likely Pathogenic"]
    vus_v = [v for v in classified if v["classification"] == "VUS"]
    lb_v = [v for v in classified if v["classification"] == "Likely Benign"]
    b_v = [v for v in classified if v["classification"] == "Benign"]
    report += [
        "--- SUMMARY ---",
        f"Total variants analyzed: {qc_stats['total_input']}",
        f"Passed quality filter: {qc_stats['passed_qc']}",
        f"Rare variants (AF <= 1%): {qc_stats['rare_count']}",
        f"In gene panel: {qc_stats['panel_count']}",
        f"Classified: {len(classified)}",
        "",
        f"  Pathogenic:        {len(path_v)}",
        f"  Likely Pathogenic: {len(lp_v)}",
        f"  VUS:               {len(vus_v)}",
        f"  Likely Benign:     {len(lb_v)}",
        f"  Benign:            {len(b_v)}",
        "",
    ]
    if path_v:
        report.append("--- PATHOGENIC VARIANTS (Reportable) ---")
        for v in path_v:
            report.append(format_variant_line(v))
        report.append("")
    if lp_v:
        report.append("--- LIKELY PATHOGENIC VARIANTS (Reportable) ---")
        for v in lp_v:
            report.append(format_variant_line(v))
        report.append("")
    if vus_v:
        report.append("--- VARIANTS OF UNCERTAIN SIGNIFICANCE ---")
        for v in vus_v:
            report.append(format_variant_line(v))
        report.append("")
    report += [
        "--- QUALITY METRICS ---",
        f"Mean QUAL score: {qc_stats['mean_qual']}",
        f"Mean depth: {qc_stats['mean_dp']}",
        f"Variants filtered (low quality): {qc_stats['total_input'] - qc_stats['passed_qc']}",
        "",
        "--- LIMITATIONS ---",
        "- This analysis covers exonic regions only",
        "- Structural variants and CNVs are not assessed",
        "- Intronic and regulatory variants may be missed",
        "- Classification is based on a simplified scoring model",
        "",
        "--- END OF REPORT ---",
    ]
    return report


def main():
    variants = read_vcf("data/patient.vcf")
    gene_db = read_tsv("data/gene_db.tsv")
    clinvar_db = read_tsv("data/clinvar_db.tsv")
    cancer_panel = read_tsv("data/cancer_panel.tsv")
    patient_meta = read_tsv("data/patient_info.tsv")
    patient_info = patient_meta[0]

    validate_vcf(variants)

    qc_passed = quality_filter(variants, 30.0, 10)

    annotated = annotate_variants(qc_passed, gene_db, clinvar_db)

    rare_variants = frequency_filter(annotated, 0.01)

    panel_matched = panel_filter(rare_variants, cancer_panel)

    classified = [score_variant(v) for v in panel_matched]

    qc_stats = compute_qc_stats(variants, qc_passed, rare_variants, panel_matched)

    report_lines = build_report(patient_info, classified, qc_stats)

    os.makedirs("data/output", exist_ok=True)
    with open("data/output/clinical_report.txt", "w") as f:
        f.write("\n".join(report_lines) + "\n")

    if classified:
        fieldnames = list(classified[0].keys())
        with open("data/output/classified_variants.tsv", "w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames, delimiter="\t")
            writer.writeheader()
            writer.writerows(classified)


if __name__ == "__main__":
    main()
