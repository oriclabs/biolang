# Write a record of a correctness run.
#
# A run that leaves nothing behind cannot be cited afterwards, and "we compared
# against BioPython" is a much weaker claim than a table saying which tasks were
# compared, against which versions, on which data, and to what tolerance.
#
# Dot-sourced by validate.ps1 and validate_real.ps1.
#
#   Write-Evidence -Slug synthetic -Suite "..." -DataDesc "..." -Evidence $rows `
#                  -ScriptDir $ScriptDir -BL $BL -PY $PY -RS $RS -HasR $HasR `
#                  -PySummary "9 passed, 0 failed, 0 skipped" -RSummary "..."

function Write-Evidence {
    param(
        [string]$Slug, [string]$Suite, [string]$DataDesc,
        [object[]]$Evidence, [string]$ScriptDir,
        [string]$BL, [string]$PY, [string]$RS, [bool]$HasR,
        [string]$PySummary, [string]$RSummary
    )

    $resultsDir = Join-Path $ScriptDir "results"
    if (-not (Test-Path $resultsDir)) {
        New-Item -ItemType Directory -Path $resultsDir -Force | Out-Null
    }

    # Version strings are the point of the record: a result is only meaningful
    # against the implementations that produced it.
    $prev = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $blVer = ((& $BL --version) | Out-String).Trim()
        $pyVer = ((& $PY --version) | Out-String).Trim()
        $rVer = if ($HasR) { ((& $RS --version 2>&1) | Select-Object -First 1 | Out-String).Trim() }
                else { "not installed" }
    } finally {
        $ErrorActionPreference = $prev
    }

    $stamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $record = [pscustomobject]@{
        suite     = $Suite
        data      = $DataDesc
        generated = $stamp
        platform  = [System.Environment]::OSVersion.VersionString
        biolang   = $blVer
        python    = $pyVer
        r         = $rVer
        tolerance = "floats 1e-6; integers and strings exact"
        summary   = [pscustomobject]@{ vs_python = $PySummary; vs_r = $RSummary }
        results   = $Evidence
    }
    $record | ConvertTo-Json -Depth 6 |
        Set-Content -Path (Join-Path $resultsDir "$Slug.json") -Encoding UTF8

    $md = New-Object System.Collections.ArrayList
    [void]$md.Add("# $Suite")
    [void]$md.Add("")
    [void]$md.Add("Generated $stamp on $([System.Environment]::OSVersion.VersionString)")
    [void]$md.Add("")
    [void]$md.Add("| | |")
    [void]$md.Add("|---|---|")
    [void]$md.Add("| BioLang | $blVer |")
    [void]$md.Add("| Python | $pyVer |")
    [void]$md.Add("| R | $rVer |")
    [void]$md.Add("| Data | $DataDesc |")
    [void]$md.Add("| Tolerance | floats 1e-6; integers and strings exact |")
    [void]$md.Add("")
    [void]$md.Add("| Task | Reference | Result |")
    [void]$md.Add("|---|---|---|")
    foreach ($e in $Evidence) {
        [void]$md.Add("| $($e.task) | $($e.reference) | $($e.result.ToUpper()) |")
    }
    [void]$md.Add("")
    [void]$md.Add("vs Python: $PySummary")
    if ($HasR) { [void]$md.Add("") ; [void]$md.Add("vs R: $RSummary") }
    [void]$md.Add("")
    ($md -join "`n") | Set-Content -Path (Join-Path $resultsDir "$Slug.md") -Encoding UTF8

    Write-Host ""
    Write-Host "Evidence: $(Join-Path $resultsDir "$Slug.md")" -ForegroundColor Cyan
}
