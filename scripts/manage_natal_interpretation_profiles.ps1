<#
.SYNOPSIS
    Gere les profils d'interpretation natal_prompter en base canonique.

.DESCRIPTION
    -Submit : enregistre un JSON (fichier ou -Json inline)
    -List   : liste les profils actifs/inactifs
    -Get    : rappelle le profile_json
    -Delete : soft-delete (is_active=false) ou -Hard pour suppression physique

.EXAMPLE
    .\scripts\manage_natal_interpretation_profiles.ps1 -Submit -Path config\natal_interpretation_profiles\natal_premium.json

.EXAMPLE
    .\scripts\manage_natal_interpretation_profiles.ps1 -List

.EXAMPLE
    .\scripts\manage_natal_interpretation_profiles.ps1 -Get -ProfileCode natal_premium

.EXAMPLE
    .\scripts\manage_natal_interpretation_profiles.ps1 -Delete -ProfileCode natal_light
#>
param(
    [switch]$Submit,
    [string]$Path = "",
    [string]$Json = "",
    [switch]$List,
    [string]$ProfileCode = "",
    [switch]$Get,
    [switch]$Delete,
    [switch]$Hard,
    [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$NatalPrompterProduct = "natal_prompter"
$AllowedModes = @("single_pass", "chapter_orchestrated")
$AllowedReasoning = @("none", "minimal", "low", "medium", "high")

function Import-DotEnv {
    param([string]$EnvPath)
    if (-not (Test-Path -LiteralPath $EnvPath)) { return }
    Get-Content -LiteralPath $EnvPath | ForEach-Object {
        $line = $_.Trim()
        if ($line -eq "" -or $line.StartsWith("#")) { return }
        $eq = $line.IndexOf("=")
        if ($eq -lt 1) { return }
        $name = $line.Substring(0, $eq).Trim()
        $value = $line.Substring($eq + 1).Trim().Trim('"')
        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name, "Process"))) {
            [Environment]::SetEnvironmentVariable($name, $value, "Process")
        }
    }
}

function Escape-SqlLiteral {
    param([string]$Value)
    return $Value.Replace("'", "''")
}

function New-DollarQuotedSql {
    param([string]$Content)
    $tag = "profile_json_body"
    while ($Content -like "*`$$tag`$*") {
        $tag = "${tag}_x"
    }
    return "`$$tag`$$Content`$$tag`$"
}

function Ensure-InterpretationProfilesSchema {
    $sqlPath = Join-Path $repoRoot "astral_llm\crates\astral_llm_infra\sql\llm_interpretation_profiles.sql"
    if (-not (Test-Path -LiteralPath $sqlPath)) {
        throw "Fichier schema introuvable : $sqlPath"
    }
    $sql = Get-Content -LiteralPath $sqlPath -Raw
    Invoke-ProjectPsql -Sql $sql | Out-Null
}

function Invoke-ProjectPsql {
    param([string]$Sql)

    $url = $env:DATABASE_URL
    if ([string]::IsNullOrWhiteSpace($url)) {
        throw "DATABASE_URL absent (.env a la racine du depot)."
    }

    if (Get-Command psql -ErrorAction SilentlyContinue) {
        $prev = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $out = & psql $url -v ON_ERROR_STOP=1 -t -A -c $Sql 2>&1
        $ErrorActionPreference = $prev
        if ($LASTEXITCODE -ne 0) {
            throw ($out | Out-String)
        }
        return ($out | Out-String).Trim()
    }

    $user = if ($env:POSTGRES_USER) { $env:POSTGRES_USER } else { "postgres" }
    $db = if ($env:POSTGRES_DB) { $env:POSTGRES_DB } else { $user }
    Push-Location $repoRoot
    try {
        $prev = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $out = docker compose exec -T postgres psql -U $user -d $db -v ON_ERROR_STOP=1 -t -A -c $Sql 2>&1
        $ErrorActionPreference = $prev
        if ($LASTEXITCODE -ne 0) {
            throw ($out | Out-String)
        }
        return ($out | Out-String).Trim()
    } finally {
        Pop-Location
    }
}

function Test-ProfileDocument {
    param($Doc)

    $required = @(
        "profile_code", "product_code", "generation_mode",
        "max_domains", "max_chapters", "max_output_tokens", "max_reasoning_effort",
        "chapter_models", "chapter_word_targets", "quality", "evidence"
    )
    foreach ($key in $required) {
        if (-not ($Doc.PSObject.Properties.Name -contains $key)) {
            throw "Champ requis manquant dans le JSON : $key"
        }
    }

    if ($Doc.product_code -ne $NatalPrompterProduct) {
        throw "product_code doit etre '$NatalPrompterProduct' (recu: $($Doc.product_code))"
    }

    if ([string]::IsNullOrWhiteSpace($Doc.profile_code)) {
        throw "profile_code ne peut pas etre vide"
    }

    if ($AllowedModes -notcontains $Doc.generation_mode) {
        throw "generation_mode invalide : $($Doc.generation_mode) (attendu: $($AllowedModes -join ', '))"
    }

    $effort = "$($Doc.max_reasoning_effort)".ToLower()
    if ($AllowedReasoning -notcontains $effort) {
        throw "max_reasoning_effort invalide : $effort"
    }

    if (-not $Doc.chapter_models.default_model) {
        throw "chapter_models.default_model requis"
    }

    if ($Doc.evidence.enabled -eq $true -and -not $Doc.evidence.policy) {
        throw "evidence.policy requis lorsque evidence.enabled est true"
    }
}

