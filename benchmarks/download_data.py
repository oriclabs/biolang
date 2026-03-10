#!/usr/bin/env python3
"""Download real-world bioinformatics datasets for benchmarking.

Downloads data from NCBI, UniProt, ENCODE, and Ensembl into data_real/.
All sources are freely available, no authentication required.

Datasets:
  1. E. coli K-12 MG1655 genome (FASTA) — NCBI RefSeq
  2. SARS-CoV-2 reference genome (FASTA) — NCBI RefSeq
  3. ClinVar variants (VCF) — NCBI FTP
  4. UniProt E. coli proteome (FASTA) — UniProt REST
  5. Human chromosome 22 (FASTA) — NCBI RefSeq
  6. ENCODE ChIP-seq peaks (BED) — ENCODE portal
  7. Ensembl gene annotations (GFF3) — Ensembl FTP

Usage:
  python download_data.py          # Download all
  python download_data.py ecoli    # Download specific dataset
  python download_data.py encode   # Download ENCODE BED peaks
  python download_data.py ensembl  # Download Ensembl GFF3
"""
import gzip
import hashlib
import os
import shutil
import sys
import urllib.parse
import urllib.request

DATA_DIR = "data_real"

DATASETS = {
    "ecoli_genome": {
        "url": "https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/000/005/845/GCF_000005845.2_ASM584v2/GCF_000005845.2_ASM584v2_genomic.fna.gz",
        "file": "ecoli_genome.fa.gz",
        "desc": "E. coli K-12 MG1655 complete genome (~1.4 MB gz, ~4.6 MB)",
    },
    "sarscov2_genome": {
        "url": "https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/009/858/895/GCF_009858895.2_ASM985889v3/GCF_009858895.2_ASM985889v3_genomic.fna.gz",
        "file": "sarscov2_genome.fa.gz",
        "desc": "SARS-CoV-2 reference genome (~10 KB gz, ~30 KB)",
    },
    "clinvar_vcf": {
        "url": "https://ftp.ncbi.nlm.nih.gov/pub/clinvar/vcf_GRCh38/clinvar.vcf.gz",
        "file": "clinvar.vcf.gz",
        "desc": "ClinVar GRCh38 variant database (~30 MB gz)",
    },
    "ecoli_proteome": {
        "url": "https://rest.uniprot.org/uniprotkb/stream?query=(organism_id:83333)+AND+(reviewed:true)&format=fasta&compressed=true",
        "file": "ecoli_proteome.fa.gz",
        "desc": "UniProt reviewed E. coli K-12 proteome (~1.5 MB gz, ~4.5 MB)",
    },
    "human_chr22": {
        "url": "https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/000/001/405/GCF_000001405.40_GRCh38.p14/GCF_000001405.40_GRCh38.p14_assembly_structure/Primary_Assembly/assembled_chromosomes/FASTA/chr22.fna.gz",
        "file": "human_chr22.fa.gz",
        "desc": "Human chromosome 22 (~12 MB gz, ~51 MB)",
    },
    "encode_peaks": {
        # ENCODE H3K27ac ChIP-seq replicated peaks (GM12878) — narrowPeak BED format
        "url": "https://www.encodeproject.org/files/ENCFF361XMX/@@download/ENCFF361XMX.bed.gz",
        "file": "encode_h3k27ac_peaks.bed.gz",
        "desc": "ENCODE H3K27ac ChIP-seq peaks, GM12878 ENCFF361XMX (~1.2 MB gz)",
    },
    "encode_peaks_ctcf": {
        # ENCODE CTCF ChIP-seq IDR peaks (GM12878) — narrowPeak BED format
        "url": "https://www.encodeproject.org/files/ENCFF797SDL/@@download/ENCFF797SDL.bed.gz",
        "file": "encode_ctcf_peaks.bed.gz",
        "desc": "ENCODE CTCF ChIP-seq peaks, GM12878 ENCFF797SDL (~0.6 MB gz)",
    },
    "ensembl_gff3": {
        "url": "https://ftp.ensembl.org/pub/release-112/gff3/homo_sapiens/Homo_sapiens.GRCh38.112.chromosome.22.gff3.gz",
        "file": "ensembl_chr22.gff3.gz",
        "desc": "Ensembl GRCh38.112 chr22 gene annotations (~1.5 MB gz)",
    },
}


