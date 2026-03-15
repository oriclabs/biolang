#!/usr/bin/env python3
"""Download real-world biological data for correctness validation.

Downloads small, stable datasets from NCBI and derives additional files.
All files are saved to benchmarks/correctness/real_data/.

Sources:
  - E. coli K-12 MG1655 genome (GCF_000005845.2) — NCBI RefSeq
  - S. cerevisiae S288C genome (GCF_000146045.2) — NCBI RefSeq
  - ClinVar VCF (GRCh38, subset) — NCBI ClinVar
  - E. coli K-12 GFF3 annotation — NCBI RefSeq

Usage:
  python download_real_data.py
"""

import gzip
import io
import os
import sys
import urllib.request
import urllib.error

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "real_data")

DOWNLOADS = {
    "ecoli_genome": {
        "url": "https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/000/005/845/GCF_000005845.2_ASM584v2/GCF_000005845.2_ASM584v2_genomic.fna.gz",
        "output": "ecoli_genome.fa",
        "description": "E. coli K-12 MG1655 genome",
    },
    "yeast_genome": {
        "url": "https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/000/146/045/GCF_000146045.2_R64/GCF_000146045.2_R64_genomic.fna.gz",
        "output": "yeast_genome.fa",
        "description": "S. cerevisiae S288C genome (R64)",
    },
    "clinvar_vcf": {
        "url": "https://ftp.ncbi.nlm.nih.gov/pub/clinvar/vcf_GRCh38/clinvar.vcf.gz",
        "output": "clinvar.vcf",
        "description": "ClinVar VCF (GRCh38, first 5000 variants)",
        "max_records": 5000,
    },
    "ecoli_gff": {
        "url": "https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/000/005/845/GCF_000005845.2_ASM584v2/GCF_000005845.2_ASM584v2_genomic.gff.gz",
        "output": "ecoli_annotation.gff",
        "description": "E. coli K-12 GFF3 annotation",
    },
}


def download_gz(url, description):
    """Download a gzipped file and return decompressed content as string."""
    print(f"  Downloading {description}...")
    print(f"    {url}")
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "BioLang-Benchmark/1.0"})
        with urllib.request.urlopen(req, timeout=120) as resp:
            compressed = resp.read()
        print(f"    Downloaded {len(compressed) / 1024 / 1024:.1f} MB compressed")
        return gzip.decompress(compressed).decode("utf-8", errors="replace")
    except urllib.error.URLError as e:
        print(f"    ERROR: {e}")
        return None


def download_fasta(key, info):
    """Download a FASTA genome."""
    outpath = os.path.join(DATA_DIR, info["output"])
    if os.path.exists(outpath):
        print(f"  {info['output']} already exists, skipping")
        return True

    content = download_gz(info["url"], info["description"])
    if content is None:
        return False

    with open(outpath, "w", newline="\n") as f:
        f.write(content)

    # Count sequences
    n_seqs = content.count("\n>") + (1 if content.startswith(">") else 0)
    total_bp = sum(len(line) for line in content.split("\n") if not line.startswith(">"))
    print(f"    Saved: {n_seqs} sequences, {total_bp:,} bp")
    return True


def download_vcf_subset(info):
    """Download ClinVar VCF and extract first N variant records."""
    outpath = os.path.join(DATA_DIR, info["output"])
    if os.path.exists(outpath):
        print(f"  {info['output']} already exists, skipping")
        return True

    max_records = info.get("max_records", 5000)
    print(f"  Downloading {info['description']}...")
    print(f"    {info['url']}")
    print(f"    (will extract first {max_records} variant records)")

    try:
        req = urllib.request.Request(info["url"], headers={"User-Agent": "BioLang-Benchmark/1.0"})
        with urllib.request.urlopen(req, timeout=300) as resp:
            # Stream decompression — don't load entire VCF into memory
            decompressor = gzip.GzipFile(fileobj=io.BytesIO(resp.read()))
            lines = []
            record_count = 0
            for raw_line in decompressor:
                line = raw_line.decode("utf-8", errors="replace")
                if line.startswith("#"):
                    lines.append(line)
                else:
                    if record_count >= max_records:
                        break
                    lines.append(line)
                    record_count += 1
    except urllib.error.URLError as e:
        print(f"    ERROR: {e}")
        return False

    with open(outpath, "w", newline="\n") as f:
        f.writelines(lines)

    print(f"    Saved: {record_count} variant records")
    return True


