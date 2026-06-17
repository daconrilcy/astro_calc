$ErrorActionPreference = "Stop"

$script:E2ERepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
. (Join-Path $script:E2ERepoRoot "scripts\lib\astral_http_auth.ps1")

function Get-E2ERepoRoot {
    return $script:E2ERepoRoot
}

function Initialize-E2E {
    $repoRoot = Get-E2ERepoRoot
    Import-AstralDotEnv -RepoRoot $repoRoot
    return $repoRoot
}

function Wait-E2EReady {
    param(
        [Parameter(Mandatory = $true)][string]$BaseUrl,
        [string]$ServiceName = "service",
        [int]$TimeoutSec = 90
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $ready = Invoke-RestMethod -Uri "$BaseUrl/health/ready" -Method Get -TimeoutSec 5
            if ($ready.status -eq "ready") {
                Write-Host "OK $ServiceName ready ($BaseUrl)" -ForegroundColor Green
                return
            }
        } catch { }
        Start-Sleep -Seconds 2
    }
    throw "$ServiceName not ready at $BaseUrl"
}

function Get-E2EJsonFile {
    param([Parameter(Mandatory = $true)][string]$Path)
    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function New-E2ENatalEngineRequest {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)
    $path = Join-Path $RepoRoot "contracts\integration\examples\natal_calculation_request_v1.paris_1990.json"
    $request = Get-E2EJsonFile -Path $path
    $request.projection.level = "compact"
    return $request
}

function New-E2ESimplifiedNatalRequest {
    return [ordered]@{
        request_contract_version = "astro_simplified_natal_request_v1"
        birth = [ordered]@{
            date = "1990-06-15"
            time = "14:30"
            timezone = "Europe/Paris"
            location = [ordered]@{
                latitude = 48.8566
                longitude = 2.3522
            }
        }
    }
}

function New-E2EHoroscopeCalculationRequest {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$ChartCalculationId
    )
    $slotsPath = Join-Path $RepoRoot "json_db\horoscope_time_slot_profiles.json"
    $slotsJson = Get-E2EJsonFile -Path $slotsPath
    $slots = @(
        $slotsJson.data |
            Where-Object { $_.service_code -eq "horoscope_basic_daily_natal_3_slots" } |
            Sort-Object sort_order |
            ForEach-Object {
                [ordered]@{
                    slot_code = $_.slot_code
                    start_local_time = $_.start_local_time
                    end_local_time = $_.end_local_time
                    reference_local_time = $_.reference_local_time
                }
            }
    )
    if ($slots.Count -eq 0) {
        throw "No horoscope slots found in json_db/horoscope_time_slot_profiles.json"
    }
    return [ordered]@{
        contract_version = "horoscope_calculation_request"
        service_code = "horoscope_basic_daily_natal_3_slots"
        chart_calculation_id = $ChartCalculationId
        period = [ordered]@{
            date = "2026-06-06"
            timezone = "Europe/Paris"
        }
        slots = $slots
    }
}

function New-E2EHoroscopePublicPayload {
    param([Parameter(Mandatory = $true)][string]$ChartCalculationId)
    return [ordered]@{
        date = "2026-06-06"
        timezone = "Europe/Paris"
        target_language = "fr"
        chart_calculation_id = $ChartCalculationId
        audience_level = "general"
    }
}

function New-E2EHoroscopePremiumPublicPayload {
    param([Parameter(Mandatory = $true)][string]$ChartCalculationId)
    return [ordered]@{
        date = "2026-06-06"
        timezone = "Europe/Paris"
        target_language = "fr"
        chart_calculation_id = $ChartCalculationId
        location = [ordered]@{
            latitude = 48.8566
            longitude = 2.3522
            label = "Paris"
        }
        audience_level = "general"
        detail_level = "premium_rich"
    }
}

