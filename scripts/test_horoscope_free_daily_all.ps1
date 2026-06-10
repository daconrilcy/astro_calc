<#
.SYNOPSIS
    Lance la suite complete de validation du service horoscope_free_daily.

.EXAMPLE
    .\scripts\test_horoscope_free_daily_all.ps1

.EXAMPLE
    .\scripts\test_horoscope_free_daily_all.ps1 -BaseUrl http://127.0.0.1:8081 -CalculatorUrl http://127.0.0.1:8080
#>
param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [switch]$SkipRustChecks,
    [switch]$SkipSmoke
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
. "$PSScriptRoot\lib\astral_http_auth.ps1"
. "$PSScriptRoot\lib\horoscope_e2e_fake_provider.ps1"
Import-AstralDotEnv -RepoRoot $repoRoot

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
        Invoke-Step "Horoscope Free Daily: Rust service tests" {
            cargo test -p astral_llm_api --test horoscope_v1_tests
            if ($LASTEXITCODE -ne 0) { throw "horoscope_v1_tests failed" }
        }

        Invoke-Step "Horoscope Free Daily: integration services tests" {
            cargo test -p astral_llm_api --test integration_services_tests
            if ($LASTEXITCODE -ne 0) { throw "integration_services_tests failed" }
        }

        Invoke-Step "Horoscope Free Daily: integration jobs tests" {
            cargo test -p astral_llm_api --test integration_jobs_tests
            if ($LASTEXITCODE -ne 0) { throw "integration_jobs_tests failed" }
        }

        Invoke-Step "Horoscope Free Daily: published contracts tests" {
            cargo test -p astral_llm_api --test contracts_publish_tests
            if ($LASTEXITCODE -ne 0) { throw "contracts_publish_tests failed" }
        }
    }

    if (-not $SkipSmoke) {
        $horoscopeFakeProviderArmed = $false
        try {
            Enable-HoroscopeE2eFakeLlmProvider -RepoRoot $repoRoot
            $horoscopeFakeProviderArmed = $true

            Invoke-Step "Horoscope Basic Daily fake smoke non-regression" {
                & (Join-Path $repoRoot "scripts\test_horoscope_basic_daily_fake.ps1") `
                    -BaseUrl $BaseUrl `
                    -CalculatorUrl $CalculatorUrl
            }

            Invoke-Step "Horoscope Free Daily fake smoke" {
                & (Join-Path $repoRoot "scripts\test_horoscope_free_daily_fake.ps1") `
                    -BaseUrl $BaseUrl `
                    -CalculatorUrl $CalculatorUrl
            }
        } finally {
            if ($horoscopeFakeProviderArmed) {
                Restore-HoroscopeE2eLlmProvider -RepoRoot $repoRoot
            }
        }
    }
} finally {
    Pop-Location
}
