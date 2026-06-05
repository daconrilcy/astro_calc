<#
.SYNOPSIS
    Suite E2E complete natal simplifie : calculateur + lectures + cas negatifs.

.DESCRIPTION
    Enchaine test_natal_simplified_calculator.ps1 puis test_natal_simplified_reading.ps1.
    Couvre les 6 niveaux input_precision, le cas equinoxe ambigu, et les erreurs 422.

.EXAMPLE
    .\scripts\test_natal_simplified_e2e.ps1

.EXAMPLE
    .\scripts\test_natal_simplified_e2e.ps1 -SkipReading

.EXAMPLE
    .\scripts\test_natal_simplified_e2e.ps1 -UseReal -SubmitProfile
#>
param(
    [string]$CalculatorBase = "",
    [string]$LlmBase = "",
    [string[]]$Case = @(),
    [switch]$SkipReading,
    [switch]$SkipCalculator,
    [switch]$UseReal,
    [switch]$SubmitProfile,
    [switch]$SaveOutputs,
    [switch]$Bootstrap,
    [int]$MinWordsPerChapter = 30,
    [int]$WaitReadySec = 120,
    [int]$TimeoutSec = 180
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

Write-Host "=== Natal simplifie — suite E2E ===" -ForegroundColor Cyan
Write-Host "Cas : matrice input_precision (6) + equinoxe + negatifs 422"
Write-Host ""

$commonArgs = @{
    Case = $Case
    SaveOutputs = $SaveOutputs
    WaitReadySec = $WaitReadySec
    TimeoutSec = $TimeoutSec
}
if ($Bootstrap) { $commonArgs.Bootstrap = $true }
if ($CalculatorBase) { $commonArgs.CalculatorBase = $CalculatorBase }
if ($LlmBase) { $commonArgs.LlmBase = $LlmBase }

if (-not $SkipCalculator) {
    Write-Host "--- Phase 1/2 : calculateur ---" -ForegroundColor Cyan
    & (Join-Path $PSScriptRoot "test_natal_simplified_calculator.ps1") @commonArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host ""
}

if (-not $SkipReading) {
    Write-Host "--- Phase 2/2 : lecture orchestrée ---" -ForegroundColor Cyan
    $readingArgs = @{
        Case = $Case
        SaveOutputs = $SaveOutputs
        WaitReadySec = $WaitReadySec
        TimeoutSec = $TimeoutSec
        MinWordsPerChapter = $MinWordsPerChapter
    }
    if ($LlmBase) { $readingArgs.LlmBase = $LlmBase }
    if ($UseReal) { $readingArgs.UseReal = $true }
    if ($SubmitProfile) { $readingArgs.SubmitProfile = $true }

    if ($Bootstrap) {
        Write-Host "Restart astral_llm_api (ASTRAL_CALCULATOR_HOST Docker)..." -ForegroundColor Cyan
        docker compose restart astral_llm_api | Out-Null
        Start-Sleep -Seconds 3
    }

    & (Join-Path $PSScriptRoot "test_natal_simplified_reading.ps1") @readingArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host ""
Write-Host "Suite E2E natal simplifie OK." -ForegroundColor Green
