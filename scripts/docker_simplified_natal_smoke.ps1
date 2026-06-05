#!/usr/bin/env pwsh
# Smoke rapide natal simplifie — delegue a la suite E2E (date_only + lecture fake).
param(
    [string]$CalculatorBase = "http://127.0.0.1:8080",
    [string]$LlmBase = "http://127.0.0.1:8081",
    [switch]$SkipReading
)

$ErrorActionPreference = "Stop"
$args = @{
    CalculatorBase = $CalculatorBase
    LlmBase          = $LlmBase
    Case             = @("date_only")
}
if ($SkipReading) {
    $args.SkipReading = $true
    & "$PSScriptRoot\test_natal_simplified_calculator.ps1" @args
} else {
    & "$PSScriptRoot\test_natal_simplified_e2e.ps1" @args
}
exit $LASTEXITCODE
