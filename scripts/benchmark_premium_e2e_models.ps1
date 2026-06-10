param(
    [string]$RequestPath = "",
    [string]$OutputDir = "",
    [string]$BaseUrl = "",
    [string]$ApiKey = "",
    [switch]$IncludeOracle,
    [int]$TimeoutSec = 900,
    # GPT-5 consomment des tokens de raisonnement dans max_output_tokens : 1200 coupe souvent avant le message.
    [int]$MaxOutputTokens = 4096,
    [int]$GlobalMaxTokens = 0
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$generateScript = Join-Path $PSScriptRoot "generate_premium_reading_e2e.ps1"

if ([string]::IsNullOrWhiteSpace($RequestPath)) {
    $richDefault = Join-Path $repoRoot "request-premium-rich.json"
    $RequestPath = if (Test-Path -LiteralPath $richDefault) {
        $richDefault
    } else {
        Join-Path $repoRoot "request-premium.json"
    }
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\benchmark_premium"
}

if (-not (Test-Path -LiteralPath $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$models = @(
    @{ Model = "gpt-4.1"; Label = "baseline" },
    @{ Model = "gpt-5-mini"; Label = "economique" },
    @{ Model = "gpt-5-mini"; Label = "production_probable" },
    @{ Model = "gpt-5.4"; Label = "premium_qualite_prix" },
    @{ Model = "gpt-5.5"; Label = "qualite_max_raisonnable" }
)

if ($IncludeOracle) {
    $models += @{ Model = "gpt-5.5-pro"; Label = "oracle_qualite"; Oracle = $true }
}

$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$summaryPath = Join-Path $OutputDir "benchmark_summary_$stamp.jsonl"
$results = @()

foreach ($entry in $models) {
    $model = $entry.Model
    $label = $entry.Label
    $idempotencyKey = "bench-premium-$label-$stamp"
    $outputPath = Join-Path $OutputDir "${label}_${model}_$stamp.json"

    Write-Host ""
    Write-Host "=== Run $label ($model) ===" -ForegroundColor Cyan

    $bodyObject = Get-Content -Raw -LiteralPath $RequestPath | ConvertFrom-Json
    if (-not $bodyObject.engine) {
        $bodyObject | Add-Member -NotePropertyName engine -NotePropertyValue ([PSCustomObject]@{}) -Force
    }
    # Serde ProviderKind : rename_all snake_case => "open_ai" (pas "openai")
    $bodyObject.engine.provider = "open_ai"
    $bodyObject.engine.model = $model
    $bodyObject.engine.max_output_tokens = $MaxOutputTokens
    $allowOracle = [bool]($entry.Oracle)
    $bodyObject.engine | Add-Member -NotePropertyName allow_oracle_benchmark -NotePropertyValue $allowOracle -Force

    if ($GlobalMaxTokens -gt 0) {
        if (-not $bodyObject.response_contract) {
            $bodyObject | Add-Member -NotePropertyName response_contract -NotePropertyValue ([PSCustomObject]@{}) -Force
        }
        $bodyObject.response_contract | Add-Member -NotePropertyName global_max_tokens -NotePropertyValue $GlobalMaxTokens -Force
    }

    $tempRequest = Join-Path $env:TEMP "astral_benchmark_$stamp`_$label.json"
    $bodyObject | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $tempRequest -Encoding utf8

    $exitCode = 0
    try {
        & $generateScript `
            -RequestPath $tempRequest `
            -OutputPath $outputPath `
            -IdempotencyKey $idempotencyKey `
            -BaseUrl $BaseUrl `
            -ApiKey $ApiKey `
            -TimeoutSec $TimeoutSec
        $exitCode = $LASTEXITCODE
    } catch {
        $exitCode = 1
        Write-Host "Echec $model : $_" -ForegroundColor Red
    }

    $runMeta = [ordered]@{
        label           = $label
        model           = $model
        exit_code       = $exitCode
        output_path     = $outputPath
        idempotency_key = $idempotencyKey
        finished_at     = (Get-Date).ToString("o")
    }

    if (Test-Path -LiteralPath $outputPath) {
        $resp = Get-Content -Raw -LiteralPath $outputPath | ConvertFrom-Json
        $runMeta.run_id = $resp.run_id
        if ($resp.reading -and $resp.reading.quality) {
            $runMeta.quality = $resp.reading.quality
        }
    }

    $results += $runMeta
    ($runMeta | ConvertTo-Json -Compress) | Add-Content -LiteralPath $summaryPath -Encoding utf8
}

Write-Host ""
Write-Host "Benchmark termine. Resume : $summaryPath"
Write-Host "Comparer via audit : .\scripts\show_generation_run.ps1 -RunId <run_id>"
Write-Host "Metriques + cout   : .\scripts\summarize_benchmark_runs.ps1 -SummaryPath $summaryPath"

if ($results | Where-Object { $_.exit_code -ne 0 }) {
    exit 1
}
