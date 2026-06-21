<#
.SYNOPSIS
    Met a jour le stack Docker Astral et lance les verifications refactorees.

.EXAMPLE
    .\scripts\docker_update_integration_stack.ps1

.EXAMPLE
    .\scripts\docker_update_integration_stack.ps1 -SkipBuild -SkipSmoke

.EXAMPLE
    .\scripts\docker_update_integration_stack.ps1 -LegacyCutover

.EXAMPLE
    .\scripts\docker_update_integration_stack.ps1 -SkipHostDbPort
#>
param(
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [string]$LlmUrl = "http://127.0.0.1:8081",
    [string]$GatewayUrl = "http://127.0.0.1:8082",
    [int]$ReadyTimeoutSec = 180,
    [switch]$SkipBuild,
    [switch]$SkipImport,
    [switch]$SkipLlmSync,
    [switch]$SkipCatalogueSubmit,
    [switch]$SkipSmoke,
    [switch]$SkipRustChecks,
    [switch]$SkipHostDbPort,
    [switch]$LegacyCutover
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "lib\astral_http_auth.ps1")
. (Join-Path $PSScriptRoot "lib\simplified_e2e_llm_provider.ps1")
. (Join-Path $PSScriptRoot "lib\horoscope_e2e_fake_provider.ps1")
. (Join-Path $PSScriptRoot "lib\sync_llm_catalog.ps1")
Import-AstralDotEnv -RepoRoot $repoRoot

$calculatorHeaders = New-AstralAuthHeaders -Service calculator
$llmHeaders = New-AstralAuthHeaders -Service llm
$executedSuites = [System.Collections.Generic.List[string]]::new()
$excludedSuites = [System.Collections.Generic.List[string]]::new()

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Action
    )

    Write-Host "`n== $Name ==" -ForegroundColor Cyan
    $started = Get-Date
    try {
        & $Action
    } finally {
        $elapsed = ((Get-Date) - $started).TotalSeconds
        Write-Host ("Duration: {0}s" -f [math]::Round($elapsed, 1)) -ForegroundColor DarkGray
    }
    Write-Host "OK: $Name" -ForegroundColor Green
}

function Assert-RequiredEnv {
    foreach ($name in @("POSTGRES_DB", "POSTGRES_USER", "POSTGRES_PASSWORD", "DATABASE_URL")) {
        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) {
            throw "$name is required in .env"
        }
    }
}

function Invoke-DockerCompose {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$ComposeArgs
    )

    $composeFiles = @("-f", "docker-compose.yml")
    $hostDbPortComposeFile = Join-Path $repoRoot "docker-compose.dev-db-port.yml"
    if (-not $SkipHostDbPort -and (Test-Path -LiteralPath $hostDbPortComposeFile)) {
        $composeFiles += @("-f", "docker-compose.dev-db-port.yml")
    }
    if ($LegacyCutover) {
        $composeFiles += @("-f", "docker-compose.legacy-cutover.yml")
    }

    & docker compose @composeFiles @ComposeArgs
}

function Invoke-DockerComposeChecked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Description,
        [Parameter(Mandatory = $true)]
        [string[]]$ComposeArgs
    )

    $started = Get-Date
    Invoke-DockerCompose -ComposeArgs $ComposeArgs
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed: docker compose $($ComposeArgs -join ' ')"
    }
    $elapsed = ((Get-Date) - $started).TotalSeconds
    $isDetachedNoBuildUp = $ComposeArgs[0] -eq "up" -and $ComposeArgs -contains "-d" -and $ComposeArgs -contains "--no-build"
    if ($elapsed -gt 90 -and $isDetachedNoBuildUp) {
        throw "$Description did not return in a reasonable delay ($([math]::Round($elapsed, 1))s). Verify docker compose is not attached unexpectedly."
    }
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
            $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 10 -SkipHttpErrorCheck
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