function Invoke-E2ENatalCalculation {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$CalculatorUrl,
        [Parameter(Mandatory = $true)][hashtable]$Headers
    )
    $request = New-E2ENatalEngineRequest -RepoRoot $RepoRoot
    $response = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/internal/calculations/natal" -Headers $Headers -Body $request
    if ($response.response_contract_version -ne "astro_engine_response_v1") {
        throw "Unexpected natal response contract"
    }
    if ($response.calculation_result.status -ne "completed") {
        throw "Natal calculation did not complete"
    }
    return $response
}

function New-E2EReadingRequestFromEngine {
    param([Parameter(Mandatory = $true)]$EngineResponse)
    return [ordered]@{
        request_id = "real-e2e-$([guid]::NewGuid().ToString())"
        product_context = [ordered]@{
            product_code = "natal_prompter"
            interpretation_profile_code = "natal_basic"
            user_language = "fr"
            audience_level = "beginner"
        }
        astro_result = [ordered]@{
            contract_version = $EngineResponse.audit_payload.contract_version
            chart_type = "natal"
            data = $EngineResponse.audit_payload.payload
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
}

function Invoke-E2EJobAndWait {
    param(
        [Parameter(Mandatory = $true)][string]$LlmUrl,
        [Parameter(Mandatory = $true)][hashtable]$Headers,
        [Parameter(Mandatory = $true)][hashtable]$Body,
        [int]$PollTimeoutSec = 240,
        [int]$PollIntervalMs = 5000,
        [int]$RepeatLogEvery = 6
    )
    $jobHeaders = $Headers.Clone()
    $jobHeaders["Idempotency-Key"] = "real-e2e-$($Body.service_code)-$([guid]::NewGuid().ToString())"
    $accepted = Invoke-RestMethod -Uri "$LlmUrl/v1/jobs" -Method Post -Headers $jobHeaders `
        -ContentType "application/json" -Body ($Body | ConvertTo-Json -Depth 80)
    if (-not $accepted.run_id) {
        throw "Submit $($Body.service_code) did not return run_id"
    }
    Write-Host "  $($Body.service_code) submitted run_id=$($accepted.run_id)"

    $startedAt = Get-Date
    $deadline = $startedAt.AddSeconds($PollTimeoutSec)
    $lastStatus = ""
    $lastStatusJson = $null
    $repeatCount = 0
    while ((Get-Date) -lt $deadline) {
        $status = Invoke-RestMethod -Uri "$LlmUrl/v1/jobs/$($accepted.run_id)" -Method Get -Headers $Headers
        $lastStatusJson = $status
        if ($status.status -eq $lastStatus) {
            $repeatCount += 1
        } else {
            $lastStatus = $status.status
            $repeatCount = 1
        }
        $elapsedSec = [Math]::Round(((Get-Date) - $startedAt).TotalSeconds, 0)
        if ($repeatCount -eq 1 -or $repeatCount % $RepeatLogEvery -eq 0) {
            Write-Host "  $($Body.service_code) status=$($status.status) elapsed=${elapsedSec}s/$($PollTimeoutSec)s"
        }
        if ($status.status -eq "completed") {
            if (-not $status.result) {
                throw "Completed job $($Body.service_code) has no result"
            }
            return $status
        }
        if ($status.status -in @("failed", "safety_rejected", "cancelled", "expired")) {
            throw "Job $($Body.service_code) ended with $($status.status): $($status | ConvertTo-Json -Depth 20)"
        }
        $remainingMs = [Math]::Max(0, [Math]::Round(($deadline - (Get-Date)).TotalMilliseconds, 0))
        if ($remainingMs -le 0) {
            break
        }
        Start-Sleep -Milliseconds ([Math]::Min($PollIntervalMs, $remainingMs))
    }
    $lastPayload = if ($null -ne $lastStatusJson) {
        $lastStatusJson | ConvertTo-Json -Depth 20 -Compress
    } else {
        "{}"
    }
    throw "Timeout waiting for job $($Body.service_code) run_id=$($accepted.run_id) after $($PollTimeoutSec)s; last status was '$lastStatus'. Last payload: $lastPayload. Check astral_llm_worker logs if it stays running."
}
