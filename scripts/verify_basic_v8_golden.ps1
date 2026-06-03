param(
    [string]$GeneratedPayloadPath = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$crateDir = Join-Path $repoRoot "astral_calculator"
$goldenPath = Join-Path $repoRoot "tests\golden\basic_payload_v8_paris_1990.json"
$workDir = Join-Path $repoRoot "target\basic_v8_golden_diff"
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

$generated = if ([string]::IsNullOrWhiteSpace($GeneratedPayloadPath)) {
    Read-JsonFile $goldenPath
} else {
    Read-JsonFile $GeneratedPayloadPath
}

New-Item -ItemType Directory -Force -Path $workDir | Out-Null
[System.IO.File]::WriteAllText(
    $payloadUnderTestPath,
    ($generated | ConvertTo-Json -Depth 100),
    $utf8NoBom
)

Push-Location $crateDir
try {
    $schemaEnv = Save-Env @("BASIC_V8_SCHEMA_PAYLOAD_PATH")
    try {
        $env:BASIC_V8_SCHEMA_PAYLOAD_PATH = $payloadUnderTestPath
        cargo test --quiet --test contract_basic_v8_tests external_payload_matches_json_schema_v8_when_requested
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
    throw "Generated Basic v8 projection differs from golden. See $generatedOut and $goldenOut."
}

Write-Host "Generated Basic v8 projection matches golden."
