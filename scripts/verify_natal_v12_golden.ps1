param(
    [string]$GeneratedPayloadPath = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$crateDir = Join-Path $repoRoot "astral_calculator"
$goldenPath = Join-Path $repoRoot "tests\golden\natal_payload_v12_paris_1990.json"
$workDir = Join-Path $repoRoot "target\natal_v12_golden_diff"
$payloadUnderTestPath = Join-Path $workDir "payload_under_test.json"
$utf8NoBom = New-Object System.Text.UTF8Encoding $false

function Read-JsonFile($path) {
    Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
}

function Contract-Projection($payload) {
    [ordered]@{
        product_code = $payload.product_code
        reference_version_id = $payload.reference_version_id
        birth_datetime_utc = $payload.birth_datetime_utc
        angles = $payload.angles
        dignities = $payload.dignities
        chart_emphasis = $payload.chart_emphasis
        rulership_context = $payload.rulership_context
        house_axis_emphasis = $payload.house_axis_emphasis
        lunar_phase_context = $payload.lunar_phase_context
        signal_keys = @($payload.signals | ForEach-Object { $_.signal_key })
        reading_plan = $payload.reading_plan
    }
}

function Projection-Json($payload) {
    Contract-Projection $payload | ConvertTo-Json -Depth 100 -Compress
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
        "ASTRAL_SUBJECT_LABEL",
        "ASTRAL_BIRTH_DATETIME_UTC",
        "ASTRAL_LATITUDE_DEG",
        "ASTRAL_LONGITUDE_DEG",
        "ASTRAL_ALTITUDE_M",
        "ASTRAL_REFERENCE_VERSION_ID",
        "ASTRAL_CALCULATION_PROFILE_ID",
        "ASTRAL_ZODIACAL_REFERENCE_SYSTEM_ID",
        "ASTRAL_COORDINATE_REFERENCE_SYSTEM_ID",
        "ASTRAL_HOUSE_SYSTEM_ID",
        "ASTRAL_PRODUCT_CODE",
        "ASTRAL_EPHEMERIS_PATH"
    )
    $savedEnv = Save-Env $envNames

    Push-Location $crateDir
    try {
        [Environment]::SetEnvironmentVariable("ASTRAL_OUTPUT_FILE", $null, "Process")
        $env:ASTRAL_OUTPUT_MODE = "stdout"
        $env:ASTRAL_SUBJECT_LABEL = "Test"
        $env:ASTRAL_BIRTH_DATETIME_UTC = "1990-01-02T03:04:05Z"
        $env:ASTRAL_LATITUDE_DEG = "48.8566"
        $env:ASTRAL_LONGITUDE_DEG = "2.3522"
        [Environment]::SetEnvironmentVariable("ASTRAL_ALTITUDE_M", $null, "Process")
        $env:ASTRAL_REFERENCE_VERSION_ID = "1"
        [Environment]::SetEnvironmentVariable("ASTRAL_CALCULATION_PROFILE_ID", $null, "Process")
        $env:ASTRAL_ZODIACAL_REFERENCE_SYSTEM_ID = "1"
        $env:ASTRAL_COORDINATE_REFERENCE_SYSTEM_ID = "1"
        $env:ASTRAL_HOUSE_SYSTEM_ID = "1"
        $env:ASTRAL_PRODUCT_CODE = "basic"
        $env:ASTRAL_EPHEMERIS_PATH = "..\ephe\se-2026a"

        $generatedJsonLines = cargo run --quiet --features swisseph-engine
        if ($LASTEXITCODE -ne 0) {
            throw "cargo run failed with exit code $LASTEXITCODE"
        }
        $generatedJson = $generatedJsonLines -join [Environment]::NewLine
        $generated = $generatedJson | ConvertFrom-Json
    } finally {
        Pop-Location
        Restore-Env $savedEnv
    }
} else {
    $generated = Read-JsonFile $GeneratedPayloadPath
}

New-Item -ItemType Directory -Force -Path $workDir | Out-Null
[System.IO.File]::WriteAllText(
    $payloadUnderTestPath,
    ($generated | ConvertTo-Json -Depth 100),
    $utf8NoBom
)

Push-Location $crateDir
try {
    $schemaEnv = Save-Env @("NATAL_V12_SCHEMA_PAYLOAD_PATH")
    try {
        $env:NATAL_V12_SCHEMA_PAYLOAD_PATH = $payloadUnderTestPath
        cargo test --quiet --test contract_basic_v8_tests external_payload_matches_json_schema_v12_when_requested
        if ($LASTEXITCODE -ne 0) {
            throw "schema validation failed for $payloadUnderTestPath"
        }
    } finally {
        Restore-Env $schemaEnv
    }
} finally {
    Pop-Location
}

$golden = Read-JsonFile $goldenPath
$generatedProjection = Projection-Json $generated
$goldenProjection = Projection-Json $golden

if ($generatedProjection -ne $goldenProjection) {
    $generatedOut = Join-Path $workDir "generated_projection.json"
    $goldenOut = Join-Path $workDir "golden_projection.json"
    [System.IO.File]::WriteAllText($generatedOut, $generatedProjection, $utf8NoBom)
    [System.IO.File]::WriteAllText($goldenOut, $goldenProjection, $utf8NoBom)
    throw "Generated natal v12 projection differs from golden. See $generatedOut and $goldenOut."
}

Write-Host "Generated natal v12 projection matches golden."
