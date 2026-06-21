# Bascule temporaire du routeur LLM en fake pour les tests E2E natal simplifie.

$script:OriginalProductModelsConf = $null
$script:OriginalProfileJsonByCode = @{}
$script:SimplifiedFakeComposeOverridePath = $null
$script:SimplifiedE2eProfileCodes = @(
    "natal_simplified",
    "natal_light",
    "natal_basic",
    "natal_premium"
)

function Get-SimplifiedE2eComposeFileArgs {
    param([string]$RepoRoot)

    $args = @("-f", (Join-Path $RepoRoot "docker-compose.yml"))
    $hostDbPortComposeFile = Join-Path $RepoRoot "docker-compose.dev-db-port.yml"
    if (Test-Path -LiteralPath $hostDbPortComposeFile) {
        $args += @("-f", $hostDbPortComposeFile)
    }
    return $args
}

function Invoke-SimplifiedE2ePsql {
    param(
        [string]$RepoRoot,
        [string]$Sql
    )

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
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = "docker"
    $psi.WorkingDirectory = $RepoRoot
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.RedirectStandardInput = $true
    if ($psi.PSObject.Properties.Name -contains "StandardInputEncoding") {
        $psi.StandardInputEncoding = [System.Text.UTF8Encoding]::new($false)
    }
    if ($psi.PSObject.Properties.Name -contains "StandardOutputEncoding") {
        $psi.StandardOutputEncoding = [System.Text.UTF8Encoding]::new($false)
    }
    if ($psi.PSObject.Properties.Name -contains "StandardErrorEncoding") {
        $psi.StandardErrorEncoding = [System.Text.UTF8Encoding]::new($false)
    }
    foreach ($arg in @(
        "compose",
        "exec",
        "-T",
        "postgres",
        "psql",
        "-U",
        $user,
        "-d",
        $db,
        "-v",
        "ON_ERROR_STOP=1",
        "-t",
        "-A"
    )) {
        [void]$psi.ArgumentList.Add($arg)
    }
    $process = [System.Diagnostics.Process]::Start($psi)
    $process.StandardInput.WriteLine($Sql)
    $process.StandardInput.Close()
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw (($stdout + $stderr) | Out-String)
    }
    return ($stdout | Out-String).Trim()
}

function New-SimplifiedE2eDollarQuotedSql {
    param([string]$Value)

    $tag = "json"
    while ($Value.Contains("`$$tag`$")) {
        $tag = "$tag$([guid]::NewGuid().ToString('N').Substring(0, 8))"
    }
    return "`$$tag`$$Value`$$tag`$"
}

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

    $script:OriginalProfileJsonByCode = @{}
    foreach ($profileCode in $script:SimplifiedE2eProfileCodes) {
        $escapedProfileCode = $profileCode.Replace("'", "''")
        $profileJson = Invoke-SimplifiedE2ePsql -RepoRoot $RepoRoot -Sql "SELECT profile_json::text FROM llm_interpretation_profiles WHERE profile_code = '$escapedProfileCode' AND is_active = true;"
        if (-not [string]::IsNullOrWhiteSpace($profileJson)) {
            $script:OriginalProfileJsonByCode[$profileCode] = $profileJson
        }
    }

    Write-Host "E2E : bascule natal_prompter / natal_simplified -> provider fake (sans OpenAI)..." -ForegroundColor Cyan
    & (Join-Path $RepoRoot "scripts\set_product_llm_models.ps1") `
        -Product natal_prompter `
        -Provider fake `
        -Chapters fake-model `
        -Summary fake-model | Out-Null

    $profileSql = @"
UPDATE llm_interpretation_profiles
SET profile_json = jsonb_set(
    jsonb_set(
        jsonb_set(profile_json, '{chapter_models,default_provider}', '"fake"'::jsonb, true),
        '{chapter_models,default_model}', '"fake-model"'::jsonb, true
    ),
    '{chapter_models,summary_model}', '"fake-model"'::jsonb, true
),
updated_at = NOW()
WHERE profile_code IN ('natal_simplified', 'natal_light', 'natal_basic', 'natal_premium');
"@
    Invoke-SimplifiedE2ePsql -RepoRoot $RepoRoot -Sql $profileSql | Out-Null

    if (Get-Command docker -ErrorAction SilentlyContinue) {
        $script:SimplifiedFakeComposeOverridePath = Join-Path $RepoRoot "output\simplified-e2e-fake-provider.override.yml"
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $script:SimplifiedFakeComposeOverridePath) | Out-Null
        @"
services:
  astral_llm_api:
    environment:
      ASTRAL_LLM_ENABLE_FAKE: "true"
      ASTRAL_LLM_DEFAULT_PROVIDER: fake
      ASTRAL_LLM_DEFAULT_MODEL: fake-model
  astral_llm_worker:
    environment:
      ASTRAL_LLM_ENABLE_FAKE: "true"
      ASTRAL_LLM_DEFAULT_PROVIDER: fake
      ASTRAL_LLM_DEFAULT_MODEL: fake-model
"@ | Set-Content -LiteralPath $script:SimplifiedFakeComposeOverridePath -Encoding utf8
        $composeFileArgs = Get-SimplifiedE2eComposeFileArgs -RepoRoot $RepoRoot
        docker compose @composeFileArgs -f $script:SimplifiedFakeComposeOverridePath up -d --no-build --force-recreate astral_llm_api astral_llm_worker | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "docker compose fake override failed" }
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

    foreach ($profileCode in $script:OriginalProfileJsonByCode.Keys) {
        $profileJson = New-SimplifiedE2eDollarQuotedSql $script:OriginalProfileJsonByCode[$profileCode]
        $escapedProfileCode = $profileCode.Replace("'", "''")
        $restoreProfileSql = @"
UPDATE llm_interpretation_profiles
SET profile_json = $profileJson::jsonb,
    updated_at = NOW()
WHERE profile_code = '$escapedProfileCode';
"@
        Invoke-SimplifiedE2ePsql -RepoRoot $RepoRoot -Sql $restoreProfileSql | Out-Null
    }

    if (Get-Command docker -ErrorAction SilentlyContinue) {
        $composeFileArgs = Get-SimplifiedE2eComposeFileArgs -RepoRoot $RepoRoot
        docker compose @composeFileArgs up -d --no-build --force-recreate astral_llm_api astral_llm_worker | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "docker compose restore after fake override failed" }
        if (-not [string]::IsNullOrWhiteSpace($script:SimplifiedFakeComposeOverridePath) -and (Test-Path -LiteralPath $script:SimplifiedFakeComposeOverridePath)) {
            Remove-Item -LiteralPath $script:SimplifiedFakeComposeOverridePath -Force
        }
        Start-Sleep -Seconds 3
    }

    $script:OriginalProductModelsConf = $null
    $script:OriginalProfileJsonByCode = @{}
    $script:SimplifiedFakeComposeOverridePath = $null
}