def download_gff(info):
    """Download a GFF3 annotation file."""
    outpath = os.path.join(DATA_DIR, info["output"])
    if os.path.exists(outpath):
        print(f"  {info['output']} already exists, skipping")
        return True

    content = download_gz(info["url"], info["description"])
    if content is None:
        return False

    with open(outpath, "w", newline="\n") as f:
        f.write(content)

    n_features = sum(1 for line in content.split("\n") if line and not line.startswith("#"))
    print(f"    Saved: {n_features} feature lines")
    return True


def derive_bed_from_gff(gff_path, bed_path):
    """Extract gene features from GFF3 as BED intervals."""
    if os.path.exists(bed_path):
        print(f"  {os.path.basename(bed_path)} already exists, skipping")
        return True

    print(f"  Deriving BED from GFF (gene features)...")
    genes = []
    with open(gff_path) as f:
        for line in f:
            if line.startswith("#"):
                continue
            parts = line.strip().split("\t")
            if len(parts) >= 9 and parts[2] == "gene":
                chrom = parts[0]
                start = int(parts[3]) - 1  # GFF is 1-based, BED is 0-based
                end = int(parts[4])
                # Extract gene name from attributes
                attrs = parts[8]
                name = "."
                for attr in attrs.split(";"):
                    if attr.startswith("Name="):
                        name = attr.split("=", 1)[1]
                        break
                genes.append(f"{chrom}\t{start}\t{end}\t{name}\n")

    with open(bed_path, "w", newline="\n") as f:
        f.writelines(genes)

    print(f"    Saved: {len(genes)} gene intervals")
    return True


def derive_csv_from_vcf(vcf_path, csv_path):
    """Extract key columns from VCF into CSV for group-by analysis."""
    if os.path.exists(csv_path):
        print(f"  {os.path.basename(csv_path)} already exists, skipping")
        return True

    print(f"  Deriving CSV from ClinVar VCF...")
    rows = []
    with open(vcf_path) as f:
        for line in f:
            if line.startswith("#"):
                continue
            parts = line.strip().split("\t")
            if len(parts) < 8:
                continue
            chrom = parts[0]
            pos = parts[1]
            ref = parts[3]
            alt = parts[4]

            # Parse INFO field for CLNSIG and GENEINFO
            info = parts[7]
            clnsig = "not_provided"
            gene = "unknown"
            for kv in info.split(";"):
                if kv.startswith("CLNSIG="):
                    clnsig = kv.split("=", 1)[1]
                elif kv.startswith("GENEINFO="):
                    gene = kv.split("=", 1)[1].split(":")[0]

            var_len = max(len(ref), len(alt))
            rows.append(f"{chrom},{pos},{ref},{alt},{clnsig},{gene},{var_len}\n")

    with open(csv_path, "w", newline="\n") as f:
        f.write("chrom,pos,ref,alt,clnsig,gene,var_len\n")
        f.writelines(rows)

    print(f"    Saved: {len(rows)} rows")
    return True


def main():
    print("=== BioLang Real-World Data Download ===\n")

    os.makedirs(DATA_DIR, exist_ok=True)

    success = True

    # Download genomes
    for key in ["ecoli_genome", "yeast_genome"]:
        if not download_fasta(key, DOWNLOADS[key]):
            success = False
        print()

    # Download ClinVar VCF subset
    if not download_vcf_subset(DOWNLOADS["clinvar_vcf"]):
        success = False
    print()

    # Download GFF annotation
    if not download_gff(DOWNLOADS["ecoli_gff"]):
        success = False
    print()

    # Derive BED from GFF
    gff_path = os.path.join(DATA_DIR, "ecoli_annotation.gff")
    bed_path = os.path.join(DATA_DIR, "ecoli_genes.bed")
    if os.path.exists(gff_path):
        derive_bed_from_gff(gff_path, bed_path)
    print()

    # Derive CSV from VCF
    vcf_path = os.path.join(DATA_DIR, "clinvar.vcf")
    csv_path = os.path.join(DATA_DIR, "clinvar_variants.csv")
    if os.path.exists(vcf_path):
        derive_csv_from_vcf(vcf_path, csv_path)
    print()

    # Summary
    print("=== Data Summary ===")
    for fname in sorted(os.listdir(DATA_DIR)):
        fpath = os.path.join(DATA_DIR, fname)
        size = os.path.getsize(fpath)
        if size > 1024 * 1024:
            print(f"  {fname:30s} {size / 1024 / 1024:.1f} MB")
        else:
            print(f"  {fname:30s} {size / 1024:.0f} KB")

    if success:
        print("\nAll downloads complete.")
    else:
        print("\nSome downloads failed. Re-run to retry (existing files are skipped).")
        sys.exit(1)


if __name__ == "__main__":
    main()