function Get-ProfileJsonText {
    if (-not [string]::IsNullOrWhiteSpace($Json)) {
        return $Json
    }
    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw "Preciser -Path ou -Json pour -Submit"
    }
    $full = if ([System.IO.Path]::IsPathRooted($Path)) { $Path } else { Join-Path $repoRoot $Path }
    if (-not (Test-Path -LiteralPath $full)) {
        throw "Fichier introuvable : $full"
    }
    return Get-Content -LiteralPath $full -Raw
}

Import-DotEnv (Join-Path $repoRoot ".env")
Ensure-InterpretationProfilesSchema

$actionCount = @($Submit, $List, $Get, $Delete) | Where-Object { $_ } | Measure-Object | Select-Object -ExpandProperty Count
if ($actionCount -ne 1) {
    throw "Preciser exactement une action : -Submit, -List, -Get ou -Delete"
}

if ($List) {
    $q = @"
SELECT profile_code, product_code, schema_version, is_active, updated_at::text
FROM llm_interpretation_profiles
ORDER BY profile_code;
"@
    Write-Host "Profils en base :"
    Invoke-ProjectPsql -Sql $q | ForEach-Object { Write-Host $_ }
    exit 0
}

if ($Get) {
    if ([string]::IsNullOrWhiteSpace($ProfileCode)) {
        throw "Preciser -ProfileCode avec -Get"
    }
    $pc = Escape-SqlLiteral $ProfileCode
    $q = "SELECT profile_json::text FROM llm_interpretation_profiles WHERE profile_code = '$pc' AND is_active = true;"
    $raw = Invoke-ProjectPsql -Sql $q
    if ([string]::IsNullOrWhiteSpace($raw)) {
        throw "Profil actif introuvable : $ProfileCode"
    }
    $pretty = $raw | ConvertFrom-Json | ConvertTo-Json -Depth 20
    if (-not [string]::IsNullOrWhiteSpace($OutFile)) {
        $pretty | Set-Content -LiteralPath $OutFile -Encoding utf8
        Write-Host "Ecrit : $OutFile"
    } else {
        Write-Host $pretty
    }
    exit 0
}

if ($Delete) {
    if ([string]::IsNullOrWhiteSpace($ProfileCode)) {
        throw "Preciser -ProfileCode avec -Delete"
    }
    $pc = Escape-SqlLiteral $ProfileCode
    if ($Hard) {
        $sql = "DELETE FROM llm_interpretation_profiles WHERE profile_code = '$pc';"
        Invoke-ProjectPsql -Sql $sql | Out-Null
        Write-Host "Supprime (hard) : $ProfileCode"
    } else {
        $sql = @"
UPDATE llm_interpretation_profiles
SET is_active = false, updated_at = NOW()
WHERE profile_code = '$pc';
"@
        Invoke-ProjectPsql -Sql $sql | Out-Null
        Write-Host "Desactive (soft) : $ProfileCode"
    }
    Write-Host "Redemarrer astral_llm_api pour prendre en compte le changement."
    exit 0
}

if ($Submit) {
    $text = Get-ProfileJsonText
    try {
        $doc = $text | ConvertFrom-Json
    } catch {
        throw "JSON invalide : $_"
    }
    Test-ProfileDocument -Doc $doc

    $profileCode = $doc.profile_code
    $productCode = $doc.product_code
    $schemaVersion = if ($doc.schema_version) { $doc.schema_version } else { "v1" }
    $jsonCompact = ($doc | ConvertTo-Json -Depth 30 -Compress)
    $jsonQuoted = New-DollarQuotedSql $jsonCompact
    $pc = Escape-SqlLiteral $profileCode
    $prod = Escape-SqlLiteral $productCode
    $sv = Escape-SqlLiteral $schemaVersion

    $sql = @"
INSERT INTO llm_interpretation_profiles (
    profile_code, product_code, schema_version, profile_json, is_active, updated_at
) VALUES (
    '$pc', '$prod', '$sv', $jsonQuoted::jsonb, true, NOW()
)
ON CONFLICT (profile_code) DO UPDATE SET
    product_code = EXCLUDED.product_code,
    schema_version = EXCLUDED.schema_version,
    profile_json = EXCLUDED.profile_json,
    is_active = true,
    updated_at = NOW();
"@
    Invoke-ProjectPsql -Sql $sql | Out-Null
    Write-Host "OK profil enregistre : $profileCode"
    Write-Host "Redemarrer astral_llm_api pour recharger le catalogue."
    exit 0
}

throw "Aucune action executee."
