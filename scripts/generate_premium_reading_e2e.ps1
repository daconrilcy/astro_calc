param(
    [string]$RequestPath = "",
    [string]$OutputPath = "",
    [string]$IdempotencyKey = "",
    [string]$BaseUrl = "",
    [string]$ApiKey = "",
    [int]$TimeoutSec = 600
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

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

if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "e2e-premium-$(Get-Date -Format 'yyyyMMddHHmmss')"
}

if ([string]::IsNullOrWhiteSpace($RequestPath)) {
    $RequestPath = Join-Path $repoRoot "request-premium.json"
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot "output\premium_reading_e2e.json"
}
if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    $llmHost = if ($env:ASTRAL_LLM_HOST) { $env:ASTRAL_LLM_HOST } else { "127.0.0.1" }
    $llmPort = if ($env:ASTRAL_LLM_PORT) { $env:ASTRAL_LLM_PORT } else { "8081" }
    $BaseUrl = "http://${llmHost}:${llmPort}"
}

if ([string]::IsNullOrWhiteSpace($ApiKey)) {
    $ApiKey = $env:ASTRAL_LLM_API_KEY
}

if (-not (Test-Path -LiteralPath $RequestPath)) {
    throw "Fichier requete introuvable : $RequestPath"
}

$outputDir = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($outputDir) -and -not (Test-Path -LiteralPath $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

$headers = @{
    "Content-Type"    = "application/json"
    "Idempotency-Key" = $IdempotencyKey
}

if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
    $headers["Authorization"] = "Bearer $ApiKey"
}

$uri = "$($BaseUrl.TrimEnd('/'))/v1/readings/generate"
$bodyObject = Get-Content -Raw -LiteralPath $RequestPath | ConvertFrom-Json
$bodyObject.idempotency_key = $IdempotencyKey
$body = $bodyObject | ConvertTo-Json -Depth 20 -Compress

Write-Host "POST $uri"
Write-Host "Request : $RequestPath"
Write-Host "Idempotency-Key : $IdempotencyKey"
Write-Host "Output  : $OutputPath"

$logDir = Join-Path $repoRoot "output\logs"
if (-not (Test-Path -LiteralPath $logDir)) {
    New-Item -ItemType Directory -Path $logDir -Force | Out-Null
}
$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$clientLogPath = Join-Path $logDir "premium_reading_e2e_${stamp}.json"

try {
    $raw = Invoke-WebRequest `
        -Uri $uri `
        -Method POST `
        -Headers $headers `
        -Body $body `
        -TimeoutSec $TimeoutSec `
        -SkipHttpErrorCheck

    $payloadText = $raw.Content
    $payloadText | Set-Content -LiteralPath $clientLogPath -Encoding utf8

    $response = $null
    if (-not [string]::IsNullOrWhiteSpace($payloadText)) {
        $response = $payloadText | ConvertFrom-Json
    }

    if ($response.run_id) {
        Write-Host "Audit run : .\scripts\show_generation_run.ps1 -RunId $($response.run_id)"
    }

    if ($raw.StatusCode -ge 200 -and $raw.StatusCode -lt 300) {
        $payloadText | Set-Content -LiteralPath $OutputPath -Encoding utf8
        Write-Host "Reponse enregistree dans $OutputPath"
        exit 0
    }

    $errorCode = $response.error.code
    Write-Host "HTTP $($raw.StatusCode) : $errorCode"
    Write-Host "Journal : $clientLogPath"

    if ($errorCode -eq "IDEMPOTENCY_PAYLOAD_MISMATCH" -or $response.error -eq "IDEMPOTENCY_PAYLOAD_MISMATCH") {
        Write-Host "Cle Idempotency-Key deja utilisee avec un payload different. Utilisez une nouvelle cle."
        exit 3
    }

    if ($errorCode -eq "READING_QUALITY_FAILED") {
        Write-Host "Echec qualite editoriale (attendu possible en E2E). Voir logs API et audit run."
        exit 2
    }

    exit 1
}
catch {
    Write-Host "Erreur reseau ou parsing : $_"
    exit 1
}
