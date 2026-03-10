#!/usr/bin/env bash
# BioLang Benchmark Suite — Linux/macOS
# Usage: ./run_all.sh [category_filter]
#   category_filter: "all", "language", "pipelines", or category name (e.g. "kmer", "file_io", "qc")
#
# Requires: bl (BioLang), python3 (with biopython), Rscript (with Bioconductor)
# Generate data first: python3 generate_data.py

set -euo pipefail

RUNS=3
CATEGORY_FILTER="${1:-all}"

cd "$(dirname "$0")"

# ── Activate venv if present ──
if [[ -z "${VIRTUAL_ENV:-}" && -f ".venv/bin/activate" ]]; then
  echo "Activating Python venv (.venv) ..."
  source .venv/bin/activate
fi

# ── Dependency check ──
echo "Checking dependencies ..."
MISSING=""
if ! command -v bl &>/dev/null; then
  echo "  [WARN] bl not found — BioLang benchmarks will fail"
  echo "         Install: cargo install --path ../crates/bl-cli"
  MISSING="$MISSING bl"
fi
if ! command -v python3 &>/dev/null; then
  echo "  [WARN] python3 not found — Python benchmarks will fail"
  MISSING="$MISSING python3"
else
  if ! python3 -c "from Bio import SeqIO" &>/dev/null; then
    echo "  [WARN] biopython not installed — most Python benchmarks will fail"
    echo "         Fix: ./setup_python.sh  (or: pip install biopython)"
    MISSING="$MISSING biopython"
  fi
fi
if ! command -v Rscript &>/dev/null; then
  echo "  [INFO] Rscript not found — R benchmarks will be skipped"
else
  if ! Rscript -e 'library(Biostrings)' &>/dev/null 2>&1; then
    echo "  [WARN] R Bioconductor packages missing — R benchmarks will fail"
    echo "         Fix: ./setup_r.sh"
    MISSING="$MISSING r-bioc"
  fi
fi
if [[ -n "$MISSING" ]]; then
  echo ""
  echo "  Missing:$MISSING"
  echo "  Run ./setup_python.sh and/or ./setup_r.sh to install dependencies."
  echo "  Continuing anyway — failed benchmarks will show as 'null'."
  echo ""
fi

if [[ ! -d "data" ]]; then
  echo "No data/ directory. Run: python3 generate_data.py"
  exit 1
fi

# ── Platform info ──
collect_platform() {
  echo "date=$(date '+%Y-%m-%d %H:%M:%S')"
  echo "os=$(uname -srm)"
  if [[ "$(uname)" == "Darwin" ]]; then
    echo "cpu=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo unknown)"
    echo "cpu_cores=$(sysctl -n hw.ncpu 2>/dev/null || echo unknown)"
    local ram_bytes=$(sysctl -n hw.memsize 2>/dev/null || echo 0)
    echo "ram=$(( ram_bytes / 1073741824 )) GB"
  else
    echo "cpu=$(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2 | xargs || echo unknown)"
    echo "cpu_cores=$(nproc 2>/dev/null || echo unknown)"
    local ram_kb=$(grep MemTotal /proc/meminfo 2>/dev/null | awk '{print $2}' || echo 0)
    echo "ram=$(( ram_kb / 1048576 )) GB"
  fi
  echo "biolang=$(bl --version 2>/dev/null || echo 'not found')"
  echo "python=$(python3 --version 2>/dev/null | sed 's/Python //' || echo 'not found')"
  echo "r=$(Rscript --version 2>&1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo 'not found')"
  echo "rustc=$(rustc --version 2>/dev/null | sed 's/rustc //' || echo 'not found')"
}

loc() {
  grep -cvE '^\s*(#|//|"""|$)' "$1" 2>/dev/null || echo 0
}

# Temp files for passing data out of best_time (avoids subshell variable loss)
_BT_OUTPUT_FILE=$(mktemp)
_BT_VALID_FILE=$(mktemp)

