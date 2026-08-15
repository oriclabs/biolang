param(
    [string]$BioLangExe = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$bookRoot = Join-Path $repoRoot "books\biostatistics\book\src"
if (-not $BioLangExe) {
    $BioLangExe = Join-Path $repoRoot "target\debug\bl.exe"
}
if (-not (Test-Path -LiteralPath $BioLangExe -PathType Leaf)) {
    throw "BioLang executable not found: $BioLangExe (run cargo build -p bl-cli)"
}

$chapters = @(Get-ChildItem -LiteralPath $bookRoot -Filter "day-*.md" | Sort-Object Name)
if ($chapters.Count -ne 30) {
    throw "Expected 30 day chapters, found $($chapters.Count)"
}

$scratch = Join-Path ([System.IO.Path]::GetTempPath()) ("biolang-book-audit-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $scratch | Out-Null
$failures = @()
$warnings = @()
$blocksChecked = 0

# Claims on this list previously taught invalid pass/fail rules. Keep the list
# narrow: words such as "always" are legitimate in mathematical identities and
# should be reviewed by a person, not blindly rejected by a linter.
$forbiddenClaims = @(
    "p > 0.05, normality assumption is reasonable",
    "flat prior is always safe",
    "make no assumptions about the shape of your data distribution",
    "Microbiome data, cytokine levels, survival times, and ordinal scales almost always require non-parametric",
    "Normality violated -> use Kruskal-Wallis"
)

try {
    foreach ($chapter in $chapters) {
        $markdown = Get-Content -LiteralPath $chapter.FullName -Raw -Encoding UTF8
        foreach ($claim in $forbiddenClaims) {
            if ($markdown.Contains($claim)) {
                $failures += "$($chapter.Name): obsolete absolute claim: $claim"
            }
        }

        $matches = [regex]::Matches(
            $markdown,
            '(?ms)^```(?:bio|biolang)[^\r\n]*\r?\n(.*?)^```\s*$'
        )
        for ($index = 0; $index -lt $matches.Count; $index++) {
            $blocksChecked++
            $code = $matches[$index].Groups[1].Value
            $tempFile = Join-Path $scratch ("{0}-{1:D3}.bl" -f $chapter.BaseName, ($index + 1))
            Set-Content -LiteralPath $tempFile -Value $code -Encoding UTF8
            $output = & $BioLangExe check --no-gpu $tempFile 2>&1
            if ($LASTEXITCODE -ne 0) {
                $failures += "$($chapter.Name) BioLang block $($index + 1): $($output -join ' ')"
            }
            if ($code -match '(?im)^\s*#\s*(output|result)\s*:.*\d') {
                $warnings += "$($chapter.Name) block $($index + 1) contains a pasted numeric output/result; verify it is measured."
            }
        }
    }
}
finally {
    if (Test-Path -LiteralPath $scratch) {
        Remove-Item -LiteralPath $scratch -Recurse -Force
    }
}

Write-Host "Biostatistics book audit"
Write-Host "  chapters:       $($chapters.Count)"
Write-Host "  BioLang blocks: $blocksChecked"
Write-Host "  warnings:       $($warnings.Count)"
foreach ($warning in $warnings) { Write-Warning $warning }

if ($failures.Count -gt 0) {
    Write-Error ("Audit failed:`n- " + ($failures -join "`n- "))
    exit 1
}

Write-Host "  result:         PASS"
