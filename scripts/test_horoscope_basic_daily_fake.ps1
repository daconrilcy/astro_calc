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
    $IdempotencyKey = "horoscope-basic-daily-slot-based-fake-$([guid]::NewGuid().ToString('N'))"
}
$headers["Idempotency-Key"] = $IdempotencyKey
$calcHeaders = New-AstralAuthHeaders -Service calculator

$natalRequestPath = Join-Path $repoRoot "contracts\integration\examples\natal_calculation_request_v1.paris_1990.json"
if (-not (Test-Path -LiteralPath $natalRequestPath)) {
    throw "Fixture introuvable : $natalRequestPath"
}
$natalRequest = Get-Content -LiteralPath $natalRequestPath -Raw | ConvertFrom-Json
$natalResponse = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/internal/calculations/natal" -Headers $calcHeaders -Body $natalRequest
if ($natalResponse.calculation_result.status -ne "completed") {
    throw "Natal calculation did not complete"
}
$chartCalculationId = [string]$natalResponse.calculation_result.chart_calculation_id
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
    throw "Natal calculation did not return chart_calculation_id"
}

$body = @{
    service_code = "horoscope_basic_daily_natal_3_slots"
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
        if ([string]$status.result.reading.quality.provider -ne "fake") {
            throw "Horoscope fake smoke expected provider=fake, got $($status.result.reading.quality.provider)"
        }
        if ($status.result.reading.contract_version -ne "horoscope_response") {
            throw "Unexpected horoscope contract version"
        }
        if (-not $status.result.interpretation_request.day_overview) {
            throw "Missing horoscope day_overview in interpretation request"
        }
        if (-not $status.result.interpretation_request.slots -or $status.result.interpretation_request.slots.Count -ne 3) {
            throw "Missing slot-based horoscope interpretation request"
        }
        foreach ($slot in $status.result.interpretation_request.slots) {
            if (-not $slot.required_evidence_keys -or $slot.required_evidence_keys.Count -lt 1) {
                throw "Slot $($slot.slot_code) missing required_evidence_keys"
            }
        }
        $readingSlots = $status.result.reading.slots
        if (-not $readingSlots -or $readingSlots.Count -ne 3) {
            throw "Unexpected horoscope reading slot count"
        }
        if ($readingSlots[1].title -ne "Après-midi") {
            throw "French slot label was not preserved"
        }
        if (($readingSlots | ForEach-Object { $_.text }) -match "les signaux du jour invitent") {
            throw "Generic horoscope wording leaked into fake reading"
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
