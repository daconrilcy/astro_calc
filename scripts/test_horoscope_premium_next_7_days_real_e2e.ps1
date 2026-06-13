param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [int]$TimeoutSec = 240,
    [string]$IdempotencyKey = "",
    [string]$AnchorDate = "2026-06-07",
    [string]$UseExistingChartCalculationId = "",
    [switch]$SubmitCatalogue,
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

. "$PSScriptRoot\lib\astral_http_auth.ps1"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Import-AstralDotEnv -RepoRoot $repoRoot

if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("OPENAI_API_KEY"))) {
    throw "OPENAI_API_KEY is required for real horoscope premium period E2E"
}
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\horoscope_premium_period_real"
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ($SubmitCatalogue) {
    & (Join-Path $repoRoot "scripts\manage_integration_services.ps1") -Submit
}

$headers = New-AstralAuthHeaders -Service llm
if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "horoscope-period-premium-next-7-real-$([guid]::NewGuid().ToString('N'))"
}
$headers["Idempotency-Key"] = $IdempotencyKey
$calcHeaders = New-AstralAuthHeaders -Service calculator

function Assert-HttpReady {
    param([string]$Url, $Headers)
    Invoke-WebRequest -Uri $Url -Headers $Headers -UseBasicParsing -TimeoutSec 10 | Out-Null
}

function ConvertFrom-JsonPreserveDates {
    param([string]$Json)
    if ((Get-Command ConvertFrom-Json).Parameters.ContainsKey("DateKind")) {
        return $Json | ConvertFrom-Json -DateKind String
    }
    return $Json | ConvertFrom-Json
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
        throw "Real horoscope premium period E2E requires a real LLM provider. Current default_provider='$defaultProvider', default_model='$defaultModel'."
    }
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
        throw ".env must define ASTRAL_LLM_DEFAULT_PROVIDER with a real provider for real horoscope premium period E2E"
    }
    if ([string]::IsNullOrWhiteSpace($expectedModel)) {
        throw ".env must define ASTRAL_LLM_DEFAULT_MODEL for real horoscope premium period E2E"
    }
    if ($BaseUrl -notmatch "^(http://)?(127\.0\.0\.1|localhost):") {
        throw "Running LLM provider is fake and BaseUrl is not local; cannot sync Docker containers from .env"
    }

    $overrideDir = Join-Path $RepoRoot "output\horoscope_premium_period_real"
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

function Assert-ArrayCount {
    param($Items, [int]$Expected, [string]$Label)
    $count = @($Items).Count
    if ($count -ne $Expected) {
        throw "$Label must contain $Expected entries, got $count"
    }
}

function Assert-CanonicalUtcFields {
    param([string]$Json)
    $matches = [regex]::Matches($Json, '"[^"]*_utc"\s*:\s*"([^"]+)"')
    if ($matches.Count -eq 0) {
        throw "Expected at least one *_utc field in real premium period output"
    }
    $bad = @($matches | Where-Object { $_.Groups[1].Value -notmatch '\+00:00$' })
    if ($bad.Count -gt 0) {
        $sample = @($bad | Select-Object -First 5 | ForEach-Object { $_.Value }) -join "; "
        throw "All *_utc fields must be canonical +00:00. Invalid count=$($bad.Count). Sample: $sample"
    }
}

function Assert-PremiumText {
    param([AllowEmptyString()][string]$Text, [string]$Label)
    $forbidden = "period:|natal_|fake_|theme_code|evidence_key|snapshot|source_snapshot|scan_plan|raw_transits|transit_exact|transit_active|moon_house_by_day|HOROSCOPE_"
    if ($Text -match $forbidden) {
        throw "Technical code leaked in ${Label}: $Text"
    }
    if ($Text -match "(?im)(\b(et|a|de|pour|avec|sans|dans|sur|vers|la|le|les|des|du|au|aux|un|une|ce|cet|cette)\s*[.!?]\s*$)") {
        throw "Broken sentence detected in ${Label}: $Text"
    }
}

Assert-HttpReady "$CalculatorUrl/health/ready" $calcHeaders
Assert-HttpReady "$BaseUrl/health/ready" $headers
Assert-RealLlmProviderReady -BaseUrl $BaseUrl -Headers $headers

$services = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/services" -Headers $headers
$service = @($services.services | Where-Object { $_.service_code -eq "horoscope_premium_next_7_days_natal" })[0]
if (-not $service) {
    throw "Service horoscope_premium_next_7_days_natal not listed"
}
if ($service.availability -ne "beta") {
    throw "Service horoscope_premium_next_7_days_natal must be beta for real E2E, got $($service.availability)"
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
    service_code = "horoscope_premium_next_7_days_natal"
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
        throw "Real horoscope premium period E2E ended with $($status.status): $statusRaw"
    }
}
if (-not $status -or $status.status -ne "completed") {
    throw "Timeout waiting for real horoscope premium period E2E"
}
Assert-CanonicalUtcFields $statusRaw

