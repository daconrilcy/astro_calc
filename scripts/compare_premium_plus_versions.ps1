param(
    [string]$V1Path = "",
    [string]$V2Path = "",
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($V1Path)) {
    $V1Path = Join-Path $repoRoot "output\premium_plus_reading_e2e.json"
}
if ([string]::IsNullOrWhiteSpace($V2Path)) {
    $V2Path = Join-Path $repoRoot "output\premium_plus_reading_e2e_v2.json"
}
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\benchmark_premium_plus"
}

function Get-WordCount([string]$Text) {
    if ([string]::IsNullOrWhiteSpace($Text)) { return 0 }
    return ($Text -split '\s+').Where({ $_ -ne '' }).Count
}

function Test-GlobalFiller([string]$FactId) {
    $fillers = @(
        'element_balance',
        'modality_balance',
        'house_axis',
        'dominant_planet',
        'house_emphasis'
    )
    foreach ($f in $fillers) {
        if ($FactId -like "$f*") { return $true }
    }
    return $false
}

function Measure-Reading($payload) {
    $reading = $payload.reading
    if (-not $reading) { $reading = $payload }
    $chapters = @($reading.chapters)
    $byChapter = @{}
    $basisByChapter = @{}
    $fillerByChapter = @{}
    $totalWords = 0

    foreach ($ch in $chapters) {
        $wc = Get-WordCount $ch.body
        $byChapter[$ch.code] = $wc
        $totalWords += $wc
        $basis = @($ch.astro_basis)
        $basisByChapter[$ch.code] = $basis.Count
        $fillerByChapter[$ch.code] = @($basis | Where-Object {
            $_.fact_id -and (Test-GlobalFiller $_.fact_id)
        }).Count
    }

    $repairs = @()
    if ($payload.execution_audit) {
        $steps = @($payload.execution_audit.chapter_steps)
        foreach ($step in $steps) {
            if ($step.attempt -and $step.attempt -like '*repair*') {
                $repairs += [pscustomobject]@{
                    chapter = $step.chapter_code
                    attempt = $step.attempt
                }
            }
        }
    }

    $audit = $payload.execution_audit
    return [pscustomobject]@{
        label = $null
        chapter_count = $chapters.Count
        word_count_total = $totalWords
        word_count_by_chapter = $byChapter
        astro_basis_count_by_chapter = $basisByChapter
        global_filler_count_by_chapter = $fillerByChapter
        too_short_repairs = @($repairs | Where-Object { $_.attempt -like '*too_short*' }).Count
        opening_repairs = @($repairs | Where-Object { $_.attempt -like '*opening*' }).Count
        evidence_repairs = @($repairs | Where-Object { $_.attempt -like '*evidence*' }).Count
        latency_ms = if ($audit) { $audit.total_duration_ms } else { $null }
        token_input = if ($audit) { $audit.total_input_tokens } else { $null }
        token_output = if ($audit) { $audit.total_output_tokens } else { $null }
    }
}

if (-not (Test-Path -LiteralPath $V1Path)) {
    Write-Warning "V1 introuvable: $V1Path"
}
if (-not (Test-Path -LiteralPath $V2Path)) {
    Write-Warning "V2 introuvable: $V2Path — generez d'abord une lecture v2."
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$jsonOut = Join-Path $OutputDir "compare_premium_plus_$timestamp.json"
$csvOut = Join-Path $OutputDir "compare_premium_plus_$timestamp.csv"

$rows = @()
foreach ($entry in @(
    @{ Path = $V1Path; Label = "v1" },
    @{ Path = $V2Path; Label = "v2" }
)) {
    if (-not (Test-Path -LiteralPath $entry.Path)) { continue }
    $raw = Get-Content -LiteralPath $entry.Path -Raw | ConvertFrom-Json
    $m = Measure-Reading $raw
    $m.label = $entry.Label
    $rows += $m
}

$report = [pscustomobject]@{
    generated_at = (Get-Date).ToString("o")
    v1_path = $V1Path
    v2_path = $V2Path
    versions = $rows
}

$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $jsonOut -Encoding UTF8

$csvLines = @("version,chapter_count,word_count_total,too_short_repairs,opening_repairs,evidence_repairs,latency_ms,token_input,token_output")
foreach ($r in $rows) {
    $csvLines += ("{0},{1},{2},{3},{4},{5},{6},{7},{8}" -f `
        $r.label, $r.chapter_count, $r.word_count_total, `
        $r.too_short_repairs, $r.opening_repairs, $r.evidence_repairs, `
        $r.latency_ms, $r.token_input, $r.token_output)
}
$csvLines | Set-Content -LiteralPath $csvOut -Encoding UTF8

Write-Host "Comparaison exportee:"
Write-Host "  JSON: $jsonOut"
Write-Host "  CSV:  $csvOut"
foreach ($r in $rows) {
    Write-Host ("[{0}] chapters={1} words={2} too_short_repairs={3} opening_repairs={4}" -f `
        $r.label, $r.chapter_count, $r.word_count_total, $r.too_short_repairs, $r.opening_repairs)
}
