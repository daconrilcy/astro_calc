<#
.SYNOPSIS
    Pousse en base les profils d'interpretation et les modeles produit LLM canoniques.

.DESCRIPTION
    Source operationnelle :
      - config/natal_interpretation_profiles/*.json  -> llm_interpretation_profiles
      - config/llm_product_models.conf               -> llm_product_default_engine

    Redemarrer astral_llm_api et astral_llm_worker apres cet appel pour recharger le catalogue.
#>
function Sync-AstralLlmCatalog {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RepoRoot
    )

    $profileDir = Join-Path $RepoRoot "config\natal_interpretation_profiles"
    if (-not (Test-Path -LiteralPath $profileDir)) {
        throw "Interpretation profiles directory not found: $profileDir"
    }

    $profilesScript = Join-Path $RepoRoot "scripts\manage_natal_interpretation_profiles.ps1"
    $modelsScript = Join-Path $RepoRoot "scripts\set_product_llm_models.ps1"
    $providerCatalogScript = Join-Path $RepoRoot "scripts\sync_provider_model_catalog.py"
    foreach ($path in @($profilesScript, $modelsScript, $providerCatalogScript)) {
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Required script not found: $path"
        }
    }

    Get-ChildItem -LiteralPath $profileDir -Filter "*.json" |
        Sort-Object Name |
        ForEach-Object {
            Write-Host "  submit $($_.Name)"
            & $profilesScript -Submit -Path $_.FullName
        }

    Write-Host "  sync provider/model catalog"
    & python $providerCatalogScript

    Write-Host "  sync config/llm_product_models.conf"
    & $modelsScript
}
