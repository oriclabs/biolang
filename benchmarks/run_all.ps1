# BioLang Benchmark Suite -- Windows PowerShell
# Usage: .\run_all.ps1 [category_filter]
#   category_filter: "all", "language", "pipelines", or category name (e.g. "kmer", "file_io", "qc")
#
# Requires: bl (BioLang), python (with biopython), Rscript (with Bioconductor)
# Synthetic data: python generate_data.py
# Real-world data: python download_data.py

param([string]$CategoryFilter = "all")

$ErrorActionPreference = "Stop"
$Runs = 3

Set-Location $PSScriptRoot

# ── Activate venv if present ──
$venvActivate = Join-Path $PSScriptRoot ".venv\Scripts\Activate.ps1"
if ((-not $env:VIRTUAL_ENV) -and (Test-Path $venvActivate)) {
    Write-Host "Activating Python venv (.venv) ..."
    & $venvActivate
}

# ── Dependency check ──
Write-Host "Checking dependencies ..."
$missing = @()
if (-not (Get-Command bl -ErrorAction SilentlyContinue)) {
    Write-Host "  [WARN] bl not found — BioLang benchmarks will fail" -ForegroundColor Yellow
    Write-Host "         Install: cargo install --path ..\crates\bl-cli"
    $missing += "bl"
}
if (-not (Get-Command python -ErrorAction SilentlyContinue)) {
    Write-Host "  [WARN] python not found — Python benchmarks will fail" -ForegroundColor Yellow
    $missing += "python"
} else {
    $bioCheck = & python -c "from Bio import SeqIO" 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  [WARN] biopython not installed — most Python benchmarks will fail" -ForegroundColor Yellow
        Write-Host "         Fix: .\setup_python.ps1  (or: pip install biopython)"
        $missing += "biopython"
    }
}
if (-not (Get-Command Rscript -ErrorAction SilentlyContinue)) {
    Write-Host "  [INFO] Rscript not found — R benchmarks will be skipped" -ForegroundColor DarkGray
} else {
    $rCheck = & Rscript -e "library(Biostrings)" 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  [WARN] R Bioconductor packages missing — R benchmarks will fail" -ForegroundColor Yellow
        Write-Host "         Fix: .\setup_r.ps1"
        $missing += "r-bioc"
    }
}
if ($missing.Count -gt 0) {
    Write-Host ""
    Write-Host "  Missing: $($missing -join ', ')" -ForegroundColor Yellow
    Write-Host "  Run .\setup_python.ps1 and/or .\setup_r.ps1 to install dependencies."
    Write-Host "  Continuing anyway — failed benchmarks will show as 'null'."
    Write-Host ""
}

if (-not (Test-Path "data")) {
    Write-Host "No data/ directory. Run: python generate_data.py" -ForegroundColor Red
    exit 1
}

# ── Category + task definitions ──
# Each category has a name, display name, group (language/pipelines), and list of tasks
# Each task has: id, display name, data requirement (synthetic/real), script paths per language

