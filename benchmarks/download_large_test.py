#!/usr/bin/env python3
"""Download real large public datasets for BLViewer stress testing.
All files are from public databases — no authentication needed."""
import os, urllib.request, gzip, shutil, subprocess

OUT = os.path.join(os.path.dirname(__file__), "data_large_real")
os.makedirs(OUT, exist_ok=True)

def download(url, dest, desc):
    if os.path.exists(dest):
        sz = os.path.getsize(dest) / (1024*1024)
        print(f"  SKIP {desc} ({sz:.1f} MB already exists)")
        return
    print(f"  Downloading {desc}...")
    print(f"    URL: {url}")
    try:
        urllib.request.urlretrieve(url, dest)
        sz = os.path.getsize(dest) / (1024*1024)
        print(f"    Done: {sz:.1f} MB")
    except Exception as e:
        print(f"    FAILED: {e}")
        if os.path.exists(dest):
            os.remove(dest)

def decompress_gz(src, dest, desc):
    if os.path.exists(dest):
        sz = os.path.getsize(dest) / (1024*1024)
        print(f"  SKIP {desc} ({sz:.1f} MB already exists)")
        return
    if not os.path.exists(src):
        print(f"  SKIP {desc} (source not found)")
        return
    print(f"  Decompressing {desc}...")
    with gzip.open(src, 'rb') as f_in, open(dest, 'wb') as f_out:
        shutil.copyfileobj(f_in, f_out)
    sz = os.path.getsize(dest) / (1024*1024)
    print(f"    Done: {sz:.1f} MB")

def head_lines(src, dest, n, desc):
    """Extract first n non-header lines (preserve headers starting with # or @)."""
    if os.path.exists(dest):
        sz = os.path.getsize(dest) / (1024*1024)
        print(f"  SKIP {desc} ({sz:.1f} MB already exists)")
        return
    if not os.path.exists(src):
        print(f"  SKIP {desc} (source not found)")
        return
    print(f"  Extracting {n} data rows for {desc}...")
    count = 0
    with open(src, 'r', errors='replace') as fin, open(dest, 'w') as fout:
        for line in fin:
            if line.startswith('#') or line.startswith('@'):
                fout.write(line)
            else:
                fout.write(line)
                count += 1
                if count >= n:
                    break
    sz = os.path.getsize(dest) / (1024*1024)
    print(f"    Done: {sz:.1f} MB ({count} data rows)")


print("=" * 60)
print("Downloading real large datasets for BLViewer testing")
print("=" * 60)

# ─── 1. ClinVar VCF — full current release (~700MB uncompressed) ─
print("\n[1/8] ClinVar VCF (full human variant database)")
download(
    "https://ftp.ncbi.nlm.nih.gov/pub/clinvar/vcf_GRCh38/clinvar.vcf.gz",
    os.path.join(OUT, "clinvar_full.vcf.gz"),
    "ClinVar VCF (GRCh38)"
)
decompress_gz(
    os.path.join(OUT, "clinvar_full.vcf.gz"),
    os.path.join(OUT, "clinvar_full.vcf"),
    "ClinVar VCF full"
)

# ─── 2. dbSNP VCF chr22 — (~150MB compressed, ~1GB uncompressed) ─
print("\n[2/8] dbSNP chr22 (common human variants)")
download(
    "https://ftp.ncbi.nih.gov/snp/organisms/human_9606/VCF/common_all_20180418.vcf.gz",
    os.path.join(OUT, "dbsnp_common.vcf.gz"),
    "dbSNP common variants"
)
# dbSNP is huge — just decompress and take 500K rows
if os.path.exists(os.path.join(OUT, "dbsnp_common.vcf.gz")) and not os.path.exists(os.path.join(OUT, "dbsnp_500k.vcf")):
    decompress_gz(
        os.path.join(OUT, "dbsnp_common.vcf.gz"),
        os.path.join(OUT, "dbsnp_common.vcf"),
        "dbSNP common VCF"
    )
    head_lines(
        os.path.join(OUT, "dbsnp_common.vcf"),
        os.path.join(OUT, "dbsnp_500k.vcf"),
        500000,
        "dbSNP 500K subset"
    )

# ─── 3. ENCODE BED — all CTCF sites genome-wide (~3M peaks) ─────
print("\n[3/8] ENCODE CTCF ChIP-seq peaks (genome-wide, many cell types)")
download(
    "https://www.encodeproject.org/files/ENCFF706QLS/@@download/ENCFF706QLS.bed.gz",
    os.path.join(OUT, "encode_ctcf_gw.bed.gz"),
    "ENCODE CTCF genome-wide BED"
)
decompress_gz(
    os.path.join(OUT, "encode_ctcf_gw.bed.gz"),
    os.path.join(OUT, "encode_ctcf_gw.bed"),
    "ENCODE CTCF genome-wide BED"
)