best_time() {
  local cmd="$1"
  local best=999999
  local tmpout tmperr
  tmpout=$(mktemp)
  tmperr=$(mktemp)
  echo "" > "$_BT_OUTPUT_FILE"
  echo "true" > "$_BT_VALID_FILE"
  for i in $(seq 1 $RUNS); do
    local t exit_code=0
    t=$( { TIMEFORMAT='%R'; time eval "$cmd" > "$tmpout" 2>"$tmperr"; } 2>&1 ) || exit_code=$?
    # Check for failure: non-zero exit, or stderr contains error/traceback
    if [[ $exit_code -ne 0 ]] || grep -qiE '(error|traceback|exception|not found|no such file|cannot open|fatal)' "$tmperr" 2>/dev/null; then
      echo "false" > "$_BT_VALID_FILE"
      local errmsg
      errmsg=$(head -3 "$tmperr" | tr '\n' ' ')
      echo "    [FAIL] $errmsg" >&2
      break
    fi
    if (( $(echo "$t < $best" | bc -l) )); then
      best=$t
      cp "$tmpout" "$_BT_OUTPUT_FILE"
    fi
  done
  rm -f "$tmpout" "$tmperr"
  if [[ "$(cat "$_BT_VALID_FILE")" == "false" ]]; then
    echo "null"
  else
    echo "$best"
  fi
}

# Read captured output after best_time call
get_last_output() {
  cat "$_BT_OUTPUT_FILE"
}

is_last_valid() {
  [[ "$(cat "$_BT_VALID_FILE")" == "true" ]]
}

# ── Collect platform ──
declare -A PLAT
while IFS='=' read -r key val; do
  PLAT[$key]="$val"
done < <(collect_platform)

echo "================================================================"
echo "BioLang Benchmark Suite"
echo "================================================================"
echo "  Date:     ${PLAT[date]}"
echo "  OS:       ${PLAT[os]}"
echo "  CPU:      ${PLAT[cpu]}"
echo "  Cores:    ${PLAT[cpu_cores]}"
echo "  RAM:      ${PLAT[ram]}"
echo "  BioLang:  ${PLAT[biolang]}"
echo "  Python:   ${PLAT[python]}"
echo "  R:        ${PLAT[r]}"
echo "  Rust:     ${PLAT[rustc]}"
echo "  Runs:     $RUNS (best of)"
echo "================================================================"
echo ""