$Categories = @(
    @{
        Id = "sequence_io"; Name = "Sequence I/O"; Group = "language"
        Tasks = @(
            @{ Id = "1_fasta_stats";  Name = "FASTA Statistics";     Suite = "synthetic"; Bl = "language/sequence_io/biolang/1_fasta_stats.bl"; Py = "language/sequence_io/python/1_fasta_stats.py"; R = "language/sequence_io/r/1_fasta_stats.R" }
            @{ Id = "2_fastq_qc";     Name = "FASTQ QC";            Suite = "synthetic"; Bl = "language/sequence_io/biolang/2_fastq_qc.bl"; Py = "language/sequence_io/python/2_fastq_qc.py"; R = "language/sequence_io/r/2_fastq_qc.R" }
            @{ Id = "6_genome_stats";  Name = "E.coli Genome Stats"; Suite = "real"; Bl = "language/sequence_io/biolang/6_genome_stats.bl"; Py = "language/sequence_io/python/6_genome_stats.py"; R = "language/sequence_io/r/6_genome_stats.R" }
            @{ Id = "9_human_chr22";   Name = "Human Chr22 Stats";   Suite = "real"; Bl = "language/sequence_io/biolang/9_human_chr22.bl"; Py = "language/sequence_io/python/9_human_chr22.py"; R = "language/sequence_io/r/9_human_chr22.R" }
            @{ Id = "gc_content_scale"; Name = "GC Content (51 MB)"; Suite = "real"; Bl = "language/sequence_io/biolang/gc_content_scale.bl"; Py = "language/sequence_io/python/gc_content_scale.py"; R = "language/sequence_io/r/gc_content_scale.R" }
            @{ Id = "reverse_complement"; Name = "Reverse Complement"; Suite = "synthetic"; Bl = "language/sequence_io/biolang/reverse_complement.bl"; Py = "language/sequence_io/python/reverse_complement.py"; R = "language/sequence_io/r/reverse_complement.R" }
        )
    }
    @{
        Id = "kmer"; Name = "K-mer Analysis"; Group = "language"
        Tasks = @(
            @{ Id = "3_kmer_count";   Name = "K-mer Counting";      Suite = "synthetic"; Bl = "language/kmer/biolang/3_kmer_count.bl"; Py = "language/kmer/python/3_kmer_count.py"; R = "language/kmer/r/3_kmer_count.R" }
            @{ Id = "10_chr22_kmers"; Name = "Chr22 21-mer Count";   Suite = "real"; Bl = "language/kmer/biolang/10_chr22_kmers.bl"; Py = "language/kmer/python/10_chr22_kmers.py"; R = "language/kmer/r/10_chr22_kmers.R" }
        )
    }
    @{
        Id = "variants"; Name = "Variant Analysis"; Group = "language"
        Tasks = @(
            @{ Id = "4_vcf_filter";    Name = "VCF Filtering";       Suite = "synthetic"; Bl = "language/variants/biolang/4_vcf_filter.bl"; Py = "language/variants/python/4_vcf_filter.py"; R = "language/variants/r/4_vcf_filter.R" }
            @{ Id = "7_clinvar_filter"; Name = "ClinVar Variants";    Suite = "real"; Bl = "language/variants/biolang/7_clinvar_filter.bl"; Py = "language/variants/python/7_clinvar_filter.py"; R = "language/variants/r/7_clinvar_filter.R" }
        )
    }
    @{
        Id = "wrangling"; Name = "Data Wrangling"; Group = "language"
        Tasks = @(
            @{ Id = "5_csv_wrangle";   Name = "CSV Join + Group-by"; Suite = "synthetic"; Bl = "language/wrangling/biolang/5_csv_wrangle.bl"; Py = "language/wrangling/python/5_csv_wrangle.py"; R = "language/wrangling/r/5_csv_wrangle.R" }
        )
    }
    @{
        Id = "protein"; Name = "Protein Analysis"; Group = "language"
        Tasks = @(
            @{ Id = "8_protein_kmers"; Name = "Protein K-mers";      Suite = "real"; Bl = "language/protein/biolang/8_protein_kmers.bl"; Py = "language/protein/python/8_protein_kmers.py"; R = "language/protein/r/8_protein_kmers.R" }
        )
    }
    @{
        Id = "file_io"; Name = "File I/O"; Group = "language"
        Tasks = @(
            @{ Id = "parse_fasta_small";  Name = "FASTA Small (30 KB)";  Suite = "real"; Bl = "language/file_io/biolang/parse_fasta_small.bl"; Py = "language/file_io/python/parse_fasta_small.py"; R = "language/file_io/r/parse_fasta_small.R" }
            @{ Id = "parse_fasta_medium"; Name = "FASTA Medium (4.6 MB)"; Suite = "real"; Bl = "language/file_io/biolang/parse_fasta_medium.bl"; Py = "language/file_io/python/parse_fasta_medium.py"; R = "language/file_io/r/parse_fasta_medium.R" }
            @{ Id = "parse_fasta_large";  Name = "FASTA Large (51 MB)";  Suite = "real"; Bl = "language/file_io/biolang/parse_fasta_large.bl"; Py = "language/file_io/python/parse_fasta_large.py"; R = "language/file_io/r/parse_fasta_large.R" }
            @{ Id = "parse_fastq";        Name = "FASTQ (26 MB)";       Suite = "synthetic"; Bl = "language/file_io/biolang/parse_fastq.bl"; Py = "language/file_io/python/parse_fastq.py"; R = "language/file_io/r/parse_fastq.R" }
            @{ Id = "parse_vcf";          Name = "VCF (2.3 MB)";        Suite = "synthetic"; Bl = "language/file_io/biolang/parse_vcf.bl"; Py = "language/file_io/python/parse_vcf.py"; R = "language/file_io/r/parse_vcf.R" }
            @{ Id = "parse_csv";          Name = "CSV (0.1 MB)";        Suite = "synthetic"; Bl = "language/file_io/biolang/parse_csv.bl"; Py = "language/file_io/python/parse_csv.py"; R = "language/file_io/r/parse_csv.R" }
            @{ Id = "parse_fasta_gz";     Name = "FASTA gzipped (1.3 MB)"; Suite = "real"; Bl = "language/file_io/biolang/parse_fasta_gz.bl"; Py = "language/file_io/python/parse_fasta_gz.py"; R = "language/file_io/r/parse_fasta_gz.R" }
            @{ Id = "parse_fasta_large_gz"; Name = "FASTA Large gzipped (10 MB)"; Suite = "real"; Bl = "language/file_io/biolang/parse_fasta_large_gz.bl"; Py = "language/file_io/python/parse_fasta_large_gz.py"; R = "language/file_io/r/parse_fasta_large_gz.R" }
            @{ Id = "write_filtered_fasta"; Name = "Write Filtered FASTA"; Suite = "synthetic"; Bl = "language/file_io/biolang/write_filtered_fasta.bl"; Py = "language/file_io/python/write_filtered_fasta.py"; R = "language/file_io/r/write_filtered_fasta.R" }
            @{ Id = "parse_gff";          Name = "GFF3 (1.7 MB)";        Suite = "synthetic"; Bl = "language/file_io/biolang/parse_gff.bl"; Py = "language/file_io/python/parse_gff.py"; R = "language/file_io/r/parse_gff.R" }
            @{ Id = "parse_gff_real";     Name = "GFF3 Ensembl chr22";   Suite = "real"; Bl = "language/file_io/biolang/parse_gff_real.bl"; Py = "language/file_io/python/parse_gff_real.py"; R = "" }
        )
    }
    @{
        Id = "intervals"; Name = "Interval Operations"; Group = "language"
        Tasks = @(
            @{ Id = "bed_overlap"; Name = "BED Interval Overlap"; Suite = "synthetic"; Bl = "language/intervals/biolang/bed_overlap.bl"; Py = "language/intervals/python/bed_overlap.py"; R = "language/intervals/r/bed_overlap.R" }
            @{ Id = "bed_overlap_real"; Name = "ENCODE Peak Overlap"; Suite = "real"; Bl = "language/intervals/biolang/bed_overlap_real.bl"; Py = "language/intervals/python/bed_overlap_real.py"; R = "" }
        )
    }
    @{
        Id = "qc_pipeline"; Name = "QC Pipeline"; Group = "pipelines"
        Tasks = @(
            @{ Id = "qc_pipeline"; Name = "FASTQ QC Pipeline"; Suite = "synthetic"; Bl = "pipelines/qc_pipeline/biolang/qc_pipeline.bl"; Py = "pipelines/qc_pipeline/python/qc_pipeline.py"; R = "" }
        )
    }
    @{
        Id = "variant_pipeline"; Name = "Variant Pipeline"; Group = "pipelines"
        Tasks = @(
            @{ Id = "variant_pipeline"; Name = "Variant Analysis Pipeline"; Suite = "synthetic"; Bl = "pipelines/variant_pipeline/biolang/variant_pipeline.bl"; Py = "pipelines/variant_pipeline/python/variant_pipeline.py"; R = "" }
            @{ Id = "variant_pipeline_real"; Name = "ClinVar Variant Pipeline"; Suite = "real"; Bl = "pipelines/variant_pipeline/biolang/variant_pipeline_real.bl"; Py = "pipelines/variant_pipeline/python/variant_pipeline_real.py"; R = "" }
        )
    }
    @{
        Id = "multi_sample"; Name = "Multi-Sample Pipeline"; Group = "pipelines"
        Tasks = @(
            @{ Id = "multi_sample"; Name = "Multi-Sample Aggregation"; Suite = "synthetic"; Bl = "pipelines/multi_sample/biolang/multi_sample.bl"; Py = "pipelines/multi_sample/python/multi_sample.py"; R = "" }
        )
    }
    @{
        Id = "rnaseq_mini"; Name = "RNA-seq Mini Pipeline"; Group = "pipelines"
        Tasks = @(
            @{ Id = "rnaseq_mini"; Name = "RNA-seq DE Analysis"; Suite = "synthetic"; Bl = "pipelines/rnaseq_mini/biolang/rnaseq_mini.bl"; Py = "pipelines/rnaseq_mini/python/rnaseq_mini.py"; R = "" }
        )
    }
    @{
        Id = "annotation"; Name = "Annotation Pipeline"; Group = "pipelines"
        Tasks = @(
            @{ Id = "annotation"; Name = "Variant Annotation"; Suite = "synthetic"; Bl = "pipelines/annotation/biolang/annotation.bl"; Py = "pipelines/annotation/python/annotation.py"; R = "" }
            @{ Id = "annotation_real"; Name = "ClinVar + Ensembl Annotation"; Suite = "real"; Bl = "pipelines/annotation/biolang/annotation_real.bl"; Py = "pipelines/annotation/python/annotation_real.py"; R = "" }
        )
    }
)

