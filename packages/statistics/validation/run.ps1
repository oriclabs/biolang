param(
    [string]$BioLangExe = "",
    [string]$RscriptPath = "",
    [switch]$RequireR
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$results = Join-Path $PSScriptRoot "results"
New-Item -ItemType Directory -Force $results | Out-Null

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

Push-Location $repoRoot
try {
    if (-not $BioLangExe) {
        $BioLangExe = Join-Path $repoRoot "target\debug\bl.exe"
    }
    if (-not (Test-Path -LiteralPath $BioLangExe)) {
        & cargo build -p bl-cli
        if ($LASTEXITCODE -ne 0) { throw "cargo build -p bl-cli failed" }
    }

    $rTimer = [System.Diagnostics.Stopwatch]::StartNew()
    & $rscriptExe "packages/statistics/validation/reference.R"
    if ($LASTEXITCODE -ne 0) { throw "R reference run failed" }
    $rTimer.Stop()

    $blTimer = [System.Diagnostics.Stopwatch]::StartNew()
    & $BioLangExe run "packages/statistics/validation/biolang_reference.bl"
    if ($LASTEXITCODE -ne 0) { throw "BioLang reference run failed" }
    $blTimer.Stop()

    $expected = Get-Content "packages/statistics/validation/results/r-reference.json" -Raw | ConvertFrom-Json
    $actual = Get-Content "packages/statistics/validation/results/biolang.json" -Raw | ConvertFrom-Json
    $expectedReal = Get-Content "packages/statistics/validation/results/r-real-reference.json" -Raw | ConvertFrom-Json
    $actualReal = Get-Content "packages/statistics/validation/results/biolang-real.json" -Raw | ConvertFrom-Json
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

    $failures = @()
    $outcomes = foreach ($check in $checks) {
        $name, $reference, $observed, $tolerance = $check
        $scale = [Math]::Max(1.0, [Math]::Abs([double]$reference))
        $difference = [Math]::Abs([double]$observed - [double]$reference)
        $passed = $difference -le ([double]$tolerance * $scale)
        if (-not $passed) { $failures += $name }
        [ordered]@{
            metric = $name
            reference = [double]$reference
            biolang = [double]$observed
            absolute_difference = $difference
            tolerance = [double]$tolerance
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

    $manifest = [ordered]@{
        schema = "biolang.statistics.external-validation/v1"
        generated_utc = [DateTime]::UtcNow.ToString("o")
        r_version = (& $rscriptExe --version 2>&1 | Out-String).Trim()
        biolang_version = (& $BioLangExe --version 2>&1 | Out-String).Trim()
        r_elapsed_seconds = [Math]::Round($rTimer.Elapsed.TotalSeconds, 6)
        biolang_elapsed_seconds = [Math]::Round($blTimer.Elapsed.TotalSeconds, 6)
        metrics = $outcomes
        sample_totals_match = $sampleTotalsMatch
        passed = ($failures.Count -eq 0)
        failures = $failures
    }
    $manifest | ConvertTo-Json -Depth 8 | Set-Content "packages/statistics/validation/results/manifest.json" -Encoding UTF8
    $manifest | ConvertTo-Json -Depth 8
    if ($failures.Count -gt 0) { exit 1 }
}
finally {
    Pop-Location
}
