param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [int]$TimeoutSec = 180,
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
    throw "OPENAI_API_KEY is required for real horoscope period E2E"
}
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\horoscope_period_real"
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ($SubmitCatalogue) {
    & (Join-Path $repoRoot "scripts\manage_integration_services.ps1") -Submit
}

$headers = New-AstralAuthHeaders -Service llm
if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "horoscope-period-basic-next-7-real-$([guid]::NewGuid().ToString('N'))"
}
$headers["Idempotency-Key"] = $IdempotencyKey
$calcHeaders = New-AstralAuthHeaders -Service calculator

function Assert-RealLlmProviderReady {
    param(
        [string]$BaseUrl,
        $Headers,
        [string]$RepoRoot
    )
    $providers = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/providers" -Headers $Headers -TimeoutSec 10
    $defaultProvider = [string]$providers.default_provider
    $defaultModel = [string]$providers.default_model
    if ([string]::IsNullOrWhiteSpace($defaultProvider) -or $defaultProvider -eq "fake") {
        Sync-DockerLlmProviderFromDotEnv -RepoRoot $RepoRoot -BaseUrl $BaseUrl
        $providers = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/providers" -Headers $Headers -TimeoutSec 10
        $defaultProvider = [string]$providers.default_provider
        $defaultModel = [string]$providers.default_model
        if ([string]::IsNullOrWhiteSpace($defaultProvider) -or $defaultProvider -eq "fake") {
            throw @"
Real horoscope period E2E requires the LLM API default provider to be real.
Current /v1/providers default_provider='$defaultProvider', default_model='$defaultModel'.
The script read .env but could not align the running LLM containers.
"@
        }
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
        [string]$BaseUrl
    )
    $expectedProvider = [Environment]::GetEnvironmentVariable("ASTRAL_LLM_DEFAULT_PROVIDER")
    $expectedModel = [Environment]::GetEnvironmentVariable("ASTRAL_LLM_DEFAULT_MODEL")
    if ([string]::IsNullOrWhiteSpace($expectedProvider) -or $expectedProvider -eq "fake") {
        throw ".env must define ASTRAL_LLM_DEFAULT_PROVIDER with a real provider for real horoscope period E2E"
    }
    if ([string]::IsNullOrWhiteSpace($expectedModel)) {
        throw ".env must define ASTRAL_LLM_DEFAULT_MODEL for real horoscope period E2E"
    }
    if ($BaseUrl -notmatch "^(http://)?(127\.0\.0\.1|localhost):") {
        throw "Running LLM provider is fake and BaseUrl is not local; cannot sync Docker containers from .env"
    }

    $overrideDir = Join-Path $RepoRoot "output\horoscope_period_real"
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
    Wait-LlmReady -BaseUrl $BaseUrl -Headers $headers
}

function ConvertFrom-JsonPreserveDates {
    param([string]$Json)
    if ((Get-Command ConvertFrom-Json).Parameters.ContainsKey("DateKind")) {
        return $Json | ConvertFrom-Json -DateKind String
    }
    return $Json | ConvertFrom-Json
}

function Get-RawJsonElement {
    param(
        [System.Text.Json.JsonElement]$Element,
        [object[]]$Path
    )
    $current = $Element
    foreach ($segment in $Path) {
        if ($current.ValueKind -eq [System.Text.Json.JsonValueKind]::Object) {
            $property = $current.EnumerateObject() | Where-Object { $_.Name -eq [string]$segment } | Select-Object -First 1
            if (-not $property) {
                throw "Missing JSON path segment '$segment' in '$($Path -join ".")'"
            }
            $current = $property.Value
        } elseif ($current.ValueKind -eq [System.Text.Json.JsonValueKind]::Array -and $segment -is [int]) {
            $items = @($current.EnumerateArray())
            if ($segment -lt 0 -or $segment -ge $items.Count) {
                throw "Array index '$segment' out of bounds in '$($Path -join ".")'"
            }
            $current = $items[$segment]
        } else {
            throw "Cannot traverse JSON path '$($Path -join ".")' at segment '$segment'"
        }
    }
    return $current
}

