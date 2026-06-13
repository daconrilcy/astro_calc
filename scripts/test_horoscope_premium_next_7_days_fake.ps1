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
    $IdempotencyKey = "horoscope-period-premium-next-7-fake-$([guid]::NewGuid().ToString('N'))"
}
$headers["Idempotency-Key"] = $IdempotencyKey
$calcHeaders = New-AstralAuthHeaders -Service calculator

Assert-HttpReady "$CalculatorUrl/health/ready"
Assert-HttpReady "$BaseUrl/health/ready"

$services = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/services" -Headers $headers
$service = @($services.services | Where-Object { $_.service_code -eq "horoscope_premium_next_7_days_natal" })[0]
if (-not $service) {
    throw "Service horoscope_premium_next_7_days_natal not listed"
}
if ($service.availability -ne "beta" -and $service.availability -ne "active") {
    throw "Service horoscope_premium_next_7_days_natal must be beta or active for fake smoke, got $($service.availability)"
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
    service_code = "horoscope_premium_next_7_days_natal"
    payload = @{
        anchor_date = $AnchorDate
        timezone = "Europe/Paris"
        target_language_code = "fr"
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
        throw "Horoscope premium period fake job ended with $($status.status)"
    }
}
if (-not $completed) {
    if ($lastStatus) {
        $lastStatus | ConvertTo-Json -Depth 30
    }
    throw "Timeout waiting for horoscope premium period fake job"
}

$reading = $completed.result.reading
$writerRequest = $completed.result.writer_request
if (-not $writerRequest) {
    $writerRequest = $completed.result.interpretation_request
}
if ([string]$reading.quality.provider -ne "fake") {
    throw "Horoscope premium period fake smoke expected provider=fake, got $($reading.quality.provider)"
}
if ($reading.contract_version -ne "horoscope_period_response") {
    throw "Unexpected period horoscope contract version: $($reading.contract_version)"
}
if ($reading.service_code -ne "horoscope_premium_next_7_days_natal") {
    throw "Unexpected service_code in reading"
}
if (-not $writerRequest) {
    throw "Missing writer_request in completed result"
}
if ($writerRequest.scan_plan.scan_profile_code -ne "six_hour_7_days") {
    throw "Premium scan profile must be six_hour_7_days"
}
if ([int]$writerRequest.scan_plan.snapshot_count -ne 28 -or @($writerRequest.scan_plan.snapshots).Count -ne 28) {
    throw "Premium scan must contain exactly 28 snapshots"
}
if (@($reading.daily_timeline).Count -ne 7) {
    throw "daily_timeline must contain exactly 7 entries"
}
if (-not $reading.strategy) {
    throw "Premium reading missing strategy"
}
if (@($reading.domain_sections).Count -lt 3 -or @($reading.domain_sections).Count -gt 5) {
    throw "Premium domain_sections must contain 3 to 5 entries"
}
if (@($reading.best_windows).Count -lt 1) {
    throw "Premium best_windows must be non-empty"
}

$snapshotKeys = @($writerRequest.scan_plan.snapshots | ForEach-Object { $_.snapshot_key })
foreach ($field in @("best_windows", "watch_windows")) {
    foreach ($window in @($reading.$field)) {
        if (-not $window.evidence_keys -or @($window.evidence_keys).Count -lt 1) {
            throw "$field window $($window.date) missing evidence_keys"
        }
        if (-not $window.source_snapshot_keys -or @($window.source_snapshot_keys).Count -lt 1) {
            throw "$field window $($window.date) missing source_snapshot_keys"
        }
        foreach ($key in @($window.source_snapshot_keys)) {
            if ($snapshotKeys -notcontains $key) {
                throw "$field references unknown source_snapshot_key $key"
            }
        }
    }
}
if (@($reading.watch_windows).Count -eq 0 -and $reading.watch_summary.status -ne "none") {
    throw "watch_windows empty requires watch_summary.status = none"
}

$bestWindowKeys = @($reading.best_windows | ForEach-Object { (@($_.source_snapshot_keys) -join "|") })
foreach ($window in @($reading.watch_windows)) {
    $identity = @($window.source_snapshot_keys) -join "|"
    if ($bestWindowKeys -contains $identity) {
        throw "Best/watch window overlap for $identity"
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
