param(
    [string]$GeneratedPayloadPath = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$goldenPath = Join-Path $repoRoot "tests\golden\natal_payload_v11_paris_1990.json"
$workDir = Join-Path $repoRoot "target\natal_v11_golden_diff"
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
        signal_keys = @($payload.signals | ForEach-Object { $_.signal_key })
        reading_plan = $payload.reading_plan
    }
}

function Projection-Json($payload) {
    Contract-Projection $payload | ConvertTo-Json -Depth 100 -Compress
}

$generated = if ([string]::IsNullOrWhiteSpace($GeneratedPayloadPath)) {
    Read-JsonFile $goldenPath
} else {
    Read-JsonFile $GeneratedPayloadPath
}

$golden = Read-JsonFile $goldenPath
$generatedProjection = Projection-Json $generated
$goldenProjection = Projection-Json $golden

if ($generatedProjection -ne $goldenProjection) {
    New-Item -ItemType Directory -Force -Path $workDir | Out-Null
    $generatedOut = Join-Path $workDir "generated_projection.json"
    $goldenOut = Join-Path $workDir "golden_projection.json"
    [System.IO.File]::WriteAllText($generatedOut, $generatedProjection, $utf8NoBom)
    [System.IO.File]::WriteAllText($goldenOut, $goldenProjection, $utf8NoBom)
    throw "Generated natal v11 projection differs from golden. See $generatedOut and $goldenOut."
}

Write-Host "Generated natal v11 projection matches golden."