def download(url, dest, desc=""):
    """Download a file with progress display."""
    if os.path.exists(dest):
        size_mb = os.path.getsize(dest) / 1_000_000
        print(f"  Already exists: {dest} ({size_mb:.1f} MB)")
        return True

    print(f"  Downloading: {desc or url}")
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "BioLang-Benchmark/1.0"})
        with urllib.request.urlopen(req, timeout=120) as resp:
            total = int(resp.headers.get("Content-Length", 0))
            downloaded = 0
            with open(dest + ".tmp", "wb") as f:
                while True:
                    chunk = resp.read(65536)
                    if not chunk:
                        break
                    f.write(chunk)
                    downloaded += len(chunk)
                    if total > 0:
                        pct = downloaded * 100 // total
                        mb = downloaded / 1_000_000
                        print(f"\r    {mb:.1f} MB ({pct}%)", end="", flush=True)
                    else:
                        mb = downloaded / 1_000_000
                        print(f"\r    {mb:.1f} MB", end="", flush=True)
            print()
        os.rename(dest + ".tmp", dest)
        return True
    except Exception as e:
        print(f"\n  FAILED: {e}")
        if os.path.exists(dest + ".tmp"):
            os.remove(dest + ".tmp")
        return False


def decompress_gz(gz_path, out_path):
    """Decompress a .gz file."""
    if os.path.exists(out_path):
        return
    print(f"  Decompressing {gz_path} -> {out_path}")
    with gzip.open(gz_path, "rb") as f_in, open(out_path, "wb") as f_out:
        shutil.copyfileobj(f_in, f_out)


def prepare_clinvar_subset(gz_path, out_path, max_variants=100_000):
    """Extract first N variants from ClinVar VCF for benchmarking."""
    if os.path.exists(out_path):
        return
    print(f"  Extracting {max_variants} variants from ClinVar...")
    count = 0
    with gzip.open(gz_path, "rt") as f_in, open(out_path, "w") as f_out:
        for line in f_in:
            if line.startswith("#"):
                f_out.write(line)
                continue
            f_out.write(line)
            count += 1
            if count >= max_variants:
                break
    print(f"    Extracted {count} variants")


def prepare_clinvar_diverse(gz_path, out_path, per_chrom=1000):
    """Extract N variants per chromosome from ClinVar for multi-chrom benchmarks."""
    if os.path.exists(out_path):
        return
    target_chroms = [str(i) for i in range(1, 23)] + ["X", "Y"]
    print(f"  Extracting ~{per_chrom} variants per chromosome from ClinVar...")
    chrom_counts = {}
    header_lines = []
    data_lines = []
    with gzip.open(gz_path, "rt") as f_in:
        for line in f_in:
            if line.startswith("#"):
                header_lines.append(line)
                continue
            chrom = line.split("\t", 1)[0]
            if chrom not in target_chroms:
                continue
            cc = chrom_counts.get(chrom, 0)
            if cc < per_chrom:
                data_lines.append(line)
                chrom_counts[chrom] = cc + 1
            if all(chrom_counts.get(c, 0) >= per_chrom for c in target_chroms):
                break
    with open(out_path, "w") as f_out:
        for line in header_lines:
            f_out.write(line)
        for line in data_lines:
            f_out.write(line)
    total = sum(chrom_counts.values())
    print(f"    Extracted {total} variants across {len(chrom_counts)} chromosomes")


