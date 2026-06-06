<#
.SYNOPSIS
    Suite E2E complete natal simplifie : calculateur + lectures + cas negatifs.

.DESCRIPTION
    Enchaine test_natal_simplified_calculator.ps1 puis test_natal_simplified_reading.ps1.
    Couvre les 6 niveaux input_precision, le cas equinoxe ambigu, les erreurs 422 (calculateur seul)
    et les erreurs 400 (orchestration lecture sur entrees invalides).
    Par defaut, enregistre les reponses JSON dans output\natal_simplified\ (calculator\, reading\, e2e_summary.json).

.EXAMPLE
    .\scripts\test_natal_simplified_e2e.ps1

.EXAMPLE
    .\scripts\test_natal_simplified_e2e.ps1 -SkipReading

.EXAMPLE
    .\scripts\test_natal_simplified_e2e.ps1 -UseReal -SubmitProfile

.EXAMPLE
    .\scripts\test_natal_simplified_e2e.ps1 -NoSaveOutputs
#>
param(
    [string]$CalculatorBase = "",
    [string]$LlmBase = "",
    [string[]]$Case = @(),
    [switch]$SkipReading,
    [switch]$SkipCalculator,
    [switch]$UseReal,
    [switch]$ForceFake,
    [switch]$SubmitProfile,
    [switch]$NoSaveOutputs,
    [string]$OutputDir = "",
    [switch]$Bootstrap,
    [int]$MinWordsPerChapter = 30,
    [int]$WaitReadySec = 120,
    [int]$TimeoutSec = 180
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "lib\simplified_natal_assertions.ps1")

if ($UseReal) {
    $TimeoutSec = [Math]::Max($TimeoutSec, 900)
}

if ($UseReal -and [string]::IsNullOrWhiteSpace($OutputDir)) {
    $openAiRoot = Join-Path $repoRoot "output\natal_simplified_openai"
    $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHHmmssZ")
    $OutputDir = Join-Path $openAiRoot $ts
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\natal_simplified"
}
$calcOutputDir = Join-Path $OutputDir "calculator"
$readOutputDir = Join-Path $OutputDir "reading"
$saveOutputs = -not $NoSaveOutputs
if ($saveOutputs -or $UseReal) {
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
}
if ($saveOutputs) {
    New-Item -ItemType Directory -Force -Path $calcOutputDir, $readOutputDir | Out-Null
}

Write-Host "=== Natal simplifie - suite E2E ===" -ForegroundColor Cyan
Write-Host "Cas : matrice input_precision (6) + equinoxe + negatifs (422 calculateur, 400 orchestration)"
Write-Host ""

$commonArgs = @{
    Case = $Case
    WaitReadySec = $WaitReadySec
    TimeoutSec = $TimeoutSec
}
if ($saveOutputs) {
    $commonArgs.SaveOutputs = $true
    $commonArgs.OutputDir = $calcOutputDir
}
if ($Bootstrap) { $commonArgs.Bootstrap = $true }
if ($CalculatorBase) { $commonArgs.CalculatorBase = $CalculatorBase }
if ($LlmBase) { $commonArgs.LlmBase = $LlmBase }

