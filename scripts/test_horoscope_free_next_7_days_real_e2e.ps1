param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [int]$TimeoutSec = 180,
    [string]$IdempotencyKey = "",
    [string]$AnchorDate = "2026-06-07",
    [string]$UseExistingChartCalculationId = "",
    [switch]$SubmitCatalogue,
    [switch]$SkipWhenPlanned,
    [switch]$FailWhenPlanned,
    [switch]$StartStack,
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

. "$PSScriptRoot\lib\astral_http_auth.ps1"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Import-AstralDotEnv -RepoRoot $repoRoot

if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("OPENAI_API_KEY"))) {
    throw "OPENAI_API_KEY is required in .env for real horoscope Free next 7 days E2E"
}
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\horoscope_free_period_real"
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ($SubmitCatalogue) {
    & (Join-Path $repoRoot "scripts\manage_integration_services.ps1") -Submit
}

$headers = New-AstralAuthHeaders -Service llm
if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "horoscope-period-free-next-7-real-$([guid]::NewGuid().ToString('N'))"
}
$headers["Idempotency-Key"] = $IdempotencyKey
$calcHeaders = New-AstralAuthHeaders -Service calculator

function Assert-HttpReady {
    param([string]$Url, $Headers)
    try {
        Invoke-WebRequest -Uri $Url -Headers $Headers -UseBasicParsing -TimeoutSec 10 | Out-Null
    } catch {
        throw "HTTP endpoint is not reachable: $Url. Start the local stack first with 'docker compose up -d --build' or rerun this script with -StartStack. Detail: $($_.Exception.Message)"
    }
}

function ConvertFrom-JsonPreserveDates {
    param([string]$Json)
    if ((Get-Command ConvertFrom-Json).Parameters.ContainsKey("DateKind")) {
        return $Json | ConvertFrom-Json -DateKind String
    }
    return $Json | ConvertFrom-Json
}

function ConvertTo-YamlSingleQuoted {
    param([string]$Value)
    return "'" + ($Value -replace "'", "''") + "'"
}

function Wait-LlmReady {
    param(
        [string]$BaseUrl,
        $Headers,
        [int]$TimeoutSec = 90
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $lastError = $null
    while ((Get-Date) -lt $deadline) {
        try {
            $ready = Invoke-RestMethod -Method Get -Uri "$BaseUrl/health/ready" -Headers $Headers -TimeoutSec 5
            if ($ready.status -eq "ready") { return }
            $lastError = "status=$($ready.status)"
        } catch {
            $lastError = $_.Exception.Message
        }
        Start-Sleep -Seconds 2
    }
    throw "LLM API not ready after provider sync. Last error: $lastError"
}

function Sync-DockerLlmProviderFromDotEnv {
    param(
        [string]$RepoRoot,
        [string]$BaseUrl,
        $Headers
    )
    $expectedProvider = [Environment]::GetEnvironmentVariable("ASTRAL_LLM_DEFAULT_PROVIDER")
    $expectedModel = [Environment]::GetEnvironmentVariable("ASTRAL_LLM_DEFAULT_MODEL")
    if ([string]::IsNullOrWhiteSpace($expectedProvider) -or $expectedProvider -eq "fake") {
        throw ".env must define ASTRAL_LLM_DEFAULT_PROVIDER with a real provider for real horoscope Free next 7 days E2E"
    }
    if ([string]::IsNullOrWhiteSpace($expectedModel)) {
        throw ".env must define ASTRAL_LLM_DEFAULT_MODEL for real horoscope Free next 7 days E2E"
    }
    if ($BaseUrl -notmatch "^(http://)?(127\.0\.0\.1|localhost):") {
        throw "Running LLM provider is fake and BaseUrl is not local; cannot sync Docker containers from .env"
    }

    $overrideDir = Join-Path $RepoRoot "output\horoscope_free_period_real"
    New-Item -ItemType Directory -Force -Path $overrideDir | Out-Null
    $overridePath = Join-Path $overrideDir "docker-compose.real-provider.override.yml"
    $providerYaml = ConvertTo-YamlSingleQuoted $expectedProvider
    $modelYaml = ConvertTo-YamlSingleQuoted $expectedModel
    @"
services:
  astral_llm_api:
    environment:
      ASTRAL_LLM_DEFAULT_PROVIDER: $providerYaml
      ASTRAL_LLM_DEFAULT_MODEL: $modelYaml
  astral_llm_worker:
    environment:
      ASTRAL_LLM_DEFAULT_PROVIDER: $providerYaml
      ASTRAL_LLM_DEFAULT_MODEL: $modelYaml
"@ | Set-Content -LiteralPath $overridePath -Encoding UTF8

    Write-Host "Syncing astral_llm_api and astral_llm_worker from .env ($expectedProvider / $expectedModel)..." -ForegroundColor Yellow
    & docker compose -f (Join-Path $RepoRoot "docker-compose.yml") -f $overridePath up -d --no-build --force-recreate astral_llm_api astral_llm_worker | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "docker compose provider sync failed"
    }
    Wait-LlmReady -BaseUrl $BaseUrl -Headers $Headers
}

