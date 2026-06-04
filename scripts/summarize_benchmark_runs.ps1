param(
    [string]$SummaryPath = "",
    [string]$OutputCsv = "",
    [string]$EnrichedJsonl = "",
    [string]$BaseUrl = "",
    [string]$ApiKey = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$benchmarkDir = Join-Path $repoRoot "output\benchmark_premium"

function Import-DotEnv {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }

    Get-Content -LiteralPath $Path | ForEach-Object {
        $line = $_.Trim()
        if ($line -eq "" -or $line.StartsWith("#")) {
            return
        }
        $eq = $line.IndexOf("=")
        if ($eq -lt 1) {
            return
        }
        $name = $line.Substring(0, $eq).Trim()
        $value = $line.Substring($eq + 1).Trim()
        if ($value.StartsWith('"') -and $value.EndsWith('"')) {
            $value = $value.Substring(1, $value.Length - 2)
        }
        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name, "Process"))) {
            [Environment]::SetEnvironmentVariable($name, $value, "Process")
        }
    }
}

Import-DotEnv (Join-Path $repoRoot ".env")

if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    $llmHost = if ($env:ASTRAL_LLM_HOST) { $env:ASTRAL_LLM_HOST } else { "127.0.0.1" }
    $llmPort = if ($env:ASTRAL_LLM_PORT) { $env:ASTRAL_LLM_PORT } else { "8081" }
    $BaseUrl = "http://${llmHost}:${llmPort}"
}

if ([string]::IsNullOrWhiteSpace($ApiKey)) {
    $ApiKey = $env:ASTRAL_LLM_API_KEY
}

$headers = @{}
if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
    $headers["Authorization"] = "Bearer $ApiKey"
}

if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    $latest = Get-ChildItem -LiteralPath $benchmarkDir -Filter "benchmark_summary_*.jsonl" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $latest) {
        throw "Aucun benchmark_summary_*.jsonl dans $benchmarkDir. Passez -SummaryPath."
    }
    $SummaryPath = $latest.FullName
}

if (-not (Test-Path -LiteralPath $SummaryPath)) {
    throw "Fichier introuvable : $SummaryPath"
}

$stamp = [regex]::Match($SummaryPath, "benchmark_summary_(\d{8}_\d{6})\.jsonl$").Groups[1].Value
if ([string]::IsNullOrWhiteSpace($stamp)) {
    $stamp = Get-Date -Format "yyyyMMdd_HHmmss"
}

if ([string]::IsNullOrWhiteSpace($OutputCsv)) {
    $OutputCsv = Join-Path $benchmarkDir "benchmark_metrics_$stamp.csv"
}

if ([string]::IsNullOrWhiteSpace($EnrichedJsonl)) {
    $EnrichedJsonl = Join-Path $benchmarkDir "benchmark_metrics_$stamp.jsonl"
}

# Tarifs OpenAI (USD / 1M tokens) - reference benchmark, pas source produit.
$prices = @{
    "gpt-4.1"       = @{ input = 2.00; output = 8.00 }
    "gpt-5-mini"    = @{ input = 0.25; output = 2.00 }
    "gpt-5.4-mini"  = @{ input = 0.75; output = 4.50 }
    "gpt-5.4"       = @{ input = 2.50; output = 15.00 }
    "gpt-5.5"       = @{ input = 5.00; output = 30.00 }
    "gpt-5.5-pro"   = @{ input = 10.00; output = 60.00 }
}

$successStepStatuses = @("generated", "repaired")

function Get-EstimatedCostUsd {
    param(
        [string]$Model,
        [int]$TokenInput,
        [int]$TokenOutput
    )

    if (-not $prices.ContainsKey($Model)) {
        return $null
    }
    $price = $prices[$Model]
    $costInput = ($TokenInput / 1000000.0) * $price.input
    $costOutput = ($TokenOutput / 1000000.0) * $price.output
    [math]::Round($costInput + $costOutput, 5)
}

