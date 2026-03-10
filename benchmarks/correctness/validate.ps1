# Correctness validation: run BioLang and Python on the same tasks, compare JSON outputs.
# Usage: .\validate.ps1 [-BL bl] [-PY python]
param(
    [string]$BL = "bl",
    [string]$PY = "python"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$DataDir = Split-Path -Parent $ScriptDir

$Tasks = @("gc_content", "kmer_count", "vcf_filter", "reverse_complement", "translate", "csv_groupby")
$Pass = 0
$Fail = 0
$Skip = 0

$CompareScript = @"
import json, sys
def compare(a, b, path='', tol=1e-6):
    if type(a) != type(b):
        return [f'{path}: type mismatch {type(a).__name__} vs {type(b).__name__}']
    if isinstance(a, dict):
        errs = []
        for k in set(list(a.keys()) + list(b.keys())):
            if k not in a: errs.append(f'{path}.{k}: missing in BioLang')
            elif k not in b: errs.append(f'{path}.{k}: missing in Python')
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

with open(sys.argv[1]) as f: bl_data = json.load(f)
with open(sys.argv[2]) as f: py_data = json.load(f)
errs = compare(bl_data, py_data)
if errs:
    for e in errs[:10]: print(f'  DIFF: {e}')
    sys.exit(1)
"@

Write-Host "=== BioLang Correctness Validation ==="
Write-Host ""

Set-Location $DataDir

foreach ($task in $Tasks) {
    $label = $task.PadRight(25)
    Write-Host -NoNewline "  $label "

    $blScript = Join-Path $ScriptDir "biolang\$task.bl"
    $pyScript = Join-Path $ScriptDir "python\$task.py"
    $blOut = [System.IO.Path]::GetTempFileName()
    $pyOut = [System.IO.Path]::GetTempFileName()

    # Run Python
    try {
        & $PY $pyScript > $pyOut 2>$null
    } catch {
        Write-Host "SKIP (Python failed)" -ForegroundColor Yellow
        $Skip++
        Remove-Item -Force $blOut, $pyOut -ErrorAction SilentlyContinue
        continue
    }

    # Run BioLang
    try {
        & $BL run $blScript > $blOut 2>$null
    } catch {
        Write-Host "FAIL (BioLang crashed)" -ForegroundColor Red
        $Fail++
        Remove-Item -Force $blOut, $pyOut -ErrorAction SilentlyContinue
        continue
    }

    # Compare
    $compareFile = [System.IO.Path]::GetTempFileName() + ".py"
    Set-Content -Path $compareFile -Value $CompareScript
    $result = & $PY $compareFile $blOut $pyOut 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "PASS" -ForegroundColor Green
        $Pass++
    } else {
        Write-Host "FAIL" -ForegroundColor Red
        $result | ForEach-Object { Write-Host $_ }
        $Fail++
    }

    Remove-Item -Force $blOut, $pyOut, $compareFile -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Results: $Pass passed, $Fail failed, $Skip skipped out of $($Tasks.Count) tasks"
if ($Fail -gt 0) { exit 1 }