function Get-RawJsonString {
    param(
        [string]$Json,
        [object[]]$Path
    )
    $doc = [System.Text.Json.JsonDocument]::Parse($Json)
    try {
        $element = Get-RawJsonElement -Element $doc.RootElement -Path $Path
        if ($element.ValueKind -ne [System.Text.Json.JsonValueKind]::String) {
            throw "JSON path '$($Path -join ".")' is not a string"
        }
        return $element.GetString()
    } finally {
        $doc.Dispose()
    }
}

function Get-RawJsonArrayPropertyStrings {
    param(
        [string]$Json,
        [object[]]$ArrayPath,
        [string]$PropertyName
    )
    $doc = [System.Text.Json.JsonDocument]::Parse($Json)
    try {
        $array = Get-RawJsonElement -Element $doc.RootElement -Path $ArrayPath
        if ($array.ValueKind -ne [System.Text.Json.JsonValueKind]::Array) {
            throw "JSON path '$($ArrayPath -join ".")' is not an array"
        }
        $values = @()
        foreach ($item in $array.EnumerateArray()) {
            $property = $item.EnumerateObject() | Where-Object { $_.Name -eq $PropertyName } | Select-Object -First 1
            if (-not $property -or $property.Value.ValueKind -ne [System.Text.Json.JsonValueKind]::String) {
                throw "Array item '$($ArrayPath -join ".")' is missing string property '$PropertyName'"
            }
            $values += $property.Value.GetString()
        }
        return $values
    } finally {
        $doc.Dispose()
    }
}

if ($BaseUrl -match "^(http://)?(127\.0\.0\.1|localhost):") {
    Sync-DockerLlmProviderFromDotEnv -RepoRoot $repoRoot -BaseUrl $BaseUrl
}
Assert-RealLlmProviderReady -BaseUrl $BaseUrl -Headers $headers -RepoRoot $repoRoot

$chartCalculationId = $UseExistingChartCalculationId
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
    $natalRequestPath = Join-Path $repoRoot "contracts\integration\examples\natal_calculation_request_v1.paris_1990.json"
    $natalRequest = Get-Content -LiteralPath $natalRequestPath -Raw | ConvertFrom-Json
    $natalResponse = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/calculations/natal" -Headers $calcHeaders -Body $natalRequest
    if ($natalResponse.calculation_result.status -ne "completed") {
        throw "Natal calculation did not complete"
    }
    $chartCalculationId = [string]$natalResponse.calculation_result.chart_calculation_id
}

$body = @{
    service_code = "horoscope_basic_next_7_days_natal"
    payload = @{
        anchor_date = $AnchorDate
        timezone = "Europe/Paris"
        target_language = "fr"
        chart_calculation_id = $chartCalculationId
        audience_level = "general"
    }
} | ConvertTo-Json -Depth 20

$submitRaw = (Invoke-WebRequest -Method Post -Uri "$BaseUrl/v1/jobs" -Headers $headers -ContentType "application/json" -Body $body).Content
$submit = ConvertFrom-JsonPreserveDates $submitRaw
$deadline = (Get-Date).AddSeconds($TimeoutSec)
$status = $null
$statusRaw = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 3
    $statusRaw = (Invoke-WebRequest -Method Get -Uri "$BaseUrl/v1/jobs/$($submit.run_id)" -Headers $headers).Content
    $status = ConvertFrom-JsonPreserveDates $statusRaw
    if ($status.status -eq "completed") { break }
    if ($status.status -eq "failed" -or $status.status -eq "safety_rejected") {
        throw "Real horoscope period E2E ended with $($status.status): $($status | ConvertTo-Json -Depth 20)"
    }
}
if (-not $status -or $status.status -ne "completed") {
    throw "Timeout waiting for real horoscope period E2E"
}

$reading = $status.result.reading
$calculation = $status.result.calculation
$interpretation = $status.result.interpretation_request
if ($reading.contract_version -ne "horoscope_period_response_v1") {
    throw "Unexpected reading contract: $($reading.contract_version)"
}
if (@($reading.daily_timeline).Count -ne 7) {
    throw "Real period reading daily_timeline must contain 7 entries"
}
if (-not $calculation -or -not $interpretation) {
    throw "Real period response must include calculation and interpretation_request"
}

function Assert-UtcString {
    param(
        [string]$Value,
        [string]$Label
    )
    if ([string]::IsNullOrWhiteSpace($Value) -or -not ($Value.EndsWith("Z") -or $Value.EndsWith("+00:00"))) {
        throw "$Label must be normalized UTC, got '$Value'"
    }
}

