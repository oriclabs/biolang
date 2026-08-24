param(
    [int[]]$Sizes = @(1000, 5000, 20000, 100000),
    [ValidateRange(1, 20)][int]$Repeats = 5,
    [string]$Output = "packages/statistics/validation/plot-benchmark.json"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$executable = Join-Path $repoRoot "target\release\examples\plot_dense_benchmark.exe"

Push-Location $repoRoot
try {
    & cargo build --release -p bl-runtime --example plot_dense_benchmark
    if ($LASTEXITCODE -ne 0) { throw "dense plot benchmark build failed" }

    $cases = @()
    foreach ($size in $Sizes) {
        foreach ($mode in @("off", "on")) {
            $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
            $startInfo.FileName = $executable
            $startInfo.Arguments = "--size $size --raster $mode --repeats $Repeats"
            $startInfo.WorkingDirectory = $repoRoot
            $startInfo.UseShellExecute = $false
            $startInfo.CreateNoWindow = $true
            $startInfo.RedirectStandardOutput = $true
            $startInfo.RedirectStandardError = $true

            $process = [System.Diagnostics.Process]::new()
            $process.StartInfo = $startInfo
            if (-not $process.Start()) { throw "could not start dense plot benchmark" }
            $stdoutTask = $process.StandardOutput.ReadToEndAsync()
            $stderrTask = $process.StandardError.ReadToEndAsync()
            $peakBytes = 0L
            while (-not $process.WaitForExit(10)) {
                try {
                    $process.Refresh()
                    $peakBytes = [Math]::Max($peakBytes, [long]$process.WorkingSet64)
                }
                catch {
                    # It can exit between WaitForExit and Refresh.
                }
            }
            $process.WaitForExit()
            try {
                $process.Refresh()
                $peakBytes = [Math]::Max($peakBytes, [long]$process.PeakWorkingSet64)
            }
            catch {
                # Keep the sampled peak if platform process accounting is unavailable.
            }
            $stdout = $stdoutTask.Result
            $stderr = $stderrTask.Result
            if ($process.ExitCode -ne 0) {
                throw "dense plot benchmark failed: $stderr"
            }
            $measurement = $stdout | ConvertFrom-Json
            $reportedPeak = if ($peakBytes -gt 0) { $peakBytes } else { $null }
            $measurement | Add-Member -NotePropertyName peak_working_set_bytes -NotePropertyValue $reportedPeak
            $cases += $measurement
            $process.Dispose()
        }
    }

    $manifest = [ordered]@{
        schema = "biolang.plot.dense-benchmark/v1"
        generated_utc = [DateTime]::UtcNow.ToString("o")
        platform = [System.Environment]::OSVersion.VersionString
        architecture = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
        processor = $env:PROCESSOR_IDENTIFIER
        build = "release"
        renderer = "umap_plot SVG with vector or embedded tiny-skia PNG point layer"
        default_raster_threshold = 20000
        repetitions_per_process = $Repeats
        timing = "median render-only elapsed time; input generation excluded"
        memory = "maximum sampled process working set; includes prepared input and renderer"
        cases = $cases
    }
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Output -Encoding UTF8
    $manifest | ConvertTo-Json -Depth 8
}
finally {
    Pop-Location
}