function Assert-RunningServices {
    param([string[]]$Services)

    $running = Invoke-DockerCompose -ComposeArgs @("ps", "--services", "--status", "running")
    if ($LASTEXITCODE -ne 0) {
        throw "docker compose ps failed"
    }
    $runningSet = @{}
    foreach ($line in ($running | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })) {
        $runningSet[$line.Trim()] = $true
    }
    foreach ($service in $Services) {
        if (-not $runningSet.ContainsKey($service)) {
            throw "Service not running: $service"
        }
    }
}

function Invoke-CommandChecked {
    param(
        [string]$Name,
        [scriptblock]$Action,
        [string]$FailureHint = ""
    )

    try {
        & $Action
        if ($LASTEXITCODE -ne 0) {
            throw "$Name failed"
        }
    } catch {
        if ($FailureHint) {
            throw "$Name failed. $FailureHint Error: $($_.Exception.Message)"
        }
        throw
    }
}

function Assert-HostDatabaseEndpoint {
    if ($SkipHostDbPort -or [string]::IsNullOrWhiteSpace($env:DATABASE_URL)) {
        return
    }

    $match = [regex]::Match($env:DATABASE_URL, '^[^:]+://[^@/]+@(?<host>[^:/]+)(:(?<port>\d+))?/')
    if (-not $match.Success) {
        return
    }

    $hostName = $match.Groups["host"].Value
    if ($hostName -notin @("localhost", "127.0.0.1", "::1")) {
        return
    }

    $port = 5432
    if ($match.Groups["port"].Success) {
        $port = [int]$match.Groups["port"].Value
    } elseif (-not [string]::IsNullOrWhiteSpace($env:POSTGRES_PORT)) {
        $port = [int]$env:POSTGRES_PORT
    }

    $client = [System.Net.Sockets.TcpClient]::new()
    try {
        $connect = $client.BeginConnect($hostName, $port, $null, $null)
        if (-not $connect.AsyncWaitHandle.WaitOne(3000)) {
            throw "PostgreSQL host endpoint is not reachable at ${hostName}:${port}. docker-compose.dev-db-port.yml must publish this port for host cargo tests."
        }
        $client.EndConnect($connect)
    } finally {
        $client.Close()
    }
}

function Invoke-RustTest {
    param(
        [string]$Name,
        [string[]]$CargoArgs
    )

    $cargoArgs = @($CargoArgs)
    Invoke-Step $Name {
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "Command failed: cargo $($cargoArgs -join ' ')"
        }
        $executedSuites.Add("cargo $($cargoArgs -join ' ')") | Out-Null
    }
}

function Clear-TmpTargetArtifacts {
    $tmpTargetPath = Join-Path $repoRoot "tmp_target"
    if (-not (Test-Path $tmpTargetPath)) {
        return
    }

    Write-Host "`n== Cleanup tmp_target ==" -ForegroundColor Cyan

    Get-ChildItem -LiteralPath $tmpTargetPath -Force | ForEach-Object {
        Remove-Item -LiteralPath $_.FullName -Recurse -Force
    }

    Write-Host "OK: Cleanup tmp_target" -ForegroundColor Green
}

function New-GatewayNatalRequestBody {
    param(
        [string]$AudienceLevel = "general",
        [bool]$IncludeBirthTime = $true
    )

    $body = [ordered]@{
        context = [ordered]@{
            request_id = "gateway-smoke"
            idempotency_key = [guid]::NewGuid().ToString()
            target_language_code = "fr"
            audience_level = $AudienceLevel
        }
        birth = [ordered]@{
            date = "1990-06-15"
            timezone = "Europe/Paris"
            location = [ordered]@{
                latitude = 48.8566
                longitude = 2.3522
                label = "Paris"
            }
        }
    }
    if ($IncludeBirthTime) {
        $body.birth.time = "14:30:00"
    }
    return $body
}

