# Real-world correctness validation: BioLang vs Python vs R on real biological data.
# Requires: python download_real_data.py first
# Usage: .\validate_real.ps1 [-BL bl] [-PY python] [-RS Rscript]
param(
    [string]$BL = "bl",
    [string]$PY = "python",
    [string]$RS = "Rscript"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

$Tasks = @("gc_content", "kmer_count", "vcf_filter", "reverse_complement", "translate", "csv_groupby", "gff_features", "sequence_stats", "bed_intervals")

# Check real_data exists
if (-not (Test-Path (Join-Path $ScriptDir "real_data\ecoli_genome.fa"))) {
    Write-Host "Real-world data not found. Run first:"
    Write-Host "  python $(Join-Path $ScriptDir 'download_real_data.py')"
    exit 1
}

$CompareScript = @"
import json, sys
def compare(a, b, path='', tol=1e-6):
    if type(a) != type(b):
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
"@

$HasR = $false
try { & $RS --version 2>$null; $HasR = $true } catch {}

Write-Host "=== BioLang Real-World Correctness Validation ==="
Write-Host "Data: E. coli K-12, S. cerevisiae, ClinVar, ENCODE"
Write-Host ""

Set-Location $ScriptDir

function Run-Comparison($label, $refCmd, $refScript, $blScript, $task) {
    $blOut = [System.IO.Path]::GetTempFileName()
    $refOut = [System.IO.Path]::GetTempFileName()
    $cmpFile = [System.IO.Path]::GetTempFileName() + ".py"
    Set-Content -Path $cmpFile -Value $CompareScript

    try {
        & $refCmd $refScript > $refOut 2>$null
        if ($LASTEXITCODE -ne 0) { throw "failed" }
    } catch {
        Write-Host "SKIP ($label failed)" -ForegroundColor Yellow
        Remove-Item -Force $blOut, $refOut, $cmpFile -ErrorAction SilentlyContinue
        return "skip"
    }

    try {
        & $BL run $blScript > $blOut 2>$null
        if ($LASTEXITCODE -ne 0) { throw "failed" }
    } catch {
        Write-Host "FAIL (BioLang crashed)" -ForegroundColor Red
        Remove-Item -Force $blOut, $refOut, $cmpFile -ErrorAction SilentlyContinue
        return "fail"
    }

    & $PY $cmpFile $blOut $refOut 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "PASS" -ForegroundColor Green
        Remove-Item -Force $blOut, $refOut, $cmpFile -ErrorAction SilentlyContinue
        return "pass"
    } else {
        Write-Host "FAIL" -ForegroundColor Red
        & $PY $cmpFile $blOut $refOut 2>$null
        Remove-Item -Force $blOut, $refOut, $cmpFile -ErrorAction SilentlyContinue
        return "fail"
    }
}

# BioLang vs Python
Write-Host "--- BioLang vs Python (BioPython) ---"
$PyPass = 0; $PyFail = 0; $PySkip = 0
foreach ($task in $Tasks) {
    $padded = $task.PadRight(25)
    Write-Host -NoNewline "  $padded "
    $blScript = Join-Path $ScriptDir "real_world\biolang\$task.bl"
    $pyScript = Join-Path $ScriptDir "real_world\python\$task.py"
    $result = Run-Comparison "Python" $PY $pyScript $blScript $task
    switch ($result) { "pass" { $PyPass++ } "fail" { $PyFail++ } "skip" { $PySkip++ } }
}

# BioLang vs R
$RPass = 0; $RFail = 0; $RSkip = 0
if ($HasR) {
    Write-Host ""
    Write-Host "--- BioLang vs R (Bioconductor) ---"
    foreach ($task in $Tasks) {
        $rScript = Join-Path $ScriptDir "real_world\r\$task.R"
        if (-not (Test-Path $rScript)) { continue }
        $padded = $task.PadRight(25)
        Write-Host -NoNewline "  $padded "
        $blScript = Join-Path $ScriptDir "real_world\biolang\$task.bl"
        $result = Run-Comparison "R" $RS $rScript $blScript $task
        switch ($result) { "pass" { $RPass++ } "fail" { $RFail++ } "skip" { $RSkip++ } }
    }
}

Write-Host ""
Write-Host "=== Summary ==="
Write-Host "  vs Python: $PyPass passed, $PyFail failed, $PySkip skipped"
if ($HasR) {
    Write-Host "  vs R:      $RPass passed, $RFail failed, $RSkip skipped"
}

$totalFail = $PyFail + $RFail
if ($totalFail -gt 0) { exit 1 }
