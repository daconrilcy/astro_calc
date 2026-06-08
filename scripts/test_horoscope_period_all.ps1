<#
.SYNOPSIS
    Lance la suite complete de validation des services horoscope period.
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
        Invoke-Step "Horoscope Period: time window tests" {
            cargo test -p astral_time_window
            if ($LASTEXITCODE -ne 0) { throw "astral_time_window tests failed" }
        }
        Invoke-Step "Horoscope Period: Rust service tests" {
            cargo test -p astral_llm_api --test horoscope_v1_tests
            if ($LASTEXITCODE -ne 0) { throw "horoscope_v1_tests failed" }
        }
        Invoke-Step "Horoscope Period: integration services tests" {
            cargo test -p astral_llm_api --test integration_services_tests
            if ($LASTEXITCODE -ne 0) { throw "integration_services_tests failed" }
        }
        Invoke-Step "Horoscope Period: integration jobs tests" {
            cargo test -p astral_llm_api --test integration_jobs_tests
            if ($LASTEXITCODE -ne 0) { throw "integration_jobs_tests failed" }
        }
        Invoke-Step "Horoscope Period: published contracts tests" {
            cargo test -p astral_llm_api --test contracts_publish_tests
            if ($LASTEXITCODE -ne 0) { throw "contracts_publish_tests failed" }
        }
    }

    if (-not $SkipFakeSmoke) {
        Invoke-Step "Horoscope Period: free next 7 days fake smoke" {
            & (Join-Path $repoRoot "scripts\test_horoscope_free_next_7_days_fake.ps1") `
                -BaseUrl $BaseUrl `
                -CalculatorUrl $CalculatorUrl
        }
        Invoke-Step "Horoscope Period: basic next 7 days fake smoke" {
            & (Join-Path $repoRoot "scripts\test_horoscope_basic_next_7_days_fake.ps1") `
                -BaseUrl $BaseUrl `
                -CalculatorUrl $CalculatorUrl
        }
        Invoke-Step "Horoscope Period: premium next 7 days fake smoke" {
            & (Join-Path $repoRoot "scripts\test_horoscope_premium_next_7_days_fake.ps1") `
                -BaseUrl $BaseUrl `
                -CalculatorUrl $CalculatorUrl
        }
    }

    if ($RunRealE2E) {
        Invoke-Step "Horoscope Period: real E2E" {
            & (Join-Path $repoRoot "scripts\test_horoscope_basic_next_7_days_real_e2e.ps1") `
                -BaseUrl $BaseUrl `
                -CalculatorUrl $CalculatorUrl
        }
        $premiumReal = Join-Path $repoRoot "scripts\test_horoscope_premium_next_7_days_real_e2e.ps1"
        if (Test-Path -LiteralPath $premiumReal) {
            Invoke-Step "Horoscope Premium Period: real E2E" {
                & $premiumReal -BaseUrl $BaseUrl -CalculatorUrl $CalculatorUrl
            }
        }
    }
} finally {
    Pop-Location
}