# ─── 4. Ensembl GFF3 — full human annotation (~40MB compressed, ~300MB uncompressed) ─
print("\n[4/8] Ensembl full human gene annotation GFF3")
download(
    "https://ftp.ensembl.org/pub/release-112/gff3/homo_sapiens/Homo_sapiens.GRCh38.112.gff3.gz",
    os.path.join(OUT, "ensembl_human_full.gff3.gz"),
    "Ensembl human GFF3"
)
decompress_gz(
    os.path.join(OUT, "ensembl_human_full.gff3.gz"),
    os.path.join(OUT, "ensembl_human_full.gff3"),
    "Ensembl human GFF3"
)

# ─── 5. UniProt TSV — full human proteome with annotations ──────
print("\n[5/8] UniProt human proteome TSV")
download(
    "https://rest.uniprot.org/uniprotkb/stream?compressed=false&fields=accession%2Creviewed%2Cid%2Cprotein_name%2Cgene_names%2Corganism_name%2Clength%2Cgo_id%2Cgo_p%2Cgo_f%2Cgo_c%2Ccc_subcellular_location%2Cft_domain%2Ccc_disease&format=tsv&query=%28organism_id%3A9606%29+AND+%28reviewed%3Atrue%29",
    os.path.join(OUT, "uniprot_human.tsv"),
    "UniProt human reviewed proteome TSV"
)

# ─── 6. GENCODE GTF — full human annotation (~50MB compressed, ~1.5GB uncompressed) ─
print("\n[6/8] GENCODE comprehensive human annotation GTF")
download(
    "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/release_46/gencode.v46.annotation.gtf.gz",
    os.path.join(OUT, "gencode_v46.gtf.gz"),
    "GENCODE v46 GTF"
)
decompress_gz(
    os.path.join(OUT, "gencode_v46.gtf.gz"),
    os.path.join(OUT, "gencode_v46.gtf"),
    "GENCODE v46 GTF"
)

# ─── 7. RefSeq human genome FASTA (chr1 — ~250MB) ───────────────
print("\n[7/8] Human chromosome 1 FASTA (largest chromosome)")
download(
    "https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/000/001/405/GCF_000001405.40_GRCh38.p14/GCF_000001405.40_GRCh38.p14_assembly_structure/Primary_Assembly/assembled_chromosomes/FASTA/chr1.fna.gz",
    os.path.join(OUT, "human_chr1.fa.gz"),
    "Human chr1 FASTA"
)
decompress_gz(
    os.path.join(OUT, "human_chr1.fa.gz"),
    os.path.join(OUT, "human_chr1.fa"),
    "Human chr1 FASTA"
)

# ─── 8. SRA FASTQ — real Illumina reads (small run) ─────────────
print("\n[8/8] Real Illumina FASTQ reads")
# E. coli MiSeq run — ~50MB compressed, ~200MB uncompressed
download(
    "https://ftp.sra.ebi.ac.uk/vol1/fastq/SRR190/001/SRR1900341/SRR1900341_1.fastq.gz",
    os.path.join(OUT, "ecoli_miseq_R1.fq.gz"),
    "E. coli MiSeq R1 FASTQ"
)
decompress_gz(
    os.path.join(OUT, "ecoli_miseq_R1.fq.gz"),
    os.path.join(OUT, "ecoli_miseq_R1.fq"),
    "E. coli MiSeq R1 FASTQ"
)

# ─── Summary ─────────────────────────────────────────────────────
print("\n" + "=" * 60)
print("Files in:", OUT)
print("=" * 60)
total = 0
for fn in sorted(os.listdir(OUT)):
    fp = os.path.join(OUT, fn)
    if fn.endswith(".gz"):
        continue  # skip compressed, show decompressed
    sz = os.path.getsize(fp)
    total += sz
    if sz > 1024*1024*1024:
        print(f"  {fn}: {sz / (1024*1024*1024):.1f} GB")
    elif sz > 1024*1024:
        print(f"  {fn}: {sz / (1024*1024):.0f} MB")
    else:
        print(f"  {fn}: {sz / 1024:.0f} KB")
print(f"\nTotal (uncompressed): {total / (1024*1024*1024):.1f} GB")
print("\nRecommended test order (small → large):")
print("  1. uniprot_human.tsv        (~20K rows, real protein data)")
print("  2. encode_ctcf_gw.bed       (~100K+ peaks)")
print("  3. clinvar_full.vcf          (~2M variants, real clinical)")
print("  4. ensembl_human_full.gff3   (~3M features)")
print("  5. gencode_v46.gtf           (~3M features, largest GFF)")
print("  6. dbsnp_500k.vcf            (500K real SNPs)")
print("  7. ecoli_miseq_R1.fq         (real sequencing reads)")
print("  8. human_chr1.fa             (248MB single sequence)")
