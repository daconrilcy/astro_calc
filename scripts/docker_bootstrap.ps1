<#
.SYNOPSIS
    Bootstrap du stack Docker Astral apres docker compose up.
#>
param(
    [string]$CalculatorUrl = "http://localhost:8080",
    [string]$LlmUrl = "http://localhost:8081"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "lib\astral_http_auth.ps1")
. (Join-Path $PSScriptRoot "lib\sync_llm_catalog.ps1")
Import-AstralDotEnv -RepoRoot $repoRoot

function Wait-HttpOk {
    param(
        [string]$Url,
        [int]$Retries = 45,
        [int]$DelaySec = 2
    )
    for ($i = 1; $i -le $Retries; $i++) {
        try {
            $r = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 5
            if ($r.StatusCode -ge 200 -and $r.StatusCode -lt 300) {
                return $true
            }
        } catch {
            Start-Sleep -Seconds $DelaySec
        }
    }
    return $false
}

function Wait-HttpReady {
    param(
        [string]$Url,
        [int]$Retries = 60,
        [int]$DelaySec = 2
    )
    for ($i = 1; $i -le $Retries; $i++) {
        try {
            $r = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 5
            if ($r.StatusCode -ge 200 -and $r.StatusCode -lt 300) {
                return $true
            }
            if ($r.StatusCode -eq 503 -and $i -eq $Retries) {
                Write-Host "  Derniere reponse readiness : $($r.Content)" -ForegroundColor Yellow
            }
        } catch {
            if ($_.Exception.Response -and $i -eq $Retries) {
                try {
                    $stream = $_.Exception.Response.Content.ReadAsStringAsync().Result
                    if ($stream) {
                        Write-Host "  Derniere reponse readiness : $stream" -ForegroundColor Yellow
                    }
                } catch { }
            }
            Start-Sleep -Seconds $DelaySec
        }
    }
    return $false
}

Write-Host "== Bootstrap Docker Astral ==" -ForegroundColor Cyan

Write-Host "`n[1/5] PostgreSQL"
docker compose exec -T postgres pg_isready -U $env:POSTGRES_USER -d $env:POSTGRES_DB | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "PostgreSQL non pret. Lancez : docker compose up -d postgres"
}
Write-Host "  OK" -ForegroundColor Green

Write-Host "`n[2/5] Services live"
if (-not (Wait-HttpOk "$LlmUrl/health/live")) {
    throw "astral_llm_api inaccessible sur $LlmUrl"
}
if (-not (Wait-HttpOk "$CalculatorUrl/health/live")) {
    throw "astral_calculator_api inaccessible sur $CalculatorUrl"
}
Write-Host "  OK" -ForegroundColor Green

Write-Host "`n[3/5] Referentiels LLM (profils + modeles)"
Sync-AstralLlmCatalog -RepoRoot $repoRoot
Write-Host "  OK" -ForegroundColor Green

Write-Host "`n[4/5] Reload astral_llm_api (catalogue en memoire)"
docker compose restart astral_llm_api | Out-Null
if (-not (Wait-HttpReady "$LlmUrl/health/ready")) {
    throw @"
astral_llm_api /health/ready indisponible apres restart.
Causes frequentes :
  prompts manquants dans l'image (rebuild : docker compose up -d --build astral_llm_api)
  profils non charges (relancer l'etape 3)
  base PostgreSQL inaccessible
Verifier : Invoke-WebRequest $LlmUrl/health/ready -SkipHttpErrorCheck
"@
}
Write-Host "  OK" -ForegroundColor Green

Write-Host "`n[5/6] Referentiel calculateur (json_db -> PostgreSQL)"
$simplifiedTableCheck = @"
SELECT to_regclass('public.astral_simplified_calculation_policies') IS NOT NULL AS ok;
"@
$tableExists = docker compose exec -T postgres psql -U $env:POSTGRES_USER -d $env:POSTGRES_DB -tAc $simplifiedTableCheck 2>$null
$tableExists = ($tableExists | ForEach-Object { $_.Trim() })
if ($tableExists -ne "t") {
    Write-Host "  Tables natal simplifie absentes — import json_db..." -ForegroundColor Yellow
    python (Join-Path $repoRoot "scripts\import_json_db_to_postgres.py")
    if ($LASTEXITCODE -ne 0) { throw "import_json_db_to_postgres.py a echoue" }
} else {
    Write-Host "  Tables natal simplifie presentes" -ForegroundColor Green
}

Write-Host "`n[6/6] Etat reference calculateur"
$calcHeaders = New-AstralAuthHeaders -Service calculator
$ref = Invoke-AstralJson -Method Get -Uri "$CalculatorUrl/v1/reference/status" -Headers $calcHeaders
Write-Host ("  status={0}" -f $ref.status)
foreach ($key in @("zodiac_signs", "planets", "houses", "aspects", "rulesets", "ephemeris_path")) {
    $value = $ref.checks.$key
    $color = if ($value) { "Green" } else { "Red" }
    Write-Host ("  {0}: {1}" -f $key, $value) -ForegroundColor $color
}

if ($ref.status -ne "ready") {
    throw @"
Calculator reference data MISSING or ephemerides unavailable.
Actions possibles :
  python scripts/import_json_db_to_postgres.py
  verifier ./ephe/se-2026a contient des fichiers .se1 (volume ./ephe:/app/ephe:ro)
  redemarrer : docker compose up -d --build
"@
}

Write-Host "`nBootstrap termine." -ForegroundColor Green
