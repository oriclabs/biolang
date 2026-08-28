param(
    [string]$BioLangExe = "",
    [string]$RscriptPath = "",
    [string]$PythonPath = "",
    [ValidateRange(1, 20)][int]$BenchmarkRepeats = 3,
    [switch]$RequireR,
    [switch]$RequirePython
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$results = Join-Path $PSScriptRoot "results"
New-Item -ItemType Directory -Force $results | Out-Null

function Quote-NativeArgument([string]$Value) {
    if ($Value -notmatch '[\s"]') { return $Value }
    return '"' + ($Value -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"'
}

function Invoke-ValidationProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$ArgumentList
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    $startInfo.Arguments = (($ArgumentList | ForEach-Object { Quote-NativeArgument $_ }) -join ' ')
    $startInfo.WorkingDirectory = $repoRoot
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    if (-not $process.Start()) { throw "Could not start $FilePath" }
    $processStarted = $process.StartTime
    $relatedProcessName = [System.IO.Path]::GetFileNameWithoutExtension($FilePath)
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $peakBytes = 0L
    while (-not $process.WaitForExit(20)) {
        # On Windows Rscript starts a second Rscript process. Track processes
        # with the same executable name that began in this invocation window,
        # otherwise only the small launcher (about 8 MiB) would be reported.
        $workingSet = 0L
        foreach ($related in @(Get-Process -Name $relatedProcessName -ErrorAction SilentlyContinue)) {
            try {
                if ($related.StartTime -ge $processStarted.AddMilliseconds(-250)) {
                    $workingSet += [long]$related.WorkingSet64
                }
            }
            catch {
                # The process may exit between enumeration and inspection.
            }
        }
        $peakBytes = [Math]::Max($peakBytes, $workingSet)
    }
    $process.WaitForExit()
    $timer.Stop()
    $stdout = $stdoutTask.Result
    $stderr = $stderrTask.Result
    if ($stdout) { [Console]::Out.Write($stdout) }
    if ($stderr) { [Console]::Error.Write($stderr) }
    $exitCode = $process.ExitCode
    $process.Dispose()

    [pscustomobject]@{
        exit_code = $exitCode
        elapsed_seconds = $timer.Elapsed.TotalSeconds
        peak_working_set_bytes = $peakBytes
    }
}

function Get-Percentile([double[]]$Values, [double]$Probability) {
    if ($Values.Count -eq 0) { return $null }
    $sorted = @($Values | Sort-Object)
    if ($sorted.Count -eq 1) { return [double]$sorted[0] }
    $position = $Probability * ($sorted.Count - 1)
    $lower = [Math]::Floor($position)
    $upper = [Math]::Ceiling($position)
    $fraction = $position - $lower
    return [double]$sorted[$lower] + $fraction * ([double]$sorted[$upper] - [double]$sorted[$lower])
}

$rscriptExe = ""
if ($RscriptPath) {
    if (-not (Test-Path -LiteralPath $RscriptPath -PathType Leaf)) {
        throw "Rscript was not found at the requested path: $RscriptPath"
    }
    $rscriptExe = (Resolve-Path -LiteralPath $RscriptPath).Path
}
else {
    $rscript = Get-Command Rscript -ErrorAction SilentlyContinue
    if ($rscript) { $rscriptExe = $rscript.Source }
}
if (-not $rscriptExe) {
    $message = "Rscript is unavailable; external reference validation was not run. Install R or add Rscript to PATH."
    if ($RequireR) { throw $message }
    Write-Warning $message
    exit 2
}

function Compare-Geometry {
    param(
        [Parameter(Mandatory = $true)][string]$Section,
        [Parameter(Mandatory = $true)][object]$Expected,
        [Parameter(Mandatory = $true)][object]$Actual,
        [Parameter(Mandatory = $true)][string[]]$Fields,
        [string[]]$ExactFields = @(),
        [double]$Tolerance = 1e-12
    )

    foreach ($field in $Fields) {
        $referenceValues = @($Expected.$field)
        $observedValues = @($Actual.$field)
        if ($referenceValues.Count -ne $observedValues.Count) {
            throw "$Section geometry length mismatch for $field"
        }
        $fieldTolerance = if ($field -in $ExactFields) { 0 } else { $Tolerance }
        for ($index = 0; $index -lt $referenceValues.Count; $index++) {
            $script:checks += ,@("$Section.$field.$index", $referenceValues[$index], $observedValues[$index], $fieldTolerance)
        }
    }
}

$pythonExe = ""
if ($PythonPath) {
    if (-not (Test-Path -LiteralPath $PythonPath -PathType Leaf)) {
        throw "Python was not found at the requested path: $PythonPath"
    }
    $pythonExe = (Resolve-Path -LiteralPath $PythonPath).Path
}
else {
    $python = Get-Command python -ErrorAction SilentlyContinue
    if ($python) { $pythonExe = $python.Source }
}
if ($pythonExe) {
    & $pythonExe -c "import numpy, statsmodels"
    if ($LASTEXITCODE -ne 0) {
        if ($RequirePython) { throw "Python is available but NumPy/statsmodels is not installed" }
        Write-Warning "NumPy or statsmodels is unavailable; the supplemental Python plot oracle will be skipped."
        $pythonExe = ""
    }
}
elseif ($RequirePython) {
    throw "Python is unavailable; the supplemental NumPy plot oracle cannot run."
}

Push-Location $repoRoot
try {
    $usingRepositoryBuild = -not $BioLangExe
    if ($usingRepositoryBuild) {
        $BioLangExe = Join-Path $repoRoot "target\debug\bl.exe"
        & cargo build -p bl-cli
        if ($LASTEXITCODE -ne 0) { throw "cargo build -p bl-cli failed" }
    }
    if (-not (Test-Path -LiteralPath $BioLangExe)) {
        throw "BioLang executable was not found: $BioLangExe"
    }

    $rSuites = [System.Collections.Generic.List[object]]::new()
    $blSuites = [System.Collections.Generic.List[object]]::new()
    for ($repeat = 1; $repeat -le $BenchmarkRepeats; $repeat++) {
        $rRuns = @(
            Invoke-ValidationProcess $rscriptExe @("packages/statistics/validation/reference.R")
            Invoke-ValidationProcess $rscriptExe @("packages/statistics/validation/inference_reference.R")
            Invoke-ValidationProcess $rscriptExe @("packages/statistics/validation/plot_reference.R")
        )
        if (@($rRuns | Where-Object { $_.exit_code -ne 0 }).Count -gt 0) {
            throw "R reference run failed on repetition $repeat"
        }
        $rSuites.Add([pscustomobject]@{
            repetition = $repeat
            elapsed_seconds = ($rRuns | Measure-Object -Property elapsed_seconds -Sum).Sum
            peak_working_set_bytes = ($rRuns | Measure-Object -Property peak_working_set_bytes -Maximum).Maximum
        })

        $blRuns = @(
            Invoke-ValidationProcess $BioLangExe @("run", "packages/statistics/validation/biolang_reference.bl")
            Invoke-ValidationProcess $BioLangExe @("run", "packages/statistics/validation/biolang_inference.bl")
            Invoke-ValidationProcess $BioLangExe @("run", "packages/statistics/validation/biolang_plot_reference.bl")
        )
        if (@($blRuns | Where-Object { $_.exit_code -ne 0 }).Count -gt 0) {
            throw "BioLang reference run failed on repetition $repeat"
        }
        $blSuites.Add([pscustomobject]@{
            repetition = $repeat
            elapsed_seconds = ($blRuns | Measure-Object -Property elapsed_seconds -Sum).Sum
            peak_working_set_bytes = ($blRuns | Measure-Object -Property peak_working_set_bytes -Maximum).Maximum
        })
    }
    $numpyPlotRun = $null
    if ($pythonExe) {
        $numpyPlotRun = Invoke-ValidationProcess $pythonExe @("packages/statistics/validation/plot_reference.py")
        if ($numpyPlotRun.exit_code -ne 0) { throw "NumPy plot reference run failed" }
    }

    $expected = Get-Content "packages/statistics/validation/results/r-reference.json" -Raw | ConvertFrom-Json
    $actual = Get-Content "packages/statistics/validation/results/biolang.json" -Raw | ConvertFrom-Json
    $expectedReal = Get-Content "packages/statistics/validation/results/r-real-reference.json" -Raw | ConvertFrom-Json
    $actualReal = Get-Content "packages/statistics/validation/results/biolang-real.json" -Raw | ConvertFrom-Json
    $expectedGlm = Get-Content "packages/statistics/validation/results/r-glm-reference.json" -Raw | ConvertFrom-Json
    $actualGlm = Get-Content "packages/statistics/validation/results/biolang-glm.json" -Raw | ConvertFrom-Json
    $expectedMixed = Get-Content "packages/statistics/validation/results/r-mixed-reference.json" -Raw | ConvertFrom-Json
    $actualMixed = Get-Content "packages/statistics/validation/results/biolang-mixed.json" -Raw | ConvertFrom-Json
    $expectedCox = Get-Content "packages/statistics/validation/results/r-cox-reference.json" -Raw | ConvertFrom-Json
    $actualCox = Get-Content "packages/statistics/validation/results/biolang-cox.json" -Raw | ConvertFrom-Json
    $expectedInference = Get-Content "packages/statistics/validation/results/r-inference-reference.json" -Raw | ConvertFrom-Json
    $actualInference = Get-Content "packages/statistics/validation/results/biolang-inference.json" -Raw | ConvertFrom-Json
    $expectedPlot = Get-Content "packages/statistics/validation/results/r-plot-reference.json" -Raw | ConvertFrom-Json
    $actualPlot = Get-Content "packages/statistics/validation/results/biolang-plot.json" -Raw | ConvertFrom-Json
    $expectedNumpyPlot = if ($pythonExe) {
        Get-Content "packages/statistics/validation/results/numpy-plot-reference.json" -Raw | ConvertFrom-Json
    } else { $null }
    $checks = @(
        @("descriptive.mean", $expected.descriptive.mean, $actual.descriptive.mean, 1e-12),
        @("descriptive.median", $expected.descriptive.median, $actual.descriptive.median, 1e-12),
        @("descriptive.variance", $expected.descriptive.variance, $actual.descriptive.variance, 1e-12),
        @("descriptive.sd", $expected.descriptive.sd, $actual.descriptive.sd, 1e-12),
        @("descriptive.q1", $expected.descriptive.q1, $actual.descriptive.q1, 1e-12),
        @("descriptive.q3", $expected.descriptive.q3, $actual.descriptive.q3, 1e-12),
        @("descriptive.mad", $expected.descriptive.mad, $actual.descriptive.mad, 1e-12),
        @("descriptive.skewness", $expected.descriptive.skewness, $actual.descriptive.skewness, 1e-12),
        @("log1p.mean", $expected.log1p.mean, $actual.log1p.mean, 1e-12),
        @("log1p.median", $expected.log1p.median, $actual.log1p.median, 1e-12),
        @("log1p.sd", $expected.log1p.sd, $actual.log1p.sd, 1e-12),
        @("log1p.skewness", $expected.log1p.skewness, $actual.log1p.skewness, 1e-12),
        @("relationship.pearson", $expected.relationship.pearson, $actual.relationship.pearson, 1e-12),
        @("relationship.spearman", $expected.relationship.spearman, $actual.relationship.spearman, 1e-12),
        @("relationship.slope", $expected.relationship.slope, $actual.relationship.slope, 1e-12),
        @("relationship.intercept", $expected.relationship.intercept, $actual.relationship.intercept, 1e-12),
        @("matrix.sample_total_ratio", $expected.matrix.sample_total_ratio, $actual.matrix.sample_total_ratio, 1e-12),
        @("linear_diagnostics.residual_mse", $expected.linear_diagnostics.residual_mse, $actual.linear_diagnostics.residual_mse, 1e-12),
        # R and BioLang use independent inverse-normal quantile approximations.
        @("linear_diagnostics.normal_qq_correlation", $expected.linear_diagnostics.normal_qq_correlation, $actual.linear_diagnostics.normal_qq_correlation, 1e-9),
        @("linear_diagnostics.scale_correlation", $expected.linear_diagnostics.scale_correlation, $actual.linear_diagnostics.scale_correlation, 1e-12),
        @("linear_diagnostics.curvature_correlation", $expected.linear_diagnostics.curvature_correlation, $actual.linear_diagnostics.curvature_correlation, 1e-12),
        @("linear_diagnostics.durbin_watson", $expected.linear_diagnostics.durbin_watson, $actual.linear_diagnostics.durbin_watson, 1e-12),
        @("linear_diagnostics.cook_threshold", $expected.linear_diagnostics.cook_threshold, $actual.linear_diagnostics.cook_threshold, 1e-12),
        @("linear_diagnostics.maximum_cook_distance", $expected.linear_diagnostics.maximum_cook_distance, $actual.linear_diagnostics.maximum_cook_distance, 1e-12),
        @("linear_diagnostics.cook_review_flags", $expected.linear_diagnostics.cook_review_flags, $actual.linear_diagnostics.cook_review_flags, 0),
        @("linear_diagnostics.standardized_residual_flags", $expected.linear_diagnostics.standardized_residual_flags, $actual.linear_diagnostics.standardized_residual_flags, 0),
        @("associations.pearson", $expected.associations.pearson, $actual.associations.pearson, 1e-12),
        @("associations.spearman", $expected.associations.spearman, $actual.associations.spearman, 1e-12),
        @("associations.cramers_v", $expected.associations.cramers_v, $actual.associations.cramers_v, 1e-12),
        @("associations.eta_squared", $expected.associations.eta_squared, $actual.associations.eta_squared, 1e-12),
        @("associations.mixed_screening_score", $expected.associations.mixed_screening_score, $actual.associations.mixed_screening_score, 1e-12),
        @("distribution_clues.variance_mean_ratio", $expected.distribution_clues.variance_mean_ratio, $actual.distribution_clues.variance_mean_ratio, 1e-12),
        @("distribution_clues.expected_poisson_zeros", $expected.distribution_clues.expected_poisson_zeros, $actual.distribution_clues.expected_poisson_zeros, 1e-12),
        @("distribution_clues.normal_log_likelihood", $expected.distribution_clues.normal_log_likelihood, $actual.distribution_clues.normal_log_likelihood, 1e-12),
        @("distribution_clues.normal_aic", $expected.distribution_clues.normal_aic, $actual.distribution_clues.normal_aic, 1e-12),
        @("distribution_clues.poisson_log_likelihood", $expected.distribution_clues.poisson_log_likelihood, $actual.distribution_clues.poisson_log_likelihood, 1e-12),
        @("distribution_clues.poisson_aic", $expected.distribution_clues.poisson_aic, $actual.distribution_clues.poisson_aic, 1e-12),
        @("distribution_clues.negative_binomial_theta", $expected.distribution_clues.negative_binomial_theta, $actual.distribution_clues.negative_binomial_theta, 1e-12),
        @("distribution_clues.negative_binomial_log_likelihood", $expected.distribution_clues.negative_binomial_log_likelihood, $actual.distribution_clues.negative_binomial_log_likelihood, 1e-12),
        @("distribution_clues.negative_binomial_aic", $expected.distribution_clues.negative_binomial_aic, $actual.distribution_clues.negative_binomial_aic, 1e-12),
        @("multiple_linear.coef0", $expected.multiple_linear.coef0, $actual.multiple_linear.coef0, 1e-9),
        @("multiple_linear.coef1", $expected.multiple_linear.coef1, $actual.multiple_linear.coef1, 1e-9),
        @("multiple_linear.coef2", $expected.multiple_linear.coef2, $actual.multiple_linear.coef2, 1e-9),
        @("multiple_linear.coef3", $expected.multiple_linear.coef3, $actual.multiple_linear.coef3, 1e-9),
        @("multiple_linear.r_squared", $expected.multiple_linear.r_squared, $actual.multiple_linear.r_squared, 1e-10),
        @("multiple_linear.adjusted_r_squared", $expected.multiple_linear.adjusted_r_squared, $actual.multiple_linear.adjusted_r_squared, 1e-10),
        @("multiple_linear.residual_mse", $expected.multiple_linear.residual_mse, $actual.multiple_linear.residual_mse, 1e-9),
        @("multiple_linear.maximum_vif", $expected.multiple_linear.maximum_vif, $actual.multiple_linear.maximum_vif, 1e-9),
        @("multiple_linear.normal_qq_correlation", $expected.multiple_linear.normal_qq_correlation, $actual.multiple_linear.normal_qq_correlation, 1e-9),
        @("multiple_linear.scale_correlation", $expected.multiple_linear.scale_correlation, $actual.multiple_linear.scale_correlation, 1e-9),
        @("multiple_linear.durbin_watson", $expected.multiple_linear.durbin_watson, $actual.multiple_linear.durbin_watson, 1e-9),
        @("multiple_linear.maximum_cook", $expected.multiple_linear.maximum_cook, $actual.multiple_linear.maximum_cook, 1e-8),
        @("multiple_linear.cook_flags", $expected.multiple_linear.cook_flags, $actual.multiple_linear.cook_flags, 0),
        @("multiple_linear.leverage_flags", $expected.multiple_linear.leverage_flags, $actual.multiple_linear.leverage_flags, 0),
        @("omics.zero_fraction", $expected.omics.zero_fraction, $actual.omics.zero_fraction, 1e-12),
        @("omics.sample_total_cv", $expected.omics.sample_total_cv, $actual.omics.sample_total_cv, 1e-12),
        @("omics.median_sample_zero_fraction", $expected.omics.median_sample_zero_fraction, $actual.omics.median_sample_zero_fraction, 1e-12),
        @("omics.feature_mean_variance_correlation", $expected.omics.feature_mean_variance_correlation, $actual.omics.feature_mean_variance_correlation, 1e-12),
        @("robust_linear.intercept", $expected.robust_linear.intercept, $actual.robust_linear.intercept, 1e-5),
        @("robust_linear.slope", $expected.robust_linear.slope, $actual.robust_linear.slope, 1e-5),
        @("robust_linear.scale", $expected.robust_linear.scale, $actual.robust_linear.scale, 1e-4),
        @("weighted.mean", $expected.weighted.mean, $actual.weighted.mean, 1e-12),
        @("weighted.variance", $expected.weighted.variance, $actual.weighted.variance, 1e-12),
        @("weighted.effective_n", $expected.weighted.effective_n, $actual.weighted.effective_n, 1e-12),
        @("time_series.acf1", $expected.time_series.acf1, $actual.time_series.acf1, 1e-12),
        @("time_series.acf2", $expected.time_series.acf2, $actual.time_series.acf2, 1e-12),
        @("time_series.acf3", $expected.time_series.acf3, $actual.time_series.acf3, 1e-12),
        @("time_series.ljung_box_q", $expected.time_series.ljung_box_q, $actual.time_series.ljung_box_q, 1e-12),
        @("time_series.ljung_box_p", $expected.time_series.ljung_box_p, $actual.time_series.ljung_box_p, 1e-10),
        @("time_series.trend", $expected.time_series.trend, $actual.time_series.trend, 1e-12),
        @("cluster.between_ms", $expected.cluster.between_ms, $actual.cluster.between_ms, 1e-12),
        @("cluster.within_ms", $expected.cluster.within_ms, $actual.cluster.within_ms, 1e-12),
        @("cluster.effective_size", $expected.cluster.effective_size, $actual.cluster.effective_size, 1e-12),
        @("cluster.icc", $expected.cluster.icc, $actual.cluster.icc, 1e-12),
        @("cluster.effective_n", $expected.cluster.effective_n, $actual.cluster.effective_n, 1e-12),
        @("means.arithmetic", $expected.means.arithmetic, $actual.means.arithmetic, 1e-12),
        @("means.geometric", $expected.means.geometric, $actual.means.geometric, 1e-12),
        @("means.harmonic", $expected.means.harmonic, $actual.means.harmonic, 1e-12),
        @("means.trimmed", $expected.means.trimmed, $actual.means.trimmed, 1e-12),
        @("means.rms", $expected.means.rms, $actual.means.rms, 1e-12)
    )
    $checks += @(
        @("real.airquality.observations", $expectedReal.airquality.observations, $actualReal.airquality.observations, 0),
        @("real.airquality.mean", $expectedReal.airquality.mean, $actualReal.airquality.mean, 1e-12),
        @("real.airquality.median", $expectedReal.airquality.median, $actualReal.airquality.median, 1e-12),
        @("real.airquality.sd", $expectedReal.airquality.sd, $actualReal.airquality.sd, 1e-12),
        @("real.airquality.skewness", $expectedReal.airquality.skewness, $actualReal.airquality.skewness, 1e-12),
        @("real.airquality.log_skewness", $expectedReal.airquality.log_skewness, $actualReal.airquality.log_skewness, 1e-12),
        @("real.airquality.weighted_mean", $expectedReal.airquality.weighted_mean, $actualReal.airquality.weighted_mean, 1e-12),
        @("real.airquality.weighted_variance", $expectedReal.airquality.weighted_variance, $actualReal.airquality.weighted_variance, 1e-12),
        @("real.airquality.effective_n", $expectedReal.airquality.effective_n, $actualReal.airquality.effective_n, 1e-12),
        @("real.nile.observations", $expectedReal.nile.observations, $actualReal.nile.observations, 0),
        @("real.nile.acf1", $expectedReal.nile.acf1, $actualReal.nile.acf1, 1e-12),
        @("real.nile.acf2", $expectedReal.nile.acf2, $actualReal.nile.acf2, 1e-12),
        @("real.nile.acf3", $expectedReal.nile.acf3, $actualReal.nile.acf3, 1e-12),
        @("real.nile.ljung_box_q", $expectedReal.nile.ljung_box_q, $actualReal.nile.ljung_box_q, 1e-12),
        @("real.nile.ljung_box_p", $expectedReal.nile.ljung_box_p, $actualReal.nile.ljung_box_p, 1e-10),
        @("real.nile.trend", $expectedReal.nile.trend, $actualReal.nile.trend, 1e-12),
        @("real.chickweight.observations", $expectedReal.chickweight.observations, $actualReal.chickweight.observations, 0),
        @("real.chickweight.clusters", $expectedReal.chickweight.clusters, $actualReal.chickweight.clusters, 0),
        @("real.chickweight.between_ms", $expectedReal.chickweight.between_ms, $actualReal.chickweight.between_ms, 1e-12),
        @("real.chickweight.within_ms", $expectedReal.chickweight.within_ms, $actualReal.chickweight.within_ms, 1e-12),
        @("real.chickweight.effective_size", $expectedReal.chickweight.effective_size, $actualReal.chickweight.effective_size, 1e-12),
        @("real.chickweight.icc", $expectedReal.chickweight.icc, $actualReal.chickweight.icc, 1e-12),
        @("real.lung.observations", $expectedReal.lung.observations, $actualReal.lung.observations, 0),
        @("real.lung.events", $expectedReal.lung.events, $actualReal.lung.events, 0),
        @("real.lung.final_survival", $expectedReal.lung.final_survival, $actualReal.lung.final_survival, 1e-12)
    )
    $checks += @(
        @("glm.binomial.coef0", $expectedGlm.binomial.coef0, $actualGlm.binomial.coef0, 1e-7),
        @("glm.binomial.coef1", $expectedGlm.binomial.coef1, $actualGlm.binomial.coef1, 1e-7),
        @("glm.binomial.coef2", $expectedGlm.binomial.coef2, $actualGlm.binomial.coef2, 1e-7),
        @("glm.binomial.null_deviance", $expectedGlm.binomial.null_deviance, $actualGlm.binomial.null_deviance, 1e-10),
        @("glm.binomial.residual_deviance", $expectedGlm.binomial.residual_deviance, $actualGlm.binomial.residual_deviance, 1e-7),
        @("glm.binomial.aic", $expectedGlm.binomial.aic, $actualGlm.binomial.aic, 1e-7),
        @("glm.binomial.dispersion", $expectedGlm.binomial.dispersion, $actualGlm.binomial.dispersion, 1e-7),
        @("glm.binomial.brier", $expectedGlm.binomial.brier, $actualGlm.binomial.brier, 1e-7),
        @("glm.binomial.maximum_leverage", $expectedGlm.binomial.maximum_leverage, $actualGlm.binomial.maximum_leverage, 1e-7),
        @("glm.binomial.maximum_cook", $expectedGlm.binomial.maximum_cook, $actualGlm.binomial.maximum_cook, 1e-6),
        @("glm.poisson.coef0", $expectedGlm.poisson.coef0, $actualGlm.poisson.coef0, 1e-7),
        @("glm.poisson.coef1", $expectedGlm.poisson.coef1, $actualGlm.poisson.coef1, 1e-7),
        @("glm.poisson.coef2", $expectedGlm.poisson.coef2, $actualGlm.poisson.coef2, 1e-7),
        @("glm.poisson.coef3", $expectedGlm.poisson.coef3, $actualGlm.poisson.coef3, 1e-7),
        @("glm.poisson.null_deviance", $expectedGlm.poisson.null_deviance, $actualGlm.poisson.null_deviance, 1e-10),
        @("glm.poisson.residual_deviance", $expectedGlm.poisson.residual_deviance, $actualGlm.poisson.residual_deviance, 1e-7),
        @("glm.poisson.aic", $expectedGlm.poisson.aic, $actualGlm.poisson.aic, 1e-7),
        @("glm.poisson.dispersion", $expectedGlm.poisson.dispersion, $actualGlm.poisson.dispersion, 1e-7),
        @("glm.poisson.expected_zeros", $expectedGlm.poisson.expected_zeros, $actualGlm.poisson.expected_zeros, 1e-7),
        @("glm.poisson.maximum_leverage", $expectedGlm.poisson.maximum_leverage, $actualGlm.poisson.maximum_leverage, 1e-7),
        @("glm.poisson.maximum_cook", $expectedGlm.poisson.maximum_cook, $actualGlm.poisson.maximum_cook, 1e-6)
    )
    $checks += @(
        @("mixed.fixed_intercept", $expectedMixed.fixed_intercept, $actualMixed.fixed_intercept, 1e-6),
        @("mixed.fixed_time", $expectedMixed.fixed_time, $actualMixed.fixed_time, 1e-6),
        @("mixed.random_intercept_variance", $expectedMixed.random_intercept_variance, $actualMixed.random_intercept_variance, 1e-5),
        @("mixed.residual_variance", $expectedMixed.residual_variance, $actualMixed.residual_variance, 1e-5),
        @("mixed.icc", $expectedMixed.icc, $actualMixed.icc, 1e-5),
        @("mixed.clusters", $expectedMixed.clusters, $actualMixed.clusters, 0),
        @("mixed.observations", $expectedMixed.observations, $actualMixed.observations, 0)
    )
    $checks += @(
        @("cox.coef_age", $expectedCox.coef_age, $actualCox.coef_age, 1e-7),
        @("cox.coef_sex", $expectedCox.coef_sex, $actualCox.coef_sex, 1e-7),
        @("cox.se_age", $expectedCox.se_age, $actualCox.se_age, 1e-7),
        @("cox.se_sex", $expectedCox.se_sex, $actualCox.se_sex, 1e-7),
        @("cox.partial_log_likelihood", $expectedCox.partial_log_likelihood, $actualCox.partial_log_likelihood, 1e-8),
        @("cox.likelihood_ratio", $expectedCox.likelihood_ratio, $actualCox.likelihood_ratio, 1e-8),
        @("cox.aic_partial", $expectedCox.aic_partial, $actualCox.aic_partial, 1e-8),
        @("cox.final_baseline_hazard", $expectedCox.final_baseline_hazard, $actualCox.final_baseline_hazard, 1e-7),
        @("cox.martingale_sum", $expectedCox.martingale_sum, $actualCox.martingale_sum, 1e-7),
        @("cox.martingale_sum_squares", $expectedCox.martingale_sum_squares, $actualCox.martingale_sum_squares, 1e-7),
        @("cox.schoenfeld_age_time_correlation", $expectedCox.schoenfeld_age_time_correlation, $actualCox.schoenfeld_age_time_correlation, 1e-7),
        @("cox.schoenfeld_sex_time_correlation", $expectedCox.schoenfeld_sex_time_correlation, $actualCox.schoenfeld_sex_time_correlation, 1e-7),
        @("cox.observations", $expectedCox.observations, $actualCox.observations, 0),
        @("cox.events", $expectedCox.events, $actualCox.events, 0)
    )

    # Classic tests are evaluated live rather than only against constants
    # copied into Rust unit tests. A fifth item marks a deliberate R/BioLang
    # convention difference; those rows must remain different or the
    # documentation and API choice need review.
    $checks += @(
        @("inference.ttest.statistic", $expectedInference.ttest_pooled.statistic, $actualInference.ttest.statistic, 1e-12),
        @("inference.ttest.p_value", $expectedInference.ttest_pooled.p_value, $actualInference.ttest.p_value, 1e-11),
        @("inference.ttest.df", $expectedInference.ttest_pooled.df, $actualInference.ttest.df, 0),
        @("inference.ttest.standard_error", $expectedInference.ttest_pooled.standard_error, $actualInference.ttest.standard_error, 1e-12),
        @("inference.ttest.confidence_lower", $expectedInference.ttest_pooled.confidence_lower, $actualInference.ttest.confidence_lower, 1e-10),
        @("inference.ttest.confidence_upper", $expectedInference.ttest_pooled.confidence_upper, $actualInference.ttest.confidence_upper, 1e-10),
        @("inference.ttest.cohens_d", $expectedInference.ttest_pooled.cohens_d, $actualInference.ttest.cohens_d, 1e-12),
        @("inference.ttest.hedges_g", $expectedInference.ttest_pooled.hedges_g, $actualInference.ttest.hedges_g, 1e-12),
        @("inference.ttest_welch.statistic", $expectedInference.ttest_r_default.statistic, $actualInference.ttest_welch.statistic, 1e-12),
        @("inference.ttest_welch.p_value", $expectedInference.ttest_r_default.p_value, $actualInference.ttest_welch.p_value, 1e-11),
        @("inference.ttest_welch.df", $expectedInference.ttest_r_default.df, $actualInference.ttest_welch.df, 1e-12),
        @("inference.ttest_welch.standard_error", $expectedInference.ttest_r_default.standard_error, $actualInference.ttest_welch.standard_error, 1e-12),
        @("inference.ttest_welch.confidence_lower", $expectedInference.ttest_r_default.confidence_lower, $actualInference.ttest_welch.confidence_lower, 1e-10),
        @("inference.ttest_welch.confidence_upper", $expectedInference.ttest_r_default.confidence_upper, $actualInference.ttest_welch.confidence_upper, 1e-10),
        @("convention.ttest.r_default_p_value", $expectedInference.ttest_r_default.p_value, $actualInference.ttest.p_value, 1e-12, "expected_convention_difference"),
        @("convention.ttest.r_default_df", $expectedInference.ttest_r_default.df, $actualInference.ttest.df, 1e-12, "expected_convention_difference"),
        @("inference.ttest_one.statistic", $expectedInference.ttest_one.statistic, $actualInference.ttest_one.statistic, 1e-12),
        @("inference.ttest_one.p_value", $expectedInference.ttest_one.p_value, $actualInference.ttest_one.p_value, 1e-11),
        @("inference.ttest_one.df", $expectedInference.ttest_one.df, $actualInference.ttest_one.df, 0),
        @("inference.ttest_one.confidence_lower", $expectedInference.ttest_one.confidence_lower, $actualInference.ttest_one.confidence_lower, 1e-10),
        @("inference.ttest_one.confidence_upper", $expectedInference.ttest_one.confidence_upper, $actualInference.ttest_one.confidence_upper, 1e-10),
        @("inference.ttest_one.cohens_d", $expectedInference.ttest_one.cohens_d, $actualInference.ttest_one.cohens_d, 1e-12),
        @("edge.tiny_ttest_one.statistic", $expectedInference.ttest_tiny.statistic, $actualInference.ttest_tiny.statistic, 1e-12),
        @("edge.tiny_ttest_one.p_value", $expectedInference.ttest_tiny.p_value, $actualInference.ttest_tiny.p_value, 1e-11),
        @("edge.tiny_ttest_one.df", $expectedInference.ttest_tiny.df, $actualInference.ttest_tiny.df, 0),
        @("inference.ttest_paired.statistic", $expectedInference.ttest_paired.statistic, $actualInference.ttest_paired.statistic, 1e-12),
        @("inference.ttest_paired.p_value", $expectedInference.ttest_paired.p_value, $actualInference.ttest_paired.p_value, 1e-11),
        @("inference.ttest_paired.df", $expectedInference.ttest_paired.df, $actualInference.ttest_paired.df, 0),
        @("inference.ttest_paired.confidence_lower", $expectedInference.ttest_paired.confidence_lower, $actualInference.ttest_paired.confidence_lower, 1e-10),
        @("inference.ttest_paired.confidence_upper", $expectedInference.ttest_paired.confidence_upper, $actualInference.ttest_paired.confidence_upper, 1e-10),
        @("inference.ttest_paired.cohens_dz", $expectedInference.ttest_paired.cohens_dz, $actualInference.ttest_paired.cohens_dz, 1e-12),
        @("inference.wilcoxon.statistic", $expectedInference.wilcoxon_normal.statistic, $actualInference.wilcoxon.statistic, 1e-12),
        @("inference.wilcoxon.p_value", $expectedInference.wilcoxon_normal.p_value, $actualInference.wilcoxon.p_value, 1e-7),
        @("inference.wilcoxon.rank_biserial", $expectedInference.wilcoxon_normal.rank_biserial, $actualInference.wilcoxon.rank_biserial, 1e-12),
        @("inference.wilcoxon_continuity.statistic", $expectedInference.wilcoxon_continuity.statistic, $actualInference.wilcoxon_continuity.statistic, 1e-12),
        @("inference.wilcoxon_continuity.p_value", $expectedInference.wilcoxon_continuity.p_value, $actualInference.wilcoxon_continuity.p_value, 2e-7),
        @("inference.wilcoxon_exact.statistic", $expectedInference.wilcoxon_r_default.statistic, $actualInference.wilcoxon_exact.statistic, 1e-12),
        @("inference.wilcoxon_exact.p_value", $expectedInference.wilcoxon_r_default.p_value, $actualInference.wilcoxon_exact.p_value, 1e-12),
        @("convention.wilcoxon.r_default_p_value", $expectedInference.wilcoxon_r_default.p_value, $actualInference.wilcoxon.p_value, 1e-12, "expected_convention_difference"),
        @("edge.wilcoxon_ties.statistic", $expectedInference.wilcoxon_ties.statistic, $actualInference.wilcoxon_ties.statistic, 1e-12),
        @("edge.wilcoxon_ties.p_value", $expectedInference.wilcoxon_ties.p_value, $actualInference.wilcoxon_ties.p_value, 1e-7),
        @("inference.wilcoxon_paired_normal.statistic", $expectedInference.wilcoxon_paired_normal.statistic, $actualInference.wilcoxon_paired_normal.statistic, 1e-12),
        @("inference.wilcoxon_paired_normal.p_value", $expectedInference.wilcoxon_paired_normal.p_value, $actualInference.wilcoxon_paired_normal.p_value, 1e-7),
        @("inference.wilcoxon_paired_normal.rank_biserial", $expectedInference.wilcoxon_paired_normal.rank_biserial, $actualInference.wilcoxon_paired_normal.rank_biserial, 1e-12),
        @("inference.wilcoxon_paired_continuity.statistic", $expectedInference.wilcoxon_paired_continuity.statistic, $actualInference.wilcoxon_paired_continuity.statistic, 1e-12),
        @("inference.wilcoxon_paired_continuity.p_value", $expectedInference.wilcoxon_paired_continuity.p_value, $actualInference.wilcoxon_paired_continuity.p_value, 2e-7),
        @("inference.wilcoxon_paired_exact.statistic", $expectedInference.wilcoxon_paired_exact.statistic, $actualInference.wilcoxon_paired_exact.statistic, 1e-12),
        @("inference.wilcoxon_paired_exact.p_value", $expectedInference.wilcoxon_paired_exact.p_value, $actualInference.wilcoxon_paired_exact.p_value, 1e-12),
        @("inference.anova.f_statistic", $expectedInference.anova.f_statistic, $actualInference.anova.f_statistic, 1e-12),
        @("inference.anova.p_value", $expectedInference.anova.p_value, $actualInference.anova.p_value, 1e-10),
        @("inference.anova.df_between", $expectedInference.anova.df_between, $actualInference.anova.df_between, 0),
        @("inference.anova.df_within", $expectedInference.anova.df_within, $actualInference.anova.df_within, 0),
        @("inference.anova_welch.f_statistic", $expectedInference.anova_welch.f_statistic, $actualInference.anova_welch.f_statistic, 1e-11),
        @("inference.anova_welch.p_value", $expectedInference.anova_welch.p_value, $actualInference.anova_welch.p_value, 1e-10),
        @("inference.anova_welch.df_between", $expectedInference.anova_welch.df_between, $actualInference.anova_welch.df_between, 1e-12),
        @("inference.anova_welch.df_within", $expectedInference.anova_welch.df_within, $actualInference.anova_welch.df_within, 1e-11),
        @("inference.anova_welch.ss_between", $expectedInference.anova_welch.ss_between, $actualInference.anova_welch.ss_between, 1e-12),
        @("inference.anova_welch.ss_within", $expectedInference.anova_welch.ss_within, $actualInference.anova_welch.ss_within, 1e-12),
        @("inference.anova_welch.ss_total", $expectedInference.anova_welch.ss_total, $actualInference.anova_welch.ss_total, 1e-12),
        @("inference.anova_welch.eta_squared", $expectedInference.anova_welch.eta_squared, $actualInference.anova_welch.eta_squared, 1e-12),
        @("inference.anova_welch.omega_squared", $expectedInference.anova_welch.omega_squared, $actualInference.anova_welch.omega_squared, 1e-12),
        @("inference.kruskal_wallis.h_statistic", $expectedInference.kruskal_wallis.h_statistic, $actualInference.kruskal_wallis.h_statistic, 1e-12),
        @("inference.kruskal_wallis.p_value", $expectedInference.kruskal_wallis.p_value, $actualInference.kruskal_wallis.p_value, 1e-10),
        @("inference.kruskal_wallis.df", $expectedInference.kruskal_wallis.df, $actualInference.kruskal_wallis.df, 0),
        @("inference.kruskal_wallis.epsilon_squared", $expectedInference.kruskal_wallis.epsilon_squared, $actualInference.kruskal_wallis.epsilon_squared, 1e-12),
        @("inference.tukey_hsd.critical_value", $expectedInference.tukey_hsd.critical_value, $actualInference.tukey_hsd.critical_value, 2e-5),
        @("inference.tukey_hsd.mean_square_within", $expectedInference.tukey_hsd.mean_square_within, $actualInference.tukey_hsd.mean_square_within, 1e-12),
        @("inference.fisher.p_value", $expectedInference.fisher.p_value, $actualInference.fisher.p_value, 1e-12),
        @("inference.fisher.sample_odds_ratio", $expectedInference.fisher.sample_odds_ratio, $actualInference.fisher.odds_ratio, 1e-12),
        # R qnorm and BioLang's independent inverse-normal approximation differ
        # slightly before exponentiation widens the upper odds-ratio endpoint.
        @("inference.fisher.wald_lower", $expectedInference.fisher.wald_lower, $actualInference.fisher.confidence_lower, 5e-9),
        @("inference.fisher.wald_upper", $expectedInference.fisher.wald_upper, $actualInference.fisher.confidence_upper, 5e-9),
        @("convention.fisher.r_conditional_odds_ratio", $expectedInference.fisher.r_conditional_odds_ratio, $actualInference.fisher.odds_ratio, 1e-12, "expected_convention_difference"),
        @("inference.chi_square.statistic", $expectedInference.chi_square.statistic, $actualInference.chi_square.statistic, 1e-12),
        @("inference.chi_square.p_value", $expectedInference.chi_square.p_value, $actualInference.chi_square.p_value, 1e-12),
        @("inference.chi_square.df", $expectedInference.chi_square.df, $actualInference.chi_square.df, 0),
        @("inference.correlation.pearson", $expectedInference.correlation.pearson, $actualInference.correlation.pearson, 1e-12)
    )
    foreach ($field in @("mean_differences", "p_values", "confidence_lower", "confidence_upper")) {
        $referenceValues = @($expectedInference.tukey_hsd.$field)
        $observedValues = @($actualInference.tukey_hsd.$field)
        for ($index = 0; $index -lt $referenceValues.Count; $index++) {
            $tolerance = if ($field -eq "mean_differences") { 1e-12 } else { 2e-5 }
            $checks += ,@("inference.tukey_hsd.$field.$index", $referenceValues[$index], $observedValues[$index], $tolerance)
        }
    }
    foreach ($field in @("raw_p_values", "adjusted_p_values")) {
        $referenceValues = @($expectedInference.pairwise_welch_holm.$field)
        $observedValues = @($actualInference.pairwise_welch_holm.$field)
        for ($index = 0; $index -lt $referenceValues.Count; $index++) {
            $checks += ,@("inference.pairwise_welch_holm.$field.$index", $referenceValues[$index], $observedValues[$index], 1e-10)
        }
    }
    foreach ($method in @("bh", "bonferroni", "holm")) {
        $referenceAdjusted = @($expectedInference.p_adjust.$method)
        $observedAdjusted = @($actualInference.p_adjust.$method)
        for ($index = 0; $index -lt $referenceAdjusted.Count; $index++) {
            $checks += ,@("inference.p_adjust.$method.$index", $referenceAdjusted[$index], $observedAdjusted[$index], 1e-12)
        }
        $referenceBoundary = @($expectedInference.p_adjust_boundary.$method)
        $observedBoundary = @($actualInference.p_adjust_boundary.$method)
        for ($index = 0; $index -lt $referenceBoundary.Count; $index++) {
            $checks += ,@("edge.p_adjust_boundary.$method.$index", $referenceBoundary[$index], $observedBoundary[$index], 1e-12)
        }
    }
    foreach ($fixture in @("edge_right", "edge_left", "edge_right_open", "edge_left_open", "airquality")) {
        Compare-Geometry "plot.histogram.$fixture" $expectedPlot.$fixture $actualPlot.$fixture `
            @("left", "right", "counts", "density") @("left", "right", "counts") 1e-12
    }
    foreach ($fixture in @("box_type7", "box_tukey", "air_box_type7", "air_box_tukey")) {
        Compare-Geometry "plot.boxplot.$fixture" $expectedPlot.$fixture $actualPlot.$fixture `
            @("summary", "outliers") @() 1e-12
    }
    foreach ($fixture in @("ecdf", "air_ecdf")) {
        Compare-Geometry "plot.$fixture" $expectedPlot.$fixture $actualPlot.$fixture `
            @("x", "counts", "cumulative", "fraction") @("counts", "cumulative") 1e-12
    }
    foreach ($fixture in @("normal_qq", "air_normal_qq")) {
        Compare-Geometry "plot.$fixture" $expectedPlot.$fixture $actualPlot.$fixture `
            @("theoretical", "sample", "line") @() 1e-9
    }
    foreach ($fixture in @("violin", "air_violin")) {
        Compare-Geometry "plot.$fixture" $expectedPlot.$fixture $actualPlot.$fixture `
            @("bandwidth", "x", "density", "scaled") @() 1e-10
    }
    Compare-Geometry "plot.linear_fit_air" $expectedPlot.linear_fit_air $actualPlot.linear_fit_air `
        @("slope", "intercept", "residual_mse", "x", "fitted", "confidence_lower", "confidence_upper", "prediction_lower", "prediction_upper") @() 2e-9
    Compare-Geometry "plot.clinical_survival" $expectedPlot.clinical_survival $actualPlot.clinical_survival `
        @("time", "n_risk", "n_event", "n_censor", "survival", "std_error") @("time", "n_risk", "n_event", "n_censor") 1e-12
    $checks += ,@("plot.clinical_roc.auc", $expectedPlot.clinical_roc.auc, $actualPlot.clinical_roc.auc, 1e-12)
    Compare-Geometry "plot.clinical_forest" $expectedPlot.clinical_forest $actualPlot.clinical_forest `
        @("estimate", "lower", "upper", "weight") @() 1e-12
    Compare-Geometry "plot.genomic_manhattan" $expectedPlot.genomic_manhattan $actualPlot.genomic_manhattan `
        @("chromosome_index", "offset", "genome_position", "neg_log10_p", "significant") @("chromosome_index", "significant") 1e-12
    Compare-Geometry "plot.genetic_qq" $expectedPlot.genetic_qq $actualPlot.genetic_qq `
        @("rank", "p_value", "expected_p", "expected_neg_log10_p", "observed_neg_log10_p", "envelope_lower", "envelope_upper", "lambda_gc") @("rank") 2e-9
    Compare-Geometry "plot.genomic_rainfall" $expectedPlot.genomic_rainfall $actualPlot.genomic_rainfall `
        @("source_row", "position", "previous_position", "distance", "plotted_distance", "log10_distance", "duplicate_position") @("source_row", "duplicate_position") 1e-12
    Compare-Geometry "plot.genomic_ideogram" $expectedPlot.genomic_ideogram $actualPlot.genomic_ideogram `
        @("chromosome_length", "source_row", "chromosome_index", "start", "end", "length") @("source_row", "chromosome_index") 1e-12
    Compare-Geometry "plot.genomic_cnv" $expectedPlot.genomic_cnv $actualPlot.genomic_cnv `
        @("chromosome_offset", "source_row", "chromosome_index", "start", "end", "genome_start", "genome_end", "genome_midpoint", "log2ratio", "state") @("source_row", "chromosome_index", "state") 1e-12
    Compare-Geometry "plot.genomic_coverage" $expectedPlot.genomic_coverage $actualPlot.genomic_coverage `
        @("source_row", "original_start", "original_end", "start", "end", "position", "value", "clipped") @("source_row", "clipped") 1e-12
    Compare-Geometry "plot.regional_genome" $expectedPlot.regional_genome $actualPlot.regional_genome `
        @("source_row", "original_start", "original_end", "start", "end", "length", "lane", "clipped") @("source_row", "lane", "clipped") 1e-12
    Compare-Geometry "plot.regional_lollipop" $expectedPlot.regional_lollipop $actualPlot.regional_lollipop `
        @("source_row", "position", "height", "domain", "y_max") @("source_row") 1e-12
    Compare-Geometry "plot.regional_sashimi" $expectedPlot.regional_sashimi $actualPlot.regional_sashimi `
        @("coverage_source_row", "coverage_position", "coverage_depth", "junction_source_row", "junction_start", "junction_end", "junction_span", "junction_count", "junction_lane", "arc_fraction", "stroke_width", "max_count", "max_depth") @("coverage_source_row", "junction_source_row", "junction_lane") 1e-12
    Compare-Geometry "plot.circular_circos" $expectedPlot.circular_circos $actualPlot.circular_circos @(
        "segment_chromosome_index", "segment_source_row", "segment_start", "segment_end",
        "segment_size", "segment_angle_start", "segment_angle_end",
        "track_index", "track_point_index", "track_source_row", "track_chromosome_index",
        "track_start", "track_end", "track_value", "track_angle_start", "track_angle_end",
        "track_radial_inner", "track_radial_outer",
        "link_index", "link_source_row", "link_source_chromosome_index", "link_source_start",
        "link_source_end", "link_target_chromosome_index", "link_target_start", "link_target_end",
        "link_source_angle_start", "link_source_angle_end", "link_target_angle_start",
        "link_target_angle_end", "link_weight", "link_stroke_width"
    ) @(
        "segment_chromosome_index", "segment_source_row", "track_index", "track_point_index",
        "track_source_row", "track_chromosome_index", "link_index", "link_source_row",
        "link_source_chromosome_index", "link_target_chromosome_index"
    ) 1e-12
    if ($expectedNumpyPlot) {
        foreach ($fixture in @("edge_left", "airquality_left")) {
            foreach ($field in @("left", "right", "counts", "density")) {
                $referenceValues = @($expectedNumpyPlot.$fixture.$field)
                $observedValues = @($actualPlot.$fixture.$field)
                if ($referenceValues.Count -ne $observedValues.Count) {
                    throw "NumPy histogram geometry length mismatch for $fixture.$field"
                }
                for ($index = 0; $index -lt $referenceValues.Count; $index++) {
                    $tolerance = if ($field -eq "density") { 1e-12 } else { 0 }
                    $checks += ,@("plot.numpy_histogram.$fixture.$field.$index", $referenceValues[$index], $observedValues[$index], $tolerance)
                }
            }
        }
        foreach ($fixture in @("box_type7", "air_box_type7")) {
            foreach ($field in @("summary", "outliers")) {
                $referenceValues = @($expectedNumpyPlot.$fixture.$field)
                $observedValues = @($actualPlot.$fixture.$field)
                if ($referenceValues.Count -ne $observedValues.Count) {
                    throw "NumPy box geometry length mismatch for $fixture.$field"
                }
                for ($index = 0; $index -lt $referenceValues.Count; $index++) {
                    $checks += ,@("plot.numpy_boxplot.$fixture.$field.$index", $referenceValues[$index], $observedValues[$index], 1e-12)
                }
            }
        }
        foreach ($fixture in @("ecdf", "normal_qq", "violin", "air_ecdf", "air_normal_qq", "air_violin")) {
            $fields = switch ($fixture) {
                { $_ -in @("ecdf", "air_ecdf") } { @("x", "counts", "cumulative", "fraction"); break }
                { $_ -in @("normal_qq", "air_normal_qq") } { @("theoretical", "sample", "line"); break }
                { $_ -in @("violin", "air_violin") } { @("bandwidth", "x", "density", "scaled"); break }
            }
            foreach ($field in $fields) {
                $referenceValues = @($expectedNumpyPlot.$fixture.$field)
                $observedValues = @($actualPlot.$fixture.$field)
                if ($referenceValues.Count -ne $observedValues.Count) {
                    throw "NumPy $fixture geometry length mismatch for $field"
                }
                for ($index = 0; $index -lt $referenceValues.Count; $index++) {
                    $tolerance = if ($field -in @("counts", "cumulative")) { 0 } else { 1e-9 }
                    $checks += ,@("plot.numpy_$fixture.$field.$index", $referenceValues[$index], $observedValues[$index], $tolerance)
                }
            }
        }
        foreach ($field in @("slope", "intercept", "residual_mse", "x", "fitted", "confidence_lower", "confidence_upper", "prediction_lower", "prediction_upper")) {
            $referenceValues = @($expectedNumpyPlot.linear_fit_air.$field)
            $observedValues = @($actualPlot.linear_fit_air.$field)
            if ($referenceValues.Count -ne $observedValues.Count) {
                throw "statsmodels linear-fit geometry length mismatch for $field"
            }
            for ($index = 0; $index -lt $referenceValues.Count; $index++) {
                $checks += ,@("plot.statsmodels_linear_fit.$field.$index", $referenceValues[$index], $observedValues[$index], 2e-9)
            }
        }
    }

    $failures = @()
    $outcomes = foreach ($check in $checks) {
        $name, $reference, $observed, $tolerance = $check[0..3]
        $declaredClassification = if ($check.Count -ge 5) { [string]$check[4] } else { "" }
        $scale = [Math]::Max(1.0, [Math]::Abs([double]$reference))
        $difference = [Math]::Abs([double]$observed - [double]$reference)
        $absoluteLimit = [double]$tolerance * $scale
        $numericallyEquivalent = $difference -le $absoluteLimit
        $referenceMagnitude = [Math]::Abs([double]$reference)
        # Relative error is undefined near zero. Keep the absolute error and
        # tolerance there instead of manufacturing an alarming percentage by
        # dividing floating-point noise by an arbitrary tiny denominator.
        $relativeDifference = if ($referenceMagnitude -gt 1e-12) {
            $difference / $referenceMagnitude
        }
        else {
            $null
        }

        if ($declaredClassification -eq "expected_convention_difference") {
            $passed = -not $numericallyEquivalent
            $classification = if ($passed) {
                "expected_convention_difference"
            }
            else {
                "convention_changed_review_required"
            }
        }
        else {
            $passed = $numericallyEquivalent
            $classification = if ($passed) { "numerically_equivalent" } else { "biolang_mismatch" }
        }
        if (-not $passed) { $failures += $name }
        [ordered]@{
            metric = $name
            reference = [double]$reference
            biolang = [double]$observed
            absolute_difference = $difference
            relative_difference = $relativeDifference
            tolerance_scale = "max(1, abs(reference))"
            tolerance = [double]$tolerance
            absolute_tolerance = $absoluteLimit
            classification = $classification
            passed = $passed
        }
    }
    $expectedTotals = @($expected.matrix.sample_totals)
    $actualTotals = @($actual.matrix.sample_totals)
    $sampleTotalsMatch = $expectedTotals.Count -eq $actualTotals.Count
    if ($sampleTotalsMatch) {
        for ($index = 0; $index -lt $expectedTotals.Count; $index++) {
            $referenceTotal = [double]$expectedTotals[$index]
            $observedTotal = [double]$actualTotals[$index]
            $scale = [Math]::Max(1.0, [Math]::Abs($referenceTotal))
            if ([Math]::Abs($observedTotal - $referenceTotal) -gt (1e-12 * $scale)) {
                $sampleTotalsMatch = $false
                break
            }
        }
    }
    if (-not $sampleTotalsMatch) {
        $failures += "matrix.sample_totals"
    }

    $accuracyRows = @($outcomes | Where-Object { $_.classification -eq "numerically_equivalent" })
    $accuracyGroups = @("all") + @(
        $accuracyRows |
            ForEach-Object { ([string]$_.metric -split '\.')[0] } |
            Sort-Object -Unique
    )
    $accuracySummary = foreach ($groupName in $accuracyGroups) {
        $rows = if ($groupName -eq "all") {
            $accuracyRows
        }
        else {
            @($accuracyRows | Where-Object { ([string]$_.metric -split '\.')[0] -eq $groupName })
        }
        if ($rows.Count -eq 0) { continue }

        $referenceMean = ($rows | ForEach-Object { [double]$_.reference } | Measure-Object -Average).Average
        $observedMean = ($rows | ForEach-Object { [double]$_.biolang } | Measure-Object -Average).Average
        $sumCross = 0.0
        $sumReferenceSquares = 0.0
        $sumSquaredError = 0.0
        $relativeErrors = [System.Collections.Generic.List[double]]::new()
        foreach ($row in $rows) {
            $referenceValue = [double]$row.reference
            $observedValue = [double]$row.biolang
            $centeredReference = $referenceValue - $referenceMean
            $sumCross += $centeredReference * ($observedValue - $observedMean)
            $sumReferenceSquares += $centeredReference * $centeredReference
            $sumSquaredError += ($observedValue - $referenceValue) * ($observedValue - $referenceValue)
            if ($null -ne $row.relative_difference) {
                $relativeErrors.Add([double]$row.relative_difference)
            }
        }
        $slope = if ($sumReferenceSquares -gt 0.0) { $sumCross / $sumReferenceSquares } else { $null }
        $intercept = if ($null -ne $slope) { $observedMean - $slope * $referenceMean } else { $null }
        [ordered]@{
            group = $groupName
            metrics = $rows.Count
            regression_slope = $slope
            regression_intercept = $intercept
            rmse = [Math]::Sqrt($sumSquaredError / $rows.Count)
            median_relative_error = Get-Percentile $relativeErrors.ToArray() 0.5
            p95_relative_error = Get-Percentile $relativeErrors.ToArray() 0.95
            maximum_relative_error = if ($relativeErrors.Count -gt 0) {
                ($relativeErrors | Measure-Object -Maximum).Maximum
            }
            else {
                $null
            }
        }
    }

    $rElapsedSeconds = Get-Percentile @($rSuites | ForEach-Object { [double]$_.elapsed_seconds }) 0.5
    $blElapsedSeconds = Get-Percentile @($blSuites | ForEach-Object { [double]$_.elapsed_seconds }) 0.5
    $rPeakBytes = ($rSuites | Measure-Object -Property peak_working_set_bytes -Maximum).Maximum
    $blPeakBytes = ($blSuites | Measure-Object -Property peak_working_set_bytes -Maximum).Maximum
    $classificationCounts = [ordered]@{}
    foreach ($outcome in $outcomes) {
        $classificationName = [string]$outcome.classification
        if (-not $classificationCounts.Contains($classificationName)) {
            $classificationCounts[$classificationName] = 0
        }
        $classificationCounts[$classificationName]++
    }

    $manifest = [ordered]@{
        schema = "biolang.statistics.external-validation/v2"
        generated_utc = [DateTime]::UtcNow.ToString("o")
        r_version = (& $rscriptExe --version 2>&1 | Out-String).Trim()
        biolang_version = (& $BioLangExe --version 2>&1 | Out-String).Trim()
        numpy_version = if ($pythonExe) { (& $pythonExe -c "import numpy; print(numpy.__version__)" | Out-String).Trim() } else { $null }
        statsmodels_version = if ($pythonExe) { (& $pythonExe -c "import statsmodels; print(statsmodels.__version__)" | Out-String).Trim() } else { $null }
        measurement = [ordered]@{
            scope = "Three fresh R and BioLang processes; one supplemental NumPy/statsmodels plot-reference process when available"
            memory_metric = "Maximum aggregate working set of same-named backend processes started in the invocation window"
            repetitions = $BenchmarkRepeats
            elapsed_summary = "median of repetitions"
            memory_summary = "maximum sampled peak across repetitions"
            r_elapsed_seconds = [Math]::Round($rElapsedSeconds, 6)
            biolang_elapsed_seconds = [Math]::Round($blElapsedSeconds, 6)
            r_peak_working_set_bytes = [long]$rPeakBytes
            biolang_peak_working_set_bytes = [long]$blPeakBytes
            r_repetitions = $rSuites
            biolang_repetitions = $blSuites
            numpy_plot_reference = $numpyPlotRun
        }
        metrics = $outcomes
        classification_counts = $classificationCounts
        scale_sensitive_accuracy = $accuracySummary
        relative_error_denominator = "abs(reference); omitted when abs(reference) <= 1e-12"
        sample_totals_match = $sampleTotalsMatch
        passed = ($failures.Count -eq 0)
        failures = $failures
    }
    $manifest | ConvertTo-Json -Depth 8 | Set-Content "packages/statistics/validation/results/manifest.json" -Encoding UTF8

    $outcomes | ForEach-Object { [pscustomobject]$_ } |
        Export-Csv "packages/statistics/validation/results/checks.csv" -NoTypeInformation -Encoding UTF8

    $statusText = if ($manifest.passed) { "PASS" } else { "FAIL" }
    $report = @(
        "# BioLang statistics external validation",
        "",
        "**Status:** $statusText",
        "",
        "The R and BioLang programs run independently on the same deterministic inputs. R is a development oracle only; it is not linked or bundled with BioLang.",
        "",
        "## Result summary",
        "",
        "| Item | Result |",
        "|---|---:|",
        "| Numeric metrics compared | $($outcomes.Count) |",
        "| Numerically equivalent | $($classificationCounts['numerically_equivalent']) |",
        "| Expected convention differences | $($classificationCounts['expected_convention_difference']) |",
        "| Failures requiring review | $($failures.Count) |",
        "| Matrix sample totals | $(if ($sampleTotalsMatch) { 'match' } else { 'mismatch' }) |",
        "",
        "## Runtime and memory",
        "",
        "| Backend | Median elapsed seconds | Maximum peak working set (MiB) |",
        "|---|---:|---:|",
        ("| R | {0:N3} | {1:N1} |" -f $rElapsedSeconds, ($rPeakBytes / 1MB)),
        ("| BioLang | {0:N3} | {1:N1} |" -f $blElapsedSeconds, ($blPeakBytes / 1MB)),
        "",
        "Each backend is run $BenchmarkRepeats times. Elapsed time is the median full-suite time; memory is the maximum sampled peak. The full suite starts three fresh processes per backend, and R's launcher and worker are included. The supplemental NumPy oracle runs once and is excluded from backend timing.",
        "",
        "## Scale-sensitive accuracy",
        "",
        "Correlation is intentionally not used as a parity gate. Slope, intercept, RMSE, and relative-error percentiles expose proportional or offset bias.",
        "",
        "| Group | Metrics | Slope | Intercept | RMSE | Median relative error | P95 relative error |",
        "|---|---:|---:|---:|---:|---:|---:|"
    )
    foreach ($summary in $accuracySummary) {
        $slopeText = if ($null -eq $summary.regression_slope) { "n/a" } else { "{0:G6}" -f $summary.regression_slope }
        $interceptText = if ($null -eq $summary.regression_intercept) { "n/a" } else { "{0:G6}" -f $summary.regression_intercept }
        $report += "| $($summary.group) | $($summary.metrics) | $slopeText | $interceptText | $('{0:G6}' -f $summary.rmse) | $('{0:P3}' -f $summary.median_relative_error) | $('{0:P3}' -f $summary.p95_relative_error) |"
    }
    $report += @(
        "",
        "## Interpretation",
        "",
        "- ``numerically_equivalent``: within the metric's declared scale-aware tolerance.",
        "- ``expected_convention_difference``: deliberately differs from R's default, while a separately configured R calculation matches BioLang.",
        "- ``biolang_mismatch``: outside tolerance and requires investigation; it is not relabelled as an expected difference automatically.",
        "",
        "Machine-readable details are in `manifest.json`; one row per comparison is in `checks.csv`."
    )
    if ($failures.Count -gt 0) {
        $report += @("", "## Failures", "")
        foreach ($failure in $failures) { $report += "- $failure" }
    }
    $report | Set-Content "packages/statistics/validation/results/report.md" -Encoding UTF8

    $manifest | ConvertTo-Json -Depth 8
    if ($failures.Count -gt 0) { exit 1 }
}
finally {
    Pop-Location
}
