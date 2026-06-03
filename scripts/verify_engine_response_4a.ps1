param(
    [string]$GeneratedPayloadPath = "",
    [ValidateSet("compact", "standard", "rich")]
    [string]$ProjectionLevel = "rich"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$crateDir = Join-Path $repoRoot "rust_sqlx_connection_test"
$responseSchema = Join-Path $crateDir "schemas\astro_engine_response_v1.schema.json"
$llmSchema = Join-Path $crateDir "schemas\llm_projection_natal_v1.schema.json"

function Read-JsonFile($path) {
    Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
}

function Save-Env($names) {
    $values = @{}
    foreach ($name in $names) {
        $values[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
    }
    $values
}

function Restore-Env($values) {
    foreach ($name in $values.Keys) {
        [Environment]::SetEnvironmentVariable($name, $values[$name], "Process")
    }
}

if ([string]::IsNullOrWhiteSpace($GeneratedPayloadPath)) {
    $envNames = @(
        "ASTRAL_OUTPUT_FILE",
        "ASTRAL_OUTPUT_MODE",
        "ASTRAL_OUTPUT_CONTRACT",
        "ASTRAL_SUBJECT_LABEL",
        "ASTRAL_BIRTH_DATETIME_UTC",
        "ASTRAL_BIRTH_DATE",
        "ASTRAL_BIRTH_TIME",
        "ASTRAL_BIRTH_TIMEZONE",
        "ASTRAL_LATITUDE_DEG",
        "ASTRAL_LONGITUDE_DEG",
        "ASTRAL_LOCATION_LABEL",
        "ASTRAL_PROJECTION_LEVEL",
        "ASTRAL_REFERENCE_VERSION_ID",
        "ASTRAL_PRODUCT_CODE",
        "ASTRAL_EPHEMERIS_PATH"
    )
    $savedEnv = Save-Env $envNames

    Push-Location $crateDir
    try {
        [Environment]::SetEnvironmentVariable("ASTRAL_OUTPUT_FILE", $null, "Process")
        $env:ASTRAL_OUTPUT_MODE = "stdout"
        $env:ASTRAL_OUTPUT_CONTRACT = "engine"
        $env:ASTRAL_SUBJECT_LABEL = "Test"
        $env:ASTRAL_BIRTH_DATETIME_UTC = "1990-01-02T03:04:05Z"
        $env:ASTRAL_BIRTH_TIMEZONE = "UTC"
        [Environment]::SetEnvironmentVariable("ASTRAL_BIRTH_DATE", $null, "Process")
        [Environment]::SetEnvironmentVariable("ASTRAL_BIRTH_TIME", $null, "Process")
        $env:ASTRAL_LATITUDE_DEG = "48.8566"
        $env:ASTRAL_LONGITUDE_DEG = "2.3522"
        $env:ASTRAL_LOCATION_LABEL = "Paris, France"
        $env:ASTRAL_PROJECTION_LEVEL = $ProjectionLevel
        $env:ASTRAL_REFERENCE_VERSION_ID = "1"
        $env:ASTRAL_PRODUCT_CODE = "basic"
        $env:ASTRAL_EPHEMERIS_PATH = "..\ephe\se-2026a"

        $generatedJsonLines = cargo run --quiet --features swisseph-engine
        if ($LASTEXITCODE -ne 0) {
            throw "cargo run failed with exit code $LASTEXITCODE"
        }
        $generated = ($generatedJsonLines -join [Environment]::NewLine) | ConvertFrom-Json
    }
    finally {
        Pop-Location
        Restore-Env $savedEnv
    }
} else {
    $generated = Read-JsonFile $GeneratedPayloadPath
}

$required = @(
    "response_contract_version",
    "request_echo",
    "calculation_result",
    "audit_payload",
    "llm_payload"
)
foreach ($key in $required) {
    if (-not ($generated.PSObject.Properties.Name -contains $key)) {
        throw "missing top-level key: $key (not an astro_engine_response_v1 envelope)"
    }
}

if ($generated.response_contract_version -ne "astro_engine_response_v1") {
    throw "response_contract_version must be astro_engine_response_v1"
}
if ($generated.audit_payload.contract_version -ne "natal_structured_v13") {
    throw "audit_payload.contract_version must be natal_structured_v13"
}
if ($generated.llm_payload.contract_version -ne "llm_projection_natal_v1") {
    throw "llm_payload.contract_version must be llm_projection_natal_v1"
}
if ($generated.llm_payload.projection_level -ne $ProjectionLevel) {
    throw "llm_payload.projection_level must be $ProjectionLevel"
}
if ($generated.request_echo.projection_level -ne $ProjectionLevel) {
    throw "request_echo.projection_level must be $ProjectionLevel"
}

Write-Host "Generated astro_engine_response_v1 envelope is structurally valid (projection=$ProjectionLevel)."