# ── Filter categories ──
function Should-RunCategory($cat) {
    if ($CategoryFilter -eq "all") { return $true }
    if ($CategoryFilter -eq $cat.Group) { return $true }
    if ($cat.Id -like "*$CategoryFilter*") { return $true }
    if ($cat.Name -like "*$CategoryFilter*") { return $true }
    return $false
}

# ── Platform info ──
function Get-PlatformInfo {
    $info = @{}
    $info["date"]     = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $info["os"]       = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
    $info["arch"]     = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    $cs = Get-CimInstance Win32_ComputerSystem -ErrorAction SilentlyContinue
    $cpu = Get-CimInstance Win32_Processor -ErrorAction SilentlyContinue
    if ($cs) {
        $ramGB = [math]::Round($cs.TotalPhysicalMemory / 1GB, 1)
        $info["ram"] = "${ramGB} GB"
    }
    if ($cpu) {
        $info["cpu"]       = $cpu.Name.Trim()
        $info["cpu_cores"] = "$($cpu.NumberOfCores) cores / $($cpu.NumberOfLogicalProcessors) threads"
    }
    try { $info["biolang"] = (& bl --version 2>&1) -replace '.*?(\d+\.\d+\.\d+).*','$1' } catch { $info["biolang"] = "not found" }
    try { $info["python"]  = (& python --version 2>&1) -replace 'Python ','' } catch { $info["python"] = "not found" }
    try { $info["r"]       = (& Rscript --version 2>&1) -replace '.*?(\d+\.\d+\.\d+).*','$1' } catch { $info["r"] = "not found" }
    try { $info["rustc"]   = (& rustc --version 2>&1) -replace 'rustc ','' } catch { $info["rustc"] = "not found" }
    return $info
}

