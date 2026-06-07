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

$internalTones = @($interpretation.daily_plans | ForEach-Object { [string]$_.tone } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
if ($internalTones.Count -lt 2) {
    throw "Real period interpretation should expose at least two distinct internal daily tones, got: $($internalTones -join ', ')"
}

$validTensionDates = New-Object System.Collections.Generic.HashSet[string]
$eventScores = @()
$previousScore = $null
foreach ($event in @($interpretation.period_events)) {
    $tone = [string]$event.tone
    $aspect = [string]$event.aspect
    $date = [string]$event.date
    $score = [double]$event.score
    if ($score -le 0.0 -or $score -gt 1.0) {
        throw "Period event score out of range for $date`: $score"
    }
    if ($null -ne $previousScore -and $score -gt $previousScore) {
        throw "Period events are not sorted by score desc: $score after $previousScore"
    }
    $previousScore = $score
    $eventScores += ([Math]::Round($score, 2))
    if ($tone -eq "careful" -or $aspect -in @("square", "opposition")) {
        [void]$validTensionDates.Add($date)
    }
}
if (($eventScores | Sort-Object -Unique).Count -lt 2) {
    throw "Period event scores are not discriminating: $($eventScores -join ', ')"
}
foreach ($snapshot in @($calculation.snapshots)) {
    foreach ($fact in @($snapshot.transits_to_natal)) {
        $aspectValue = [string]$fact.aspect
        $isContextFact = $fact.fact_type -in @("transit_context", "moon_house_by_day") -or [string]::IsNullOrWhiteSpace($aspectValue)
        if ($isContextFact -and $null -ne $fact.orb_deg) {
            throw "Context period fact exposes orb_deg: $($snapshot.date) $($fact.fact_type) $($fact.transiting_object) orb=$($fact.orb_deg)"
        }
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
    foreach ($skyAspect in @($snapshot.current_sky_aspects)) {
        if ([string]$skyAspect.aspect -eq "context" -and $null -ne $skyAspect.orb_deg) {
            throw "Context current_sky_aspects entry exposes orb_deg: $($snapshot.date) orb=$($skyAspect.orb_deg)"
        }
    }
}
$readingWatchDays = @($reading.watch_days)
$watchSummaryStatus = [string]$reading.watch_summary.status
if ($validTensionDates.Count -eq 0) {
    if ($readingWatchDays.Count -ne 0 -or $watchSummaryStatus -ne "none") {
        throw "No valid period tension should expose empty watch_days and watch_summary.status=none"
    }
}
if ($validTensionDates.Count -gt 0) {
    $watchDates = @($readingWatchDays | ForEach-Object { [string]$_.date })
    if ($watchDates.Count -lt 1) {
        throw "Valid period tension exists but reading.watch_days is empty"
    }
    if ($watchSummaryStatus -ne "active") {
        throw "Valid period tension must expose watch_summary.status=active"
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
if (@($reading.key_days).Count -gt 2) {
    throw "Real period key_days must contain at most 2 entries"
}
$keyDates = @($reading.key_days | ForEach-Object { [string]$_.date })
$bestDatesForOverlap = @($reading.best_days | ForEach-Object { [string]$_.date })
foreach ($date in $bestDatesForOverlap) {
    if ($keyDates -contains $date) {
        throw "Real period best_days must not duplicate key_days date $date"
    }
}
$bestThemes = @($reading.best_days | ForEach-Object { [string]$_.title })
if (($bestThemes | Sort-Object -Unique).Count -ne $bestThemes.Count) {
    throw "Real period best_days should use distinct qualitative titles/themes"
}

$forbiddenPublicPattern = "period:|natal_|fake_|theme_code|evidence_key|snapshot|transit_exact|transit_active|moon_house_by_day|organization|relationship|energy|clarity|integration|\bfocused\b|\bfocus\b|\bsupportive\b|\bcareful\b|\bactive\b|\bmixed\b|\bfluid\b|\btense\b"
$forbiddenGuidancePattern = "Personnaliser ce signal|Relier ce signal|Relier ce domaine|rester sur un conseil générique|donne le relief principal|en prose utilisateur|summary_hint|advice_hint|personalization_hint|natal_focus_hint"
$forbiddenMetaPersonalizationPattern = "plus personnel que générique|conseil générique|ce qui rend le conseil|cette nuance reste liée|avec un écho personnel autour de|secteur personnel activé|adaptez le geste au secteur personnel|la lecture relie|zones personnelles déjà mises en évidence|zones personnelles|zones natales activées|secteurs personnels|thème natal comme fil directeur|le point d'appui concerne"
$brokenSentencePattern = "(?im)(\b(et|à|de|pour|avec|sans|dans|sur|vers|la|le|les|des|du|au|aux|un|une|ce|cet|cette)\s*[.!?]\s*$|\b(à|de)\s+(la|l')\s*[.!?]\s*$)"
function Test-BadFrenchColonSpacing {
    param([AllowEmptyString()][string]$Text)
    for ($i = 0; $i -lt $Text.Length; $i++) {
        if ($Text[$i] -ne ':') {
            continue
        }
        $before = if ($i -gt 0) { $Text[$i - 1] } else { [char]0 }
        $after = if ($i + 1 -lt $Text.Length) { $Text[$i + 1] } else { [char]0 }
        if ([char]::IsDigit($before) -and [char]::IsDigit($after)) {
            continue
        }
        if (($i -gt 0 -and -not [char]::IsWhiteSpace($before)) -or ($i + 1 -lt $Text.Length -and -not [char]::IsWhiteSpace($after))) {
            return $true
        }
    }
    return $false
}
function Assert-PublicPeriodTextQuality {
    param(
        [Parameter(Mandatory=$true)][AllowEmptyString()][string]$Text,
        [Parameter(Mandatory=$true)][string]$Label
    )
    if ($Text -match $forbiddenPublicPattern -or $Text -match "slot_|slot:|raw_transits") {
        throw "Technical code leaked in ${Label}: $Text"
    }
    if ($Text -match $forbiddenGuidancePattern) {
        throw "Internal writer guidance leaked in ${Label}: $Text"
    }
    if ($Text -match $forbiddenMetaPersonalizationPattern) {
        throw "Meta personalization language leaked in ${Label}: $Text"
    }
    if ($Text -match $brokenSentencePattern) {
        throw "Broken sentence detected in ${Label}: $Text"
    }
    if (Test-BadFrenchColonSpacing -Text $Text) {
        throw "French typography colon spacing failed in ${Label}: $Text"
    }
}
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
    $reading.watch_summary.text,
    $reading.advice.main,
    $reading.advice.best_use,
    $reading.advice.avoid
)
$seenTexts = @{}
$personalizedDayCount = 0
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
    Assert-PublicPeriodTextQuality -Text ([string]$day.day_label) -Label "daily_timeline[$($day.date)].day_label"
    Assert-PublicPeriodTextQuality -Text ([string]$day.theme) -Label "daily_timeline[$($day.date)].theme"
    Assert-PublicPeriodTextQuality -Text ([string]$day.tone) -Label "daily_timeline[$($day.date)].tone"
    Assert-PublicPeriodTextQuality -Text ([string]$day.text) -Label "daily_timeline[$($day.date)].text"
    Assert-PublicPeriodTextQuality -Text ([string]$day.advice) -Label "daily_timeline[$($day.date)].advice"
    if ($public -match "thème natal|zone natale|maison|sensibilité|besoins émotionnels|communiquer|penser|attachement|agir|responsabilité|limites|relations directes|besoin de sens|habitudes|rythme de travail") {
        $personalizedDayCount += 1
    }
    $allPublicText += $public
    $normalized = ($day.text -replace "\s+", " ").Trim().ToLowerInvariant()
    if ($seenTexts.ContainsKey($normalized)) {
        throw "Real period daily_timeline is repetitive: $($day.date)"
    }
    $seenTexts[$normalized] = $true
}
if ($personalizedDayCount -lt 4) {
    throw "Real period reading must personalize at least 4 daily_timeline entries, got $personalizedDayCount"
}

$domainEvidenceSets = @()
$domainCount = @($reading.domain_sections).Count
if ($domainCount -lt 2 -or $domainCount -gt 4) {
    throw "Real period reading domain_sections must contain 2 to 4 entries, got $domainCount"
}
foreach ($section in @($reading.domain_sections)) {
    if (-not $section.evidence_keys -or @($section.evidence_keys).Count -lt 1) {
        throw "Domain section $($section.domain) missing evidence"
    }
    if ("$($section.domain) $($section.title) $($section.text)" -notmatch "thème natal|zone natale|maison|sensibilité|besoins émotionnels|communiquer|penser|attachement|agir|responsabilité|limites|relations directes|besoin de sens|habitudes|rythme de travail") {
        throw "Domain section $($section.domain) missing natal personalization"
    }
    Assert-PublicPeriodTextQuality -Text ([string]$section.domain) -Label "domain_sections[$($section.domain)].domain"
    Assert-PublicPeriodTextQuality -Text ([string]$section.title) -Label "domain_sections[$($section.domain)].title"
    Assert-PublicPeriodTextQuality -Text ([string]$section.text) -Label "domain_sections[$($section.domain)].text"
    $domainEvidenceSets += ((@($section.evidence_keys) | Sort-Object) -join "|")
    $allPublicText += "$($section.domain) $($section.title) $($section.text)"
}
if (($domainEvidenceSets | Sort-Object -Unique).Count -lt [Math]::Min(2, @($reading.domain_sections).Count)) {
    throw "Domain sections reuse the same evidence set"
}
foreach ($marker in @($reading.key_days) + @($reading.best_days) + @($reading.watch_days)) {
    if ($marker.PSObject.Properties.Name -contains "fallback_reason" -and $marker.fallback_reason -eq "") {
        throw "Period marker exposes empty fallback_reason for $($marker.date)"
    }
    Assert-PublicPeriodTextQuality -Text ([string]$marker.title) -Label "period marker $($marker.date).title"
    Assert-PublicPeriodTextQuality -Text ([string]$marker.reason) -Label "period marker $($marker.date).reason"
    $allPublicText += "$($marker.title) $($marker.reason)"
}
foreach ($marker in @($interpretation.key_days) + @($interpretation.best_days) + @($interpretation.watch_days)) {
    if ($marker.PSObject.Properties.Name -contains "fallback_reason" -and $marker.fallback_reason -eq "") {
        throw "Period interpretation marker exposes empty fallback_reason for $($marker.date)"
    }
}
foreach ($evidence in @($reading.evidence_summary)) {
    if (-not $includedDates.Contains([string]$evidence.date)) {
        throw "Evidence summary date outside period: $($evidence.date)"
    }
    if (-not $allowedEvidenceKeys.Contains([string]$evidence.evidence_key)) {
        throw "Evidence summary invented evidence_key: $($evidence.evidence_key)"
    }
    Assert-PublicPeriodTextQuality -Text ([string]$evidence.label) -Label "evidence_summary[$($evidence.evidence_key)].label"
    $allPublicText += "$($evidence.label)"
}
$joinedPublicText = ($allPublicText -join "`n")
Assert-PublicPeriodTextQuality -Text ([string]$reading.week_overview.title) -Label "week_overview.title"
Assert-PublicPeriodTextQuality -Text ([string]$reading.week_overview.text) -Label "week_overview.text"
Assert-PublicPeriodTextQuality -Text ([string]$reading.week_overview.trajectory) -Label "week_overview.trajectory"
Assert-PublicPeriodTextQuality -Text ([string]$reading.watch_summary.text) -Label "watch_summary.text"
Assert-PublicPeriodTextQuality -Text ([string]$reading.advice.main) -Label "advice.main"
Assert-PublicPeriodTextQuality -Text ([string]$reading.advice.best_use) -Label "advice.best_use"
Assert-PublicPeriodTextQuality -Text ([string]$reading.advice.avoid) -Label "advice.avoid"
Assert-PublicPeriodTextQuality -Text $joinedPublicText -Label "public period response"
$overviewText = "$($reading.week_overview.text) $($reading.week_overview.trajectory)"
$overviewRepetition = [regex]::Matches($overviewText.ToLowerInvariant(), [regex]::Escape("thème natal comme fil directeur")).Count
if ($overviewRepetition -gt 1) {
    throw "Week overview repeats 'thème natal comme fil directeur' $overviewRepetition times"
}
foreach ($phrase in @("restez concret", "gardez une marge", "clarifier", "ajuster", "intégrer")) {
    $matches = [regex]::Matches($joinedPublicText.ToLowerInvariant(), [regex]::Escape($phrase)).Count
    if ($matches -gt 2) {
        throw "Repeated period vocabulary '$phrase' appears $matches times"
    }
}
$wordCount = @($joinedPublicText -split "\s+" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count
if ($wordCount -lt $targetWordsMin -or $wordCount -gt $hardLimitWords) {
    throw "Real period public word count must be in [$targetWordsMin,$hardLimitWords], got $wordCount"
}

$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$jsonPath = Join-Path $OutputDir "horoscope_basic_next_7_days_real_$stamp.json"
$status | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
Write-Host "Saved real horoscope period output: $jsonPath" -ForegroundColor Green
