# BioLang installer for Windows — https://lang.bio
#
# Usage: iwr -useb https://lang.bio/install.ps1 | iex
#
# The shell installer (install.sh) covers Linux and macOS. Windows had no
# equivalent, so the documented Windows path was a manual download, unzip and
# PATH edit while every other platform got one line.
#
# Configuration, read from the environment so it survives being piped to iex
# (a param() block does not):
#   BIOLANG_INSTALL_DIR   where to put the binaries
#                         default: %LOCALAPPDATA%\Programs\BioLang\bin
#   BIOLANG_NO_MODIFY_PATH  set to 1 to skip adding that directory to PATH

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"   # Invoke-WebRequest is far slower with the progress bar

$Repo = "oriclabs/biolang"
$InstallDir = if ($env:BIOLANG_INSTALL_DIR) { $env:BIOLANG_INSTALL_DIR }
              else { Join-Path $env:LOCALAPPDATA "Programs\BioLang\bin" }

function Say  { param($m) Write-Host $m }
function Fail { param($m) Write-Host "error: $m" -ForegroundColor Red; exit 1 }

# ── Architecture ──
# Releases currently carry windows-x86_64 only. On arm64 Windows the x86_64
# build does run under emulation, but saying so is better than pretending the
# download is native.
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -eq "ARM64") {
    Say "note: no native arm64 Windows build yet; installing the x86_64 build, which runs under emulation."
} elseif ($arch -ne "AMD64") {
    Fail "Unsupported architecture: $arch. BioLang ships x86_64 builds for Windows."
}
$archive = "biolang-windows-x86_64.zip"

# ── Latest release ──
Say "Detecting latest BioLang release..."
try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" `
                                 -Headers @{ "User-Agent" = "biolang-installer" }
} catch {
    Fail "Could not reach the GitHub API. Check https://github.com/$Repo/releases"
}
$tag = $release.tag_name
if (-not $tag) { Fail "Could not determine the latest release. Check https://github.com/$Repo/releases" }
Say "Latest release: $tag"

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("biolang-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
try {
    $zip = Join-Path $tmp $archive
    $url = "https://github.com/$Repo/releases/download/$tag/$archive"
    Say "Downloading $archive ..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing `
                          -Headers @{ "User-Agent" = "biolang-installer" }
    } catch {
        Fail "Download failed. URL: $url"
    }

    # ── Checksum ──
    # Every release publishes checksums.sha256. This script is piped straight
    # from the internet into a shell, so it is worth confirming the archive is
    # the one that was published rather than trusting the transfer.
    $sumsUrl = "https://github.com/$Repo/releases/download/$tag/checksums.sha256"
    try {
        $resp = Invoke-WebRequest -Uri $sumsUrl -UseBasicParsing `
                    -Headers @{ "User-Agent" = "biolang-installer" }
        # GitHub serves this as application/octet-stream, and Windows PowerShell
        # hands back a Byte[] rather than a string for non-text content types.
        # Splitting a Byte[] on a newline silently matches nothing, which looked
        # exactly like "the archive is not listed" when it was.
        $sums = if ($resp.Content -is [byte[]]) { [Text.Encoding]::UTF8.GetString($resp.Content) }
                else { [string]$resp.Content }
    } catch {
        $sums = $null
        Say "note: checksums.sha256 not published for $tag; skipping verification."
    }
    if ($sums) {
        $line = ($sums -split "`n" | Where-Object { $_ -match [regex]::Escape($archive) } | Select-Object -First 1)
        if (-not $line) {
            Say "note: $archive is absent from checksums.sha256; skipping verification."
        } else {
            $expected = ($line.Trim() -split '\s+')[0].ToLower()
            $actual = (Get-FileHash -Path $zip -Algorithm SHA256).Hash.ToLower()
            if ($expected -ne $actual) {
                Fail "checksum mismatch for $archive`n  expected $expected`n  actual   $actual"
            }
            Say "Checksum verified."
        }
    }

    Say "Extracting..."
    Expand-Archive -Path $zip -DestinationPath $tmp -Force

    if (-not (Test-Path $InstallDir)) { New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null }
    foreach ($exe in @("bl.exe", "bl-lsp.exe")) {
        $src = Join-Path $tmp $exe
        if (Test-Path $src) {
            # A running bl.exe cannot be overwritten. Renaming it out of the way
            # works while the file is locked, which is how an in-place upgrade
            # succeeds when the old binary is still open somewhere.
            $dest = Join-Path $InstallDir $exe
            if (Test-Path $dest) {
                $old = "$dest.old"
                if (Test-Path $old) { Remove-Item $old -Force -ErrorAction SilentlyContinue }
                try { Rename-Item -Path $dest -NewName ($exe + ".old") -Force } catch {}
            }
            Copy-Item -Path $src -Destination $dest -Force
        }
    }
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

# ── PATH ──
if ($env:BIOLANG_NO_MODIFY_PATH -ne "1") {
    $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    $parts = @()
    if ($userPath) { $parts = $userPath -split ';' | Where-Object { $_ } }
    if ($parts -notcontains $InstallDir) {
        [Environment]::SetEnvironmentVariable("PATH", (($parts + $InstallDir) -join ';'), "User")
        Say "Added $InstallDir to your user PATH (open a new terminal to pick it up)."
    }
    # Make it usable in this session too, without waiting for a new terminal.
    if (($env:PATH -split ';') -notcontains $InstallDir) { $env:PATH = "$InstallDir;$env:PATH" }
}

$bl = Join-Path $InstallDir "bl.exe"
if (-not (Test-Path $bl)) { Fail "bl.exe was not found in $InstallDir after extraction." }

Say ""
Say "BioLang installed successfully!"
Say ("  bl:     " + (& $bl --version))
if (Test-Path (Join-Path $InstallDir "bl-lsp.exe")) { Say "  bl-lsp: installed" }
Say "  path:   $InstallDir"
Say ""
Say "Get started:"
Say "  bl repl          # interactive REPL"
Say "  bl run script.bl # run a script"
Say "  bl --help        # all commands"
Say ""
Say "Documentation: https://lang.bio"