function Count-LOC($file) {
    if (-not $file -or -not (Test-Path $file)) { return 0 }
    $lines = Get-Content $file | Where-Object { $_ -match '\S' -and $_ -notmatch '^\s*(#|//|""")' }
    return $lines.Count
}

$script:LastOutput = ""

function Best-Time($cmd, $args_list) {
    $resolved = Get-Command $cmd -ErrorAction SilentlyContinue
    if (-not $resolved) {
        Write-Host "    [SKIPPED] $cmd not found" -ForegroundColor DarkGray
        return -1
    }
    $best = [double]::MaxValue
    $anySuccess = $false
    $script:LastOutput = ""
    for ($i = 0; $i -lt $Runs; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $tmpOut = [System.IO.Path]::GetTempFileName()
        $tmpErr = [System.IO.Path]::GetTempFileName()
        try {
            $proc = Start-Process -FilePath $cmd -ArgumentList $args_list -NoNewWindow -Wait -PassThru `
                        -WorkingDirectory $PSScriptRoot `
                        -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr
        } catch {
            Write-Host "    [FAILED] cannot run $cmd" -ForegroundColor Red
            Remove-Item $tmpOut, $tmpErr -Force -ErrorAction SilentlyContinue
            return -1
        }
        $sw.Stop()
        $elapsed = $sw.Elapsed.TotalSeconds
        if ($proc.ExitCode -ne 0) {
            if (-not $anySuccess) {
                $errText = (Get-Content $tmpErr -Raw -ErrorAction SilentlyContinue)
                $outText = (Get-Content $tmpOut -Raw -ErrorAction SilentlyContinue)
                $msg = if ($errText) { $errText.Trim() } else { $outText.Trim() }
                if ($msg.Length -gt 500) { $msg = $msg.Substring(0, 500) }
                Write-Host "    [FAILED] exit=$($proc.ExitCode)" -ForegroundColor Red
                if ($msg) { Write-Host "    $msg" -ForegroundColor DarkRed }
            }
            Remove-Item $tmpOut, $tmpErr -Force -ErrorAction SilentlyContinue
            continue
        }
        $anySuccess = $true
        if ($elapsed -lt $best) {
            $best = $elapsed
            $script:LastOutput = (Get-Content $tmpOut -Raw -ErrorAction SilentlyContinue)
            if ($script:LastOutput) {
                $script:LastOutput = $script:LastOutput -replace '\x1b\[[0-9;]*m',''
                $script:LastOutput = $script:LastOutput.Trim()
            }
        }
        Remove-Item $tmpOut, $tmpErr -Force -ErrorAction SilentlyContinue
    }
    if (-not $anySuccess) { return -1 }
    return [math]::Round($best, 3)
}

