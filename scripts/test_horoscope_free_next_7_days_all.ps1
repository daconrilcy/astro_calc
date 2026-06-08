<#
.SYNOPSIS
    Lance la suite de validation du service horoscope Free 7 prochains jours.
#>
param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [switch]$SkipRustChecks,
    [switch]$SkipFakeSmoke,
    [switch]$RunRealE2E
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

Push-Location $repoRoot
try {
    if (-not $SkipRustChecks) {
        Invoke-Step "horoscope_free_next_7_days_natal: time window tests" {
            cargo test -p astral_time_window
            if ($LASTEXITCODE -ne 0) { throw "astral_time_window tests failed" }
        }
        Invoke-Step "horoscope_free_next_7_days_natal: Rust service tests" {
            cargo test -p astral_llm_api --test horoscope_v1_tests horoscope_free_next_7_days
            if ($LASTEXITCODE -ne 0) { throw "horoscope_v1_tests free next 7 days subset failed" }
        }
        Invoke-Step "horoscope_free_next_7_days_natal: integration services tests" {
            cargo test -p astral_llm_api --test integration_services_tests
            if ($LASTEXITCODE -ne 0) { throw "integration_services_tests failed" }
        }
        Invoke-Step "horoscope_free_next_7_days_natal: integration jobs tests" {
            cargo test -p astral_llm_api --test integration_jobs_tests
            if ($LASTEXITCODE -ne 0) { throw "integration_jobs_tests failed" }
        }
        Invoke-Step "horoscope_free_next_7_days_natal: published contracts tests" {
            cargo test -p astral_llm_api --test contracts_publish_tests
            if ($LASTEXITCODE -ne 0) { throw "contracts_publish_tests failed" }
        }
        Invoke-Step "horoscope_free_next_7_days_natal: calculator API period tests" {
            cargo test -p astral_calculator_api --test astral_calculator_api_tests horoscope_period
            if ($LASTEXITCODE -ne 0) { throw "astral_calculator_api_tests horoscope_period failed" }
        }
    }

    if (-not $SkipFakeSmoke) {
        Invoke-Step "horoscope_free_next_7_days_natal: fake smoke" {
            & (Join-Path $repoRoot "scripts\test_horoscope_free_next_7_days_fake.ps1") `
                -BaseUrl $BaseUrl `
                -CalculatorUrl $CalculatorUrl
        }
    }

    if ($RunRealE2E) {
        $realScript = Join-Path $repoRoot "scripts\test_horoscope_free_next_7_days_real_e2e.ps1"
        if (-not (Test-Path -LiteralPath $realScript)) {
            throw "Missing optional real E2E script: $realScript"
        }
        Invoke-Step "horoscope_free_next_7_days_natal: real E2E" {
            & $realScript -BaseUrl $BaseUrl -CalculatorUrl $CalculatorUrl
        }
    }
} finally {
    Pop-Location
}
