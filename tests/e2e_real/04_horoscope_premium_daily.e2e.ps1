param(
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [string]$LlmUrl = "http://127.0.0.1:8081",
    [int]$ReadyTimeoutSec = 90,
    [int]$PollTimeoutSec = 300
)

$ErrorActionPreference = "Stop"
. "$PSScriptRoot\lib\real_e2e_common.ps1"
$repoRoot = Initialize-E2E
$calcHeaders = New-AstralAuthHeaders -Service calculator
$llmHeaders = New-AstralAuthHeaders -Service llm
$serviceCode = "horoscope_premium_daily_local_2h_slots"

function Assert-NoTechnicalSlotCode {
    param(
        [Parameter(Mandatory = $true)][string]$Text,
        [string]$Context = "public text"
    )
    if ($Text -match "slot_") {
        throw "Technical slot code leaked in $Context"
    }
}

function Convert-SlotLabelForComparison {
    param([string]$Label)
    return ($Label -replace "[^0-9:]", "")
}

Write-Host "=== Real Docker E2E: horoscope premium daily local 2h slots ===" -ForegroundColor Cyan
Wait-E2EReady -BaseUrl $CalculatorUrl -ServiceName "calculator" -TimeoutSec $ReadyTimeoutSec
Wait-E2EReady -BaseUrl $LlmUrl -ServiceName "llm" -TimeoutSec $ReadyTimeoutSec

$servicesResponse = Invoke-RestMethod -Uri "$LlmUrl/v1/services" -Method Get -Headers $llmHeaders
$service = @($servicesResponse.services | Where-Object { $_.service_code -eq $serviceCode }) | Select-Object -First 1
if (-not $service) {
    throw "Service $serviceCode not returned by /v1/services"
}
if ($service.availability -notin @("active", "beta")) {
    throw "Service $serviceCode is not executable in real E2E, availability=$($service.availability)"
}
if (-not $service.contracts -or $service.contracts.payload -ne "horoscope_premium_daily_local_request_v1") {
    throw "Unexpected payload contract in catalogue for $serviceCode"
}
Write-Host "OK catalogue exposes $serviceCode availability=$($service.availability)"

$contract = Invoke-RestMethod -Uri "$LlmUrl/v1/services/$serviceCode/contract" -Method Get -Headers $llmHeaders
if ($contract.service_code -ne $serviceCode) {
    throw "Contract detail service_code mismatch"
}
if ($contract.contracts.payload -ne "horoscope_premium_daily_local_request_v1") {
    throw "Unexpected payload contract detail for $serviceCode"
}
if ($contract.contracts.reading_output -ne "horoscope_response_v1") {
    throw "Unexpected reading output contract detail for $serviceCode"
}
Write-Host "OK contract detail"

$schema = Invoke-RestMethod -Uri "$LlmUrl/v1/schemas/horoscope_premium_daily_local_request_v1" -Method Get -Headers $llmHeaders
if ($schema.title -ne "horoscope_premium_daily_local_request_v1") {
    throw "Unexpected Premium payload schema title"
}
Write-Host "OK Premium payload schema"

$engineResponse = Invoke-E2ENatalCalculation -RepoRoot $repoRoot -CalculatorUrl $CalculatorUrl -Headers $calcHeaders
$chartCalculationId = [string]$engineResponse.calculation_result.chart_calculation_id
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
    throw "Missing chart_calculation_id"
}
Write-Host "OK natal calculation chart_calculation_id=$chartCalculationId"

$payload = New-E2EHoroscopePremiumPublicPayload -ChartCalculationId $chartCalculationId
$body = @{
    service_code = $serviceCode
    payload = $payload
    user_language = "fr"
    audience_level = "beginner"
}

$status = Invoke-E2EJobAndWait -LlmUrl $LlmUrl -Headers $llmHeaders -Body $body -PollTimeoutSec $PollTimeoutSec
$result = $status.result
if (-not $result.calculation) {
    throw "Premium result missing calculation"
}
if (-not $result.interpretation_request) {
    throw "Premium result missing interpretation_request"
}
if (-not $result.reading) {
    throw "Premium result missing reading"
}

