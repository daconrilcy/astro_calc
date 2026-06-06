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

function New-ServicePayload {
    param(
        [Parameter(Mandatory = $true)]$Service,
        [Parameter(Mandatory = $true)]$EngineResponse,
        [Parameter(Mandatory = $true)][string]$ChartCalculationId
    )

    if ($Service.service_code -eq "horoscope_basic_daily_natal_3_slots") {
        return New-E2EHoroscopePublicPayload -ChartCalculationId $ChartCalculationId
    }

    if ($Service.calculation_mode -eq "simplified_natal") {
        return New-E2ESimplifiedNatalRequest
    }

    if ($Service.calculation_mode -eq "full_natal") {
        return New-E2ENatalEngineRequest -RepoRoot $repoRoot
    }

    if ($Service.service_code -like "*_from_payload") {
        $request = New-E2EReadingRequestFromEngine -EngineResponse $EngineResponse
        $request.product_context.interpretation_profile_code = $Service.interpretation_profile_code
        return $request
    }

    throw "No real E2E payload builder for service '$($Service.service_code)' (calculation_mode=$($Service.calculation_mode))"
}

Write-Host "=== Real Docker E2E: integration catalogue services ===" -ForegroundColor Cyan
Wait-E2EReady -BaseUrl $CalculatorUrl -ServiceName "calculator" -TimeoutSec $ReadyTimeoutSec
Wait-E2EReady -BaseUrl $LlmUrl -ServiceName "llm" -TimeoutSec $ReadyTimeoutSec

$servicesResponse = Invoke-RestMethod -Uri "$LlmUrl/v1/services" -Method Get -Headers $llmHeaders
$services = @($servicesResponse.services | Where-Object { $_.availability -in @("active", "beta") })
if ($services.Count -eq 0) {
    throw "No active/beta integration service returned by /v1/services"
}
Write-Host "Services active/beta: $($services.service_code -join ', ')"

$engineResponse = Invoke-E2ENatalCalculation -RepoRoot $repoRoot -CalculatorUrl $CalculatorUrl -Headers $calcHeaders
$chartCalculationId = [string]$engineResponse.calculation_result.chart_calculation_id
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) { throw "Missing chart_calculation_id" }

foreach ($service in $services) {
    Write-Host "--- Service $($service.service_code) ---" -ForegroundColor Cyan
    $contract = Invoke-RestMethod -Uri "$LlmUrl/v1/services/$($service.service_code)/contract" -Method Get -Headers $llmHeaders
    if ($contract.service_code -ne $service.service_code) {
        throw "Contract detail mismatch for $($service.service_code)"
    }

    $payload = New-ServicePayload -Service $service -EngineResponse $engineResponse -ChartCalculationId $chartCalculationId
    $body = @{
        service_code = $service.service_code
        payload = $payload
        user_language = "fr"
        audience_level = "beginner"
    }

    $status = Invoke-E2EJobAndWait -LlmUrl $LlmUrl -Headers $llmHeaders -Body $body -PollTimeoutSec $PollTimeoutSec

    if ($service.service_code -eq "horoscope_basic_daily_natal_3_slots") {
        if ($status.result.reading.contract_version -ne "horoscope_response_v1") {
            throw "Horoscope service returned unexpected reading contract"
        }
    } elseif ($service.service_code -like "*_from_payload") {
        if ($status.result.reading.status -ne "success") {
            throw "$($service.service_code) from-payload reading did not succeed"
        }
    } else {
        if (-not $status.result.calculation) {
            throw "$($service.service_code) result missing calculation"
        }
        if ($status.result.reading.status -ne "success") {
            throw "$($service.service_code) reading did not succeed"
        }
    }
    Write-Host "OK service $($service.service_code)" -ForegroundColor Green
}

Write-Host "=== Integration catalogue real E2E PASSED ===" -ForegroundColor Green