def decompress_narrowpeak(gz_path, out_path):
    """Decompress ENCODE narrowPeak .bed.gz to standard 3-column BED."""
    if os.path.exists(out_path):
        return
    print(f"  Converting {gz_path} -> {out_path} (3-column BED)")
    count = 0
    with gzip.open(gz_path, "rt") as f_in, open(out_path, "w") as f_out:
        for line in f_in:
            if line.startswith("#") or line.startswith("track"):
                continue
            fields = line.strip().split("\t")
            if len(fields) >= 3:
                f_out.write(f"{fields[0]}\t{fields[1]}\t{fields[2]}\n")
                count += 1
    print(f"    Extracted {count} peaks")


def extract_gene_annotations_csv(gff_path, csv_path):
    """Extract gene annotations from Ensembl GFF3 into a CSV for benchmarks."""
    print(f"  Extracting gene annotations from {gff_path}")
    import re
    genes = []
    pathways = [
        "cell_cycle", "apoptosis", "signal_transduction", "metabolism",
        "immune_response", "dna_repair", "transcription", "translation",
        "chromatin_remodeling", "rna_processing", "protein_folding",
        "cell_adhesion", "ion_transport", "lipid_metabolism", "other",
    ]
    count = 0
    with open(gff_path) as f:
        for line in f:
            if line.startswith("#"):
                continue
            fields = line.strip().split("\t")
            if len(fields) < 9 or fields[2] != "gene":
                continue
            attrs = fields[8]
            name_match = re.search(r'Name=([^;]+)', attrs)
            biotype_match = re.search(r'biotype=([^;]+)', attrs)
            desc_match = re.search(r'description=([^;]+)', attrs)
            name = name_match.group(1) if name_match else f"gene_{count}"
            biotype = biotype_match.group(1) if biotype_match else "unknown"
            desc = desc_match.group(1) if desc_match else ""
            desc = urllib.parse.unquote(desc).replace(",", ";")
            chrom = fields[0].replace("22", "chr22") if not fields[0].startswith("chr") else fields[0]
            # Assign a pathway deterministically based on gene name hash
            pathway = pathways[hash(name) % len(pathways)]
            genes.append((name, chrom, biotype, pathway, desc[:100]))
            count += 1

    with open(csv_path, "w") as f:
        f.write("gene,chromosome,biotype,pathway,description\n")
        for g in genes:
            f.write(f"{g[0]},{g[1]},{g[2]},{g[3]},{g[4]}\n")
    print(f"    Extracted {count} genes")