$calculation = $result.calculation
if ($calculation.contract_version -ne "horoscope_calculation_response_v1") {
    throw "Unexpected Premium calculation contract"
}
if ($calculation.service_code -ne $serviceCode) {
    throw "Unexpected Premium calculation service_code"
}
if (-not $calculation.slots -or $calculation.slots.Count -ne 12) {
    throw "Premium calculation must return exactly 12 slots"
}
foreach ($slot in $calculation.slots) {
    if ([string]::IsNullOrWhiteSpace([string]$slot.reference_datetime_utc)) {
        throw "Premium calculation slot $($slot.slot_code) missing reference_datetime_utc"
    }
    if (-not $slot.local_chart) {
        throw "Premium calculation slot $($slot.slot_code) missing local_chart"
    }
    if (-not $slot.local_chart.ascendant -or -not $slot.local_chart.midheaven) {
        throw "Premium calculation slot $($slot.slot_code) missing ascendant or midheaven"
    }
    if (-not $slot.local_chart.houses -or $slot.local_chart.houses.Count -ne 12) {
        throw "Premium calculation slot $($slot.slot_code) must expose 12 local houses"
    }
}
Write-Host "OK Premium calculation local slots"

$interpretation = $result.interpretation_request
if ($interpretation.service_code -ne $serviceCode) {
    throw "Unexpected Premium interpretation service_code"
}
if (-not $interpretation.slots -or $interpretation.slots.Count -ne 12) {
    throw "Premium interpretation must include 12 slot plans"
}
if (-not $interpretation.best_slots -or -not $interpretation.watch_slots) {
    throw "Premium interpretation missing best_slots or watch_slots"
}
if (-not $interpretation.domain_sections -or $interpretation.domain_sections.Count -lt 1) {
    throw "Premium interpretation missing domain_sections"
}
Write-Host "OK Premium interpretation request"

$reading = $result.reading
if ($reading.contract_version -ne "horoscope_response_v1") {
    throw "Unexpected Premium reading contract"
}
if ($reading.service_code -ne $serviceCode) {
    throw "Unexpected Premium reading service_code"
}
if (-not $reading.period.location_label -or $reading.period.location_label -ne "Paris") {
    throw "Premium reading missing expected location_label"
}
if (-not $reading.timeline -or $reading.timeline.Count -ne 12) {
    throw "Premium reading timeline must contain exactly 12 entries"
}
if ((Convert-SlotLabelForComparison -Label ([string]$reading.timeline[0].slot_label)) -ne "00:0002:00") {
    throw "Unexpected first Premium timeline label"
}
if ((Convert-SlotLabelForComparison -Label ([string]$reading.timeline[11].slot_label)) -ne "22:0000:00") {
    throw "Unexpected last Premium timeline label"
}

$timelineLabels = @($reading.timeline | ForEach-Object { [string]$_.slot_label })
$bestLabels = @($reading.best_slots | ForEach-Object { [string]$_.slot_label })
$watchLabels = @($reading.watch_slots | ForEach-Object { [string]$_.slot_label })
foreach ($label in $bestLabels + $watchLabels) {
    if ($timelineLabels -notcontains $label) {
        throw "Premium best/watch slot '$label' is not present in timeline"
    }
}
foreach ($label in $bestLabels) {
    if ($watchLabels -contains $label) {
        throw "Premium slot '$label' appears in both best_slots and watch_slots"
    }
}

foreach ($slot in $reading.timeline) {
    if (-not $slot.evidence_keys -or $slot.evidence_keys.Count -lt 1) {
        throw "Premium timeline slot $($slot.slot_label) missing evidence_keys"
    }
    Assert-NoTechnicalSlotCode -Text ([string]$slot.title) -Context "timeline title $($slot.slot_label)"
    Assert-NoTechnicalSlotCode -Text ([string]$slot.text) -Context "timeline text $($slot.slot_label)"
    Assert-NoTechnicalSlotCode -Text ([string]$slot.advice) -Context "timeline advice $($slot.slot_label)"
}
if (-not $reading.domain_sections -or $reading.domain_sections.Count -lt 1) {
    throw "Premium reading missing domain_sections"
}
Write-Host "OK Premium reading shape and guards"

Write-Host "=== Horoscope Premium Daily real E2E PASSED ===" -ForegroundColor Green