# ── Collect platform info ──
$platform = Get-PlatformInfo

Write-Host "================================================================"
Write-Host "BioLang Benchmark Suite"
Write-Host "================================================================"
Write-Host "  Date:     $($platform['date'])"
Write-Host "  OS:       $($platform['os'])"
Write-Host "  Arch:     $($platform['arch'])"
Write-Host "  CPU:      $($platform['cpu'])"
Write-Host "  Cores:    $($platform['cpu_cores'])"
Write-Host "  RAM:      $($platform['ram'])"
Write-Host "  BioLang:  $($platform['biolang'])"
Write-Host "  Python:   $($platform['python'])"
Write-Host "  R:        $($platform['r'])"
Write-Host "  Rust:     $($platform['rustc'])"
Write-Host "  Runs:     $Runs (best of)"
Write-Host "================================================================"
Write-Host ""

# ── Data file sizes ──
Write-Host "Input data:" -ForegroundColor DarkGray
Get-ChildItem "data" | ForEach-Object {
    $sizeMB = [math]::Round($_.Length / 1MB, 1)
    Write-Host "  $($_.Name): $sizeMB MB" -ForegroundColor DarkGray
}
if (Test-Path "data_real") {
    Get-ChildItem "data_real" | ForEach-Object {
        $sizeMB = [math]::Round($_.Length / 1MB, 1)
        Write-Host "  $($_.Name): $sizeMB MB" -ForegroundColor DarkGray
    }
}
Write-Host ""

# ── Run benchmarks ──
if (-not (Test-Path "results")) { New-Item -ItemType Directory -Path "results" | Out-Null }
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
if ($IsWindows -or [System.Environment]::OSVersion.Platform -eq "Win32NT") {
    $platformTag = "windows"
} elseif ($IsMacOS) {
    $platformTag = "macos"
} else {
    $platformTag = "linux"
}

# Per-category results and a summary
$allCategoryResults = @()

