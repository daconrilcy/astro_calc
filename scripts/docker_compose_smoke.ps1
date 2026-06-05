<#
.SYNOPSIS
    Smoke E2E HTTP : calculateur -> mapping -> LLM (fake).
#>
param(
    [string]$CalculatorUrl = "http://localhost:8080",
    [string]$LlmUrl = "http://localhost:8081"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "lib\astral_http_auth.ps1")
Import-AstralDotEnv -RepoRoot $repoRoot

$requestPath = Join-Path $repoRoot "contracts\integration\examples\natal_calculation_request_v1.paris_1990.json"
if (-not (Test-Path -LiteralPath $requestPath)) {
    throw "Fixture introuvable : $requestPath"
}

$calcHeaders = New-AstralAuthHeaders -Service calculator
$llmHeaders = New-AstralAuthHeaders -Service llm

Write-Host "== Smoke E2E calculator -> llm ==" -ForegroundColor Cyan

Write-Host "`n[1/3] POST /v1/calculations/natal"
$engineRequest = Get-Content -LiteralPath $requestPath -Raw | ConvertFrom-Json
$engineResponse = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/calculations/natal" -Headers $calcHeaders -Body $engineRequest
if ($engineResponse.response_contract_version -ne "astro_engine_response_v1") {
    throw "Reponse calculateur inattendue"
}
if ($engineResponse.calculation_result.status -ne "completed") {
    throw "Calcul non termine"
}
Write-Host "  chart_calculation_id=$($engineResponse.calculation_result.chart_calculation_id)" -ForegroundColor Green

Write-Host "`n[2/3] Mapping engine -> reading request"
$readingRequest = [ordered]@{
    request_id = "smoke-$(Get-Date -Format 'yyyyMMddHHmmss')"
    product_context = [ordered]@{
        product_code = "natal_prompter"
        interpretation_profile_code = "natal_basic"
        user_language = "fr"
        audience_level = "beginner"
    }
    astro_result = [ordered]@{
        contract_version = $engineResponse.audit_payload.contract_version
        chart_type = "natal"
        data = $engineResponse.audit_payload.payload
    }
    astrologer_profile = [ordered]@{
        tone = "warm"
        jargon_level = "beginner"
        wording_style = "clear"
        preferred_domains = @("identity", "emotional_life", "relationships")
        forbidden_wording = @()
    }
    engine = [ordered]@{
        provider = "fake"
        model = "fake-model"
        allow_fallback = $true
    }
    response_contract = [ordered]@{
        output_schema_version = "natal_reading_v1"
        generation_mode = "chapter_orchestrated"
        format = "structured_json"
        include_astro_sources = $true
        include_legal_disclaimer = $true
    }
}

Write-Host "`n[3/3] POST /v1/readings/generate (fake)"
$readingResponse = Invoke-AstralJson -Method Post -Uri "$LlmUrl/v1/readings/generate" -Headers $llmHeaders -Body $readingRequest

if ($readingResponse.status -ne "success") {
    throw "Generation LLM echouee : $($readingResponse | ConvertTo-Json -Depth 5 -Compress)"
}

if ($readingResponse.reading.schema_version -ne "natal_reading_v1") {
    throw "schema_version inattendu : $($readingResponse.reading.schema_version)"
}

Write-Host "  run_id=$($readingResponse.run_id)" -ForegroundColor Green
Write-Host "  chapters=$($readingResponse.reading.chapters.Count)" -ForegroundColor Green
Write-Host "`nSmoke E2E OK." -ForegroundColor Green
