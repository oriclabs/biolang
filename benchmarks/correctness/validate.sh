#!/usr/bin/env bash
# Correctness validation: run BioLang, Python, and R on the same tasks, compare JSON outputs.
# Usage: ./validate.sh [bl_binary] [python_binary] [rscript_binary]
set -euo pipefail

BL="${1:-bl}"
PY="${2:-python}"
RS="${3:-Rscript}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DATA_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

TASKS=(gc_content kmer_count vcf_filter reverse_complement translate csv_groupby gff_features sequence_stats bed_intervals)
PASS=0
FAIL=0
SKIP=0

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

compare_json() {
    # Compare two JSON files with float tolerance
    python3 -c "
import json, sys

def compare(a, b, path='', tol=1e-6):
    if type(a) != type(b):
        # Allow int vs float if values match
        if isinstance(a, (int, float)) and isinstance(b, (int, float)):
            if abs(float(a) - float(b)) > tol:
                return [f'{path}: {a} vs {b} (diff={abs(float(a)-float(b)):.2e})']
            return []
        return [f'{path}: type mismatch {type(a).__name__} vs {type(b).__name__}']
    if isinstance(a, dict):
        errs = []
        for k in set(list(a.keys()) + list(b.keys())):
            if k not in a: errs.append(f'{path}.{k}: missing in first')
            elif k not in b: errs.append(f'{path}.{k}: missing in second')
            else: errs.extend(compare(a[k], b[k], f'{path}.{k}', tol))
        return errs
    if isinstance(a, list):
        if len(a) != len(b): return [f'{path}: length {len(a)} vs {len(b)}']
        errs = []
        for i in range(len(a)):
            errs.extend(compare(a[i], b[i], f'{path}[{i}]', tol))
        return errs
    if isinstance(a, float) or isinstance(b, float):
        fa, fb = float(a), float(b)
        if abs(fa - fb) > tol:
            return [f'{path}: {fa} vs {fb} (diff={abs(fa-fb):.2e})']
        return []
    if a != b:
        sa, sb = str(a)[:80], str(b)[:80]
        return [f'{path}: {sa} vs {sb}']
    return []

with open(sys.argv[1]) as f: a = json.load(f)
with open(sys.argv[2]) as f: b = json.load(f)
errs = compare(a, b)
if errs:
    for e in errs[:10]: print(f'  DIFF: {e}')
    sys.exit(1)
" "$1" "$2"
}

# Check R availability
HAS_R=false
if command -v "$RS" &>/dev/null; then
    HAS_R=true
fi

echo "=== BioLang Correctness Validation ==="
echo "BioLang: $($BL --version 2>/dev/null || echo "$BL")"
echo "Python:  $($PY --version 2>/dev/null || echo "$PY")"
if $HAS_R; then
    echo "R:       $($RS --version 2>/dev/null | head -1 || echo "$RS")"
else
    echo "R:       not found (R tests will be skipped)"
fi
echo ""

cd "$DATA_DIR"

echo "--- BioLang vs Python (BioPython) ---"
for task in "${TASKS[@]}"; do
    printf "  %-25s " "$task"

    bl_script="$SCRIPT_DIR/biolang/${task}.bl"
    py_script="$SCRIPT_DIR/python/${task}.py"
    bl_out=$(mktemp)
    py_out=$(mktemp)

    # Run Python (gold standard)
    if ! $PY "$py_script" > "$py_out" 2>/dev/null; then
        printf "${YELLOW}SKIP${NC} (Python failed)\n"
        SKIP=$((SKIP + 1))
        rm -f "$bl_out" "$py_out"
        continue
    fi

    # Run BioLang
    if ! $BL run "$bl_script" > "$bl_out" 2>/dev/null; then
        printf "${RED}FAIL${NC} (BioLang crashed)\n"
        FAIL=$((FAIL + 1))
        rm -f "$bl_out" "$py_out"
        continue
    fi

    # Compare outputs
    if compare_json "$bl_out" "$py_out" 2>/dev/null; then
        printf "${GREEN}PASS${NC}\n"
        PASS=$((PASS + 1))
    else
        printf "${RED}FAIL${NC}\n"
        compare_json "$bl_out" "$py_out" 2>/dev/null || true
        FAIL=$((FAIL + 1))
    fi

    rm -f "$bl_out" "$py_out"
done

# R/Bioconductor validation
R_PASS=0
R_FAIL=0
R_SKIP=0
if $HAS_R; then
    echo ""
    echo "--- BioLang vs R (Bioconductor) ---"
    for task in "${TASKS[@]}"; do
        r_script="$SCRIPT_DIR/r/${task}.R"
        bl_script="$SCRIPT_DIR/biolang/${task}.bl"

        # Skip tasks without R implementation
        if [ ! -f "$r_script" ]; then
            continue
        fi

        printf "  %-25s " "$task"

        bl_out=$(mktemp)
        r_out=$(mktemp)

        # Run R
        if ! $RS "$r_script" > "$r_out" 2>/dev/null; then
            printf "${YELLOW}SKIP${NC} (R failed)\n"
            R_SKIP=$((R_SKIP + 1))
            rm -f "$bl_out" "$r_out"
            continue
        fi

        # Run BioLang
        if ! $BL run "$bl_script" > "$bl_out" 2>/dev/null; then
            printf "${RED}FAIL${NC} (BioLang crashed)\n"
            R_FAIL=$((R_FAIL + 1))
            rm -f "$bl_out" "$r_out"
            continue
        fi

        # Compare
        if compare_json "$bl_out" "$r_out" 2>/dev/null; then
            printf "${GREEN}PASS${NC}\n"
            R_PASS=$((R_PASS + 1))
        else
            printf "${RED}FAIL${NC}\n"
            compare_json "$bl_out" "$r_out" 2>/dev/null || true
            R_FAIL=$((R_FAIL + 1))
        fi

        rm -f "$bl_out" "$r_out"
    done
fi

echo ""
echo "=== Summary ==="
echo "  vs Python: $PASS passed, $FAIL failed, $SKIP skipped"
if $HAS_R; then
    echo "  vs R:      $R_PASS passed, $R_FAIL failed, $R_SKIP skipped"
fi

TOTAL_FAIL=$((FAIL + R_FAIL))
[ "$TOTAL_FAIL" -eq 0 ] || exit 1
