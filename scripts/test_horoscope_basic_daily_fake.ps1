param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$ApiKey = "dev-local",
    [string]$IdempotencyKey = "horoscope-basic-daily-fake"
)

$ErrorActionPreference = "Stop"

$body = @{
    service_code = "horoscope_basic_daily_natal_3_slots"
    payload = @{
        date = "2026-06-06"
        timezone = "Europe/Paris"
        target_language = "fr"
        chart_calculation_id = "123"
        audience_level = "general"
    }
    user_language = "fr"
    audience_level = "beginner"
} | ConvertTo-Json -Depth 10

$headers = @{
    "Content-Type" = "application/json"
    "Idempotency-Key" = $IdempotencyKey
    "X-API-Key" = $ApiKey
}

$submit = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/jobs" -Headers $headers -Body $body
if (-not $submit.run_id) {
    throw "Missing run_id in submit response"
}

for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Seconds 2
    $status = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/jobs/$($submit.run_id)" -Headers @{ "X-API-Key" = $ApiKey }
    if ($status.status -eq "completed") {
        if ($status.result.reading.contract_version -ne "horoscope_response_v1") {
            throw "Unexpected horoscope contract version"
        }
        $status | ConvertTo-Json -Depth 20
        exit 0
    }
    if ($status.status -eq "failed" -or $status.status -eq "safety_rejected") {
        $status | ConvertTo-Json -Depth 20
        throw "Horoscope fake job ended with $($status.status)"
    }
}

throw "Timeout waiting for horoscope fake job"
