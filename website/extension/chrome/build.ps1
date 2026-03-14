# BLViewer Chrome Extension - Build and Package Script
# Usage:
#   .\build.ps1          Build only (sync files, load unpacked)
#   .\build.ps1 -Zip     Build + create .zip for Chrome Web Store upload

param([switch]$Zip)

$ScriptDir = $PSScriptRoot
$WebsiteDir = (Resolve-Path "$ScriptDir\..\..").Path
$Version = (Get-Content "$ScriptDir\manifest.json" | ConvertFrom-Json).version

Write-Host ""
Write-Host "  BLViewer Chrome Extension v$Version" -ForegroundColor Cyan
Write-Host "  =================================" -ForegroundColor DarkGray
Write-Host ""

# -- Step 1: Sync viewer.js from website
Write-Host "[1/4] Syncing viewer.js..." -ForegroundColor Yellow
Copy-Item "$WebsiteDir\js\viewer.js" "$ScriptDir\viewer.js" -Force
Write-Host "  OK  viewer.js" -ForegroundColor Green

# -- Step 2: Sync WASM files
Write-Host "[2/4] Syncing WASM files..." -ForegroundColor Yellow
if (Test-Path "$WebsiteDir\wasm") {
    New-Item -ItemType Directory -Path "$ScriptDir\wasm" -Force | Out-Null
    Copy-Item "$WebsiteDir\wasm\br_wasm.js" "$ScriptDir\wasm\" -Force -ErrorAction SilentlyContinue
    Copy-Item "$WebsiteDir\wasm\br_wasm_bg.wasm" "$ScriptDir\wasm\" -Force -ErrorAction SilentlyContinue
    Copy-Item "$WebsiteDir\wasm\br_wasm_bg.wasm.d.ts" "$ScriptDir\wasm\" -Force -ErrorAction SilentlyContinue
    Copy-Item "$WebsiteDir\wasm\br_wasm.d.ts" "$ScriptDir\wasm\" -Force -ErrorAction SilentlyContinue
    Write-Host "  OK  wasm/" -ForegroundColor Green
} else {
    Write-Host "  SKIP  wasm/ not found" -ForegroundColor DarkYellow
}

# -- Step 3: Sync screenshots
Write-Host "[3/4] Syncing screenshots..." -ForegroundColor Yellow
$ScreenshotSrc = "$WebsiteDir\docs\tools\screenshots"
if (Test-Path $ScreenshotSrc) {
    New-Item -ItemType Directory -Path "$ScriptDir\screenshots" -Force | Out-Null
    Copy-Item "$ScreenshotSrc\*.png" "$ScriptDir\screenshots\" -Force -ErrorAction SilentlyContinue
    $count = (Get-ChildItem "$ScriptDir\screenshots\*.png" | Measure-Object).Count
    Write-Host ("  OK  screenshots/ ({0} images)" -f $count) -ForegroundColor Green
} else {
    Write-Host "  SKIP  screenshots/ (not found)" -ForegroundColor DarkYellow
}

# -- Step 4: Validate
Write-Host "[4/4] Validating..." -ForegroundColor Yellow

$required = @(
    "manifest.json", "background.js", "popup.html", "popup.js",
    "viewer.html", "viewer.js", "help.html",
    "theme-init.js", "ext-loader.js", "wasm-loader.js",
    "icons\icon16.png", "icons\icon48.png", "icons\icon128.png"
)
$missing = @()
foreach ($f in $required) {
    if (-not (Test-Path "$ScriptDir\$f")) {
        $missing += $f
    }
}
if ($missing.Count -gt 0) {
    Write-Host "  WARN  Missing files:" -ForegroundColor Red
    foreach ($f in $missing) { Write-Host "        - $f" -ForegroundColor Red }
} else {
    Write-Host "  OK  All required files present" -ForegroundColor Green
}

# Check for inline scripts (CSP violation)
$inlineScripts = Select-String -Path "$ScriptDir\*.html" -Pattern '<script>[^<]' -SimpleMatch:$false 2>$null
if ($inlineScripts) {
    Write-Host "  WARN  Inline scripts found (CSP will block these):" -ForegroundColor Red
    foreach ($match in $inlineScripts) {
        Write-Host "        $($match.Filename):$($match.LineNumber)" -ForegroundColor Red
    }
} else {
    Write-Host "  OK  No inline scripts (CSP clean)" -ForegroundColor Green
}

Write-Host ""

# -- Zip for Chrome Web Store
if ($Zip) {
    $zipName = "blviewer-chrome-v$Version.zip"
    $zipPath = "$ScriptDir\$zipName"

    # Remove old zip
    if (Test-Path $zipPath) { Remove-Item $zipPath -Force }

    Write-Host "Packaging $zipName..." -ForegroundColor Cyan

    # Files to include
    $include = @(
        "manifest.json",
        "background.js",
        "popup.html", "popup.js",
        "viewer.html", "viewer.js",
        "help.html",
        "theme-init.js", "ext-loader.js", "wasm-loader.js"
    )

    # Create temp staging directory
    $staging = "$env:TEMP\blviewer-staging"
    if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }
    New-Item -ItemType Directory -Path $staging -Force | Out-Null

    # Copy individual files
    foreach ($f in $include) {
        Copy-Item "$ScriptDir\$f" "$staging\$f" -Force
    }

    # Copy directories
    if (Test-Path "$ScriptDir\icons") {
        New-Item -ItemType Directory -Path "$staging\icons" -Force | Out-Null
        Copy-Item "$ScriptDir\icons\*.png" "$staging\icons\" -Force
    }
    if (Test-Path "$ScriptDir\wasm") {
        New-Item -ItemType Directory -Path "$staging\wasm" -Force | Out-Null
        Copy-Item "$ScriptDir\wasm\*" "$staging\wasm\" -Force
    }
    if (Test-Path "$ScriptDir\screenshots") {
        New-Item -ItemType Directory -Path "$staging\screenshots" -Force | Out-Null
        Copy-Item "$ScriptDir\screenshots\*.png" "$staging\screenshots\" -Force
    }

    # Create zip
    Compress-Archive -Path "$staging\*" -DestinationPath $zipPath -Force

    # Cleanup staging
    Remove-Item $staging -Recurse -Force

    $size = [math]::Round((Get-Item $zipPath).Length / 1MB, 2)
    Write-Host ""
    Write-Host ("  Created: {0} ({1} MB)" -f $zipName, $size) -ForegroundColor Green
    Write-Host "  Path:    $zipPath" -ForegroundColor DarkGray
    Write-Host ""
    Write-Host "  Upload to Chrome Web Store:" -ForegroundColor White
    Write-Host "  https://chrome.google.com/webstore/devconsole" -ForegroundColor Cyan
    Write-Host ""
} else {
    Write-Host "Build complete! Load unpacked in Chrome:" -ForegroundColor Green
    Write-Host "  1. Open chrome://extensions" -ForegroundColor White
    Write-Host "  2. Enable 'Developer mode'" -ForegroundColor White
    Write-Host "  3. Click 'Load unpacked'" -ForegroundColor White
    Write-Host "  4. Select: $ScriptDir" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "To create a .zip for Chrome Web Store:" -ForegroundColor DarkGray
    Write-Host "  .\build.ps1 -Zip" -ForegroundColor DarkGray
    Write-Host ""
}
