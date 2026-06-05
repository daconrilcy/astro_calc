<#
.SYNOPSIS
    Tests HTTP reels — POST /v1/calculations/natal/simplified (tous les cas positifs + negatifs).

.DESCRIPTION
    Valide la matrice input_precision / computed_scope, llm_payload, limitations et erreurs 422.
    Necessite calculateur demarre, PostgreSQL pret, ephemerides SWE.

.EXAMPLE
    .\scripts\test_natal_simplified_calculator.ps1

.EXAMPLE
    .\scripts\test_natal_simplified_calculator.ps1 -Case date_only,complete_birth_data

.EXAMPLE
    .\scripts\test_natal_simplified_calculator.ps1 -NegativeOnly
#>
param(
    [string]$CalculatorBase = "",
    [string[]]$Case = @(),
    [switch]$PositiveOnly,
    [switch]$NegativeOnly,
    [switch]$SaveOutputs,
    [switch]$Bootstrap,
    [string]$OutputDir = "",
    [int]$WaitReadySec = 120,
    [int]$TimeoutSec = 120
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "lib\astral_http_auth.ps1")
. (Join-Path $PSScriptRoot "lib\simplified_natal_cases.ps1")
. (Join-Path $PSScriptRoot "lib\simplified_natal_assertions.ps1")
Import-AstralDotEnv -RepoRoot $repoRoot

if ([string]::IsNullOrWhiteSpace($CalculatorBase)) {
    $hostName = if ($env:ASTRAL_CALCULATOR_HOST) { $env:ASTRAL_CALCULATOR_HOST } else { "127.0.0.1" }
    $port = if ($env:ASTRAL_CALCULATOR_PORT) { $env:ASTRAL_CALCULATOR_PORT } else { "8080" }
    $CalculatorBase = "http://${hostName}:$port"
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\simplified_natal\calculator"
}
if ($SaveOutputs) {
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
}

$kind = "all"
if ($PositiveOnly) { $kind = "positive" }
if ($NegativeOnly) { $kind = "negative" }
$cases = Select-SimplifiedNatalCases -Labels $Case -Kind $kind

Write-SimplifiedTestBanner -Title "Test calculateur natal simplifie" -CalculatorBase $CalculatorBase

$headers = New-AstralAuthHeaders -Service calculator
Write-Host "Attente readiness calculateur..."
Test-AstralServiceReady -BaseUrl $CalculatorBase -Headers $headers -TimeoutSec $WaitReadySec | Out-Null

if ($Bootstrap) {
    Write-Host "Bootstrap json_db (tables natal simplifie)..." -ForegroundColor Cyan
    python (Join-Path $repoRoot "scripts\import_json_db_to_postgres.py")
    if ($LASTEXITCODE -ne 0) { throw "import_json_db_to_postgres.py a echoue" }
}

$catalogIssue = Test-SimplifiedCatalogReady -CalculatorBase $CalculatorBase -Headers $headers
if ($catalogIssue) {
    Write-Host $catalogIssue -ForegroundColor Red
    exit 1
}

$passed = 0
$failed = 0
$failuresAll = [System.Collections.Generic.List[string]]::new()

foreach ($testCase in $cases) {
    $uri = "$($CalculatorBase.TrimEnd('/'))/v1/calculations/natal/simplified"
    $result = Invoke-AstralHttpWithStatus -Method Post -Uri $uri -Headers $headers -Body $testCase.Request -TimeoutSec $TimeoutSec

    if ($testCase.ExpectedStatus) {
        $ok = ($result.StatusCode -eq $testCase.ExpectedStatus)
        if (-not $ok) {
            $failed++
            $msg = "status=$($result.StatusCode) attendu=$($testCase.ExpectedStatus)"
            $failuresAll.Add("$($testCase.Label): $msg")
            Write-SimplifiedCaseResult -Label "$($testCase.Label) ($($testCase.Description))" -Passed $false -Failures @($msg)
        } else {
            $passed++
            Write-SimplifiedCaseResult -Label "$($testCase.Label) ($($testCase.Description))" -Passed $true
        }
        continue
    }

    if (-not $result.Ok) {
        $failed++
        $msg = "HTTP $($result.StatusCode)"
        if ($result.Body.error.message) { $msg += " — $($result.Body.error.message)" }
        $failuresAll.Add("$($testCase.Label): $msg")
        Write-SimplifiedCaseResult -Label "$($testCase.Label)" -Passed $false -Failures @($msg)
        continue
    }

    if ($SaveOutputs) {
        $outPath = Join-Path $OutputDir "$($testCase.Label).json"
        $result.Body | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $outPath -Encoding utf8
    }

    $caseFailures = Assert-SimplifiedCalculatorResponse -Response $result.Body -Case $testCase
    if ($caseFailures.Count -eq 0) {
        $passed++
        Write-SimplifiedCaseResult -Label "$($testCase.Label) — scope=$($result.Body.computed_scope) facts=$($result.Body.facts.Count) ambiguous=$($result.Body.ambiguous_facts.Count)" -Passed $true
    } else {
        $failed++
        foreach ($f in $caseFailures) { $failuresAll.Add("$($testCase.Label): $f") }
        Write-SimplifiedCaseResult -Label "$($testCase.Label)" -Passed $false -Failures $caseFailures
    }
}

Write-Host ""
Write-Host "Resultat : $passed OK, $failed FAIL sur $($cases.Count) cas" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })
if ($failed -gt 0) {
    exit 1
}

Write-Host "Calculator simplified OK." -ForegroundColor Green
