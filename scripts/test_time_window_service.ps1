<#
.SYNOPSIS
    Lance les validations du service utilitaire de fenetre temporelle.

.EXAMPLE
    .\scripts\test_time_window_service.ps1
#>
param(
    [switch]$SkipFullSuite
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Action
    )
    Write-Host "`n== $Name ==" -ForegroundColor Cyan
    & $Action
    Write-Host "OK: $Name" -ForegroundColor Green
}

function Invoke-TimeWindowTest {
    param(
        [string]$Name,
        [string]$Filter
    )
    Invoke-Step $Name {
        cargo test -p astral_time_window $Filter
        if ($LASTEXITCODE -ne 0) { throw "astral_time_window test failed: $Filter" }
    }
}

Push-Location $repoRoot
try {
    Invoke-TimeWindowTest `
        -Name "Time window: next_7_days" `
        -Filter "next_7_days_resolves_from_anchor_inclusive_to_end_exclusive"

    Invoke-TimeWindowTest `
        -Name "Time window: current workweek from Sunday" `
        -Filter "current_workweek_from_sunday_resolves_to_same_iso_week_monday_to_saturday"

    Invoke-TimeWindowTest `
        -Name "Time window: next workweek from Monday" `
        -Filter "next_workweek_from_monday_uses_following_monday"

    Invoke-TimeWindowTest `
        -Name "Time window: custom date range" `
        -Filter "custom_date_range_is_inclusive_input_normalized_to_exclusive_end"

    Invoke-TimeWindowTest `
        -Name "Time window: public schemas" `
        -Filter "request_and_response_examples_match_public_schemas"

    Invoke-TimeWindowTest `
        -Name "Time window: custom schema guard" `
        -Filter "custom_date_range_request_schema_requires_custom_dates"

    if (-not $SkipFullSuite) {
        Invoke-Step "Time window: full crate suite" {
            cargo test -p astral_time_window
            if ($LASTEXITCODE -ne 0) { throw "astral_time_window full test suite failed" }
        }
    }
} finally {
    Pop-Location
}
