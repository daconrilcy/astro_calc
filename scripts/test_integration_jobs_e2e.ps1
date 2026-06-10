<#
.SYNOPSIS
    E2E jobs d'intégration — natal_simplified pilote (status=queued partout).

.EXAMPLE
    .\scripts\test_integration_jobs_e2e.ps1

.EXAMPLE
    .\scripts\test_integration_jobs_e2e.ps1 -AllowRealProvider
#>
param(
    [string]$LlmBase = "http://127.0.0.1:8081",
    [int]$PollTimeoutSec = 300,
    [int]$PollIntervalMs = 2000,
    [switch]$AllowRealProvider,
    [switch]$AllowProductFakeOverride
)

$ErrorActionPreference = "Stop"

. "$PSScriptRoot\lib\astral_http_auth.ps1"
Import-AstralDotEnv -RepoRoot (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$authHeaders = New-AstralAuthHeaders -Service llm

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

function Invoke-JobPost {
    param([string]$Base, [hashtable]$Body, [string]$IdempotencyKey, [hashtable]$Headers)
    $headers = $Headers.Clone()
    $headers["Idempotency-Key"] = $IdempotencyKey
    return Invoke-WebRequest -Uri "$Base/v1/jobs" -Method Post -Headers $headers `
        -Body ($Body | ConvertTo-Json -Depth 20) -SkipHttpErrorCheck
}

Write-Host "=== Integration jobs E2E (natal_simplified) ===" -ForegroundColor Cyan
Wait-LlmReady -Base $LlmBase

$providers = Invoke-RestMethod -Uri "$LlmBase/v1/providers" -Method Get -Headers $authHeaders -TimeoutSec 10
$defaultProvider = [string]$providers.default_provider
$defaultModel = [string]$providers.default_model
Write-Host "Provider runtime default_provider=$defaultProvider default_model=$defaultModel"
if (-not $AllowRealProvider -and -not $AllowProductFakeOverride -and $defaultProvider -ne "fake") {
    throw "Integration jobs E2E is a local smoke and expects default_provider=fake. Current default_provider='$defaultProvider', default_model='$defaultModel'. Reconfigure the LLM runtime to fake, run through docker_update_integration_stack.ps1 so it applies the natal_prompter fake override, or rerun with -AllowRealProvider to intentionally use the real provider."
}
if (-not $AllowRealProvider -and $AllowProductFakeOverride -and $defaultProvider -ne "fake") {
    Write-Host "OK product-level fake override assumed for natal_prompter; global default_provider remains '$defaultProvider'." -ForegroundColor Yellow
}

# 1. Catalogue
$services = Invoke-RestMethod -Uri "$LlmBase/v1/services" -Method Get
$svc = $services.services | Where-Object { $_.service_code -eq "natal_simplified" }
if (-not $svc) { throw "natal_simplified not in catalogue" }
if ($svc.availability -ne "active") { throw "expected active, got $($svc.availability)" }
Write-Host "OK catalogue natal_simplified active"

# 2. Contract
$contract = Invoke-RestMethod -Uri "$LlmBase/v1/services/natal_simplified/contract" -Method Get
if ($contract.contracts.payload -ne "astro_simplified_natal_request_v1") {
    throw "unexpected payload_contract"
}
Write-Host "OK contract detail"

# 3. Submit job
$idempotencyKey = "e2e-integration-{0}" -f ([guid]::NewGuid().ToString())
$jobBody = @{
    service_code   = "natal_simplified"
    user_language  = "fr"
    audience_level = "beginner"
    payload        = @{
        request_contract_version = "astro_simplified_natal_request_v1"
        birth                    = @{
            date     = "1990-06-15"
            time     = "14:30"
            timezone = "Europe/Paris"
            location = @{ latitude = 48.8566; longitude = 2.3522 }
        }
    }
}

$post = Invoke-JobPost -Base $LlmBase -Body $jobBody -IdempotencyKey $idempotencyKey -Headers $authHeaders
if ($post.StatusCode -ne 202) {
    throw "expected 202 on submit, got $($post.StatusCode): $($post.Content)"
}
$accepted = $post.Content | ConvertFrom-Json
if ($accepted.status -ne "queued") {
    throw "expected status=queued on submit, got $($accepted.status)"
}
$runId = $accepted.run_id
Write-Host "OK submit 202 queued run_id=$runId"

# 4. Idempotent replay in progress
$replay = Invoke-JobPost -Base $LlmBase -Body $jobBody -IdempotencyKey $idempotencyKey -Headers $authHeaders
if ($replay.StatusCode -notin @(200, 202)) {
    throw "expected 200/202 on replay, got $($replay.StatusCode)"
}
Write-Host "OK idempotent replay $($replay.StatusCode)"

# 5. Cross-service 409
$otherBody = $jobBody.Clone()
$otherBody.service_code = "natal_basic"
$conflict = Invoke-JobPost -Base $LlmBase -Body $otherBody -IdempotencyKey $idempotencyKey -Headers $authHeaders
if ($conflict.StatusCode -ne 409) {
    Write-Warning "cross-service 409 expected (service may be unknown=404); got $($conflict.StatusCode)"
} else {
    Write-Host "OK idempotency cross-service 409"
}

# 6. Poll until terminal
$deadline = (Get-Date).AddSeconds($PollTimeoutSec)
$terminal = $false
while ((Get-Date) -lt $deadline -and -not $terminal) {
    Start-Sleep -Milliseconds $PollIntervalMs
    $status = Invoke-RestMethod -Uri "$LlmBase/v1/jobs/$runId" -Method Get -Headers $authHeaders
    Write-Host "  poll status=$($status.status)"
    if ($status.status -in @("completed", "failed", "safety_rejected")) {
        $terminal = $true
        if ($status.status -ne "completed") {
            if ($status.error.code -eq "PROVIDER_RATE_LIMITED") {
                throw "job failed because the real LLM provider rate-limited the request. Use the fake provider for this smoke, or rerun later with -AllowRealProvider. Status: $($status | ConvertTo-Json -Depth 5)"
            }
            throw "job failed: $($status | ConvertTo-Json -Depth 5)"
        }
        if (-not $status.result) {
            throw "completed job must include result"
        }
    }
}
if (-not $terminal) {
    throw "poll timeout — is astral_llm_worker running?"
}

# 7. Replay completed → 200 + result
$done = Invoke-JobPost -Base $LlmBase -Body $jobBody -IdempotencyKey $idempotencyKey -Headers $authHeaders
if ($done.StatusCode -ne 200) {
    throw "expected 200 replay completed, got $($done.StatusCode)"
}
$doneBody = $done.Content | ConvertFrom-Json
if (-not $doneBody.result) {
    throw "200 replay must include result"
}
Write-Host "OK completed replay 200 with result"

Write-Host "=== Integration jobs E2E PASSED ===" -ForegroundColor Green
exit 0
