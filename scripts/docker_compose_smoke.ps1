<#
.SYNOPSIS
    Smoke E2E HTTP recommande : full natal via jobs async (fake).
#>
param(
    [string]$LlmUrl = "http://localhost:8081"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "lib\astral_http_auth.ps1")
Import-AstralDotEnv -RepoRoot $repoRoot

$llmHeaders = New-AstralAuthHeaders -Service llm

Write-Host "== Smoke E2E jobs async ==" -ForegroundColor Cyan

Write-Host "`n[1/4] GET /v1/services"
$services = Invoke-AstralJson -Method Get -Uri "$LlmUrl/v1/services" -Headers $llmHeaders
$service = @($services.services | Where-Object { $_.service_code -eq "natal_basic" })[0]
if ($null -eq $service) {
    throw "Service natal_basic introuvable dans le catalogue"
}
if ($service.availability -notin @("active", "beta")) {
    throw "Service natal_basic indisponible pour le smoke"
}
Write-Host "  natal_basic=$($service.availability)" -ForegroundColor Green

Write-Host "`n[2/4] POST /v1/jobs"
$idempotencyKey = "smoke-jobs-$(Get-Date -Format 'yyyyMMddHHmmss')"
$jobBody = [ordered]@{
    service_code = "natal_basic"
    payload = [ordered]@{
        request_contract_version = "astro_engine_request_v1"
        calculation = [ordered]@{
            type = "natal"
        }
        birth = [ordered]@{
            date = "1990-06-15"
            time = "14:30:00"
            timezone = "Europe/Paris"
            location = [ordered]@{
                latitude = 48.8566
                longitude = 2.3522
            }
        }
        projection = [ordered]@{
            level = "compact"
        }
    }
    user_language = "fr"
    audience_level = "beginner"
}

$jobHeaders = $llmHeaders.Clone()
$jobHeaders["Idempotency-Key"] = $idempotencyKey
$submit = Invoke-WebRequest -Method Post -Uri "$LlmUrl/v1/jobs" -Headers $jobHeaders `
    -Body ($jobBody | ConvertTo-Json -Depth 20) -UseBasicParsing -SkipHttpErrorCheck
if ($submit.StatusCode -ne 202) {
    throw "Soumission job echouee : HTTP $($submit.StatusCode) $($submit.Content)"
}
$accepted = $submit.Content | ConvertFrom-Json
if ($accepted.status -ne "queued") {
    throw "Statut submit inattendu : $($accepted.status)"
}
Write-Host "  run_id=$($accepted.run_id)" -ForegroundColor Green

Write-Host "`n[3/4] Poll /v1/jobs/{run_id}"
$deadline = (Get-Date).AddSeconds(180)
$status = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 3
    $status = Invoke-AstralJson -Method Get -Uri "$LlmUrl/v1/jobs/$($accepted.run_id)" -Headers $llmHeaders
    Write-Host "  status=$($status.status)"
    if ($status.status -in @("completed", "failed", "safety_rejected")) {
        break
    }
}

if ($null -eq $status) {
    throw "Poll job impossible"
}
if ($status.status -ne "completed") {
    throw "Job non termine avec succes : $($status | ConvertTo-Json -Depth 8 -Compress)"
}

Write-Host "`n[4/4] Validation enveloppe resultat"
if (-not $status.result.calculation) {
    throw "Resultat job sans calculation"
}
if (-not $status.result.reading) {
    throw "Resultat job sans reading"
}
if ($status.result.reading.status -ne "success") {
    throw "Lecture finale inattendue : $($status.result.reading | ConvertTo-Json -Depth 8 -Compress)"
}
if ($status.result.reading.reading.schema_version -ne "natal_reading_v1") {
    throw "schema_version inattendu : $($status.result.reading.reading.schema_version)"
}

Write-Host "  calculation=ok" -ForegroundColor Green
Write-Host "  reading=ok" -ForegroundColor Green
Write-Host "  chapters=$($status.result.reading.reading.chapters.Count)" -ForegroundColor Green
Write-Host "`nSmoke E2E jobs OK." -ForegroundColor Green