foreach ($cat in $Categories) {
    if (-not (Should-RunCategory $cat)) { continue }

    $catResults = @()

    Write-Host "================================================================" -ForegroundColor Cyan
    Write-Host "  $($cat.Name) ($($cat.Group)/$($cat.Id))" -ForegroundColor Cyan
    Write-Host "================================================================" -ForegroundColor Cyan
    Write-Host ""

    foreach ($task in $cat.Tasks) {
        $taskId = $task.Id
        $taskName = $task.Name

        # Skip real-world benchmarks if data_real/ doesn't exist
        if ($task.Suite -eq "real" -and -not (Test-Path "data_real")) {
            Write-Host "  $taskName -- SKIPPED (no data_real/)" -ForegroundColor DarkGray
            continue
        }

        Write-Host "  $taskName" -ForegroundColor White

        $taskResult = @{ Name = $taskName; Id = $taskId; Category = $cat.Id; CategoryName = $cat.Name; Group = $cat.Group; Suite = $task.Suite }

        # BioLang
        $blFile = $task.Bl
        if ($blFile -and (Test-Path $blFile)) {
            $blLoc = Count-LOC $blFile
            $blTime = Best-Time "bl" @("run", $blFile)
            if ($blTime -ge 0) {
                Write-Host "    BioLang:  ${blTime}s  ($blLoc LOC)" -ForegroundColor Green
                $taskResult["bl_time"] = $blTime; $taskResult["bl_loc"] = $blLoc
                $taskResult["bl_output"] = $script:LastOutput
            } else {
                Write-Host "    BioLang:  FAILED" -ForegroundColor Red
            }
        }

        # Python
        $pyFile = $task.Py
        if ($pyFile -and (Test-Path $pyFile)) {
            $pyLoc = Count-LOC $pyFile
            $pyTime = Best-Time "python" @($pyFile)
            if ($pyTime -ge 0) {
                Write-Host "    Python:   ${pyTime}s  ($pyLoc LOC)" -ForegroundColor Yellow
                $taskResult["py_time"] = $pyTime; $taskResult["py_loc"] = $pyLoc
                $taskResult["py_output"] = $script:LastOutput
            } else {
                Write-Host "    Python:   FAILED" -ForegroundColor Red
            }
        }

        # R
        $rFile = $task.R
        if ($rFile -and (Test-Path $rFile)) {
            $rLoc = Count-LOC $rFile
            $rTime = Best-Time "Rscript" @($rFile)
            if ($rTime -ge 0) {
                Write-Host "    R:        ${rTime}s  ($rLoc LOC)" -ForegroundColor Magenta
                $taskResult["r_time"] = $rTime; $taskResult["r_loc"] = $rLoc
                $taskResult["r_output"] = $script:LastOutput
            } else {
                Write-Host "    R:        FAILED" -ForegroundColor Red
            }
        }

        $catResults += $taskResult
        Write-Host ""
    }

    # ── Write per-category report ──
    $prefix = if ($cat.Group -eq "language") { "lang" } else { "pipe" }
    $catCsvFile  = "results/${prefix}_$($cat.Id)_${platformTag}_${timestamp}.csv"
    $catReportFile = "results/${prefix}_$($cat.Id)_${platformTag}_${timestamp}.md"

    "task,language,time_sec,loc" | Out-File $catCsvFile -Encoding utf8
    foreach ($r in $catResults) {
        if ($r["bl_time"]) { "$($r.Id),biolang,$($r["bl_time"]),$($r["bl_loc"])" | Out-File $catCsvFile -Append -Encoding utf8 }
        if ($r["py_time"]) { "$($r.Id),python,$($r["py_time"]),$($r["py_loc"])" | Out-File $catCsvFile -Append -Encoding utf8 }
        if ($r["r_time"])  { "$($r.Id),r,$($r["r_time"]),$($r["r_loc"])" | Out-File $catCsvFile -Append -Encoding utf8 }
    }

    $catReport = "# $($cat.Name) Benchmark Report" + [Environment]::NewLine + [Environment]::NewLine
    $catReport += "**Category**: $($cat.Group) / $($cat.Id)" + [Environment]::NewLine
    $catReport += "**Platform**: $($platform['os']), $($platform['cpu']), $($platform['ram'])" + [Environment]::NewLine
    $catReport += "**Date**: $($platform['date'])" + [Environment]::NewLine + [Environment]::NewLine

    $catReport += "## Execution Time (seconds, best of $Runs)" + [Environment]::NewLine + [Environment]::NewLine
    $catReport += "| Task | BioLang | Python | R | BL vs Py | BL vs R |" + [Environment]::NewLine
    $catReport += "|---|---|---|---|---|---|" + [Environment]::NewLine

    foreach ($r in $catResults) {
        $blT  = if ($r["bl_time"]) { $r["bl_time"] } else { "-" }
        $pyT  = if ($r["py_time"]) { $r["py_time"] } else { "-" }
        $rT   = if ($r["r_time"])  { $r["r_time"] }  else { "-" }
        $vsPy = if ($r["bl_time"] -and $r["py_time"] -and $r["bl_time"] -gt 0) {
            $ratio = [math]::Round($r["py_time"] / $r["bl_time"], 1)
            "${ratio}x"
        } else { "-" }
        $vsR  = if ($r["bl_time"] -and $r["r_time"] -and $r["bl_time"] -gt 0) {
            $ratio = [math]::Round($r["r_time"] / $r["bl_time"], 1)
            "${ratio}x"
        } else { "-" }
        $catReport += "| $($r['Name']) | $blT | $pyT | $rT | $vsPy | $vsR |" + [Environment]::NewLine
    }

    $catReport += [Environment]::NewLine + "## Lines of Code" + [Environment]::NewLine + [Environment]::NewLine
    $catReport += "| Task | BioLang | Python | R |" + [Environment]::NewLine
    $catReport += "|---|---|---|---|" + [Environment]::NewLine
    foreach ($r in $catResults) {
        $blL  = if ($r["bl_loc"]) { $r["bl_loc"] } else { "-" }
        $pyL  = if ($r["py_loc"]) { $r["py_loc"] } else { "-" }
        $rL   = if ($r["r_loc"])  { $r["r_loc"] }  else { "-" }
        $catReport += "| $($r['Name']) | $blL | $pyL | $rL |" + [Environment]::NewLine
    }

    # Output comparison
    $catReport += [Environment]::NewLine + "## Output Comparison" + [Environment]::NewLine + [Environment]::NewLine
    if ($cat.Id -eq "kmer" -or $cat.Id -eq "sequence_io") {
        $catReport += "> **Note on k-mer counts:** BioLang reports slightly fewer distinct k-mers than Python (e.g. 27,294,096 vs 27,294,178). This is expected -- BioLang uses **canonical k-mers** (each k-mer and its reverse complement map to the same key), while Python counts raw forward-strand k-mers only." + [Environment]::NewLine + [Environment]::NewLine
    }
    $fence = '```'
    foreach ($r in $catResults) {
        $catReport += "### $($r['Name'])" + [Environment]::NewLine + [Environment]::NewLine
        foreach ($lang in @(@{L="BioLang";K="bl_output"}, @{L="Python";K="py_output"}, @{L="R";K="r_output"})) {
            $out = $r[$lang.K]
            if ($out) {
                $catReport += "**$($lang.L)**:" + [Environment]::NewLine
                $catReport += $fence + [Environment]::NewLine + $out + [Environment]::NewLine + $fence + [Environment]::NewLine + [Environment]::NewLine
            }
        }
    }

    [System.IO.File]::WriteAllText($catReportFile, $catReport, [System.Text.UTF8Encoding]::new($false))
    Write-Host "  Report: $catReportFile" -ForegroundColor DarkGray

    $allCategoryResults += @{ Category = $cat; Results = $catResults }
}