Assert-UtcString (Get-RawJsonString $statusRaw @("result", "reading", "period_resolution", "start_datetime_utc")) "period_resolution.start_datetime_utc"
Assert-UtcString (Get-RawJsonString $statusRaw @("result", "reading", "period_resolution", "end_datetime_utc")) "period_resolution.end_datetime_utc"
foreach ($value in Get-RawJsonArrayPropertyStrings $statusRaw @("result", "calculation", "scan_plan", "snapshots") "reference_datetime_utc") {
    Assert-UtcString $value "scan_plan snapshot reference_datetime_utc"
}
foreach ($value in Get-RawJsonArrayPropertyStrings $statusRaw @("result", "calculation", "snapshots") "reference_datetime_utc") {
    Assert-UtcString $value "calculation snapshot reference_datetime_utc"
}
foreach ($snapshot in @($calculation.snapshots)) {
    foreach ($fact in @($snapshot.transits_to_natal)) {
        if ([string]$fact.source -match "^fake|fake_") {
            throw "Real period calculation used fake source: $($fact.source)"
        }
        if ([string]$fact.source -ne "swisseph_period_calculator_v1") {
            throw "Real period calculation used non-SwissEphemeris source: $($fact.source)"
        }
    }
}
$provider = [string]$reading.quality.provider
if ($provider -ne "openai") {
    throw "Real period writer used invalid provider: '$provider'"
}
if ([bool]$reading.quality.fallback_used) {
    throw "Real period writer unexpectedly used fallback"
}
if ([string]::IsNullOrWhiteSpace([string]$reading.quality.model)) {
    throw "Real period writer did not expose model"
}

$includedDates = New-Object System.Collections.Generic.HashSet[string]
foreach ($date in @($interpretation.period_resolution.included_dates)) {
    [void]$includedDates.Add([string]$date)
}
$allowedEvidenceKeys = New-Object System.Collections.Generic.HashSet[string]
foreach ($evidence in @($interpretation.evidence)) {
    [void]$allowedEvidenceKeys.Add([string]$evidence.evidence_key)
}

$validTensionDates = New-Object System.Collections.Generic.HashSet[string]
foreach ($event in @($interpretation.period_events)) {
    $tone = [string]$event.tone
    $aspect = [string]$event.aspect
    $date = [string]$event.date
    if ($tone -eq "careful" -or $aspect -in @("square", "opposition")) {
        [void]$validTensionDates.Add($date)
    }
}
foreach ($snapshot in @($calculation.snapshots)) {
    foreach ($fact in @($snapshot.transits_to_natal)) {
        if ($fact.fact_type -eq "transit_to_natal" -and -not [string]::IsNullOrWhiteSpace([string]$fact.aspect)) {
            $orb = [double]$fact.orb_deg
            if ($orb -gt 6.0) {
                throw "Named period aspect exceeds 6.0 deg orb: $($snapshot.date) $($fact.transiting_object) $($fact.aspect) orb=$orb"
            }
            if ([string]$fact.aspect -in @("square", "opposition")) {
                [void]$validTensionDates.Add([string]$snapshot.date)
            }
        }
    }
}
if ($validTensionDates.Count -gt 0) {
    $watchDates = @($reading.watch_days | ForEach-Object { [string]$_.date })
    if ($watchDates.Count -lt 1) {
        throw "Valid period tension exists but reading.watch_days is empty"
    }
    foreach ($date in $validTensionDates) {
        if ($watchDates -contains $date) {
            $bestDates = @($reading.best_days | ForEach-Object { [string]$_.date })
            if ($bestDates -contains $date) {
                throw "Period day $date overlaps best_days and watch_days"
            }
            break
        }
    }
}

