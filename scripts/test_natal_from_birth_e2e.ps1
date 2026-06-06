<#
.SYNOPSIS
    E2E full natal via POST /v1/jobs (natal_basic).

.EXAMPLE
    .\scripts\test_natal_from_birth_e2e.ps1
#>
param(
    [string]$LlmBase = "http://127.0.0.1:8081",
    [int]$PollTimeoutSec = 180,
    [int]$PollIntervalMs = 3000
)

$ErrorActionPreference = "Stop"

function Wait-LlmReady {
    param([string]$Base, [int]$TimeoutSec = 60)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $r = Invoke-RestMethod -Uri "$Base/health/ready" -Method Get -TimeoutSec 5
            if ($r.status -eq "ready") { return }
        } catch { }
        Start-Sleep -Seconds 2
    }
    throw "LLM API not ready at $Base"
}

Write-Host "=== Full natal from-birth E2E (natal_basic job) ===" -ForegroundColor Cyan
Wait-LlmReady -Base $LlmBase

$svc = (Invoke-RestMethod -Uri "$LlmBase/v1/services" -Method Get).services |
    Where-Object { $_.service_code -eq "natal_basic" }
if (-not $svc -or $svc.availability -notin @("active", "beta")) {
    throw "natal_basic must be active/beta in catalogue (run manage_integration_services.ps1 -Submit)"
}

$idempotencyKey = "e2e-from-birth-{0}" -f ([guid]::NewGuid().ToString())
$jobBody = @{
    service_code   = "natal_basic"
    user_language  = "fr"
    audience_level = "beginner"
    payload        = @{
        request_contract_version = "astro_engine_request_v1"
        calculation              = @{ type = "natal" }
        birth                    = @{
            date     = "1990-06-15"
            time     = "14:30:00"
            timezone = "Europe/Paris"
            location = @{ latitude = 48.8566; longitude = 2.3522 }
        }
        projection = @{ level = "compact" }
    }
}

$headers = @{
    "Idempotency-Key" = $idempotencyKey
    "Content-Type"    = "application/json"
}
$post = Invoke-WebRequest -Uri "$LlmBase/v1/jobs" -Method Post -Headers $headers `
    -Body ($jobBody | ConvertTo-Json -Depth 20) -SkipHttpErrorCheck
if ($post.StatusCode -ne 202) {
    throw "submit failed $($post.StatusCode): $($post.Content)"
}
$accepted = $post.Content | ConvertFrom-Json
if ($accepted.status -ne "queued") {
    throw "expected queued, got $($accepted.status)"
}
$runId = $accepted.run_id
Write-Host "OK submit queued run_id=$runId"

$deadline = (Get-Date).AddSeconds($PollTimeoutSec)
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds $PollIntervalMs
    $status = Invoke-RestMethod -Uri "$LlmBase/v1/jobs/$runId" -Method Get
    Write-Host "  poll status=$($status.status)"
    if ($status.status -eq "completed") {
        if (-not $status.result.calculation) { throw "missing calculation in result" }
        if (-not $status.result.reading) { throw "missing reading in result" }
        Write-Host "=== Full natal from-birth E2E PASSED ===" -ForegroundColor Green
        exit 0
    }
    if ($status.status -in @("failed", "safety_rejected")) {
        throw "job terminal error: $($status | ConvertTo-Json -Depth 5)"
    }
}
throw "poll timeout — ensure astral_llm_worker and calculator are running"