function Get-HoroscopeChartCalculationId {
    $body = [ordered]@{
        request_contract_version = "astro_engine_request_v1"
        request_id = "gateway-horoscope-chart"
        calculation = [ordered]@{
            type = "natal"
            zodiacal_reference_system = "tropical"
            coordinate_reference_system = "geocentric"
            house_system = "placidus"
        }
        birth = [ordered]@{
            date = "1990-06-15"
            time = "14:30:00"
            timezone = "Europe/Paris"
            location = [ordered]@{
                latitude = 48.8566
                longitude = 2.3522
                label = "Paris"
            }
        }
        projection = [ordered]@{
            level = "compact"
        }
    }

    $response = Invoke-RestMethod -Uri "$CalculatorUrl/v1/internal/calculations/natal" `
        -Method Post `
        -Headers $calculatorHeaders `
        -Body ($body | ConvertTo-Json -Depth 20) `
        -ContentType "application/json" `
        -TimeoutSec 60
    $chartCalculationId = [string]$response.calculation_result.chart_calculation_id
    if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
        throw "Calculator response missing chart_calculation_id"
    }
    return $chartCalculationId
}

function Invoke-GatewayJsonPost {
    param(
        [string]$Url,
        [hashtable]$Body
    )

    return Invoke-RestMethod -Uri $Url `
        -Method Post `
        -Body ($Body | ConvertTo-Json -Depth 30) `
        -ContentType "application/json" `
        -TimeoutSec 120
}

Push-Location $repoRoot
$scriptSucceeded = $false
try {
    $excludedSuites.Add("provider_real_smoke") | Out-Null
    $excludedSuites.Add("*_real_e2e.ps1") | Out-Null
    $excludedSuites.Add("generate_premium*_e2e.ps1") | Out-Null
    $excludedSuites.Add("test_natal_premium*_profile.ps1") | Out-Null
    $excludedSuites.Add("Any workflow requiring OPENAI_API_KEY or POST /v1/internal/readings/render") | Out-Null

    Write-Host "== Docker integration stack update ==" -ForegroundColor Cyan
    Assert-RequiredEnv

    if (-not $SkipBuild) {
        Invoke-Step "Build and start containers" {
            Invoke-DockerComposeChecked -Description "docker compose up --build" -ComposeArgs @("up", "-d", "--build")
        }
    } else {
        Invoke-Step "Start existing containers" {
            Invoke-DockerComposeChecked -Description "docker compose up --no-build" -ComposeArgs @("up", "-d", "--no-build")
        }
    }

    Invoke-Step "Running services check" {
        Assert-RunningServices -Services @(
            "postgres",
            "astral_calculator_http",
            "astral_llm_api",
            "astral_llm_worker",
            "astral_gateway",
            "mercure"
        )
    }

    Invoke-Step "PostgreSQL readiness" {
        Invoke-DockerComposeChecked -Description "postgres pg_isready" -ComposeArgs @("exec", "-T", "postgres", "pg_isready", "-U", $env:POSTGRES_USER, "-d", $env:POSTGRES_DB)
    }

    Invoke-Step "Host PostgreSQL endpoint" {
        Assert-HostDatabaseEndpoint
    }

    if (-not $SkipImport) {
        Invoke-Step "Import json_db into PostgreSQL" {
            python (Join-Path $repoRoot "scripts\import_json_db_to_postgres.py")
            if ($LASTEXITCODE -ne 0) {
                throw "Command failed: python scripts/import_json_db_to_postgres.py"
            }
            $executedSuites.Add("python scripts/import_json_db_to_postgres.py") | Out-Null
        }
    }

    if (-not $SkipCatalogueSubmit) {
        Invoke-Step "Submit integration services catalogue" {
            & (Join-Path $repoRoot "scripts\manage_integration_services.ps1") -Submit
            if ($LASTEXITCODE -ne 0) {
                throw "Command failed: scripts/manage_integration_services.ps1 -Submit"
            }
            $executedSuites.Add("scripts/manage_integration_services.ps1 -Submit") | Out-Null
        }
    }

    if (-not $SkipLlmSync) {
        Invoke-Step "Sync LLM catalog (interpretation profiles + product models)" {
            Sync-AstralLlmCatalog -RepoRoot $repoRoot
            if ($LASTEXITCODE -ne 0) {
                throw "Command failed: Sync-AstralLlmCatalog"
            }
            $executedSuites.Add("Sync-AstralLlmCatalog") | Out-Null
        }
    }

    Invoke-Step "Restart in-memory catalog services" {
        Invoke-DockerComposeChecked -Description "docker compose restart runtime services" -ComposeArgs @("restart", "astral_llm_api", "astral_llm_worker", "astral_gateway")
    }

    Invoke-Step "HTTP readiness" {
        Wait-HttpReady -Url "$CalculatorUrl/health/ready" -TimeoutSec $ReadyTimeoutSec
        Wait-HttpReady -Url "$LlmUrl/health/ready" -TimeoutSec $ReadyTimeoutSec
        Wait-HttpReady -Url "$GatewayUrl/health/ready" -TimeoutSec $ReadyTimeoutSec
    }

    Invoke-Step "Public integration catalogue" {
        $services = Invoke-RestMethod -Uri "$LlmUrl/v1/services" -Method Get -Headers $llmHeaders -TimeoutSec 20
        $active = @($services.services | Where-Object { $_.availability -in @("active", "beta") })
        if ($active.Count -lt 1) {
            throw "No active integration service returned by $LlmUrl/v1/services"
        }
        Write-Host ("Active/beta services: {0}" -f (($active | ForEach-Object { $_.service_code }) -join ", "))
    }

    if (-not $SkipRustChecks) {
        Invoke-RustTest -Name "Rust tests: shared contracts" -CargoArgs @("test", "-p", "astral_contracts", "--test", "contracts_registry_tests", "--test", "inline_tests_governance_tests")
        Invoke-RustTest -Name "Rust tests: gateway" -CargoArgs @("test", "-p", "astral_gateway")
        Invoke-RustTest -Name "Rust tests: llm application" -CargoArgs @("test", "-p", "astral_llm_application", "--test", "integration_job_executor_tests", "--test", "chapter_quality_repair_tests")
        Invoke-RustTest -Name "Rust tests: published contracts" -CargoArgs @("test", "-p", "astral_llm_api", "--test", "contracts_publish_tests")
        Invoke-RustTest -Name "Rust tests: integration services" -CargoArgs @("test", "-p", "astral_llm_api", "--test", "integration_services_tests")
        Invoke-RustTest -Name "Rust tests: integration jobs" -CargoArgs @("test", "-p", "astral_llm_api", "--test", "integration_jobs_tests")
        Invoke-RustTest -Name "Rust tests: calculator api" -CargoArgs @("test", "-p", "astral_calculator_http", "--test", "astral_calculator_http_tests")
    }

    if (-not $SkipSmoke) {
        Invoke-Step "Gateway and async smokes (fake provider only)" {
            $fakeProviderArmed = $false
            $horoscopeFakeProviderArmed = $false
            try {
                Enable-SimplifiedE2eFakeLlmProvider -RepoRoot $repoRoot
                $fakeProviderArmed = $true
                Enable-HoroscopeE2eFakeLlmProvider -RepoRoot $repoRoot
                $horoscopeFakeProviderArmed = $true
                Wait-HttpReady -Url "$LlmUrl/health/ready" -TimeoutSec $ReadyTimeoutSec
                Wait-HttpReady -Url "$GatewayUrl/health/ready" -TimeoutSec $ReadyTimeoutSec

                & (Join-Path $repoRoot "scripts\test_integration_jobs_e2e.ps1") `
                    -LlmBase $LlmUrl `
                    -AllowProductFakeOverride
                if ($LASTEXITCODE -ne 0) {
                    throw "Command failed: scripts/test_integration_jobs_e2e.ps1"
                }
                $executedSuites.Add("scripts/test_integration_jobs_e2e.ps1 -AllowProductFakeOverride") | Out-Null

                $natalSimplified = Invoke-GatewayJsonPost -Url "$GatewayUrl/v2/natal/simplified/free" -Body (New-GatewayNatalRequestBody -AudienceLevel "general" -IncludeBirthTime:$false)
                if ($natalSimplified.metadata.product_code -ne "natal_simplified_free") {
                    throw "Unexpected gateway simplified product_code: $($natalSimplified.metadata.product_code)"
                }

                $natalFull = Invoke-GatewayJsonPost -Url "$GatewayUrl/v2/natal/full/basic" -Body (New-GatewayNatalRequestBody -AudienceLevel "intermediate" -IncludeBirthTime:$true)
                if ($natalFull.metadata.product_code -ne "natal_full_basic") {
                    throw "Unexpected gateway full product_code: $($natalFull.metadata.product_code)"
                }

                $chartCalculationId = Get-HoroscopeChartCalculationId
                $horoscopeDaily = Invoke-GatewayJsonPost -Url "$GatewayUrl/v2/horoscope/daily/free" -Body @{
                    date = "2026-06-14"
                    timezone = "Europe/Paris"
                    target_language = "fr"
                    chart_calculation_id = $chartCalculationId
                    audience_level = "general"
                }
                if ($horoscopeDaily.metadata.variant -ne "daily") {
                    throw "Unexpected gateway daily variant: $($horoscopeDaily.metadata.variant)"
                }

                $horoscopePeriod = Invoke-GatewayJsonPost -Url "$GatewayUrl/v2/horoscope/period/free" -Body @{
                    anchor_date = "2026-06-14"
                    timezone = "Europe/Paris"
                    target_language = "fr"
                    chart_calculation_id = $chartCalculationId
                    audience_level = "general"
                }
                if ($horoscopePeriod.metadata.variant -ne "period") {
                    throw "Unexpected gateway period variant: $($horoscopePeriod.metadata.variant)"
                }

                foreach ($legacyPath in @("/v1/readings/generate", "/v1/readings/natal/simplified")) {
                    $legacyResponse = Invoke-WebRequest -Uri "$GatewayUrl$legacyPath" -Method Post -Body "{}" -ContentType "application/json" -SkipHttpErrorCheck -TimeoutSec 20
                    if ($legacyResponse.StatusCode -ne 404) {
                        throw "Expected 404 on legacy route $legacyPath, got $($legacyResponse.StatusCode)"
                    }
                }

                $executedSuites.Add("Gateway V2 live smokes on /v2/natal/* and /v2/horoscope/*") | Out-Null
            } finally {
                if ($horoscopeFakeProviderArmed) {
                    Restore-HoroscopeE2eLlmProvider -RepoRoot $repoRoot
                }
                if ($fakeProviderArmed) {
                    Restore-SimplifiedE2eLlmProvider -RepoRoot $repoRoot
                }
                if ($horoscopeFakeProviderArmed -or $fakeProviderArmed) {
                    Wait-HttpReady -Url "$LlmUrl/health/ready" -TimeoutSec $ReadyTimeoutSec
                    Wait-HttpReady -Url "$GatewayUrl/health/ready" -TimeoutSec $ReadyTimeoutSec
                }
            }
        }
    }

    Write-Host "`nDocker integration stack is ready." -ForegroundColor Green
    Write-Host "Calculator: $CalculatorUrl"
    Write-Host "LLM API:    $LlmUrl"
    Write-Host "Gateway:    $GatewayUrl"
    Write-Host "Mercure:    http://127.0.0.1:3000"
    if ($LegacyCutover) {
        Write-Host "Legacy product_code shim: disabled via docker-compose.legacy-cutover.yml" -ForegroundColor Yellow
    }

    Write-Host "`nExecuted suites:" -ForegroundColor Cyan
    foreach ($entry in $executedSuites) {
        Write-Host " - $entry"
    }

    Write-Host "`nExcluded because real LLM engine:" -ForegroundColor Cyan
    foreach ($entry in $excludedSuites) {
        Write-Host " - $entry"
    }

    Clear-TmpTargetArtifacts
    $scriptSucceeded = $true
} catch {
    Write-Host "`nUpdate failed: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "`nDocker diagnostics:" -ForegroundColor Yellow
    Invoke-DockerCompose -ComposeArgs @("ps")
    Invoke-DockerCompose -ComposeArgs @("logs", "--tail", "80", "astral_gateway", "astral_llm_api", "astral_llm_worker", "astral_calculator_http", "postgres")
    throw
} finally {
    if (-not $scriptSucceeded) {
        Write-Host "`nTmp target cleanup skipped because the update failed." -ForegroundColor Yellow
    }
    Pop-Location
}
