<#
.SYNOPSIS
    Met a jour le stack Docker Astral et lance les verifications d'integration.

.EXAMPLE
    .\scripts\docker_update_integration_stack.ps1

.EXAMPLE
    .\scripts\docker_update_integration_stack.ps1 -SkipBuild -SkipImport
#>
param(
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [string]$LlmUrl = "http://127.0.0.1:8081",
    [int]$ReadyTimeoutSec = 120,
    [switch]$SkipBuild,
    [switch]$SkipImport,
    [switch]$SkipCatalogueSubmit,
    [switch]$SkipSmoke,
    [switch]$RunRustChecks,
    [switch]$RunRealHoroscopePeriodE2E
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "lib\astral_http_auth.ps1")
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

function Wait-HttpReady {
    param(
        [string]$Url,
        [int]$TimeoutSec
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $lastError = $null
    while ((Get-Date) -lt $deadline) {
        try {
            $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 5 -SkipHttpErrorCheck
            if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 300) {
                return
            }
            $lastError = "HTTP $($response.StatusCode): $($response.Content)"
        } catch {
            $lastError = $_.Exception.Message
        }
        Start-Sleep -Seconds 2
    }
    throw "Readiness timeout for $Url. Last error: $lastError"
}

function Assert-RequiredEnv {
    foreach ($name in @("POSTGRES_DB", "POSTGRES_USER", "POSTGRES_PASSWORD", "DATABASE_URL")) {
        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) {
            throw "$name is required in .env"
        }
    }
}

Push-Location $repoRoot
try {
    Write-Host "== Docker integration stack update ==" -ForegroundColor Cyan
    Assert-RequiredEnv

    if (-not $SkipBuild) {
        Invoke-Step "Build and start containers" {
            docker compose up -d --build
            if ($LASTEXITCODE -ne 0) { throw "docker compose up -d --build failed" }
        }
    } else {
        Invoke-Step "Start existing containers" {
            docker compose up -d
            if ($LASTEXITCODE -ne 0) { throw "docker compose up -d failed" }
        }
    }

    Invoke-Step "PostgreSQL readiness" {
        docker compose exec -T postgres pg_isready -U $env:POSTGRES_USER -d $env:POSTGRES_DB | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "PostgreSQL is not ready" }
    }

    if (-not $SkipImport) {
        Invoke-Step "Import json_db into PostgreSQL" {
            python (Join-Path $repoRoot "scripts\import_json_db_to_postgres.py")
            if ($LASTEXITCODE -ne 0) { throw "import_json_db_to_postgres.py failed" }
        }
    }

    if (-not $SkipCatalogueSubmit) {
        Invoke-Step "Submit integration services catalogue" {
            & (Join-Path $repoRoot "scripts\manage_integration_services.ps1") -Submit
        }
    }

    Invoke-Step "Restart LLM API and worker" {
        docker compose restart astral_llm_api astral_llm_worker | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "docker compose restart failed" }
    }

    Invoke-Step "HTTP readiness" {
        Wait-HttpReady -Url "$CalculatorUrl/health/ready" -TimeoutSec $ReadyTimeoutSec
        Wait-HttpReady -Url "$LlmUrl/health/ready" -TimeoutSec $ReadyTimeoutSec
    }

    Invoke-Step "Public integration catalogue" {
        $services = Invoke-RestMethod -Uri "$LlmUrl/v1/services" -Method Get -TimeoutSec 10
        $active = @($services.services | Where-Object { $_.availability -eq "active" })
        if ($active.Count -lt 1) {
            throw "No active integration service returned by $LlmUrl/v1/services"
        }
        Write-Host ("Active services: {0}" -f (($active | ForEach-Object { $_.service_code }) -join ", "))
    }

    if ($RunRustChecks) {
        Invoke-Step "Rust checks: integration services" {
            cargo test -p astral_llm_api --test integration_services_tests
            if ($LASTEXITCODE -ne 0) { throw "integration_services_tests failed" }
        }
        Invoke-Step "Rust checks: integration jobs" {
            cargo test -p astral_llm_api --test integration_jobs_tests
            if ($LASTEXITCODE -ne 0) { throw "integration_jobs_tests failed" }
        }
        Invoke-Step "Rust checks: published contracts" {
            cargo test -p astral_llm_api --test contracts_publish_tests
            if ($LASTEXITCODE -ne 0) { throw "contracts_publish_tests failed" }
        }
    }

    if (-not $SkipSmoke) {
        Invoke-Step "Time window utility service smoke" {
            & (Join-Path $repoRoot "scripts\test_time_window_service.ps1")
        }
        Invoke-Step "Integration jobs E2E smoke" {
            & (Join-Path $repoRoot "scripts\test_integration_jobs_e2e.ps1") -LlmBase $LlmUrl
        }
        Invoke-Step "Horoscope free daily full test suite" {
            & (Join-Path $repoRoot "scripts\test_horoscope_free_daily_all.ps1") -BaseUrl $LlmUrl -CalculatorUrl $CalculatorUrl
        }
        Invoke-Step "Horoscope premium daily full test suite" {
            & (Join-Path $repoRoot "scripts\test_horoscope_premium_daily_all.ps1") -BaseUrl $LlmUrl -CalculatorUrl $CalculatorUrl
        }
        Invoke-Step "Horoscope period full test suite" {
            & (Join-Path $repoRoot "scripts\test_horoscope_period_all.ps1") -BaseUrl $LlmUrl -CalculatorUrl $CalculatorUrl
        }
        if ($RunRealHoroscopePeriodE2E) {
            Invoke-Step "Horoscope period real E2E" {
                & (Join-Path $repoRoot "scripts\test_horoscope_basic_next_7_days_real_e2e.ps1") -BaseUrl $LlmUrl -CalculatorUrl $CalculatorUrl
            }
        }
    }

    Write-Host "`nDocker integration stack is ready." -ForegroundColor Green
    Write-Host "Calculator: $CalculatorUrl"
    Write-Host "LLM API:    $LlmUrl"
    Write-Host "Mercure:    http://127.0.0.1:3000"
} finally {
    Pop-Location
}