$reading = $status.result.reading
$calculation = $status.result.calculation
$writerRequest = $status.result.writer_request
if (-not $writerRequest) {
    $writerRequest = $status.result.interpretation_request
}
if (-not $reading -or -not $calculation -or -not $writerRequest) {
    throw "Real premium period response must include reading, calculation and writer_request"
}
if ($reading.contract_version -ne "horoscope_period_response") {
    throw "Unexpected reading contract: $($reading.contract_version)"
}
if ($reading.service_code -ne "horoscope_premium_next_7_days_natal") {
    throw "Unexpected service_code in reading: $($reading.service_code)"
}
if ($writerRequest.scan_plan.scan_profile_code -ne "six_hour_7_days") {
    throw "Premium scan profile must be six_hour_7_days"
}
Assert-ArrayCount $writerRequest.scan_plan.snapshots 28 "writer_request.scan_plan.snapshots"
Assert-ArrayCount $calculation.scan_plan.snapshots 28 "calculation.scan_plan.snapshots"
Assert-ArrayCount $calculation.snapshots 28 "calculation.snapshots"
Assert-ArrayCount $reading.daily_timeline 7 "reading.daily_timeline"

$snapshotsPerDate = @{}
foreach ($snapshot in @($writerRequest.scan_plan.snapshots)) {
    $date = [string]$snapshot.date
    if (-not $snapshotsPerDate.ContainsKey($date)) {
        $snapshotsPerDate[$date] = @()
    }
    $snapshotsPerDate[$date] += [string]$snapshot.reference_time_local
}
foreach ($date in $snapshotsPerDate.Keys) {
    $times = @($snapshotsPerDate[$date] | Sort-Object)
    if (($times -join ",") -ne "00:00,06:00,12:00,18:00") {
        throw "Date $date must expose 00:00,06:00,12:00,18:00 snapshots, got $($times -join ',')"
    }
}

$firstSnapshot = @($writerRequest.scan_plan.snapshots)[0]
if ([string]$firstSnapshot.reference_time_local -ne "00:00") {
    throw "First premium snapshot must be local 00:00"
}
if ([string]$firstSnapshot.date -ne $AnchorDate) {
    throw "First premium snapshot must keep local anchor date $AnchorDate, got $($firstSnapshot.date)"
}

$provider = [string]$reading.quality.provider
if ([string]::IsNullOrWhiteSpace($provider) -or $provider -eq "fake") {
    throw "Real premium period writer used invalid provider: '$provider'"
}
if ([bool]$reading.quality.fallback_used) {
    throw "Real premium period writer unexpectedly used fallback"
}
if ([string]::IsNullOrWhiteSpace([string]$reading.quality.model)) {
    throw "Real premium period writer did not expose model"
}

if (-not $reading.strategy) {
    throw "Premium reading missing strategy"
}
foreach ($field in @("title", "text", "best_use", "recovery")) {
    if ([string]::IsNullOrWhiteSpace([string]$reading.strategy.$field)) {
        throw "Premium strategy missing $field"
    }
    Assert-PremiumText ([string]$reading.strategy.$field) "strategy.$field"
}
if (-not $reading.strategy.evidence_keys -or @($reading.strategy.evidence_keys).Count -lt 1) {
    throw "Premium strategy must reference evidence"
}

$domainCount = @($reading.domain_sections).Count
if ($domainCount -lt 3 -or $domainCount -gt 5) {
    throw "Premium domain_sections must contain 3 to 5 entries, got $domainCount"
}
foreach ($section in @($reading.domain_sections)) {
    if (-not $section.evidence_keys -or @($section.evidence_keys).Count -lt 1) {
        throw "Domain section $($section.domain) missing evidence"
    }
    Assert-PremiumText ([string]$section.domain) "domain_sections.domain"
    Assert-PremiumText ([string]$section.title) "domain_sections.title"
    Assert-PremiumText ([string]$section.text) "domain_sections.text"
}

$includedDates = New-Object System.Collections.Generic.HashSet[string]
foreach ($date in @($writerRequest.period_resolution.included_dates)) {
    [void]$includedDates.Add([string]$date)
}
$snapshotKeys = New-Object System.Collections.Generic.HashSet[string]
foreach ($snapshot in @($writerRequest.scan_plan.snapshots)) {
    [void]$snapshotKeys.Add([string]$snapshot.snapshot_key)
}
$allowedEvidenceKeys = New-Object System.Collections.Generic.HashSet[string]
foreach ($evidence in @($writerRequest.evidence)) {
    [void]$allowedEvidenceKeys.Add([string]$evidence.evidence_key)
}

if (@($reading.best_windows).Count -lt 1) {
    throw "Premium best_windows must be non-empty"
}
if (@($reading.watch_windows).Count -eq 0 -and [string]$reading.watch_summary.status -ne "none") {
    throw "watch_windows empty requires watch_summary.status = none"
}

