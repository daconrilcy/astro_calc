<#
.SYNOPSIS
    Construit une requete POST /v1/readings/generate a partir d'une enveloppe astro_engine_response_v1.

.DESCRIPTION
    Lit le fichier produit par astral_calculator (--file) et assemble le JSON attendu par astral_llm_api.
    Utilise audit_payload.payload (natal_structured_v13) comme astro_result.data.

.PARAMETER EnginePath
    Chemin vers astro_engine_response_*.json. Par defaut : le plus recent dans output/.

.PARAMETER OutputPath
    Fichier de sortie pour la requete LLM.

.PARAMETER ProfileCode
    interpretation_profile_code (ex. natal_light, natal_basic, natal_premium).

.PARAMETER UserLanguage
    Langue de la lecture (product_context.user_language).

.PARAMETER AudienceLevel
    beginner | intermediate | expert

.PARAMETER UseFake
    Force engine.provider=fake et engine.model=fake-model (tests locaux sans OpenAI).

.PARAMETER Provider
    Surcharge engine.provider (ex. openai). Prioritaire sur -UseFake.

.PARAMETER Model
    Surcharge engine.model.

.EXAMPLE
    cargo run -p astral_calculator --features swisseph-engine -- --file
    .\scripts\build_reading_request_from_engine.ps1 -ProfileCode natal_basic -UseFake

.EXAMPLE
    .\scripts\build_reading_request_from_engine.ps1 `
        -EnginePath output\astro_engine_response_20260605_120000.json `
        -ProfileCode natal_premium `
        -OutputPath output\my_reading_request.json

.EXAMPLE
    .\scripts\build_reading_request_from_engine.ps1 -ProfileCode natal_basic -UseFake
    .\scripts\generate_premium_reading_e2e.ps1 -RequestPath output\my_reading_request.json
#>
param(
    [string]$EnginePath = "",
    [string]$OutputPath = "",
    [string]$ProfileCode = "natal_basic",
    [string]$UserLanguage = "fr",
    [ValidateSet("beginner", "intermediate", "expert")]
    [string]$AudienceLevel = "beginner",
    [switch]$UseFake,
    [string]$Provider = "",
    [string]$Model = "",
    [string[]]$PreferredDomains = @(
        "identity",
        "emotional_life",
        "relationships",
        "career",
        "growth_path"
    )
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$outputDir = Join-Path $repoRoot "output"

function Resolve-EngineResponsePath {
    param([string]$ExplicitPath)

    if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
        $full = if ([System.IO.Path]::IsPathRooted($ExplicitPath)) {
            $ExplicitPath
        } else {
            Join-Path $repoRoot $ExplicitPath
        }
        if (-not (Test-Path -LiteralPath $full)) {
            throw "Fichier moteur introuvable : $full"
        }
        return $full
    }

    $candidates = Get-ChildItem -LiteralPath $outputDir -Filter "astro_engine_response_*.json" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending

    if (-not $candidates -or $candidates.Count -eq 0) {
        throw @"
Aucun fichier output\astro_engine_response_*.json trouve.
Lancez d'abord :
  cargo run -p astral_calculator --features swisseph-engine -- --file
"@
    }

    return $candidates[0].FullName
}

function Test-EngineEnvelope {
    param($Engine)

    $required = @(
        "response_contract_version",
        "calculation_result",
        "audit_payload"
    )
    foreach ($key in $required) {
        if (-not ($Engine.PSObject.Properties.Name -contains $key)) {
            throw "Cle manquante dans l'enveloppe moteur : $key"
        }
    }

    if ($Engine.response_contract_version -ne "astro_engine_response_v1") {
        throw "response_contract_version attendu : astro_engine_response_v1 (recu : $($Engine.response_contract_version))"
    }

    if ($Engine.calculation_result.status -ne "completed") {
        throw "Calcul moteur non termine : status=$($Engine.calculation_result.status)"
    }

    if (-not $Engine.audit_payload.contract_version) {
        throw "audit_payload.contract_version absent"
    }

    if ($null -eq $Engine.audit_payload.payload) {
        throw "audit_payload.payload absent : impossible de construire astro_result.data"
    }
}

$engineFile = Resolve-EngineResponsePath -ExplicitPath $EnginePath
$engine = Get-Content -LiteralPath $engineFile -Raw | ConvertFrom-Json
Test-EngineEnvelope -Engine $engine

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $outputDir "my_reading_request.json"
} elseif (-not [System.IO.Path]::IsPathRooted($OutputPath)) {
    $OutputPath = Join-Path $repoRoot $OutputPath
}

$outParent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($outParent) -and -not (Test-Path -LiteralPath $outParent)) {
    New-Item -ItemType Directory -Path $outParent -Force | Out-Null
}

$request = [ordered]@{
    request_id = "reading-$(Get-Date -Format 'yyyyMMddHHmmss')"
    product_context = [ordered]@{
        product_code = "natal_prompter"
        interpretation_profile_code = $ProfileCode
        user_language = $UserLanguage
        audience_level = $AudienceLevel
    }
    astro_result = [ordered]@{
        contract_version = $Engine.audit_payload.contract_version
        chart_type = "natal"
        data = $Engine.audit_payload.payload
    }
    astrologer_profile = [ordered]@{
        tone = "warm"
        jargon_level = "beginner"
        wording_style = "clear"
        preferred_domains = @($PreferredDomains)
        forbidden_wording = @()
    }
    engine = [ordered]@{
        allow_fallback = $true
    }
    response_contract = [ordered]@{
        output_schema_version = "natal_reading_v1"
        format = "structured_json"
        include_astro_sources = $true
        include_legal_disclaimer = $true
    }
}

if ($UseFake) {
    $request.engine.provider = "fake"
    $request.engine.model = "fake-model"
}

if (-not [string]::IsNullOrWhiteSpace($Provider)) {
    $request.engine.provider = $Provider
}

if (-not [string]::IsNullOrWhiteSpace($Model)) {
    $request.engine.model = $Model
}

$request | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $OutputPath -Encoding utf8

Write-Host "Source moteur : $engineFile"
Write-Host "Profil        : $ProfileCode"
Write-Host "Contrat astro : $($Engine.audit_payload.contract_version)"
Write-Host "Requete ecrite: $OutputPath"
Write-Host ""
Write-Host "Etape suivante :"
Write-Host "  .\scripts\generate_premium_reading_e2e.ps1 -RequestPath $OutputPath"
