# Bascule temporaire du routeur LLM en fake pour les tests E2E natal simplifie.

$script:OriginalProductModelsConf = $null

function Enable-SimplifiedE2eFakeLlmProvider {
    param(
        [string]$RepoRoot = ""
    )

    if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
        $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    }

    $confPath = Join-Path $RepoRoot "config\llm_product_models.conf"
    if (Test-Path -LiteralPath $confPath) {
        $script:OriginalProductModelsConf = Get-Content -LiteralPath $confPath -Raw
    }

    Write-Host "E2E : bascule natal_prompter -> provider fake (sans OpenAI)..." -ForegroundColor Cyan
    & (Join-Path $RepoRoot "scripts\set_product_llm_models.ps1") `
        -Product natal_prompter `
        -Provider fake `
        -Chapters fake-model `
        -Summary fake-model | Out-Null

    if (Get-Command docker -ErrorAction SilentlyContinue) {
        docker compose restart astral_llm_api | Out-Null
        Start-Sleep -Seconds 3
    }
}

function Restore-SimplifiedE2eLlmProvider {
    param(
        [string]$RepoRoot = ""
    )

    if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
        $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    }

    Write-Host "E2E : restauration modeles LLM depuis config/llm_product_models.conf..." -ForegroundColor Cyan
    & (Join-Path $RepoRoot "scripts\set_product_llm_models.ps1") | Out-Null

    if (Get-Command docker -ErrorAction SilentlyContinue) {
        docker compose restart astral_llm_api | Out-Null
        Start-Sleep -Seconds 3
    }

    $script:OriginalProductModelsConf = $null
}