function Get-StepMetrics {
    param($Steps)

    if (-not $Steps -or $Steps.Count -eq 0) {
        return @{
            chapter_count         = 0
            summary_tokens        = $null
            repairs_count         = 0
            failed_steps          = 0
            avg_step_latency_ms   = $null
            max_step_latency_ms   = $null
            step_count            = 0
        }
    }

    $chapterSteps = @($Steps | Where-Object {
        $_.chapter_code -and $_.chapter_code -ne "summary"
    })
    $summaryStep = $Steps | Where-Object { $_.chapter_code -eq "summary" } | Select-Object -First 1
    $latencies = @($Steps | Where-Object { $null -ne $_.latency_ms } | ForEach-Object { [int]$_.latency_ms })

    @{
        chapter_count       = $chapterSteps.Count
        summary_tokens      = if ($summaryStep) { $summaryStep.output_tokens } else { $null }
        repairs_count       = @($Steps | Where-Object { $_.status -eq "repaired" }).Count
        failed_steps        = @($Steps | Where-Object { $successStepStatuses -notcontains $_.status }).Count
        avg_step_latency_ms = if ($latencies.Count -gt 0) {
            [math]::Round(($latencies | Measure-Object -Average).Average, 0)
        } else { $null }
        max_step_latency_ms = if ($latencies.Count -gt 0) {
            ($latencies | Measure-Object -Maximum).Maximum
        } else { $null }
        step_count          = $Steps.Count
    }
}

$rows = @()
$enrichedLines = @()

Write-Host "Resume benchmark : $SummaryPath" -ForegroundColor Cyan
Write-Host "API : $($BaseUrl.TrimEnd('/'))"

Get-Content -LiteralPath $SummaryPath | ForEach-Object {
    $item = $_ | ConvertFrom-Json
    if (-not $item.run_id) {
        Write-Warning "Ligne sans run_id (label=$($item.label)) - ignoree."
        return
    }

    $uri = "$($BaseUrl.TrimEnd('/'))/v1/runs/$($item.run_id)"
    Write-Host "GET $uri"
    $run = Invoke-RestMethod -Uri $uri -Headers $headers -Method Get

    $model = if ($run.model_used) { $run.model_used } else { $item.model }
    $tokenIn = if ($null -ne $run.token_input) { [int]$run.token_input } else { 0 }
    $tokenOut = if ($null -ne $run.token_output) { [int]$run.token_output } else { 0 }
    $stepMetrics = Get-StepMetrics -Steps $run.steps

    $qualityStatus = $null
    if ($item.output_path -and (Test-Path -LiteralPath $item.output_path)) {
        $out = Get-Content -Raw -LiteralPath $item.output_path | ConvertFrom-Json
        $qualityStatus = $out.status
    }

    $row = [PSCustomObject]@{
        label                      = $item.label
        model                      = $model
        run_id                     = $item.run_id
        exit_code                  = $item.exit_code
        status                     = $run.status
        quality_status             = $qualityStatus
        safety_status              = $run.safety_status
        fallback_used              = $run.fallback_used
        latency_ms                 = $run.latency_ms
        token_input                = $tokenIn
        token_output               = $tokenOut
        cost_usd                   = Get-EstimatedCostUsd -Model $model -TokenInput $tokenIn -TokenOutput $tokenOut
        chapter_count              = $stepMetrics.chapter_count
        summary_tokens             = $stepMetrics.summary_tokens
        step_count                 = $stepMetrics.step_count
        avg_step_latency_ms        = $stepMetrics.avg_step_latency_ms
        max_step_latency_ms        = $stepMetrics.max_step_latency_ms
        repairs_count              = $stepMetrics.repairs_count
        failed_steps               = $stepMetrics.failed_steps
        manual_quality_score       = ""
        editorial_naturalness      = ""
        astro_depth                = ""
        repetition_control         = ""
        summary_quality            = ""
        output_path                = $item.output_path
        idempotency_key            = $item.idempotency_key
    }

    $rows += $row

    $enriched = [ordered]@{}
    foreach ($prop in $item.PSObject.Properties) {
        $enriched[$prop.Name] = $prop.Value
    }
    foreach ($prop in $row.PSObject.Properties) {
        if ($prop.Name -notin @("output_path", "idempotency_key", "exit_code")) {
            $enriched[$prop.Name] = $prop.Value
        }
    }
    $enrichedLines += ($enriched | ConvertTo-Json -Compress -Depth 6)
}

$rows = $rows | Sort-Object cost_usd

$rows | Format-Table label, model, status, latency_ms, token_input, token_output, cost_usd, chapter_count, repairs_count, failed_steps -AutoSize

$rows | Export-Csv -LiteralPath $OutputCsv -NoTypeInformation -Encoding utf8
$enrichedLines | Set-Content -LiteralPath $EnrichedJsonl -Encoding utf8

Write-Host ""
Write-Host "CSV  : $OutputCsv" -ForegroundColor Green
Write-Host "JSONL enrichi : $EnrichedJsonl" -ForegroundColor Green
Write-Host 'Remplir les colonnes manual_* dans le CSV apres revue editoriale des sorties JSON.'
