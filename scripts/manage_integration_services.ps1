<#
.SYNOPSIS
    Gere le catalogue llm_integration_services en base canonique.

.DESCRIPTION
    -Submit : enregistre une ligne (fichier json_db ou -Json inline)
    -List   : liste les services
    -Get    : detail d'un service_code

.EXAMPLE
    .\scripts\manage_integration_services.ps1 -Submit -Path json_db\llm_integration_services.json

.EXAMPLE
    .\scripts\manage_integration_services.ps1 -List

.EXAMPLE
    .\scripts\manage_integration_services.ps1 -Get -ServiceCode natal_simplified
#>
param(
    [switch]$Submit,
    [string]$Path = "",
    [string]$Json = "",
    [switch]$List,
    [string]$ServiceCode = "",
    [switch]$Get
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

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
    $tag = "example_json_body"
    while ($Content -like "*`$$tag`$*") {
        $tag = "${tag}_x"
    }
    return "`$$tag`$$Content`$$tag`$"
}

function Ensure-IntegrationSchema {
    $files = @(
        "astral_llm\crates\astral_llm_infra\sql\llm_interpretation_profiles.sql",
        "astral_llm\crates\astral_llm_infra\sql\llm_integration_services.sql"
    )
    foreach ($rel in $files) {
        $sqlPath = Join-Path $repoRoot $rel
        if (-not (Test-Path -LiteralPath $sqlPath)) {
            throw "Fichier schema introuvable : $sqlPath"
        }
        Invoke-ProjectPsql -Sql (Get-Content -LiteralPath $sqlPath -Raw) | Out-Null
    }
    Invoke-ProjectPsql -Sql @"
ALTER TABLE llm_integration_services
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

DO `$`$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'llm_integration_services'::regclass
          AND contype IN ('p', 'u')
          AND conkey = ARRAY[
              (SELECT attnum
               FROM pg_attribute
               WHERE attrelid = 'llm_integration_services'::regclass
                 AND attname = 'service_code')
          ]::smallint[]
    ) THEN
        ALTER TABLE llm_integration_services
            ADD CONSTRAINT pk_llm_integration_services_service_code
            PRIMARY KEY (service_code);
    END IF;
END
`$`$;

CREATE INDEX IF NOT EXISTS idx_llm_integration_services_availability
    ON llm_integration_services (availability, sort_order);
"@ | Out-Null
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

function Test-ServiceRow {
    param($Row)
    $required = @(
        "service_code", "profile_code", "label_fr", "description_fr",
        "orchestration_mode", "calculation_mode", "payload_contract",
        "reading_output_contract", "availability", "sort_order"
    )
    foreach ($key in $required) {
        if (-not ($Row.PSObject.Properties.Name -contains $key)) {
            throw "Champ requis manquant : $key"
        }
    }
}

