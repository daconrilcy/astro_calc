<#
.SYNOPSIS
    Met a jour les modeles chapitres + summary en base (llm_product_default_engine).

.DESCRIPTION
    1. Editer config/llm_product_models.conf (ou passer -Product -Chapters -Summary)
    2. .\scripts\set_product_llm_models.ps1
    3. Redemarrer astral_llm_api (catalogue charge au boot)

.EXAMPLE
    .\scripts\set_product_llm_models.ps1

.EXAMPLE
    .\scripts\set_product_llm_models.ps1 -Product natal_prompter -Chapters gpt-5.4-mini -Summary gpt-5-nano
#>
param(
    [string]$ConfigPath = "",
    [string]$Product = "",
    [string]$Chapters = "",
    [string]$Summary = "",
    [string]$Provider = "openai",
    [switch]$Show
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

function Import-DotEnv {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return }
    Get-Content -LiteralPath $Path | ForEach-Object {
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

function Get-ProductModelRows {
    param([string]$Path)

    $rows = @()
    Get-Content -LiteralPath $Path | ForEach-Object {
        $line = $_.Trim()
        if ($line -eq "" -or $line.StartsWith("#")) { return }
        $parts = $line -split "\s+"
        if ($parts.Count -lt 3) {
            throw "Ligne invalide dans $Path : $line (attendu: product chapter summary [provider])"
        }
        $prov = if ($parts.Count -ge 4) { $parts[3] } else { "openai" }
        $rows += [PSCustomObject]@{
            Product  = $parts[0]
            Chapters = $parts[1]
            Summary  = $parts[2]
            Provider = $prov
        }
    }
    return $rows
}

Import-DotEnv (Join-Path $repoRoot ".env")

if ([string]::IsNullOrWhiteSpace($ConfigPath)) {
    $ConfigPath = Join-Path $repoRoot "config\llm_product_models.conf"
}

if ($Show) {
    $q = @"
SELECT product_code, default_model, economic_model, default_provider
FROM llm_product_default_engine WHERE is_active = true ORDER BY product_code;
"@
    Write-Host "Etat actuel (base) :"
    Invoke-ProjectPsql -Sql $q | ForEach-Object { Write-Host $_ }
    exit 0
}

$rows = @()
if (-not [string]::IsNullOrWhiteSpace($Product)) {
    if ([string]::IsNullOrWhiteSpace($Chapters) -or [string]::IsNullOrWhiteSpace($Summary)) {
        throw "Avec -Product, preciser -Chapters et -Summary."
    }
    $rows += [PSCustomObject]@{
        Product  = $Product
        Chapters = $Chapters
        Summary  = $Summary
        Provider = $Provider
    }
} else {
    if (-not (Test-Path -LiteralPath $ConfigPath)) {
        throw "Fichier introuvable : $ConfigPath"
    }
    $rows = Get-ProductModelRows -Path $ConfigPath
}

foreach ($row in $rows) {
    $pc = Escape-SqlLiteral $row.Product
    $ch = Escape-SqlLiteral $row.Chapters
    $su = Escape-SqlLiteral $row.Summary
    $pr = Escape-SqlLiteral $row.Provider
    $sql = @"
INSERT INTO llm_product_default_engine (
    product_code, default_provider, default_model, economic_model, is_active, notes
) VALUES (
    '$pc', '$pr', '$ch', '$su', true, 'config/llm_product_models.conf'
)
ON CONFLICT (product_code) DO UPDATE SET
    default_provider = EXCLUDED.default_provider,
    default_model = EXCLUDED.default_model,
    economic_model = EXCLUDED.economic_model,
    notes = EXCLUDED.notes,
    is_active = true,
    updated_at = NOW();
"@
    Invoke-ProjectPsql -Sql $sql | Out-Null
    Write-Host ("OK {0} : chapitres={1} summary={2} ({3})" -f $row.Product, $row.Chapters, $row.Summary, $row.Provider)
}

#
# Legacy products (historique) : l'API migre ces codes vers natal_prompter + interpretation_profile_code.
# On les desactive pour qu'ils ne soient pas visibles comme "moteurs" actifs dans les vues ops (-Show).
#
$deactivateLegacySql = @"
UPDATE llm_product_default_engine
SET is_active = false, updated_at = NOW()
WHERE product_code IN ('natal_basic', 'natal_premium');
"@
Invoke-ProjectPsql -Sql $deactivateLegacySql | Out-Null

Write-Host ""
Write-Host "Redemarrer astral_llm_api pour prendre en compte les modeles."
Write-Host "Verifier : .\scripts\set_product_llm_models.ps1 -Show"