$forbiddenPublicPattern = "period:|natal_|fake_|theme_code|evidence_key|snapshot|transit_exact|transit_active|moon_house_by_day|organization|relationship|energy|clarity|integration|\bfocused\b|\bfocus\b|\bsupportive\b|\bcareful\b|\bactive\b|\bmixed\b|\bfluid\b|\btense\b"
$toneLabelsPath = Join-Path $repoRoot "json_db\horoscope_tone_labels.json"
$toneLabelsDoc = Get-Content -LiteralPath $toneLabelsPath -Raw | ConvertFrom-Json
$allowedPublicTones = New-Object System.Collections.Generic.HashSet[string]
foreach ($row in @($toneLabelsDoc.data)) {
    if ($row.is_active -ne $false) {
        [void]$allowedPublicTones.Add([string]$row.label_fr)
    }
}
$detailProfilesPath = Join-Path $repoRoot "json_db\horoscope_detail_profiles.json"
$detailProfilesDoc = Get-Content -LiteralPath $detailProfilesPath -Raw | ConvertFrom-Json
$basicDetailProfile = @($detailProfilesDoc.data | Where-Object { $_.detail_profile_code -eq "basic_standard" -and $_.is_enabled -ne $false } | Select-Object -First 1)
if (-not $basicDetailProfile) {
    throw "Missing basic_standard in horoscope_detail_profiles.json"
}
$targetWordsMin = [int]$basicDetailProfile.target_words_min
$hardLimitWords = [int]$basicDetailProfile.hard_limit_words
$allPublicText = @(
    $reading.week_overview.title,
    $reading.week_overview.text,
    $reading.week_overview.trajectory,
    $reading.advice.main,
    $reading.advice.best_use,
    $reading.advice.avoid
)
$seenTexts = @{}
foreach ($day in $reading.daily_timeline) {
    if (-not $day.evidence_keys -or @($day.evidence_keys).Count -lt 1) {
        throw "Real period reading day $($day.date) missing evidence"
    }
    if ([string]$day.tone -match "^(focused|focus|supportive|careful|active|mixed|fluid|tense)$") {
        throw "Real period reading day $($day.date) exposes internal tone code: $($day.tone)"
    }
    if (-not $allowedPublicTones.Contains([string]$day.tone)) {
        throw "Real period reading day $($day.date) exposes tone outside horoscope_tone_labels: $($day.tone)"
    }
    $public = "$($day.day_label) $($day.theme) $($day.tone) $($day.text) $($day.advice)"
    if ($public -match $forbiddenPublicPattern -or $public -match "slot_|slot:|raw_transits") {
        throw "Technical code leaked in real period reading: $public"
    }
    $allPublicText += $public
    $normalized = ($day.text -replace "\s+", " ").Trim().ToLowerInvariant()
    if ($seenTexts.ContainsKey($normalized)) {
        throw "Real period daily_timeline is repetitive: $($day.date)"
    }
    $seenTexts[$normalized] = $true
}

$domainEvidenceSets = @()
foreach ($section in @($reading.domain_sections)) {
    if (-not $section.evidence_keys -or @($section.evidence_keys).Count -lt 1) {
        throw "Domain section $($section.domain) missing evidence"
    }
    $domainEvidenceSets += ((@($section.evidence_keys) | Sort-Object) -join "|")
    $allPublicText += "$($section.domain) $($section.title) $($section.text)"
}
if (($domainEvidenceSets | Sort-Object -Unique).Count -lt [Math]::Min(2, @($reading.domain_sections).Count)) {
    throw "Domain sections reuse the same evidence set"
}
foreach ($marker in @($reading.key_days) + @($reading.best_days) + @($reading.watch_days)) {
    $allPublicText += "$($marker.title) $($marker.reason)"
}
foreach ($evidence in @($reading.evidence_summary)) {
    if (-not $includedDates.Contains([string]$evidence.date)) {
        throw "Evidence summary date outside period: $($evidence.date)"
    }
    if (-not $allowedEvidenceKeys.Contains([string]$evidence.evidence_key)) {
        throw "Evidence summary invented evidence_key: $($evidence.evidence_key)"
    }
    $allPublicText += "$($evidence.label)"
}
$joinedPublicText = ($allPublicText -join "`n")
if ($joinedPublicText -match $forbiddenPublicPattern -or $joinedPublicText -match "slot_|slot:|raw_transits") {
    throw "Technical code leaked in public period response"
}
$wordCount = @($joinedPublicText -split "\s+" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count
if ($wordCount -lt $targetWordsMin -or $wordCount -gt $hardLimitWords) {
    throw "Real period public word count must be in [$targetWordsMin,$hardLimitWords], got $wordCount"
}

$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$jsonPath = Join-Path $OutputDir "horoscope_basic_next_7_days_real_$stamp.json"
$status | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
Write-Host "Saved real horoscope period output: $jsonPath" -ForegroundColor Green
