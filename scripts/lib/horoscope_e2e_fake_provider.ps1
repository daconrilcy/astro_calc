# Bascule temporaire du routeur LLM en fake pour les tests E2E horoscope.

$script:OriginalHoroscopeProductPolicy = $null

function Invoke-HoroscopeE2ePsql {
    param(
        [string]$RepoRoot,
        [string]$Sql
    )

    if ([string]::IsNullOrWhiteSpace($env:DATABASE_URL)) {
        throw "DATABASE_URL absent (.env a la racine du depot)."
    }

    if (Get-Command psql -ErrorAction SilentlyContinue) {
        $previous = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $output = & psql $env:DATABASE_URL -v ON_ERROR_STOP=1 -t -A -c $Sql 2>&1
        $ErrorActionPreference = $previous
        if ($LASTEXITCODE -ne 0) {
            throw ($output | Out-String)
        }
        return ($output | Out-String).Trim()
    }

    Push-Location $RepoRoot
    try {
        $previous = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $output = docker compose exec -T postgres psql -U postgres -d astral -v ON_ERROR_STOP=1 -t -A -c $Sql 2>&1
        $ErrorActionPreference = $previous
        if ($LASTEXITCODE -ne 0) {
            throw ($output | Out-String)
        }
        return ($output | Out-String).Trim()
    } finally {
        Pop-Location
    }
}

function Restart-HoroscopeE2eLlmServices {
    param([string]$RepoRoot)

    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        return
    }

    Push-Location $RepoRoot
    try {
        docker compose restart astral_llm_api astral_llm_worker | Out-Null
        Start-Sleep -Seconds 4
    } finally {
        Pop-Location
    }
}

function Enable-HoroscopeE2eFakeLlmProvider {
    param([string]$RepoRoot = "")

    if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
        $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    }

    if (Get-Command Import-AstralDotEnv -ErrorAction SilentlyContinue) {
        Import-AstralDotEnv -RepoRoot $RepoRoot
    }

    $script:OriginalHoroscopeProductPolicy = Invoke-HoroscopeE2ePsql -RepoRoot $RepoRoot -Sql @"
SELECT default_provider || E'\t' || default_model || E'\t' || economic_model || E'\t' || is_active::text
FROM llm_product_default_engine
WHERE product_code = 'horoscope'
LIMIT 1;
"@

    Write-Host "E2E : bascule horoscope -> provider fake (sans OpenAI)..." -ForegroundColor Cyan
    Invoke-HoroscopeE2ePsql -RepoRoot $RepoRoot -Sql @"
INSERT INTO llm_product_generation_policies (
    product_code, max_domains, max_chapters, max_output_tokens, max_reasoning_effort, allow_chapter_orchestrated, is_active
) VALUES (
    'horoscope', 5, 1, 12000, 'medium', false, true
)
ON CONFLICT (product_code) DO UPDATE SET
    max_domains = EXCLUDED.max_domains,
    max_chapters = EXCLUDED.max_chapters,
    max_output_tokens = EXCLUDED.max_output_tokens,
    max_reasoning_effort = EXCLUDED.max_reasoning_effort,
    allow_chapter_orchestrated = EXCLUDED.allow_chapter_orchestrated,
    is_active = true;
"@ | Out-Null

    Invoke-HoroscopeE2ePsql -RepoRoot $RepoRoot -Sql @"
INSERT INTO llm_product_default_engine (
    product_code, default_provider, default_model, economic_model, is_active, notes
) VALUES (
    'horoscope', 'fake', 'fake-model', 'fake-model', true, 'temporary fake smoke override'
)
ON CONFLICT (product_code) DO UPDATE SET
    default_provider = EXCLUDED.default_provider,
    default_model = EXCLUDED.default_model,
    economic_model = EXCLUDED.economic_model,
    notes = EXCLUDED.notes,
    is_active = true,
    updated_at = NOW();
"@ | Out-Null

    Restart-HoroscopeE2eLlmServices -RepoRoot $RepoRoot
}

function Restore-HoroscopeE2eLlmProvider {
    param([string]$RepoRoot = "")

    if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
        $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    }

    Write-Host "E2E : restauration provider horoscope..." -ForegroundColor Cyan
    if ([string]::IsNullOrWhiteSpace($script:OriginalHoroscopeProductPolicy)) {
        Invoke-HoroscopeE2ePsql -RepoRoot $RepoRoot -Sql "DELETE FROM llm_product_default_engine WHERE product_code = 'horoscope';" | Out-Null
    } else {
        $parts = $script:OriginalHoroscopeProductPolicy -split "`t"
        if ($parts.Count -ne 4) {
            throw "Etat initial horoscope invalide: $script:OriginalHoroscopeProductPolicy"
        }
        $provider = $parts[0].Replace("'", "''")
        $model = $parts[1].Replace("'", "''")
        $economic = $parts[2].Replace("'", "''")
        $active = if ($parts[3] -eq "true") { "true" } else { "false" }
        Invoke-HoroscopeE2ePsql -RepoRoot $RepoRoot -Sql @"
INSERT INTO llm_product_default_engine (
    product_code, default_provider, default_model, economic_model, is_active, notes
) VALUES (
    'horoscope', '$provider', '$model', '$economic', $active, 'restored after fake smoke'
)
ON CONFLICT (product_code) DO UPDATE SET
    default_provider = EXCLUDED.default_provider,
    default_model = EXCLUDED.default_model,
    economic_model = EXCLUDED.economic_model,
    notes = EXCLUDED.notes,
    is_active = EXCLUDED.is_active,
    updated_at = NOW();
"@ | Out-Null
    }

    Restart-HoroscopeE2eLlmServices -RepoRoot $RepoRoot
    $script:OriginalHoroscopeProductPolicy = $null
}
