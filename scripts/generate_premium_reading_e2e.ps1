param(
    [string]$RequestPath = "",
    [string]$OutputPath = "",
    [string]$IdempotencyKey = "",
    [string]$BaseUrl = "",
    [string]$ApiKey = "",
    [string]$Model = "",
    [string]$Provider = "",
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

function Test-JsonPayloadPrefix {
    param([string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $false
    }
    switch ($Text.TrimStart()[0]) {
        '{' { return $true }
        '[' { return $true }
        default { return $false }
    }
}

function Convert-PremiumE2eResponse {
    param([string]$PayloadText)

    if ([string]::IsNullOrWhiteSpace($PayloadText)) {
        return $null
    }

    if (-not (Test-JsonPayloadPrefix -Text $PayloadText)) {
        $maxLen = [Math]::Min(500, $PayloadText.Length)
        $excerpt = $PayloadText.Substring(0, $maxLen)
        Write-Host "Reponse non-JSON, extrait :" $excerpt
        exit 1
    }

    try {
        return ($PayloadText | ConvertFrom-Json)
    } catch {
        $maxLen = [Math]::Min(500, $PayloadText.Length)
        $excerpt = $PayloadText.Substring(0, $maxLen)
        Write-Host "Reponse non-JSON, extrait :" $excerpt
        throw
    }
}

Import-DotEnv (Join-Path $repoRoot ".env")

if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "e2e-premium-$(Get-Date -Format 'yyyyMMddHHmmss')"
}

if ([string]::IsNullOrWhiteSpace($RequestPath)) {
    $richDefault = Join-Path $repoRoot "request-premium-rich.json"
    if (Test-Path -LiteralPath $richDefault) {
        $RequestPath = $richDefault
    } else {
        $RequestPath = Join-Path $repoRoot "request-premium.json"
    }
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot "output\premium_reading_e2e.json"
}
if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    if ($env:ASTRAL_LLM_HOST) {
        $llmHost = $env:ASTRAL_LLM_HOST
    } else {
        $llmHost = "127.0.0.1"
    }
    if ($env:ASTRAL_LLM_PORT) {
        $llmPort = $env:ASTRAL_LLM_PORT
    } else {
        $llmPort = "8081"
    }
    $BaseUrl = "http://${llmHost}:${llmPort}"
}

if ([string]::IsNullOrWhiteSpace($ApiKey)) {
    $ApiKey = $env:ASTRAL_LLM_API_KEY
}

if (-not (Test-Path -LiteralPath $RequestPath)) {
    throw "Fichier requete introuvable : $RequestPath"
}

$outputDir = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($outputDir)) {
    if (-not (Test-Path -LiteralPath $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    }
}

$headers = @{
    "Content-Type"    = "application/json"
    "Idempotency-Key" = $IdempotencyKey
}

if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
    $headers["Authorization"] = "Bearer $ApiKey"
}

$uri = "{0}/v1/readings/generate" -f $BaseUrl.TrimEnd("/")
$bodyObject = Get-Content -Raw -LiteralPath $RequestPath | ConvertFrom-Json
$bodyObject.idempotency_key = $IdempotencyKey

if (-not $bodyObject.engine) {
    $emptyEngine = [PSCustomObject]@{}
    $bodyObject | Add-Member -NotePropertyName engine -NotePropertyValue $emptyEngine -Force
}
if (-not [string]::IsNullOrWhiteSpace($Provider)) {
    $bodyObject.engine.provider = $Provider
}
if (-not [string]::IsNullOrWhiteSpace($Model)) {
    $bodyObject.engine.model = $Model
}

$body = $bodyObject | ConvertTo-Json -Depth 20 -Compress

if ($bodyObject.engine.model) {
    $engineModel = $bodyObject.engine.model
} else {
    $engineModel = "defaut produit ou service"
}
if ($bodyObject.engine.provider) {
    $engineProvider = $bodyObject.engine.provider
} else {
    $engineProvider = "defaut service"
}

Write-Host "POST $uri"
Write-Host "Request : $RequestPath"
Write-Host "Engine  : $engineProvider / $engineModel"
Write-Host "Idempotency-Key : $IdempotencyKey"
Write-Host "Output  : $OutputPath"

$logDir = Join-Path $repoRoot "output\logs"
if (-not (Test-Path -LiteralPath $logDir)) {
    New-Item -ItemType Directory -Path $logDir -Force | Out-Null
}
$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$clientLogPath = Join-Path $logDir "premium_reading_e2e_$stamp.json"

try {
    $iwrParams = @{
        Uri             = $uri
        Method          = "POST"
        Headers         = $headers
        Body            = $body
        TimeoutSec      = $TimeoutSec
        UseBasicParsing = $true
    }
    if ($PSVersionTable.PSVersion.Major -ge 6) {
        $iwrParams["SkipHttpErrorCheck"] = $true
    }
    $raw = Invoke-WebRequest @iwrParams

    $payloadText = $raw.Content
    $payloadText | Set-Content -LiteralPath $clientLogPath -Encoding utf8

    $response = Convert-PremiumE2eResponse -PayloadText $payloadText

    if ($null -ne $response) {
        if ($null -ne $response.run_id) {
            Write-Host "Audit run : .\scripts\show_generation_run.ps1 -RunId $($response.run_id)"
            $promptDir = Join-Path $repoRoot "output\logs\prompts\$($response.run_id)"
            Write-Host "Prompts LLM : $promptDir"
        }
    }

    if ($raw.StatusCode -ge 200 -and $raw.StatusCode -lt 300) {
        $payloadText | Set-Content -LiteralPath $OutputPath -Encoding utf8
        Write-Host "Reponse enregistree dans $OutputPath"
        exit 0
    }

    if ($null -ne $response -and $null -ne $response.error) {
        $errorCode = $response.error.code
    } else {
        $errorCode = $null
    }
    Write-Host "HTTP $($raw.StatusCode) : $errorCode"
    Write-Host "Journal : $clientLogPath"

    if ($errorCode -eq "IDEMPOTENCY_PAYLOAD_MISMATCH") {
        Write-Host "Cle Idempotency-Key deja utilisee avec un payload different. Utilisez une nouvelle cle."
        exit 3
    }

    if ($errorCode -eq "READING_QUALITY_FAILED") {
        Write-Host "Echec qualite editoriale. Voir logs API et audit run."
        exit 2
    }

    exit 1
} catch {
    Write-Host "Erreur reseau ou parsing : $_"
    exit 1
}
