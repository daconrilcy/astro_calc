<#
.SYNOPSIS
    Lance un E2E HTTP reel pour le service horoscope_free_daily.

.DESCRIPTION
    Ce script utilise les services locaux Docker : calcul natal via le calculateur,
    soumission asynchrone via POST /v1/jobs, polling du statut, validations de la
    forme publique Free, puis ecriture des sorties JSON et Markdown.

.EXAMPLE
    .\scripts\test_horoscope_free_daily_real_e2e.ps1

.EXAMPLE
    .\scripts\test_horoscope_free_daily_real_e2e.ps1 -Date 2026-06-07 -OutputDir output\horoscope_free_daily_real
#>
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
    $lastError = $null
    while ((Get-Date) -lt $deadline) {
        try {
            $ready = Invoke-RestMethod -Uri "$BaseUrl/health/ready" -Method Get -TimeoutSec 5
            if ($ready.status -eq "ready") {
                Write-Host "OK $Name ready ($BaseUrl)" -ForegroundColor Green
                return
            }
            $lastError = "status=$($ready.status)"
        } catch {
            $lastError = $_.Exception.Message
        }
        Start-Sleep -Seconds 2
    }
    throw "$Name not ready at $BaseUrl. Last error: $lastError"
}

function Get-JsonFile {
    param([Parameter(Mandatory = $true)][string]$Path)
    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Convert-FreeHoroscopeReadingToMarkdown {
    param(
        [Parameter(Mandatory = $true)]$Status,
        [Parameter(Mandatory = $true)]$Reading
    )

    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# Horoscope Free reel")
    $lines.Add("")
    $lines.Add("- run_id: $($Status.run_id)")
    $lines.Add("- service_code: $($Reading.service_code)")
    $lines.Add("- contract_version: $($Reading.contract_version)")
    $lines.Add("- generated_at: $((Get-Date).ToString("s"))")
    $lines.Add("")
    $lines.Add("## $($Reading.summary.title)")
    $lines.Add("")
    $lines.Add([string]$Reading.summary.text)
    $lines.Add("")
    $lines.Add("**Conseil :** $($Reading.advice)")
    $lines.Add("")
    $lines.Add("**Point d'attention :** $($Reading.watch_point)")
    $lines.Add("")
    $evidenceCount = @($Reading.evidence_keys).Count
    if ($evidenceCount -gt 0) {
        $lines.Add("**Preuves retenues :** $evidenceCount")
        $lines.Add("")
    }
    return ($lines -join [Environment]::NewLine)
}

function Assert-FreeReading {
    param(
        [Parameter(Mandatory = $true)]$Status,
        [Parameter(Mandatory = $true)]$Markdown
    )

    if ($Status.service_code -ne "horoscope_free_daily") {
        throw "Unexpected job service_code: $($Status.service_code)"
    }
    if ($Status.status -ne "completed") {
        throw "Unexpected job status: $($Status.status)"
    }
    if (-not $Status.result.reading) {
        throw "Completed job has no result.reading"
    }
    if ($Status.result.reading.contract_version -ne "horoscope_response") {
        throw "Unexpected reading contract_version: $($Status.result.reading.contract_version)"
    }
    if ($Status.result.reading.service_code -ne "horoscope_free_daily") {
        throw "Unexpected reading service_code: $($Status.result.reading.service_code)"
    }
    if ($Status.result.reading.slots) {
        throw "Free public reading must not expose slots"
    }
    foreach ($field in @("summary", "advice", "watch_point", "evidence_keys", "quality")) {
        if (-not $Status.result.reading.$field) {
            throw "Free reading missing $field"
        }
    }
    if (-not $Status.result.interpretation_request.slots -or $Status.result.interpretation_request.slots.Count -ne 1) {
        throw "Free interpretation must contain exactly one internal slot"
    }
    if ($Status.result.interpretation_request.slots[0].slot_code -ne "day") {
        throw "Unexpected internal slot code: $($Status.result.interpretation_request.slots[0].slot_code)"
    }
    if (-not $Status.result.calculation.slots -or $Status.result.calculation.slots.Count -ne 1) {
        throw "Free calculation must contain exactly one slot"
    }
    if ($Status.result.calculation.slots[0].slot_code -ne "day") {
        throw "Unexpected calculation slot code: $($Status.result.calculation.slots[0].slot_code)"
    }
    $publicText = @(
        $Status.result.reading.summary.title
        $Status.result.reading.summary.text
        $Status.result.reading.advice
        $Status.result.reading.watch_point
        $Markdown
    ) -join "`n"
    if ($publicText -match "slot:day" -or $publicText -match "\bday\b" -or $publicText -match "slot_code") {
        throw "Internal day slot leaked into public reading"
    }
    if ($publicText -match "Conseil:") {
        throw "Public markdown uses invalid French typography for Conseil"
    }
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Import-AstralDotEnv -RepoRoot $repoRoot

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\horoscope_free_daily_real"
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "real-horoscope-free-daily-$([guid]::NewGuid().ToString('N'))"
}

$calculatorHeaders = New-AstralAuthHeaders -Service calculator
$llmHeaders = New-AstralAuthHeaders -Service llm
$jobHeaders = $llmHeaders.Clone()
$jobHeaders["Idempotency-Key"] = $IdempotencyKey

Write-Host "=== Horoscope Free Daily real E2E ===" -ForegroundColor Cyan
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
    service_code = "horoscope_free_daily"
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

Write-Host "Submitting horoscope_free_daily job..." -ForegroundColor Cyan
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
        $markdown = Convert-FreeHoroscopeReadingToMarkdown -Status $status -Reading $status.result.reading
        Assert-FreeReading -Status $status -Markdown $markdown

        $stamp = Get-Date -Format "yyyyMMdd_HHmmss"
        $jsonPath = Join-Path $OutputDir "horoscope_free_daily_real_$stamp.json"
        $mdPath = Join-Path $OutputDir "horoscope_free_daily_real_$stamp.md"
        $status | ConvertTo-Json -Depth 80 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
        $markdown | Set-Content -LiteralPath $mdPath -Encoding UTF8

        Write-Host ""
        Write-Host "=== Horoscope Free Daily text ===" -ForegroundColor Green
        Write-Host ""
        Write-Host "## $($status.result.reading.summary.title)" -ForegroundColor Cyan
        Write-Host $status.result.reading.summary.text
        Write-Host "Conseil : $($status.result.reading.advice)"
        Write-Host "Point d'attention : $($status.result.reading.watch_point)"
        Write-Host ""
        Write-Host "JSON: $jsonPath"
        Write-Host "MD:   $mdPath"
        exit 0
    }

    if ($status.status -in @("failed", "safety_rejected", "cancelled", "expired")) {
        $status | ConvertTo-Json -Depth 80
        throw "Horoscope Free Daily job ended with $($status.status)"
    }

    Start-Sleep -Seconds $PollIntervalSec
}

$lastPayloadJson = if ($null -ne $lastStatusPayload) {
    $lastStatusPayload | ConvertTo-Json -Depth 40 -Compress
} else {
    "{}"
}
throw "Timeout waiting for horoscope_free_daily run_id=$($submit.run_id) after $($PollTimeoutSec)s. Last payload: $lastPayloadJson"
