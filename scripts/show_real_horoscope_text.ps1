param(
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [string]$LlmUrl = "http://127.0.0.1:8081",
    [string]$Date = "2026-06-06",
    [string]$Timezone = "Europe/Paris",
    [string]$TargetLanguage = "fr",
    [string]$AudienceLevel = "beginner",
    [int]$PollTimeoutSec = 300,
    [int]$PollIntervalSec = 5,
    [string]$OutputDir = "",
    [string]$IdempotencyKey = ""
)

$ErrorActionPreference = "Stop"

. "$PSScriptRoot\lib\astral_http_auth.ps1"

function Wait-AstralReady {
    param(
        [Parameter(Mandatory = $true)][string]$BaseUrl,
        [Parameter(Mandatory = $true)][string]$Name,
        [int]$TimeoutSec = 90
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $ready = Invoke-RestMethod -Uri "$BaseUrl/health/ready" -Method Get -TimeoutSec 5
            if ($ready.status -eq "ready") {
                Write-Host "OK $Name ready ($BaseUrl)" -ForegroundColor Green
                return
            }
        } catch {
        }
        Start-Sleep -Seconds 2
    }
    throw "$Name not ready at $BaseUrl"
}

function Get-JsonFile {
    param([Parameter(Mandatory = $true)][string]$Path)
    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Convert-HoroscopeReadingToMarkdown {
    param(
        [Parameter(Mandatory = $true)]$Status,
        [Parameter(Mandatory = $true)]$Reading
    )

    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# Horoscope reel")
    $lines.Add("")
    $lines.Add("- run_id: $($Status.run_id)")
    $lines.Add("- service_code: $($Reading.service_code)")
    $lines.Add("- contract_version: $($Reading.contract_version)")
    $lines.Add("- generated_at: $((Get-Date).ToString("s"))")
    $lines.Add("")

    if ($Reading.overview) {
        $lines.Add("## Vue d'ensemble")
        $lines.Add("")
        $lines.Add([string]$Reading.overview)
        $lines.Add("")
    }

    foreach ($slot in @($Reading.slots)) {
        $lines.Add("## $($slot.title)")
        $lines.Add("")
        $lines.Add([string]$slot.text)
        $lines.Add("")
        $lines.Add("**Conseil :** $($slot.advice)")
        $lines.Add("")
        if ($slot.evidence_keys) {
            $lines.Add("**Preuves:** $(@($slot.evidence_keys) -join ', ')")
            $lines.Add("")
        }
    }

    if ($Reading.disclaimer) {
        $lines.Add("## Disclaimer")
        $lines.Add("")
        $lines.Add([string]$Reading.disclaimer)
        $lines.Add("")
    }

    return ($lines -join [Environment]::NewLine)
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Import-AstralDotEnv -RepoRoot $repoRoot

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\horoscope_real"
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "real-horoscope-$([guid]::NewGuid().ToString())"
}

$calculatorHeaders = New-AstralAuthHeaders -Service calculator
$llmHeaders = New-AstralAuthHeaders -Service llm
$jobHeaders = $llmHeaders.Clone()
$jobHeaders["Idempotency-Key"] = $IdempotencyKey

Write-Host "=== Real horoscope text generation ===" -ForegroundColor Cyan
Write-Host "Calculator: $CalculatorUrl"
Write-Host "LLM:        $LlmUrl"
Write-Host "Date:       $Date"
Write-Host "Output:     $OutputDir"

Wait-AstralReady -BaseUrl $CalculatorUrl -Name "calculator"
Wait-AstralReady -BaseUrl $LlmUrl -Name "llm"

$natalRequestPath = Join-Path $repoRoot "contracts\integration\examples\natal_calculation_request_v1.paris_1990.json"
if (-not (Test-Path -LiteralPath $natalRequestPath)) {
    throw "Natal fixture not found: $natalRequestPath"
}

$natalRequest = Get-JsonFile -Path $natalRequestPath
if ($natalRequest.projection -and $natalRequest.projection.level) {
    $natalRequest.projection.level = "compact"
}

Write-Host "Calculating natal chart..." -ForegroundColor Cyan
$natalResponse = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/calculations/natal" -Headers $calculatorHeaders -Body $natalRequest
if ($natalResponse.response_contract_version -ne "astro_engine_response_v1") {
    throw "Unexpected natal response contract: $($natalResponse.response_contract_version)"
}
if ($natalResponse.calculation_result.status -ne "completed") {
    throw "Natal calculation did not complete: $($natalResponse.calculation_result.status)"
}

$chartCalculationId = [string]$natalResponse.calculation_result.chart_calculation_id
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
    throw "Natal calculation did not return chart_calculation_id"
}
Write-Host "OK natal chart_calculation_id=$chartCalculationId" -ForegroundColor Green