function Assert-RealLlmProviderReady {
    param([string]$BaseUrl, $Headers)
    $providers = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/providers" -Headers $Headers -TimeoutSec 10
    $defaultProvider = [string]$providers.default_provider
    $defaultModel = [string]$providers.default_model
    if ([string]::IsNullOrWhiteSpace($defaultProvider) -or $defaultProvider -eq "fake") {
        Sync-DockerLlmProviderFromDotEnv -RepoRoot $repoRoot -BaseUrl $BaseUrl -Headers $Headers
        $providers = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/providers" -Headers $Headers -TimeoutSec 10
        $defaultProvider = [string]$providers.default_provider
        $defaultModel = [string]$providers.default_model
    }
    if ([string]::IsNullOrWhiteSpace($defaultProvider) -or $defaultProvider -eq "fake") {
        throw "Real horoscope Free next 7 days E2E requires a real LLM provider. Current default_provider='$defaultProvider', default_model='$defaultModel'."
    }
}

function Assert-FreePublicText {
    param([AllowEmptyString()][string]$Text, [string]$Label)
    $forbidden = "period:|natal_|fake_|theme_code|evidence_key|snapshot|source_snapshot|scan_plan|raw_transits|transit_exact|transit_active|moon_house_by_day|HOROSCOPE_"
    if ($Text -match $forbidden) {
        throw "Technical code leaked in ${Label}: $Text"
    }
    if ($Text -match "(?im)(\b(et|a|de|pour|avec|sans|dans|sur|vers|la|le|les|des|du|au|aux|un|une|ce|cet|cette)\s*[.!?]\s*$)") {
        throw "Broken sentence detected in ${Label}: $Text"
    }
}

if ($StartStack) {
    Write-Host "Starting local Docker stack..." -ForegroundColor Yellow
    Push-Location $repoRoot
    try {
        docker compose up -d --build
        if ($LASTEXITCODE -ne 0) {
            throw "docker compose up -d --build failed"
        }
    } finally {
        Pop-Location
    }
}

Assert-HttpReady "$CalculatorUrl/health/ready" $calcHeaders
Assert-HttpReady "$BaseUrl/health/ready" $headers
Assert-RealLlmProviderReady -BaseUrl $BaseUrl -Headers $headers

$services = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/services?include=planned" -Headers $headers
$service = @($services.services | Where-Object { $_.service_code -eq "horoscope_free_next_7_days_natal" })[0]
if (-not $service) {
    if (-not $SubmitCatalogue) {
        Write-Host "Service horoscope_free_next_7_days_natal not listed after include=planned. Submitting local catalogue once..." -ForegroundColor Yellow
        & (Join-Path $repoRoot "scripts\manage_integration_services.ps1") -Submit
        $services = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/services?include=planned" -Headers $headers
        $service = @($services.services | Where-Object { $_.service_code -eq "horoscope_free_next_7_days_natal" })[0]
    }
    if (-not $service) {
        throw "Service horoscope_free_next_7_days_natal not listed, even with include=planned. Run json_db import and catalogue submit, then retry."
    }
}
if ($service.availability -eq "planned") {
    if ($FailWhenPlanned) {
        throw "Service horoscope_free_next_7_days_natal is planned and not submittable yet. Promote it to beta/active before strict real E2E."
    }
    Write-Host "SKIP: horoscope_free_next_7_days_natal is planned and not submittable yet. Real E2E will run once it is beta/active." -ForegroundColor Yellow
    exit 0
}
if ($service.availability -ne "beta" -and $service.availability -ne "active") {
    throw "Service horoscope_free_next_7_days_natal must be beta or active for real E2E, got $($service.availability)"
}

$chartCalculationId = $UseExistingChartCalculationId
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
    $natalRequestPath = Join-Path $repoRoot "contracts\integration\examples\natal_calculation_request_v1.paris_1990.json"
    if (-not (Test-Path -LiteralPath $natalRequestPath)) {
        throw "Missing natal fixture: $natalRequestPath"
    }
    $natalRequest = Get-Content -LiteralPath $natalRequestPath -Raw | ConvertFrom-Json
    $natalResponse = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/calculations/natal" -Headers $calcHeaders -Body $natalRequest
    if ($natalResponse.calculation_result.status -ne "completed") {
        throw "Natal calculation did not complete"
    }
    $chartCalculationId = [string]$natalResponse.calculation_result.chart_calculation_id
}
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
    throw "Missing chart_calculation_id"
}

$bodyObject = @{
    service_code = "horoscope_free_next_7_days_natal"
    payload = @{
        anchor_date = $AnchorDate
        timezone = "Europe/Paris"
        target_language = "fr"
        chart_calculation_id = $chartCalculationId
        audience_level = "general"
    }
}
$body = $bodyObject | ConvertTo-Json -Depth 20

$submit = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/jobs" -Headers $headers -ContentType "application/json" -Body $body
if (-not $submit.run_id) {
    throw "Missing run_id in submit response"
}

