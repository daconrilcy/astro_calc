param(
    [string]$RequestPath = "",
    [string]$OutputPath = "",
    [string]$IdempotencyKey = "",
    [string]$BaseUrl = "",
    [string]$ApiKey = "",
    [string]$Model = "",
    [string]$SummaryModel = "",
    [string]$Provider = "",
    [int]$TimeoutSec = 600,
    [int]$EngineTimeoutMs = 0
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

Write-Host "Internal LLM reading client: POST /v1/internal/readings/render"

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

function Get-PremiumE2eAuditHeaders {
    param([string]$ApiKey)

    $auditHeaders = @{}
    if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
        $auditHeaders["Authorization"] = "Bearer $ApiKey"
    }
    return $auditHeaders
}

function Write-PremiumRunStepSummary {
    param(
        [Parameter(Mandatory = $true)]
        $Audit
    )

    Write-Host ""
    Write-Host "=== Resume audit ($($Audit.run_id)) ==="
    $requested = if ($Audit.model_requested) { $Audit.model_requested } else { "-" }
    $used = if ($Audit.model_used) { $Audit.model_used } else { "-" }
    $fallback = if ($Audit.fallback_used) { "oui" } else { "non" }
    Write-Host ("Moteur run : {0} -> {1} (fallback {2}) | {3} ms | {4} in / {5} out" -f `
        $requested, $used, $fallback, $Audit.latency_ms, $Audit.token_input, $Audit.token_output)

    if (-not $Audit.steps -or $Audit.steps.Count -eq 0) {
        Write-Host "Aucun step persiste (persistence desactivee ?)."
        return
    }

    Write-Host ""
    Write-Host ("{0,-16} {1,-14} {2,-10} {3,7} {4,7} {5,7}" -f "Step", "Modele", "Statut", "In", "Out", "ms")
    Write-Host ("{0}" -f ("-" * 65))

    $chapterIn = 0
    $chapterOut = 0
    $chapterMs = 0
    $summaryIn = 0
    $summaryOut = 0
    $summaryMs = 0

    foreach ($step in $Audit.steps) {
        $code = if ($step.chapter_code) { $step.chapter_code } else { "-" }
        $model = if ($step.model) { $step.model } else { "-" }
        $status = if ($step.status) { $step.status } else { "-" }
        $inTok = if ($null -ne $step.input_tokens) { [int]$step.input_tokens } else { 0 }
        $outTok = if ($null -ne $step.output_tokens) { [int]$step.output_tokens } else { 0 }
        $ms = if ($null -ne $step.latency_ms) { [int]$step.latency_ms } else { 0 }

        Write-Host ("{0,-16} {1,-14} {2,-10} {3,7} {4,7} {5,7}" -f $code, $model, $status, $inTok, $outTok, $ms)

        if ($code -eq "summary") {
            $summaryIn += $inTok
            $summaryOut += $outTok
            $summaryMs += $ms
        } else {
            $chapterIn += $inTok
            $chapterOut += $outTok
            $chapterMs += $ms
        }
    }

    Write-Host ("{0}" -f ("-" * 65))
    Write-Host ("{0,-16} {1,-14} {2,-10} {3,7} {4,7} {5,7}" -f `
        "TOTAL chapitres", "", "", $chapterIn, $chapterOut, $chapterMs)
    if ($summaryIn -gt 0 -or $summaryOut -gt 0) {
        Write-Host ("{0,-16} {1,-14} {2,-10} {3,7} {4,7} {5,7}" -f `
            "TOTAL summary", "", "", $summaryIn, $summaryOut, $summaryMs)
    }
    Write-Host ""
}

function Show-PremiumRunAuditSummary {
    param(
        [string]$RunId,
        [string]$BaseUrl,
        [string]$ApiKey
    )

    if ([string]::IsNullOrWhiteSpace($RunId)) {
        return
    }

    $auditUri = "{0}/v1/runs/{1}" -f $BaseUrl.TrimEnd("/"), $RunId
    try {
        $audit = Invoke-RestMethod -Uri $auditUri -Headers (Get-PremiumE2eAuditHeaders -ApiKey $ApiKey)
        Write-PremiumRunStepSummary -Audit $audit
    } catch {
        Write-Host "Resume audit indisponible ($auditUri) : $_"
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

$uri = "{0}/v1/internal/readings/render" -f $BaseUrl.TrimEnd("/")
$bodyObject = Get-Content -Raw -LiteralPath $RequestPath | ConvertFrom-Json
$bodyObject.idempotency_key = $IdempotencyKey

if (-not $bodyObject.engine) {
    $emptyEngine = [PSCustomObject]@{}
    $bodyObject | Add-Member -NotePropertyName engine -NotePropertyValue $emptyEngine -Force
}
if (-not [string]::IsNullOrWhiteSpace($Provider)) {
    $bodyObject.engine | Add-Member -NotePropertyName provider -NotePropertyValue $Provider -Force
}
if (-not [string]::IsNullOrWhiteSpace($Model)) {
    $bodyObject.engine | Add-Member -NotePropertyName model -NotePropertyValue $Model -Force
}
if (-not [string]::IsNullOrWhiteSpace($SummaryModel)) {
    $bodyObject.engine | Add-Member -NotePropertyName summary_model -NotePropertyValue $SummaryModel -Force
}
if ($EngineTimeoutMs -gt 0) {
    $bodyObject.engine | Add-Member -NotePropertyName timeout_ms -NotePropertyValue $EngineTimeoutMs -Force
}

$body = $bodyObject | ConvertTo-Json -Depth 20 -Compress

$engineModelProp = $bodyObject.engine.PSObject.Properties["model"]
if ($engineModelProp -and -not [string]::IsNullOrWhiteSpace([string]$engineModelProp.Value)) {
    $engineModel = [string]$engineModelProp.Value
} else {
    $engineModel = "defaut produit (chapitres)"
}
$summaryModelProp = $bodyObject.engine.PSObject.Properties["summary_model"]
if ($summaryModelProp -and -not [string]::IsNullOrWhiteSpace([string]$summaryModelProp.Value)) {
    $summaryModelLabel = [string]$summaryModelProp.Value
} elseif (-not $engineModelProp -or [string]::IsNullOrWhiteSpace([string]$engineModelProp.Value)) {
    $summaryModelLabel = "economic_model produit (ex. gpt-5-nano)"
} else {
    $summaryModelLabel = "meme que chapitres (-Model force les deux)"
}
$engineProviderProp = $bodyObject.engine.PSObject.Properties["provider"]
if ($engineProviderProp -and -not [string]::IsNullOrWhiteSpace([string]$engineProviderProp.Value)) {
    $engineProvider = [string]$engineProviderProp.Value
} else {
    $engineProvider = "defaut service"
}

Write-Host "POST $uri"
Write-Host "Request : $RequestPath"
Write-Host "Modeles prod : config\llm_product_models.conf -> .\scripts\set_product_llm_models.ps1 -Show"
Write-Host "Engine  : $engineProvider / $engineModel (summary: $summaryModelLabel)"
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
        if ($null -ne $response -and $null -ne $response.run_id) {
            Show-PremiumRunAuditSummary -RunId $response.run_id -BaseUrl $BaseUrl -ApiKey $ApiKey
        }
        exit 0
    }

    if ($null -ne $response -and $null -ne $response.error) {
        $errorCode = $response.error.code
    } else {
        $errorCode = $null
    }
    Write-Host "HTTP $($raw.StatusCode) : $errorCode"
    Write-Host "Journal : $clientLogPath"

    if ($raw.StatusCode -eq 504) {
        $serverMs = if ($env:ASTRAL_LLM_REQUEST_TIMEOUT_MS) { $env:ASTRAL_LLM_REQUEST_TIMEOUT_MS } else { "120000" }
        Write-Host "Gateway timeout : la requete HTTP a depasse ASTRAL_LLM_REQUEST_TIMEOUT_MS (+5s layer)."
        Write-Host "  Valeur serveur actuelle : ${serverMs} ms — premium_plus requiert ~900000 ms (10 appels LLM longs)."
        Write-Host "  1. Editer .env : ASTRAL_LLM_REQUEST_TIMEOUT_MS=900000"
        Write-Host "  2. Redemarrer astral_llm_api"
        Write-Host "  3. Relancer avec -TimeoutSec 1800 -EngineTimeoutMs 300000"
        exit 4
    }

    if ($errorCode -eq "IDEMPOTENCY_PAYLOAD_MISMATCH") {
        Write-Host "Cle Idempotency-Key deja utilisee avec un payload different. Utilisez une nouvelle cle."
        exit 3
    }

    if ($errorCode -eq "READING_QUALITY_FAILED") {
        Write-Host "Echec qualite editoriale."
        if ($null -ne $response.error.message) {
            Write-Host "  Message : $($response.error.message)"
        }
        if ($null -ne $response.error.details -and $null -ne $response.error.details.violations) {
            Write-Host "  Violations :"
            foreach ($v in $response.error.details.violations) {
                $ch = if ($v.chapter) { $v.chapter } else { "-" }
                $kind = if ($v.kind) { $v.kind } else { "-" }
                $phrase = if ($v.phrase) { $v.phrase } else { "" }
                Write-Host "    - [$ch] $kind : `"$phrase`""
            }
        }
        if ($null -ne $response.error.details -and $null -ne $response.error.details.warnings) {
            Write-Host "  Warnings :"
            foreach ($w in @($response.error.details.warnings)) {
                Write-Host "    - $w"
            }
        }
        Write-Host "  Relancer ou mettre a jour astral_llm_api (repair opening : jusqu'a 8 tours, consignes renforcees)."
        if ($null -ne $response.run_id) {
            Show-PremiumRunAuditSummary -RunId $response.run_id -BaseUrl $BaseUrl -ApiKey $ApiKey
        }
        exit 2
    }

    if ($null -ne $response -and $null -ne $response.run_id) {
        Show-PremiumRunAuditSummary -RunId $response.run_id -BaseUrl $BaseUrl -ApiKey $ApiKey
    }

    exit 1
} catch {
    Write-Host "Erreur reseau ou parsing : $_"
    exit 1
}
