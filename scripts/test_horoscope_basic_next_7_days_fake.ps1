param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [string]$ApiKey = "",
    [string]$IdempotencyKey = "",
    [string]$AnchorDate = "2026-06-07",
    [switch]$AssumeFakeProviderConfigured
)

$ErrorActionPreference = "Stop"

. "$PSScriptRoot\lib\astral_http_auth.ps1"
. "$PSScriptRoot\lib\horoscope_e2e_fake_provider.ps1"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Import-AstralDotEnv -RepoRoot $repoRoot
$fakeProviderEnabled = $false
trap {
    if ($fakeProviderEnabled) {
        Restore-HoroscopeE2eLlmProvider -RepoRoot $repoRoot
    }
    break
}

if (-not $AssumeFakeProviderConfigured) {
    Enable-HoroscopeE2eFakeLlmProvider -RepoRoot $repoRoot
    $fakeProviderEnabled = $true
}

function Assert-HttpReady {
    param([string]$Url)
    $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 10 -SkipHttpErrorCheck
    if ($response.StatusCode -lt 200 -or $response.StatusCode -ge 300) {
        throw "Readiness failed for $Url : HTTP $($response.StatusCode)"
    }
}

$headers = New-AstralAuthHeaders -Service llm
if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
    $headers["Authorization"] = "Bearer $ApiKey"
    $headers["X-API-Key"] = $ApiKey
}
if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "horoscope-period-basic-next-7-fake-$([guid]::NewGuid().ToString('N'))"
}
$headers["Idempotency-Key"] = $IdempotencyKey
$calcHeaders = New-AstralAuthHeaders -Service calculator

Assert-HttpReady "$CalculatorUrl/health/ready"
Assert-HttpReady "$BaseUrl/health/ready"

$services = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/services" -Headers $headers
$service = @($services.services | Where-Object { $_.service_code -eq "horoscope_basic_next_7_days_natal" })[0]
if (-not $service) {
    throw "Service horoscope_basic_next_7_days_natal not listed"
}
if ($service.availability -ne "beta") {
    throw "Service horoscope_basic_next_7_days_natal must be beta for fake smoke, got $($service.availability)"
}

$natalRequestPath = Join-Path $repoRoot "contracts\integration\examples\natal_calculation_request_v1.paris_1990.json"
if (-not (Test-Path -LiteralPath $natalRequestPath)) {
    throw "Fixture introuvable : $natalRequestPath"
}
$natalRequest = Get-Content -LiteralPath $natalRequestPath -Raw | ConvertFrom-Json
$natalResponse = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/calculations/natal" -Headers $calcHeaders -Body $natalRequest
if ($natalResponse.calculation_result.status -ne "completed") {
    throw "Natal calculation did not complete"
}
$chartCalculationId = [string]$natalResponse.calculation_result.chart_calculation_id
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
    throw "Natal calculation did not return chart_calculation_id"
}

$bodyObject = @{
    service_code = "horoscope_basic_next_7_days_natal"
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

$completed = $null
$lastStatus = $null
for ($i = 0; $i -lt 45; $i++) {
    Start-Sleep -Seconds 2
    $status = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/jobs/$($submit.run_id)" -Headers $headers
    $lastStatus = $status
    if ($status.status -eq "completed") {
        $completed = $status
        break
    }
    if ($status.status -eq "failed" -or $status.status -eq "safety_rejected") {
        $status | ConvertTo-Json -Depth 30
        throw "Horoscope period fake job ended with $($status.status)"
    }
}
if (-not $completed) {
    if ($lastStatus) {
        $lastStatus | ConvertTo-Json -Depth 30
    }
    throw "Timeout waiting for horoscope period fake job"
}

$reading = $completed.result.reading
if ([string]$reading.quality.provider -ne "fake") {
    throw "Horoscope period fake smoke expected provider=fake, got $($reading.quality.provider)"
}
if ($reading.contract_version -ne "horoscope_period_response") {
    throw "Unexpected period horoscope contract version: $($reading.contract_version)"
}
if ($reading.service_code -ne "horoscope_basic_next_7_days_natal") {
    throw "Unexpected service_code in reading"
}
if (-not $completed.result.interpretation_request.period_resolution) {
    throw "Missing period_resolution in interpretation request"
}
if (-not $completed.result.interpretation_request.scan_plan) {
    throw "Missing scan_plan in interpretation request"
}
if (@($reading.daily_timeline).Count -ne 7) {
    throw "daily_timeline must contain exactly 7 entries"
}

$includedDates = @($reading.period_resolution.included_dates)
$timelineDates = @($reading.daily_timeline | ForEach-Object { $_.date })
if ($includedDates.Count -ne 7 -or $timelineDates.Count -ne 7) {
    throw "Expected 7 included dates and 7 timeline dates"
}
for ($i = 0; $i -lt 7; $i++) {
    if ($includedDates[$i] -ne $timelineDates[$i]) {
        throw "Timeline date mismatch at index $i"
    }
}

$bestDates = @($reading.best_days | ForEach-Object { $_.date })
$watchDates = @($reading.watch_days | ForEach-Object { $_.date })
foreach ($date in $bestDates) {
    if ($watchDates -contains $date) {
        throw "Best/watch overlap for date $date"
    }
}
foreach ($day in $reading.daily_timeline) {
    if (-not $day.evidence_keys -or @($day.evidence_keys).Count -lt 1) {
        throw "Timeline day $($day.date) missing evidence_keys"
    }
    $public = "$($day.day_label) $($day.theme) $($day.text) $($day.advice)"
    if ($public -match "slot_|slot:|raw_transits") {
        throw "Technical code leaked in period timeline"
    }
}

$replay = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/jobs" -Headers $headers -ContentType "application/json" -Body $body
if ($replay.run_id -ne $submit.run_id) {
    throw "Idempotent replay returned a different run_id"
}

$completed | ConvertTo-Json -Depth 30
if ($fakeProviderEnabled) {
    Restore-HoroscopeE2eLlmProvider -RepoRoot $repoRoot
    $fakeProviderEnabled = $false
}