def prepare_all():
    os.makedirs(DATA_DIR, exist_ok=True)

    filter_arg = sys.argv[1].lower() if len(sys.argv) > 1 else "all"

    # 1. E. coli genome
    if filter_arg in ("all", "ecoli", "fasta"):
        print("\n[1/5] E. coli K-12 genome")
        ds = DATASETS["ecoli_genome"]
        gz = os.path.join(DATA_DIR, ds["file"])
        fa = os.path.join(DATA_DIR, "ecoli_genome.fa")
        if download(ds["url"], gz, ds["desc"]):
            decompress_gz(gz, fa)

    # 2. SARS-CoV-2 genome (small reference for quick tests)
    if filter_arg in ("all", "sarscov2", "fasta"):
        print("\n[2/5] SARS-CoV-2 reference genome")
        ds = DATASETS["sarscov2_genome"]
        gz = os.path.join(DATA_DIR, ds["file"])
        fa = os.path.join(DATA_DIR, "sarscov2_genome.fa")
        if download(ds["url"], gz, ds["desc"]):
            decompress_gz(gz, fa)

    # 3. ClinVar VCF
    if filter_arg in ("all", "clinvar", "vcf"):
        print("\n[3/5] ClinVar GRCh38 variants")
        ds = DATASETS["clinvar_vcf"]
        gz = os.path.join(DATA_DIR, ds["file"])
        vcf_20k = os.path.join(DATA_DIR, "clinvar_20k.vcf")
        vcf_100k = os.path.join(DATA_DIR, "clinvar_100k.vcf")
        if download(ds["url"], gz, ds["desc"]):
            prepare_clinvar_subset(gz, vcf_20k, max_variants=20_000)
            prepare_clinvar_subset(gz, vcf_100k, max_variants=100_000)
            # Multi-chromosome subset for pipeline benchmarks
            vcf_diverse = os.path.join(DATA_DIR, "clinvar_diverse.vcf")
            prepare_clinvar_diverse(gz, vcf_diverse, per_chrom=1000)

    # 4. E. coli proteome
    if filter_arg in ("all", "ecoli", "proteome", "fasta"):
        print("\n[4/5] UniProt E. coli K-12 proteome")
        ds = DATASETS["ecoli_proteome"]
        gz = os.path.join(DATA_DIR, ds["file"])
        fa = os.path.join(DATA_DIR, "ecoli_proteome.fa")
        if download(ds["url"], gz, ds["desc"]):
            decompress_gz(gz, fa)

    # 5. Human chr22 (larger FASTA benchmark)
    if filter_arg in ("all", "human", "chr22", "fasta"):
        print("\n[5/5] Human chromosome 22")
        ds = DATASETS["human_chr22"]
        gz = os.path.join(DATA_DIR, ds["file"])
        fa = os.path.join(DATA_DIR, "human_chr22.fa")
        if download(ds["url"], gz, ds["desc"]):
            decompress_gz(gz, fa)

    # 6. ENCODE ChIP-seq peaks (BED)
    if filter_arg in ("all", "encode", "bed", "intervals"):
        print("\n[6/7] ENCODE ChIP-seq peaks (H3K27ac + CTCF)")
        # H3K27ac peaks — used as "regions" in overlap benchmarks
        ds = DATASETS["encode_peaks"]
        gz = os.path.join(DATA_DIR, ds["file"])
        bed = os.path.join(DATA_DIR, "encode_h3k27ac_peaks.bed")
        if download(ds["url"], gz, ds["desc"]):
            decompress_narrowpeak(gz, bed)
        # CTCF peaks — used as "queries" in overlap benchmarks
        ds2 = DATASETS["encode_peaks_ctcf"]
        gz2 = os.path.join(DATA_DIR, ds2["file"])
        bed2 = os.path.join(DATA_DIR, "encode_ctcf_peaks.bed")
        if download(ds2["url"], gz2, ds2["desc"]):
            decompress_narrowpeak(gz2, bed2)

    # 7. Ensembl GFF3 gene annotations
    if filter_arg in ("all", "ensembl", "gff", "annotations"):
        print("\n[7/7] Ensembl GFF3 gene annotations (chr22)")
        ds = DATASETS["ensembl_gff3"]
        gz = os.path.join(DATA_DIR, ds["file"])
        gff = os.path.join(DATA_DIR, "ensembl_chr22.gff3")
        if download(ds["url"], gz, ds["desc"]):
            decompress_gz(gz, gff)
        # Also generate a gene_annotations CSV from the GFF3
        gene_csv = os.path.join(DATA_DIR, "gene_annotations.csv")
        if os.path.exists(gff) and not os.path.exists(gene_csv):
            extract_gene_annotations_csv(gff, gene_csv)

    # Summary
    print("\n" + "=" * 50)
    print("Downloaded files:")
    if os.path.exists(DATA_DIR):
        for fname in sorted(os.listdir(DATA_DIR)):
            if fname.endswith(".tmp"):
                continue
            fpath = os.path.join(DATA_DIR, fname)
            size = os.path.getsize(fpath)
            if size > 1_000_000:
                print(f"  {fname}: {size / 1_000_000:.1f} MB")
            else:
                print(f"  {fname}: {size / 1_000:.1f} KB")


if __name__ == "__main__":
    prepare_all()
