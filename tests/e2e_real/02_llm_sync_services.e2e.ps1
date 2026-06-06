param(
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [string]$LlmUrl = "http://127.0.0.1:8081",
    [int]$ReadyTimeoutSec = 90
)

$ErrorActionPreference = "Stop"
. "$PSScriptRoot\lib\real_e2e_common.ps1"
$repoRoot = Initialize-E2E
$calcHeaders = New-AstralAuthHeaders -Service calculator
$llmHeaders = New-AstralAuthHeaders -Service llm

Write-Host "=== Real Docker E2E: LLM sync services ===" -ForegroundColor Cyan
Wait-E2EReady -BaseUrl $CalculatorUrl -ServiceName "calculator" -TimeoutSec $ReadyTimeoutSec
Wait-E2EReady -BaseUrl $LlmUrl -ServiceName "llm" -TimeoutSec $ReadyTimeoutSec

$contracts = Invoke-RestMethod -Uri "$LlmUrl/v1/contracts" -Method Get -Headers $llmHeaders
if (-not $contracts.openapi) { throw "LLM contracts response missing openapi" }

$providers = Invoke-RestMethod -Uri "$LlmUrl/v1/providers" -Method Get -Headers $llmHeaders
if (-not $providers.default_provider) { throw "LLM providers response missing default_provider" }

foreach ($schema in @("generate_reading_request_v1", "generate_reading_response_v1", "natal_reading_v1")) {
    $schemaResponse = Invoke-RestMethod -Uri "$LlmUrl/v1/schemas/$schema" -Method Get -Headers $llmHeaders
    if (-not $schemaResponse.title -and -not $schemaResponse.'$schema') {
        throw "Schema $schema did not return a JSON schema"
    }
}
Write-Host "OK LLM contracts, providers and schemas"

$natal = Invoke-E2ENatalCalculation -RepoRoot $repoRoot -CalculatorUrl $CalculatorUrl -Headers $calcHeaders
$readingRequest = New-E2EReadingRequestFromEngine -EngineResponse $natal
$reading = Invoke-AstralJson -Method Post -Uri "$LlmUrl/v1/readings/generate" -Headers $llmHeaders -Body $readingRequest
if ($reading.status -ne "success") {
    throw "Reading generation failed: $($reading | ConvertTo-Json -Depth 12)"
}
if ($reading.reading.schema_version -ne "natal_reading_v1") {
    throw "Unexpected reading schema version"
}
Write-Host "OK POST /v1/readings/generate"

$validation = Invoke-AstralJson -Method Post -Uri "$LlmUrl/v1/readings/validate" -Headers $llmHeaders -Body $reading.reading
if ($validation.valid -ne $true) { throw "Generated reading validation failed" }
Write-Host "OK POST /v1/readings/validate"

$simplifiedBody = New-E2ESimplifiedNatalRequest
$simplifiedBody.user_language = "fr"
$simplifiedBody.audience_level = "beginner"
$simplifiedReading = Invoke-AstralJson -Method Post -Uri "$LlmUrl/v1/readings/natal/simplified" `
    -Headers $llmHeaders -Body $simplifiedBody
if (-not $simplifiedReading.calculation -or -not $simplifiedReading.reading) {
    throw "Simplified sync reading missing calculation or reading"
}
if ($simplifiedReading.reading.status -ne "success") {
    throw "Simplified sync reading did not succeed"
}
Write-Host "OK POST /v1/readings/natal/simplified"

Write-Host "=== LLM sync real E2E PASSED ===" -ForegroundColor Green