if (-not $SkipCalculator) {
    Write-Host "--- Phase 1/2 : calculateur ---" -ForegroundColor Cyan
    & (Join-Path $PSScriptRoot "test_natal_simplified_calculator.ps1") @commonArgs
    if ($null -ne $LASTEXITCODE -and $LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host ""
}

if (-not $SkipReading) {
    Write-Host "--- Phase 2/2 : lecture orchestree ---" -ForegroundColor Cyan
    $readingArgs = @{
        Case = $Case
        WaitReadySec = $WaitReadySec
        TimeoutSec = $TimeoutSec
        MinWordsPerChapter = $MinWordsPerChapter
    }
    if ($saveOutputs) {
        $readingArgs.SaveOutputs = $true
        $readingArgs.OutputDir = $readOutputDir
    }
    if ($LlmBase) { $readingArgs.LlmBase = $LlmBase }
    $qualityMetricsPath = $null
    if ($UseReal) {
        $readingArgs.UseReal = $true
        $qualityMetricsPath = Join-Path $OutputDir "quality_metrics.raw.json"
        $readingArgs.QualityMetricsPath = $qualityMetricsPath
    } elseif ($ForceFake -or -not $UseReal) {
        $readingArgs.ForceFake = $true
    }
    if ($SubmitProfile -or ($readingArgs.ForceFake -and -not $UseReal) -or $UseReal) {
        $readingArgs.SubmitProfile = $true
    }

    if ($Bootstrap) {
        Write-Host "Restart astral_llm_api (ASTRAL_CALCULATOR_HOST Docker)..." -ForegroundColor Cyan
        docker compose restart astral_llm_api | Out-Null
        Start-Sleep -Seconds 3
    }

    & (Join-Path $PSScriptRoot "test_natal_simplified_reading.ps1") @readingArgs
    $readingExitCode = $LASTEXITCODE
    if ($UseReal -and $qualityMetricsPath -and (Test-Path -LiteralPath $qualityMetricsPath)) {
        $rawMetrics = Get-Content -LiteralPath $qualityMetricsPath -Raw | ConvertFrom-Json
        $qualityPath = Join-Path $OutputDir "quality_summary.json"
        Export-SimplifiedQualitySummary -Metrics $rawMetrics -OutputPath $qualityPath | Out-Null
    }
    if ($null -ne $readingExitCode -and $readingExitCode -ne 0) { exit $readingExitCode }

    Write-Host ""
    Write-Host "--- Phase 2b/2 : lecture orchestration (negatifs 400) ---" -ForegroundColor Cyan
    $negativeReadingArgs = @{
        Case = $Case
        WaitReadySec = $WaitReadySec
        TimeoutSec = $TimeoutSec
        NegativeOnly = $true
    }
    if ($saveOutputs) {
        $negativeReadingArgs.SaveOutputs = $true
        $negativeReadingArgs.OutputDir = $readOutputDir
    }
    if ($LlmBase) { $negativeReadingArgs.LlmBase = $LlmBase }
    if ($UseReal) {
        $negativeReadingArgs.UseReal = $true
    } elseif ($ForceFake -or -not $UseReal) {
        $negativeReadingArgs.ForceFake = $true
    }

    & (Join-Path $PSScriptRoot "test_natal_simplified_reading.ps1") @negativeReadingArgs
    if ($null -ne $LASTEXITCODE -and $LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host ""
if ($saveOutputs) {
    Write-Host "Artefacts JSON : $OutputDir" -ForegroundColor Cyan
    Write-Host "  calculator\  - reponses POST /v1/calculations/natal/simplified"
    Write-Host "  reading\     - reponses POST /v1/readings/natal/simplified"
    $summary = [ordered]@{
        generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        output_root      = $OutputDir
        calculator_dir   = $calcOutputDir
        reading_dir      = $readOutputDir
        skip_calculator  = [bool]$SkipCalculator
        skip_reading     = [bool]$SkipReading
        use_real         = [bool]$UseReal
        cases_filter     = @($Case)
    }
    if (-not $SkipCalculator -and (Test-Path -LiteralPath $calcOutputDir)) {
        $summary.calculator_files = @(Get-ChildItem -LiteralPath $calcOutputDir -Filter "*.json" | ForEach-Object { $_.Name })
    }
    if (-not $SkipReading -and (Test-Path -LiteralPath $readOutputDir)) {
        $summary.reading_files = @(Get-ChildItem -LiteralPath $readOutputDir -Filter "*.json" | ForEach-Object { $_.Name })
    }
    $summaryPath = Join-Path $OutputDir "e2e_summary.json"
    $summary | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $summaryPath -Encoding utf8
    Write-Host "  e2e_summary.json"
}
if ($UseReal -and $qualityMetricsPath) {
    $qualityPath = Join-Path $OutputDir "quality_summary.json"
    if (Test-Path -LiteralPath $qualityMetricsPath) {
        if (-not (Test-Path -LiteralPath $qualityPath)) {
            $rawMetrics = Get-Content -LiteralPath $qualityMetricsPath -Raw | ConvertFrom-Json
            Export-SimplifiedQualitySummary -Metrics $rawMetrics -OutputPath $qualityPath | Out-Null
        }
        if (-not $saveOutputs) {
            Write-Host "Artefacts qualite : $OutputDir" -ForegroundColor Cyan
        }
        Write-Host "  quality_metrics.raw.json"
        Write-Host "  quality_summary.json"
    } else {
        Write-Warning "quality_metrics.raw.json absent — quality_summary.json non genere"
    }
}
Write-Host "Suite E2E natal simplifie OK." -ForegroundColor Green
