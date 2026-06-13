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
    $iwrParams = @{
        Uri = $Url
        UseBasicParsing = $true
        TimeoutSec = 10
    }
    if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey("SkipHttpErrorCheck")) {
        $iwrParams["SkipHttpErrorCheck"] = $true
    }
    $response = Invoke-WebRequest @iwrParams
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
    $IdempotencyKey = "horoscope-period-free-next-7-fake-$([guid]::NewGuid().ToString('N'))"
}
$headers["Idempotency-Key"] = $IdempotencyKey
$calcHeaders = New-AstralAuthHeaders -Service calculator

Assert-HttpReady "$CalculatorUrl/health/ready"
Assert-HttpReady "$BaseUrl/health/ready"

$services = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/services" -Headers $headers
$service = @($services.services | Where-Object { $_.service_code -eq "horoscope_free_next_7_days_natal" })[0]
if (-not $service) {
    throw "Service horoscope_free_next_7_days_natal not listed"
}
if ($service.availability -ne "beta" -and $service.availability -ne "active") {
    throw "Service horoscope_free_next_7_days_natal must be beta or active for fake smoke, got $($service.availability)"
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
        throw "Horoscope free period fake job ended with $($status.status)"
    }
}
if (-not $completed) {
    if ($lastStatus) {
        $lastStatus | ConvertTo-Json -Depth 30
    }
    throw "Timeout waiting for horoscope free period fake job"
}

$reading = $completed.result.reading
$writerRequest = $completed.result.writer_request
if (-not $writerRequest) {
    $writerRequest = $completed.result.interpretation_request
}
if ([string]$reading.quality.provider -ne "fake") {
    throw "Horoscope free period fake smoke expected provider=fake, got $($reading.quality.provider)"
}
if ($reading.contract_version -ne "horoscope_period_response") {
    throw "Unexpected period horoscope contract version: $($reading.contract_version)"
}
if ($reading.service_code -ne "horoscope_free_next_7_days_natal") {
    throw "Unexpected service_code in reading"
}
if (-not $writerRequest) {
    throw "Missing writer_request in completed result"
}
if ($writerRequest.scan_plan.scan_profile_code -ne "daily_noon_7_days") {
    throw "Free scan profile must be daily_noon_7_days"
}
if ([int]$writerRequest.scan_plan.snapshot_count -ne 7 -or @($writerRequest.scan_plan.snapshots).Count -ne 7) {
    throw "Free scan must contain exactly 7 snapshots"
}
foreach ($field in @("week_overview", "daily_timeline", "best_days", "watch_days", "best_windows", "watch_windows", "domain_sections", "strategy")) {
    if ($reading.PSObject.Properties.Name -contains $field) {
        throw "Free reading leaked forbidden field: $field"
    }
}
foreach ($field in @("summary", "dominant_theme", "key_days", "advice", "watch_summary", "evidence_summary")) {
    if (-not ($reading.PSObject.Properties.Name -contains $field)) {
        throw "Free reading missing $field"
    }
}

$includedDates = @($reading.period_resolution.included_dates)
if ($includedDates.Count -ne 7) {
    throw "Expected 7 included dates in period_resolution"
}
$keyDays = @($reading.key_days)
if ($keyDays.Count -lt 1 -or $keyDays.Count -gt 2) {
    throw "Free reading key_days must contain 1 to 2 entries"
}
foreach ($day in $keyDays) {
    if (-not $day.evidence_keys -or @($day.evidence_keys).Count -lt 1) {
        throw "Free key day $($day.date) missing evidence_keys"
    }
    if ($includedDates -notcontains [string]$day.date) {
        throw "Free key day date outside included dates: $($day.date)"
    }
    $public = "$($day.title) $($day.reason)"
    if ($public -match "meilleur|favorabl|créneau|creneau|slot_|slot:|raw_transits") {
        throw "Forbidden best-day leak or technical code in free key day"
    }
}

$watchStatus = [string]$reading.watch_summary.status
if ($watchStatus -notin @("none", "low", "active")) {
    throw "Free watch_summary.status must be none, low or active, got '$watchStatus'"
}
if ($watchStatus -ne "none" -and @($reading.watch_summary.evidence_keys).Count -lt 1) {
    throw "Free watch_summary must reference evidence when status is $watchStatus"
}
if (@($reading.evidence_summary).Count -lt 1 -or @($reading.evidence_summary).Count -gt 3) {
    throw "Free evidence_summary must contain 1 to 3 entries"
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