echo "Input data:"
for f in data/*; do
  size=$(du -h "$f" | cut -f1)
  echo "  $(basename "$f"): $size"
done
if [[ -d "data_real" ]]; then
  for f in data_real/*; do
    size=$(du -h "$f" | cut -f1)
    echo "  $(basename "$f"): $size"
  done
fi
echo ""

# ── Setup results ──
mkdir -p results
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
case "$(uname -s)" in
  Darwin*) PLATFORM_TAG="macos" ;;
  Linux*)  PLATFORM_TAG="linux" ;;
  MINGW*|MSYS*|CYGWIN*) PLATFORM_TAG="windows" ;;
  *)       PLATFORM_TAG="$(uname -s | tr '[:upper:]' '[:lower:]')" ;;
esac

# ── Category definitions ──
# Format: group|cat_id|cat_name|task_id|task_name|suite|bl_path|py_path|r_path
BENCHMARKS=(
  # Sequence I/O
  "language|sequence_io|Sequence I/O|1_fasta_stats|FASTA Statistics|synthetic|language/sequence_io/biolang/1_fasta_stats.bl|language/sequence_io/python/1_fasta_stats.py|language/sequence_io/r/1_fasta_stats.R"
  "language|sequence_io|Sequence I/O|2_fastq_qc|FASTQ QC|synthetic|language/sequence_io/biolang/2_fastq_qc.bl|language/sequence_io/python/2_fastq_qc.py|language/sequence_io/r/2_fastq_qc.R"
  "language|sequence_io|Sequence I/O|6_genome_stats|E.coli Genome Stats|real|language/sequence_io/biolang/6_genome_stats.bl|language/sequence_io/python/6_genome_stats.py|language/sequence_io/r/6_genome_stats.R"
  "language|sequence_io|Sequence I/O|9_human_chr22|Human Chr22 Stats|real|language/sequence_io/biolang/9_human_chr22.bl|language/sequence_io/python/9_human_chr22.py|language/sequence_io/r/9_human_chr22.R"
  "language|sequence_io|Sequence I/O|gc_content_scale|GC Content (51 MB)|real|language/sequence_io/biolang/gc_content_scale.bl|language/sequence_io/python/gc_content_scale.py|language/sequence_io/r/gc_content_scale.R"
  "language|sequence_io|Sequence I/O|reverse_complement|Reverse Complement|synthetic|language/sequence_io/biolang/reverse_complement.bl|language/sequence_io/python/reverse_complement.py|language/sequence_io/r/reverse_complement.R"
  # K-mer Analysis
  "language|kmer|K-mer Analysis|3_kmer_count|K-mer Counting|synthetic|language/kmer/biolang/3_kmer_count.bl|language/kmer/python/3_kmer_count.py|language/kmer/r/3_kmer_count.R"
  "language|kmer|K-mer Analysis|10_chr22_kmers|Chr22 21-mer Count|real|language/kmer/biolang/10_chr22_kmers.bl|language/kmer/python/10_chr22_kmers.py|language/kmer/r/10_chr22_kmers.R"
  # Variant Analysis
  "language|variants|Variant Analysis|4_vcf_filter|VCF Filtering|synthetic|language/variants/biolang/4_vcf_filter.bl|language/variants/python/4_vcf_filter.py|language/variants/r/4_vcf_filter.R"
  "language|variants|Variant Analysis|7_clinvar_filter|ClinVar Variants|real|language/variants/biolang/7_clinvar_filter.bl|language/variants/python/7_clinvar_filter.py|language/variants/r/7_clinvar_filter.R"
  # Data Wrangling
  "language|wrangling|Data Wrangling|5_csv_wrangle|CSV Join + Group-by|synthetic|language/wrangling/biolang/5_csv_wrangle.bl|language/wrangling/python/5_csv_wrangle.py|language/wrangling/r/5_csv_wrangle.R"
  # Protein Analysis
  "language|protein|Protein Analysis|8_protein_kmers|Protein K-mers|real|language/protein/biolang/8_protein_kmers.bl|language/protein/python/8_protein_kmers.py|language/protein/r/8_protein_kmers.R"
  # File I/O
  "language|file_io|File I/O|parse_fasta_small|FASTA Small (30 KB)|real|language/file_io/biolang/parse_fasta_small.bl|language/file_io/python/parse_fasta_small.py|language/file_io/r/parse_fasta_small.R"
  "language|file_io|File I/O|parse_fasta_medium|FASTA Medium (4.6 MB)|real|language/file_io/biolang/parse_fasta_medium.bl|language/file_io/python/parse_fasta_medium.py|language/file_io/r/parse_fasta_medium.R"
  "language|file_io|File I/O|parse_fasta_large|FASTA Large (51 MB)|real|language/file_io/biolang/parse_fasta_large.bl|language/file_io/python/parse_fasta_large.py|language/file_io/r/parse_fasta_large.R"
  "language|file_io|File I/O|parse_fastq|FASTQ (26 MB)|synthetic|language/file_io/biolang/parse_fastq.bl|language/file_io/python/parse_fastq.py|language/file_io/r/parse_fastq.R"
  "language|file_io|File I/O|parse_vcf|VCF (2.3 MB)|synthetic|language/file_io/biolang/parse_vcf.bl|language/file_io/python/parse_vcf.py|language/file_io/r/parse_vcf.R"
  "language|file_io|File I/O|parse_csv|CSV (0.1 MB)|synthetic|language/file_io/biolang/parse_csv.bl|language/file_io/python/parse_csv.py|language/file_io/r/parse_csv.R"
  "language|file_io|File I/O|parse_fasta_gz|FASTA gzipped (1.3 MB)|real|language/file_io/biolang/parse_fasta_gz.bl|language/file_io/python/parse_fasta_gz.py|language/file_io/r/parse_fasta_gz.R"
  "language|file_io|File I/O|parse_fasta_large_gz|FASTA Large gzipped (10 MB)|real|language/file_io/biolang/parse_fasta_large_gz.bl|language/file_io/python/parse_fasta_large_gz.py|language/file_io/r/parse_fasta_large_gz.R"
  "language|file_io|File I/O|write_filtered_fasta|Write Filtered FASTA|synthetic|language/file_io/biolang/write_filtered_fasta.bl|language/file_io/python/write_filtered_fasta.py|language/file_io/r/write_filtered_fasta.R"
  "language|file_io|File I/O|parse_gff|GFF3 (1.7 MB)|synthetic|language/file_io/biolang/parse_gff.bl|language/file_io/python/parse_gff.py|language/file_io/r/parse_gff.R"
  "language|file_io|File I/O|parse_gff_real|GFF3 Ensembl chr22|real|language/file_io/biolang/parse_gff_real.bl|language/file_io/python/parse_gff_real.py|"
  # Interval Operations
  "language|intervals|Interval Operations|bed_overlap|BED Interval Overlap|synthetic|language/intervals/biolang/bed_overlap.bl|language/intervals/python/bed_overlap.py|language/intervals/r/bed_overlap.R"
  "language|intervals|Interval Operations|bed_overlap_real|ENCODE Peak Overlap|real|language/intervals/biolang/bed_overlap_real.bl|language/intervals/python/bed_overlap_real.py|"
  # Pipelines
  "pipelines|qc_pipeline|QC Pipeline|qc_pipeline|FASTQ QC Pipeline|synthetic|pipelines/qc_pipeline/biolang/qc_pipeline.bl|pipelines/qc_pipeline/python/qc_pipeline.py|"
  "pipelines|variant_pipeline|Variant Pipeline|variant_pipeline|Variant Analysis Pipeline|synthetic|pipelines/variant_pipeline/biolang/variant_pipeline.bl|pipelines/variant_pipeline/python/variant_pipeline.py|"
  "pipelines|variant_pipeline|Variant Pipeline|variant_pipeline_real|ClinVar Variant Pipeline|real|pipelines/variant_pipeline/biolang/variant_pipeline_real.bl|pipelines/variant_pipeline/python/variant_pipeline_real.py|"
  "pipelines|multi_sample|Multi-Sample Pipeline|multi_sample|Multi-Sample Aggregation|synthetic|pipelines/multi_sample/biolang/multi_sample.bl|pipelines/multi_sample/python/multi_sample.py|"
  "pipelines|rnaseq_mini|RNA-seq Mini Pipeline|rnaseq_mini|RNA-seq DE Analysis|synthetic|pipelines/rnaseq_mini/biolang/rnaseq_mini.bl|pipelines/rnaseq_mini/python/rnaseq_mini.py|"
  "pipelines|annotation|Annotation Pipeline|annotation|Variant Annotation|synthetic|pipelines/annotation/biolang/annotation.bl|pipelines/annotation/python/annotation.py|"
  "pipelines|annotation|Annotation Pipeline|annotation_real|ClinVar + Ensembl Annotation|real|pipelines/annotation/biolang/annotation_real.bl|pipelines/annotation/python/annotation_real.py|"
)

# ── Filter function ──
should_run() {
  local group="$1" cat_id="$2"
  [[ "$CATEGORY_FILTER" == "all" ]] && return 0
  [[ "$CATEGORY_FILTER" == "$group" ]] && return 0
  [[ "$cat_id" == *"$CATEGORY_FILTER"* ]] && return 0
  return 1
}

# ── Run benchmarks by category ──
CURRENT_CAT=""
declare -A CAT_CSV CAT_REPORT
# Summary arrays
declare -a SUMMARY_CATS
declare -A SUMMARY_DATA  # key: cat_id|task_id|field -> value

for entry in "${BENCHMARKS[@]}"; do
  IFS='|' read -r group cat_id cat_name task_id task_name suite bl_path py_path r_path <<< "$entry"

  should_run "$group" "$cat_id" || continue

  # Skip real-world if no data
  if [[ "$suite" == "real" && ! -d "data_real" ]]; then
    continue
  fi

  # Category header
  if [[ "$cat_id" != "$CURRENT_CAT" ]]; then
    CURRENT_CAT="$cat_id"
    [[ "$group" == "language" ]] && prefix="lang" || prefix="pipe"
    CAT_CSV[$cat_id]="results/${prefix}_${cat_id}_${PLATFORM_TAG}_${TIMESTAMP}.csv"
    CAT_REPORT[$cat_id]="results/${prefix}_${cat_id}_${PLATFORM_TAG}_${TIMESTAMP}.md"
    echo "task,language,time_sec,loc" > "${CAT_CSV[$cat_id]}"

    # Track category order
    SUMMARY_CATS+=("$cat_id")
    SUMMARY_DATA["${cat_id}|_name"]="$cat_name"
    SUMMARY_DATA["${cat_id}|_group"]="$group"

    echo "================================================================"
    echo "  $cat_name ($group/$cat_id)"
    echo "================================================================"
    echo ""
  fi

  echo "  $task_name"

  # BioLang
  if [[ -n "$bl_path" && -f "$bl_path" ]]; then
    bl_loc=$(loc "$bl_path")
    bl_time=$(best_time "bl run $bl_path")
    if [[ "$bl_time" == "null" ]]; then
      echo "    BioLang:  FAILED  (${bl_loc} LOC)"
    else
      echo "    BioLang:  ${bl_time}s  (${bl_loc} LOC)"
      echo "$task_id,biolang,$bl_time,$bl_loc" >> "${CAT_CSV[$cat_id]}"
      SUMMARY_DATA["${cat_id}|${task_id}|bl_time"]="$bl_time"
      SUMMARY_DATA["${cat_id}|${task_id}|bl_loc"]="$bl_loc"
      SUMMARY_DATA["${cat_id}|${task_id}|bl_output"]="$(get_last_output)"
    fi
  fi

  # Python
  if [[ -n "$py_path" && -f "$py_path" ]]; then
    py_loc=$(loc "$py_path")
    py_time=$(best_time "python3 $py_path")
    if [[ "$py_time" == "null" ]]; then
      echo "    Python:   FAILED  (${py_loc} LOC) — run setup_python.sh?"
    else
      echo "    Python:   ${py_time}s  (${py_loc} LOC)"
      echo "$task_id,python,$py_time,$py_loc" >> "${CAT_CSV[$cat_id]}"
      SUMMARY_DATA["${cat_id}|${task_id}|py_time"]="$py_time"
      SUMMARY_DATA["${cat_id}|${task_id}|py_loc"]="$py_loc"
      SUMMARY_DATA["${cat_id}|${task_id}|py_output"]="$(get_last_output)"
    fi
  fi

  # R
  if [[ -n "$r_path" && -f "$r_path" ]]; then
    r_loc=$(loc "$r_path")
    r_time=$(best_time "Rscript $r_path")
    if [[ "$r_time" == "null" ]]; then
      echo "    R:        FAILED  (${r_loc} LOC) — run setup_r.sh?"
    else
      echo "    R:        ${r_time}s  (${r_loc} LOC)"
      echo "$task_id,r,$r_time,$r_loc" >> "${CAT_CSV[$cat_id]}"
      SUMMARY_DATA["${cat_id}|${task_id}|r_time"]="$r_time"
      SUMMARY_DATA["${cat_id}|${task_id}|r_loc"]="$r_loc"
      SUMMARY_DATA["${cat_id}|${task_id}|r_output"]="$(get_last_output)"
    fi
  fi

  # Track task in category
  SUMMARY_DATA["${cat_id}|${task_id}|_name"]="$task_name"
  SUMMARY_DATA["${cat_id}|${task_id}|_suite"]="$suite"
  tasks_key="${cat_id}|_tasks"
  if [[ -z "${SUMMARY_DATA[$tasks_key]:-}" ]]; then
    SUMMARY_DATA[$tasks_key]="$task_id"
  else
    SUMMARY_DATA[$tasks_key]="${SUMMARY_DATA[$tasks_key]}|$task_id"
  fi

  echo ""
done

# ── Generate per-category reports ──
for cat_id in "${SUMMARY_CATS[@]}"; do
  [[ -z "${CAT_REPORT[$cat_id]:-}" ]] && continue
  cat_name="${SUMMARY_DATA["${cat_id}|_name"]}"
  report_file="${CAT_REPORT[$cat_id]}"

  {
    echo "# $cat_name Benchmark Report"
    echo ""
    echo "**Platform**: ${PLAT[os]}, ${PLAT[cpu]}, ${PLAT[ram]}"
    echo "**Date**: ${PLAT[date]}"
    echo ""
    echo "## Execution Time (seconds, best of $RUNS)"
    echo ""
    echo "| Task | BioLang | Python | R | BL vs Py | BL vs R |"
    echo "|---|---|---|---|---|---|"

    IFS='|' read -ra task_ids <<< "${SUMMARY_DATA["${cat_id}|_tasks"]}"
    for tid in "${task_ids[@]}"; do
      tname="${SUMMARY_DATA["${cat_id}|${tid}|_name"]}"
      bl_t="${SUMMARY_DATA["${cat_id}|${tid}|bl_time"]:-"-"}"
      py_t="${SUMMARY_DATA["${cat_id}|${tid}|py_time"]:-"-"}"
      r_t="${SUMMARY_DATA["${cat_id}|${tid}|r_time"]:-"-"}"
      vs_py="-"; vs_r="-"
      if [[ "$bl_t" != "-" && "$py_t" != "-" ]]; then
        vs_py=$(echo "scale=1; $py_t / $bl_t" | bc -l 2>/dev/null || echo "-")
        vs_py="${vs_py}x"
      fi
      if [[ "$bl_t" != "-" && "$r_t" != "-" ]]; then
        vs_r=$(echo "scale=1; $r_t / $bl_t" | bc -l 2>/dev/null || echo "-")
        vs_r="${vs_r}x"
      fi
      echo "| $tname | $bl_t | $py_t | $r_t | $vs_py | $vs_r |"
    done

    echo ""
    echo "## Output Comparison"
    echo ""
    for tid in "${task_ids[@]}"; do
      tname="${SUMMARY_DATA["${cat_id}|${tid}|_name"]}"
      echo "### $tname"
      echo ""
      for lang_key in bl py r; do
        out="${SUMMARY_DATA["${cat_id}|${tid}|${lang_key}_output"]:-}"
        if [[ -n "$out" ]]; then
          label="BioLang"
          [[ "$lang_key" == "py" ]] && label="Python"
          [[ "$lang_key" == "r" ]] && label="R"
          printf '**%s**:\n```\n%s\n```\n\n' "$label" "$out"
        fi
      done
    done
  } > "$report_file"
done

# ── Generate summary report ──
SUMMARY_FILE="results/summary_${PLATFORM_TAG}_${TIMESTAMP}.md"
{
  cat << HEADER
# BioLang Benchmark Summary

## Platform

| Field | Value |
|---|---|
| Date | ${PLAT[date]} |
| OS | ${PLAT[os]} |
| CPU | ${PLAT[cpu]} |
| Cores | ${PLAT[cpu_cores]} |
| RAM | ${PLAT[ram]} |
| BioLang | ${PLAT[biolang]} |
| Python | ${PLAT[python]} |
| R | ${PLAT[r]} |
| Rust | ${PLAT[rustc]} |
| Runs | $RUNS (best of) |

HEADER

  for cat_id in "${SUMMARY_CATS[@]}"; do
    cat_name="${SUMMARY_DATA["${cat_id}|_name"]}"
    echo "## $cat_name"
    echo ""
    echo "| Task | BioLang | Python | R | BL vs Py | BL vs R | LOC BL/Py/R |"
    echo "|---|---|---|---|---|---|---|"

    IFS='|' read -ra task_ids <<< "${SUMMARY_DATA["${cat_id}|_tasks"]}"
    for tid in "${task_ids[@]}"; do
      tname="${SUMMARY_DATA["${cat_id}|${tid}|_name"]}"
      bl_t="${SUMMARY_DATA["${cat_id}|${tid}|bl_time"]:-"-"}"
      py_t="${SUMMARY_DATA["${cat_id}|${tid}|py_time"]:-"-"}"
      r_t="${SUMMARY_DATA["${cat_id}|${tid}|r_time"]:-"-"}"
      bl_l="${SUMMARY_DATA["${cat_id}|${tid}|bl_loc"]:-"-"}"
      py_l="${SUMMARY_DATA["${cat_id}|${tid}|py_loc"]:-"-"}"
      r_l="${SUMMARY_DATA["${cat_id}|${tid}|r_loc"]:-"-"}"
      vs_py="-"; vs_r="-"
      if [[ "$bl_t" != "-" && "$py_t" != "-" ]]; then
        vs_py=$(echo "scale=1; $py_t / $bl_t" | bc -l 2>/dev/null || echo "-")
        vs_py="${vs_py}x"
      fi
      if [[ "$bl_t" != "-" && "$r_t" != "-" ]]; then
        vs_r=$(echo "scale=1; $r_t / $bl_t" | bc -l 2>/dev/null || echo "-")
        vs_r="${vs_r}x"
      fi
      [[ "$bl_t" != "-" ]] && bl_t="${bl_t}s"
      [[ "$py_t" != "-" ]] && py_t="${py_t}s"
      [[ "$r_t" != "-" ]] && r_t="${r_t}s"
      echo "| $tname | $bl_t | $py_t | $r_t | $vs_py | $vs_r | $bl_l / $py_l / $r_l |"
    done
    echo ""
  done

  cat << 'METHODOLOGY'
## Methodology

- **Time**: Wall-clock via `time` builtin, best of 3 consecutive runs
- **LOC**: Non-blank, non-comment lines (idiomatic code per language)
- **Data**: Synthetic is reproducible (seed=42); real-world from public databases
- **Conditions**: All tasks ran sequentially, no other heavy processes
- **K-mer note**: BioLang counts *canonical* (strand-agnostic) 21-mers -- strictly more work than Python's forward-only approach
METHODOLOGY

  echo ""
  echo "---"
  echo "*Generated by BioLang Benchmark Suite on ${PLAT[date]}*"
} > "$SUMMARY_FILE"

# ── Generate scores YAML (single source of truth) ──
SCORES_FILE="results/scores_${PLATFORM_TAG}_${TIMESTAMP}.yaml"
{
  cat << YAMLHEADER
# BioLang Benchmark Scores — Single source of truth
# Generated by run_all.sh on ${PLAT[date]}
# Copy to website/README/book from here

platform:
  os: "${PLAT[os]}"
  cpu: "${PLAT[cpu]}"
  ram: "${PLAT[ram]}"
  biolang: "${PLAT[biolang]}"
  python: "${PLAT[python]}"
  r: "${PLAT[r]}"
  date: "$(date +%Y-%m-%d)"

categories:
YAMLHEADER

  for cat_id in "${SUMMARY_CATS[@]}"; do
    cat_name="${SUMMARY_DATA["${cat_id}|_name"]}"
    group="${SUMMARY_DATA["${cat_id}|_group"]}"

    echo "  ${cat_id}:"
    echo "    name: \"${cat_name}\""
    echo "    group: ${group}"
    echo "    tasks:"

    IFS='|' read -ra task_ids <<< "${SUMMARY_DATA["${cat_id}|_tasks"]}"
    for tid in "${task_ids[@]}"; do
      tname="${SUMMARY_DATA["${cat_id}|${tid}|_name"]}"
      tsuite="${SUMMARY_DATA["${cat_id}|${tid}|_suite"]:-synthetic}"
      bl_t="${SUMMARY_DATA["${cat_id}|${tid}|bl_time"]:-null}"
      bl_l="${SUMMARY_DATA["${cat_id}|${tid}|bl_loc"]:-null}"
      py_t="${SUMMARY_DATA["${cat_id}|${tid}|py_time"]:-null}"
      py_l="${SUMMARY_DATA["${cat_id}|${tid}|py_loc"]:-null}"

      echo "      - id: ${tid}"
      echo "        name: \"${tname}\""
      echo "        data: ${tsuite}"
      echo "        bl: { time: ${bl_t}, loc: ${bl_l} }"
      echo "        py: { time: ${py_t}, loc: ${py_l} }"

      if [[ "$group" == "language" ]]; then
        r_t="${SUMMARY_DATA["${cat_id}|${tid}|r_time"]:-null}"
        r_l="${SUMMARY_DATA["${cat_id}|${tid}|r_loc"]:-null}"
        echo "        r:  { time: ${r_t}, loc: ${r_l} }"
      fi
    done
    echo ""
  done

  echo "# null = benchmark did not run (script missing, tool not found, or data unavailable)"
} > "$SCORES_FILE"

# Cleanup temp files
rm -f "$_BT_OUTPUT_FILE" "$_BT_VALID_FILE"

echo ""
echo "================================================================"
echo "Summary:  $SUMMARY_FILE"
echo "Scores:   $SCORES_FILE"
echo "================================================================"
