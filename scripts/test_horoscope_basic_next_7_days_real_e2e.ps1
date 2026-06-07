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

$submit = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/jobs" -Headers $headers -ContentType "application/json" -Body $body
$deadline = (Get-Date).AddSeconds($TimeoutSec)
$status = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 3
    $status = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/jobs/$($submit.run_id)" -Headers $headers
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

Assert-UtcString ([string]$reading.period_resolution.start_datetime_utc) "period_resolution.start_datetime_utc"
Assert-UtcString ([string]$reading.period_resolution.end_datetime_utc) "period_resolution.end_datetime_utc"
foreach ($snapshot in @($calculation.scan_plan.snapshots)) {
    Assert-UtcString ([string]$snapshot.reference_datetime_utc) "scan_plan snapshot $($snapshot.snapshot_key)"
}
foreach ($snapshot in @($calculation.snapshots)) {
    Assert-UtcString ([string]$snapshot.reference_datetime_utc) "calculation snapshot $($snapshot.snapshot_key)"
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
if ([string]::IsNullOrWhiteSpace($provider) -or $provider -eq "fake") {
    throw "Real period writer used invalid provider: '$provider'"
}

$forbiddenPublicPattern = "period:|natal_|fake_|theme_code|evidence_key|snapshot|transit_exact|transit_active|moon_house_by_day|organization|relationship|energy|clarity|integration"
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
    $public = "$($day.day_label) $($day.theme) $($day.text) $($day.advice)"
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
    $allPublicText += "$($evidence.label)"
}
$joinedPublicText = ($allPublicText -join "`n")
if ($joinedPublicText -match $forbiddenPublicPattern -or $joinedPublicText -match "slot_|slot:|raw_transits") {
    throw "Technical code leaked in public period response"
}

$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$jsonPath = Join-Path $OutputDir "horoscope_basic_next_7_days_real_$stamp.json"
$status | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
Write-Host "Saved real horoscope period output: $jsonPath" -ForegroundColor Green