$body = @{
    service_code = "horoscope_basic_daily_natal_3_slots"
    payload = @{
        date = $Date
        timezone = $Timezone
        target_language = $TargetLanguage
        chart_calculation_id = $chartCalculationId
        audience_level = "general"
    }
    user_language = $TargetLanguage
    audience_level = $AudienceLevel
}

Write-Host "Submitting horoscope job..." -ForegroundColor Cyan
$submit = Invoke-RestMethod -Method Post -Uri "$LlmUrl/v1/jobs" -Headers $jobHeaders -ContentType "application/json" -Body ($body | ConvertTo-Json -Depth 40)
if (-not $submit.run_id) {
    throw "Submit response did not return run_id"
}
Write-Host "OK submitted run_id=$($submit.run_id)"

$startedAt = Get-Date
$deadline = $startedAt.AddSeconds($PollTimeoutSec)
$lastStatus = ""
$lastStatusPayload = $null
while ((Get-Date) -lt $deadline) {
    $status = Invoke-RestMethod -Method Get -Uri "$LlmUrl/v1/jobs/$($submit.run_id)" -Headers $llmHeaders
    $lastStatusPayload = $status
    $elapsedSec = [Math]::Round(((Get-Date) - $startedAt).TotalSeconds, 0)
    if ($status.status -ne $lastStatus) {
        $lastStatus = $status.status
        Write-Host "  status=$($status.status) elapsed=${elapsedSec}s/$($PollTimeoutSec)s"
    }

    if ($status.status -eq "completed") {
        if (-not $status.result.reading) {
            throw "Completed job has no result.reading"
        }
        if ($status.result.reading.contract_version -ne "horoscope_response_v1") {
            throw "Unexpected horoscope contract: $($status.result.reading.contract_version)"
        }

        $stamp = Get-Date -Format "yyyyMMdd_HHmmss"
        $jsonPath = Join-Path $OutputDir "horoscope_real_$stamp.json"
        $mdPath = Join-Path $OutputDir "horoscope_real_$stamp.md"
        $status | ConvertTo-Json -Depth 80 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
        $markdown = Convert-HoroscopeReadingToMarkdown -Status $status -Reading $status.result.reading
        if ($markdown -match "\[(morning|afternoon|evening)\]") {
            throw "Public markdown leaked technical slot code"
        }
        if ($markdown -match "Conseil:") {
            throw "Public markdown uses invalid French typography for Conseil"
        }
        $markdown | Set-Content -LiteralPath $mdPath -Encoding UTF8

        Write-Host ""
        Write-Host "=== Horoscope text ===" -ForegroundColor Green
        foreach ($slot in @($status.result.reading.slots)) {
            Write-Host ""
            Write-Host "## $($slot.title)" -ForegroundColor Cyan
            Write-Host $slot.text
            Write-Host "Conseil : $($slot.advice)"
        }
        Write-Host ""
        Write-Host "JSON: $jsonPath"
        Write-Host "MD:   $mdPath"
        exit 0
    }

    if ($status.status -in @("failed", "safety_rejected", "cancelled", "expired")) {
        $status | ConvertTo-Json -Depth 80
        throw "Horoscope job ended with $($status.status)"
    }

    Start-Sleep -Seconds $PollIntervalSec
}

$lastPayloadJson = if ($null -ne $lastStatusPayload) {
    $lastStatusPayload | ConvertTo-Json -Depth 40 -Compress
} else {
    "{}"
}
throw "Timeout waiting for horoscope job run_id=$($submit.run_id) after $($PollTimeoutSec)s. Last payload: $lastPayloadJson"