$deadline = (Get-Date).AddSeconds($TimeoutSec)
$status = $null
$statusRaw = ""
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 3
    $statusResponse = Invoke-WebRequest -Method Get -Uri "$BaseUrl/v1/jobs/$($submit.run_id)" -Headers $headers -UseBasicParsing -TimeoutSec 20
    $statusRaw = [string]$statusResponse.Content
    $status = ConvertFrom-JsonPreserveDates $statusRaw
    if ($status.status -eq "completed") { break }
    if ($status.status -eq "failed" -or $status.status -eq "safety_rejected") {
        throw "Real horoscope Free next 7 days E2E ended with $($status.status): $statusRaw"
    }
}
if (-not $status -or $status.status -ne "completed") {
    throw "Timeout waiting for real horoscope Free next 7 days E2E"
}

$reading = $status.result.reading
$calculation = $status.result.calculation
$writerRequest = $status.result.writer_request
if (-not $writerRequest) {
    $writerRequest = $status.result.interpretation_request
}
if (-not $reading -or -not $calculation -or -not $writerRequest) {
    throw "Real Free period response must include reading, calculation and writer_request"
}
if ($reading.contract_version -ne "horoscope_period_response") {
    throw "Unexpected reading contract: $($reading.contract_version)"
}
if ($reading.service_code -ne "horoscope_free_next_7_days_natal") {
    throw "Unexpected service_code in reading: $($reading.service_code)"
}
if ($writerRequest.scan_plan.scan_profile_code -ne "daily_noon_7_days") {
    throw "Free scan profile must be daily_noon_7_days"
}
if (@($writerRequest.scan_plan.snapshots).Count -ne 7 -or @($calculation.snapshots).Count -ne 7) {
    throw "Free period E2E must use exactly 7 daily snapshots"
}
if (-not $writerRequest.semantic_brief) {
    throw "Free writer_request must expose semantic_brief"
}

foreach ($field in @("week_overview", "daily_timeline", "best_days", "watch_days", "best_windows", "watch_windows", "domain_sections", "strategy")) {
    if ($reading.PSObject.Properties.Name -contains $field) {
        throw "Free reading leaked forbidden field: $field"
    }
}
foreach ($path in @("summary", "dominant_theme", "watch_summary", "quality")) {
    if (-not ($reading.PSObject.Properties.Name -contains $path)) {
        throw "Free reading missing $path"
    }
}
if ([string]::IsNullOrWhiteSpace([string]$reading.advice)) {
    throw "Free reading missing advice"
}

$keyDays = @($reading.key_days)
if ($keyDays.Count -lt 1 -or $keyDays.Count -gt 2) {
    throw "Free reading key_days must contain 1 to 2 entries, got $($keyDays.Count)"
}
foreach ($day in $keyDays) {
    if (-not $day.evidence_keys -or @($day.evidence_keys).Count -lt 1) {
        throw "Free key day must reference evidence"
    }
}

$evidenceSummary = @($reading.evidence_summary)
if ($evidenceSummary.Count -lt 1 -or $evidenceSummary.Count -gt 3) {
    throw "Free evidence_summary must contain 1 to 3 entries, got $($evidenceSummary.Count)"
}
$watchStatus = [string]$reading.watch_summary.status
if ($watchStatus -notin @("none", "low", "active")) {
    throw "Free watch_summary.status must be none, low or active, got '$watchStatus'"
}
if ($watchStatus -ne "none" -and @($reading.watch_summary.evidence_keys).Count -lt 1) {
    throw "Free watch_summary must reference evidence when status is $watchStatus"
}

$provider = [string]$reading.quality.provider
if ([string]::IsNullOrWhiteSpace($provider) -or $provider -eq "fake") {
    throw "Real Free period writer used invalid provider: '$provider'"
}
if ([bool]$reading.quality.fallback_used) {
    throw "Real Free period writer unexpectedly used fallback"
}
if ([string]::IsNullOrWhiteSpace([string]$reading.quality.model)) {
    throw "Real Free period writer did not expose model"
}

Assert-FreePublicText ([string]$reading.summary.title) "summary.title"
Assert-FreePublicText ([string]$reading.summary.text) "summary.text"
Assert-FreePublicText ([string]$reading.dominant_theme.theme) "dominant_theme.theme"
Assert-FreePublicText ([string]$reading.dominant_theme.text) "dominant_theme.text"
Assert-FreePublicText ([string]$reading.advice) "advice"
Assert-FreePublicText ([string]$reading.watch_summary.text) "watch_summary.text"
foreach ($day in $keyDays) {
    Assert-FreePublicText ([string]$day.reason) "key_days.reason"
}

$outputPath = Join-Path $OutputDir ("free_next_7_real_{0}.json" -f $submit.run_id)
$statusRaw | Set-Content -LiteralPath $outputPath -Encoding UTF8
Write-Host "Real Free next 7 days E2E completed: $outputPath" -ForegroundColor Green