function Submit-ServiceRow {
    param($Row)
    Test-ServiceRow -Row $Row

    $exampleSql = "NULL"
    if ($null -ne $Row.example_request_json) {
        $exampleJson = ($Row.example_request_json | ConvertTo-Json -Depth 30 -Compress)
        $exampleSql = New-DollarQuotedSql -Content $exampleJson
    }

    $calcOut = if ($null -eq $Row.calculation_output_contract) { "NULL" } else { "'$(Escape-SqlLiteral $Row.calculation_output_contract)'" }
    $syncEp = if ($null -eq $Row.sync_endpoint) { "NULL" } else { "'$(Escape-SqlLiteral $Row.sync_endpoint)'" }
    $product = if ($Row.product_code) { $Row.product_code } else { "natal_prompter" }
    $asyncEp = if ($Row.async_endpoint) { $Row.async_endpoint } else { "POST /v1/jobs" }
    $reqContract = if ($Row.service_request_contract) { $Row.service_request_contract } else { "integration_job_request_v1" }
    $respContract = if ($Row.service_response_contract) { $Row.service_response_contract } else { "integration_job_status_v1" }
    $supportsAsync = if ($null -ne $Row.supports_async) { [bool]$Row.supports_async } else { $true }
    $supportsSync = if ($null -ne $Row.supports_sync_legacy) { [bool]$Row.supports_sync_legacy } else { $false }
    $supportsMercure = if ($null -ne $Row.supports_mercure) { [bool]$Row.supports_mercure } else { $false }

    $sql = @"
INSERT INTO llm_integration_services (
    service_code, profile_code, product_code, label_fr, description_fr,
    orchestration_mode, calculation_mode, service_request_contract, payload_contract,
    service_response_contract, calculation_output_contract, reading_output_contract,
    sync_endpoint, async_endpoint, supports_async, supports_sync_legacy, supports_mercure,
    availability, example_request_json, sort_order, updated_at
) VALUES (
    '$(Escape-SqlLiteral $Row.service_code)',
    '$(Escape-SqlLiteral $Row.profile_code)',
    '$(Escape-SqlLiteral $product)',
    '$(Escape-SqlLiteral $Row.label_fr)',
    '$(Escape-SqlLiteral $Row.description_fr)',
    '$(Escape-SqlLiteral $Row.orchestration_mode)',
    '$(Escape-SqlLiteral $Row.calculation_mode)',
    '$(Escape-SqlLiteral $reqContract)',
    '$(Escape-SqlLiteral $Row.payload_contract)',
    '$(Escape-SqlLiteral $respContract)',
    $calcOut,
    '$(Escape-SqlLiteral $Row.reading_output_contract)',
    $syncEp,
    '$(Escape-SqlLiteral $asyncEp)',
    $(if ($supportsAsync) { 'true' } else { 'false' }),
    $(if ($supportsSync) { 'true' } else { 'false' }),
    $(if ($supportsMercure) { 'true' } else { 'false' }),
    '$(Escape-SqlLiteral $Row.availability)',
    $exampleSql,
    $($Row.sort_order),
    NOW()
)
ON CONFLICT (service_code) DO UPDATE SET
    profile_code = EXCLUDED.profile_code,
    product_code = EXCLUDED.product_code,
    label_fr = EXCLUDED.label_fr,
    description_fr = EXCLUDED.description_fr,
    orchestration_mode = EXCLUDED.orchestration_mode,
    calculation_mode = EXCLUDED.calculation_mode,
    service_request_contract = EXCLUDED.service_request_contract,
    payload_contract = EXCLUDED.payload_contract,
    service_response_contract = EXCLUDED.service_response_contract,
    calculation_output_contract = EXCLUDED.calculation_output_contract,
    reading_output_contract = EXCLUDED.reading_output_contract,
    sync_endpoint = EXCLUDED.sync_endpoint,
    async_endpoint = EXCLUDED.async_endpoint,
    supports_async = EXCLUDED.supports_async,
    supports_sync_legacy = EXCLUDED.supports_sync_legacy,
    supports_mercure = EXCLUDED.supports_mercure,
    availability = EXCLUDED.availability,
    example_request_json = EXCLUDED.example_request_json,
    sort_order = EXCLUDED.sort_order,
    updated_at = NOW();
"@
    Invoke-ProjectPsql -Sql $sql | Out-Null
    Write-Host "Service soumis : $($Row.service_code)"
}

Import-DotEnv -EnvPath (Join-Path $repoRoot ".env")

if ($List) {
    Ensure-IntegrationSchema
    $rows = Invoke-ProjectPsql -Sql "SELECT service_code, profile_code, availability, calculation_mode, sort_order FROM llm_integration_services ORDER BY sort_order, service_code;"
    if ([string]::IsNullOrWhiteSpace($rows)) {
        Write-Host "Aucun service en base."
    } else {
        $rows
    }
    exit 0
}

if ($Get) {
    if ([string]::IsNullOrWhiteSpace($ServiceCode)) {
        throw "-Get requiert -ServiceCode"
    }
    Ensure-IntegrationSchema
    $code = Escape-SqlLiteral $ServiceCode
    $json = Invoke-ProjectPsql -Sql "SELECT row_to_json(t) FROM llm_integration_services t WHERE service_code = '$code';"
    if ([string]::IsNullOrWhiteSpace($json)) {
        throw "Service introuvable : $ServiceCode"
    }
    Write-Output $json
    exit 0
}

if ($Submit) {
    Ensure-IntegrationSchema
    if (-not [string]::IsNullOrWhiteSpace($Json)) {
        $doc = $Json | ConvertFrom-Json
        if ($doc.data) {
            foreach ($row in $doc.data) { Submit-ServiceRow -Row $row }
        } else {
            Submit-ServiceRow -Row $doc
        }
        exit 0
    }
    if ([string]::IsNullOrWhiteSpace($Path)) {
        $Path = Join-Path $repoRoot "json_db\llm_integration_services.json"
    } elseif (-not [System.IO.Path]::IsPathRooted($Path)) {
        $Path = Join-Path $repoRoot $Path
    }
    $file = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    foreach ($row in $file.data) {
        Submit-ServiceRow -Row $row
    }
    Write-Host "Soumission terminee depuis $Path"
    exit 0
}

Write-Host "Usage: -List | -Get -ServiceCode <code> | -Submit [-Path json_db\llm_integration_services.json]"
exit 1