$bestWindowIdentities = New-Object System.Collections.Generic.HashSet[string]
foreach ($field in @("best_windows", "watch_windows")) {
    foreach ($window in @($reading.$field)) {
        if (-not $includedDates.Contains([string]$window.date)) {
            throw "$field window outside period: $($window.date)"
        }
        if (-not $window.source_snapshot_keys -or @($window.source_snapshot_keys).Count -lt 1) {
            throw "$field window $($window.date) missing source_snapshot_keys"
        }
        if (-not $window.evidence_keys -or @($window.evidence_keys).Count -lt 1) {
            throw "$field window $($window.date) missing evidence_keys"
        }
        foreach ($key in @($window.source_snapshot_keys)) {
            if (-not $snapshotKeys.Contains([string]$key)) {
                throw "$field references unknown source_snapshot_key $key"
            }
        }
        foreach ($key in @($window.evidence_keys)) {
            if (-not $allowedEvidenceKeys.Contains([string]$key)) {
                throw "$field references invented evidence_key $key"
            }
        }
        $identity = "$($window.date)|$(@($window.source_snapshot_keys) -join ',')"
        if ($field -eq "best_windows") {
            [void]$bestWindowIdentities.Add($identity)
        } elseif ($bestWindowIdentities.Contains($identity)) {
            throw "Best/watch window overlap for $identity"
        }
        foreach ($textField in @("title", "theme", "tone", "reason", "watch_point")) {
            if ($window.PSObject.Properties.Name -contains $textField) {
                Assert-PremiumText ([string]$window.$textField) "$field.$textField"
            }
        }
    }
}

$publicTextParts = @(
    $reading.week_overview.title,
    $reading.week_overview.text,
    $reading.week_overview.trajectory,
    $reading.watch_summary.text,
    $reading.advice.main,
    $reading.advice.best_use,
    $reading.advice.avoid,
    $reading.strategy.title,
    $reading.strategy.text,
    $reading.strategy.best_use,
    $reading.strategy.recovery
)
foreach ($day in @($reading.daily_timeline)) {
    if (-not $day.evidence_keys -or @($day.evidence_keys).Count -lt 1) {
        throw "Daily timeline $($day.date) missing evidence"
    }
    foreach ($field in @("day_label", "theme", "tone", "text", "advice")) {
        Assert-PremiumText ([string]$day.$field) "daily_timeline.$field"
        $publicTextParts += [string]$day.$field
    }
}
foreach ($marker in @($reading.key_days) + @($reading.best_days) + @($reading.watch_days)) {
    foreach ($field in @("title", "reason")) {
        if ($marker.PSObject.Properties.Name -contains $field) {
            Assert-PremiumText ([string]$marker.$field) "period marker.$field"
            $publicTextParts += [string]$marker.$field
        }
    }
}
foreach ($evidence in @($reading.evidence_summary)) {
    if (-not $includedDates.Contains([string]$evidence.date)) {
        throw "Evidence summary date outside period: $($evidence.date)"
    }
    if (-not $allowedEvidenceKeys.Contains([string]$evidence.evidence_key)) {
        throw "Evidence summary invented evidence_key: $($evidence.evidence_key)"
    }
    Assert-PremiumText ([string]$evidence.label) "evidence_summary.label"
    $publicTextParts += [string]$evidence.label
}

$joinedPublicText = ($publicTextParts -join "`n")
Assert-PremiumText $joinedPublicText "public premium period response"

$detailProfilesPath = Join-Path $repoRoot "json_db\horoscope_detail_profiles.json"
$detailProfilesDoc = Get-Content -LiteralPath $detailProfilesPath -Raw | ConvertFrom-Json
$premiumDetailProfile = @($detailProfilesDoc.data | Where-Object { $_.detail_profile_code -eq "premium_rich" -and $_.is_enabled -ne $false } | Select-Object -First 1)
if (-not $premiumDetailProfile) {
    throw "Missing premium_rich in horoscope_detail_profiles.json"
}
$wordCount = @($joinedPublicText -split "\s+" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count
$hardLimitWords = [int]$premiumDetailProfile.hard_limit_words
if ($wordCount -gt $hardLimitWords) {
    throw "Real premium period public word count must be <= $hardLimitWords, got $wordCount"
}

$replay = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/jobs" -Headers $headers -ContentType "application/json" -Body $body
if ($replay.run_id -ne $submit.run_id) {
    throw "Idempotent replay returned a different run_id"
}

$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$jsonPath = Join-Path $OutputDir "horoscope_premium_next_7_days_real_$stamp.json"
$statusRaw | Set-Content -LiteralPath $jsonPath -Encoding UTF8
Write-Host "Saved real horoscope premium period output: $jsonPath" -ForegroundColor Green