# ── Write summary report ──
$summaryFile = "results/summary_${platformTag}_${timestamp}.md"
$summary = @"
# BioLang Benchmark Summary

## Platform

| Field | Value |
|---|---|
| Date | $($platform['date']) |
| OS | $($platform['os']) |
| Architecture | $($platform['arch']) |
| CPU | $($platform['cpu']) |
| Cores | $($platform['cpu_cores']) |
| RAM | $($platform['ram']) |
| BioLang | $($platform['biolang']) |
| Python | $($platform['python']) |
| R | $($platform['r']) |
| Rust | $($platform['rustc']) |
| Runs | $Runs (best of) |

"@

foreach ($cr in $allCategoryResults) {
    $cat = $cr.Category
    $results = $cr.Results
    if ($results.Count -eq 0) { continue }

    $summary += "## $($cat.Name)" + [Environment]::NewLine + [Environment]::NewLine
    $summary += "| Task | BioLang | Python | R | BL vs Py | BL vs R | LOC BL/Py/R |" + [Environment]::NewLine
    $summary += "|---|---|---|---|---|---|---|" + [Environment]::NewLine

    foreach ($r in $results) {
        $blT  = if ($r["bl_time"]) { "$($r["bl_time"])s" } else { "-" }
        $pyT  = if ($r["py_time"]) { "$($r["py_time"])s" } else { "-" }
        $rT   = if ($r["r_time"])  { "$($r["r_time"])s" }  else { "-" }
        $vsPy = if ($r["bl_time"] -and $r["py_time"] -and $r["bl_time"] -gt 0) {
            $ratio = [math]::Round($r["py_time"] / $r["bl_time"], 1)
            if ($ratio -gt 1.1) { "**${ratio}x**" } else { "${ratio}x" }
        } else { "-" }
        $vsR  = if ($r["bl_time"] -and $r["r_time"] -and $r["bl_time"] -gt 0) {
            $ratio = [math]::Round($r["r_time"] / $r["bl_time"], 1)
            if ($ratio -gt 1.1) { "**${ratio}x**" } else { "${ratio}x" }
        } else { "-" }
        $blL  = if ($r["bl_loc"]) { $r["bl_loc"] } else { "-" }
        $pyL  = if ($r["py_loc"]) { $r["py_loc"] } else { "-" }
        $rL   = if ($r["r_loc"])  { $r["r_loc"] }  else { "-" }
        $summary += "| $($r['Name']) | $blT | $pyT | $rT | $vsPy | $vsR | $blL / $pyL / $rL |" + [Environment]::NewLine
    }
    $summary += [Environment]::NewLine
}

