param(
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [int]$ReadyTimeoutSec = 90
)

$ErrorActionPreference = "Stop"
. "$PSScriptRoot\lib\real_e2e_common.ps1"
$repoRoot = Initialize-E2E
$headers = New-AstralAuthHeaders -Service calculator

Write-Host "=== Real Docker E2E: calculator services ===" -ForegroundColor Cyan
Wait-E2EReady -BaseUrl $CalculatorUrl -ServiceName "calculator" -TimeoutSec $ReadyTimeoutSec

$contracts = Invoke-RestMethod -Uri "$CalculatorUrl/v1/contracts" -Method Get -Headers $headers
if ($contracts.service -ne "astral_calculator_api") { throw "Unexpected calculator contracts response" }

foreach ($schema in @(
    "astro_engine_request_v1",
    "astro_engine_response_v1",
    "astro_simplified_natal_request_v1",
    "astro_simplified_natal_response_v1",
    "horoscope_calculation_request_v1",
    "horoscope_calculation_response_v1"
)) {
    $schemaResponse = Invoke-RestMethod -Uri "$CalculatorUrl/v1/schemas/$schema" -Method Get -Headers $headers
    if (-not $schemaResponse.title -and -not $schemaResponse.'$schema') {
        throw "Schema $schema did not return a JSON schema"
    }
}
Write-Host "OK calculator contracts and schemas"

$engineRequest = New-E2ENatalEngineRequest -RepoRoot $repoRoot
$validateBody = @{ schema_version = "astro_engine_request_v1"; payload = $engineRequest }
$validate = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/calculations/validate" -Headers $headers -Body $validateBody
if ($validate.valid -ne $true) { throw "Calculator validation failed" }
Write-Host "OK POST /v1/calculations/validate"

$natal = Invoke-E2ENatalCalculation -RepoRoot $repoRoot -CalculatorUrl $CalculatorUrl -Headers $headers
$chartCalculationId = [string]$natal.calculation_result.chart_calculation_id
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) { throw "Missing chart_calculation_id" }
Write-Host "OK POST /v1/calculations/natal chart_calculation_id=$chartCalculationId"

$simplified = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/calculations/natal/simplified" `
    -Headers $headers -Body (New-E2ESimplifiedNatalRequest)
if ($simplified.response_contract_version -ne "astro_simplified_natal_response_v1") {
    throw "Unexpected simplified natal response contract"
}
if (-not $simplified.llm_payload.allowed_fact_codes) {
    throw "Simplified natal response missing llm_payload.allowed_fact_codes"
}
Write-Host "OK POST /v1/calculations/natal/simplified"

$horoscopeRequest = New-E2EHoroscopeCalculationRequest -RepoRoot $repoRoot -ChartCalculationId $chartCalculationId
$horoscope = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/calculations/horoscope/daily-natal" `
    -Headers $headers -Body $horoscopeRequest
if ($horoscope.contract_version -ne "horoscope_calculation_response_v1") {
    throw "Unexpected horoscope calculation contract"
}
if (-not $horoscope.slots -or $horoscope.slots.Count -lt 1) {
    throw "Horoscope calculation did not return slots"
}
Write-Host "OK POST /v1/calculations/horoscope/daily-natal"

Write-Host "=== Calculator real E2E PASSED ===" -ForegroundColor Green
