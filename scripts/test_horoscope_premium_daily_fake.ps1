param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [string]$ApiKey = "",
    [string]$IdempotencyKey = ""
)

$ErrorActionPreference = "Stop"

. "$PSScriptRoot\lib\astral_http_auth.ps1"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Import-AstralDotEnv -RepoRoot $repoRoot
$headers = New-AstralAuthHeaders -Service llm
if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
    $headers["Authorization"] = "Bearer $ApiKey"
    $headers["X-API-Key"] = $ApiKey
}
if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "horoscope-premium-daily-local-fake-$([guid]::NewGuid().ToString('N'))"
}
$headers["Idempotency-Key"] = $IdempotencyKey
$calcHeaders = New-AstralAuthHeaders -Service calculator

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

$body = @{
    service_code = "horoscope_premium_daily_local_2h_slots"
    payload = @{
        date = "2026-06-06"
        timezone = "Europe/Paris"
        target_language = "fr"
        chart_calculation_id = $chartCalculationId
        location = @{
            latitude = 48.8566
            longitude = 2.3522
            label = "Paris"
        }
        audience_level = "general"
        detail_level = "premium_rich"
    }
    user_language = "fr"
    audience_level = "beginner"
} | ConvertTo-Json -Depth 10

$submit = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/jobs" -Headers $headers -Body $body
if (-not $submit.run_id) {
    throw "Missing run_id in submit response"
}

for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Seconds 2
    $status = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/jobs/$($submit.run_id)" -Headers $headers
    if ($status.status -eq "completed") {
        if ([string]$status.result.reading.quality.provider -ne "fake") {
            throw "Premium horoscope fake smoke expected provider=fake, got $($status.result.reading.quality.provider)"
        }
        if ($status.result.reading.contract_version -ne "horoscope_response_v1") {
            throw "Unexpected horoscope contract version"
        }
        if ($status.result.reading.service_code -ne "horoscope_premium_daily_local_2h_slots") {
            throw "Unexpected service_code in premium reading"
        }
        if (-not $status.result.reading.timeline -or $status.result.reading.timeline.Count -ne 12) {
            throw "Premium reading timeline must contain exactly 12 entries"
        }
        if ($status.result.reading.timeline[0].slot_label -ne "00:00–02:00") {
            throw "Unexpected first premium timeline label"
        }
        if ($status.result.reading.timeline[11].slot_label -ne "22:00–00:00") {
            throw "Unexpected last premium timeline label"
        }
        if (-not $status.result.reading.best_slots -or -not $status.result.reading.watch_slots) {
            throw "Premium reading must include best_slots and watch_slots"
        }
        foreach ($slot in $status.result.reading.timeline) {
            if (-not $slot.evidence_keys -or $slot.evidence_keys.Count -lt 1) {
                throw "Premium timeline slot $($slot.slot_label) missing evidence_keys"
            }
            if (($slot.text -match "slot_") -or ($slot.title -match "slot_")) {
                throw "Technical slot code leaked in premium timeline"
            }
        }
        [pscustomobject]@{
            status         = $status.status
            run_id         = $status.run_id
            service_code   = $status.result.reading.service_code
            timeline_count = @($status.result.reading.timeline).Count
            best_slots     = @($status.result.reading.best_slots).Count
            watch_slots    = @($status.result.reading.watch_slots).Count
        } | ConvertTo-Json -Depth 4
        exit 0
    }
    if ($status.status -eq "failed" -or $status.status -eq "safety_rejected") {
        $status | ConvertTo-Json -Depth 20
        throw "Premium horoscope fake job ended with $($status.status)"
    }
}

throw "Timeout waiting for premium horoscope fake job"