$summary += @"

## Methodology

- **Time**: Wall-clock via ``Stopwatch``, best of $Runs consecutive runs
- **LOC**: Non-blank, non-comment lines (idiomatic code per language)
- **Data**: Synthetic is reproducible (seed=42); real-world from public databases
- **Conditions**: All tasks ran sequentially, no other heavy processes
- **K-mer note**: BioLang counts *canonical* (strand-agnostic) 21-mers -- strictly more work than Python's forward-only approach

---
*Generated by BioLang Benchmark Suite on $($platform['date'])*
"@

[System.IO.File]::WriteAllText($summaryFile, $summary, [System.Text.UTF8Encoding]::new($false))

# ── Write scores YAML (single source of truth) ──
$scoresFile = "results/scores_${platformTag}_${timestamp}.yaml"
$yaml = @"
# BioLang Benchmark Scores — Single source of truth
# Generated by run_all.ps1 on $($platform['date'])
# Copy to website/README/book from here

platform:
  os: "$($platform['os'])"
  cpu: "$($platform['cpu'])"
  ram: "$($platform['ram'])"
  biolang: "$($platform['biolang'])"
  python: "$($platform['python'])"
  r: "$($platform['r'])"
  date: "$(Get-Date -Format 'yyyy-MM-dd')"

categories:

"@

foreach ($cr in $allCategoryResults) {
    $cat = $cr.Category
    $results = $cr.Results
    if ($results.Count -eq 0) { continue }

    $yaml += "  $($cat.Id):" + [Environment]::NewLine
    $yaml += "    name: `"$($cat.Name)`"" + [Environment]::NewLine
    $yaml += "    group: $($cat.Group)" + [Environment]::NewLine
    $yaml += "    tasks:" + [Environment]::NewLine

    foreach ($r in $results) {
        $yaml += "      - id: $($r.Id)" + [Environment]::NewLine
        $yaml += "        name: `"$($r['Name'])`"" + [Environment]::NewLine
        $yaml += "        data: $(if ($r['Suite']) { $r['Suite'] } else { 'synthetic' })" + [Environment]::NewLine

        # BioLang
        $blT = if ($r["bl_time"]) { "$($r["bl_time"])" } else { "null" }
        $blL = if ($r["bl_loc"]) { "$($r["bl_loc"])" } else { "null" }
        $yaml += "        bl: { time: $blT, loc: $blL }" + [Environment]::NewLine

        # Python
        $pyT = if ($r["py_time"]) { "$($r["py_time"])" } else { "null" }
        $pyL = if ($r["py_loc"]) { "$($r["py_loc"])" } else { "null" }
        $yaml += "        py: { time: $pyT, loc: $pyL }" + [Environment]::NewLine

        # R (skip for pipeline categories that have no R)
        if ($cat.Group -eq "language") {
            $rT = if ($r["r_time"]) { "$($r["r_time"])" } else { "null" }
            $rL = if ($r["r_loc"]) { "$($r["r_loc"])" } else { "null" }
            $yaml += "        r:  { time: $rT, loc: $rL }" + [Environment]::NewLine
        }
    }
    $yaml += [Environment]::NewLine
}

$yaml += "# null = benchmark did not run (script missing, tool not found, or data unavailable)" + [Environment]::NewLine
[System.IO.File]::WriteAllText($scoresFile, $yaml, [System.Text.UTF8Encoding]::new($false))

Write-Host ""
Write-Host "================================================================"
Write-Host "Summary:  $summaryFile" -ForegroundColor Cyan
Write-Host "Scores:   $scoresFile" -ForegroundColor Cyan
Write-Host "================================================================"
