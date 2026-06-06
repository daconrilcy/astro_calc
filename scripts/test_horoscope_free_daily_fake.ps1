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
    $IdempotencyKey = "horoscope-free-daily-fake-$([guid]::NewGuid().ToString('N'))"
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
    service_code = "horoscope_free_daily"
    payload = @{
        date = "2026-06-06"
        timezone = "Europe/Paris"
        target_language = "fr"
        chart_calculation_id = $chartCalculationId
        audience_level = "general"
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
        if ($status.result.reading.contract_version -ne "horoscope_response_v1") {
            throw "Unexpected horoscope contract version"
        }
        if ($status.result.reading.service_code -ne "horoscope_free_daily") {
            throw "Unexpected horoscope service_code"
        }
        if (-not $status.result.interpretation_request.slots -or $status.result.interpretation_request.slots.Count -ne 1) {
            throw "Missing single internal day slot"
        }
        if ($status.result.interpretation_request.slots[0].slot_code -ne "day") {
            throw "Unexpected internal slot code"
        }
        if ($status.result.reading.slots) {
            throw "Free reading must not expose public slots"
        }
        foreach ($field in @("summary", "advice", "watch_point", "evidence_keys", "quality")) {
            if (-not $status.result.reading.$field) {
                throw "Free reading missing $field"
            }
        }
        $publicText = @(
            $status.result.reading.summary.title
            $status.result.reading.summary.text
            $status.result.reading.advice
            $status.result.reading.watch_point
        ) -join "`n"
        if ($publicText -match "slot:day" -or $publicText -match "\bday\b") {
            throw "Internal day slot leaked into public reading"
        }
        if ($publicText -match "les signaux du jour invitent") {
            throw "Generic horoscope wording leaked into fake reading"
        }
        $status | ConvertTo-Json -Depth 20
        exit 0
    }
    if ($status.status -eq "failed" -or $status.status -eq "safety_rejected") {
        $status | ConvertTo-Json -Depth 20
        throw "Horoscope free fake job ended with $($status.status)"
    }
}

throw "Timeout waiting for horoscope free fake job"
