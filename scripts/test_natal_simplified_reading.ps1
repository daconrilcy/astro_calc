<#
.SYNOPSIS
    Tests HTTP reels — POST /v1/readings/natal/simplified (orchestration calcul + lecture).

.DESCRIPTION
    Execute tous les cas positifs via le gateway LLM. Par defaut la suite E2E active -ForceFake (deterministe, sans OpenAI). Passez -UseReal pour une recette OpenAI optionnelle.
    Valide reading_completeness, structure natal_reading_v1, llm_controls et anti-degraded.
    Les cas negatifs attendent HTTP 400 INVALID_INPUT sans enveloppe orchestrée.

.EXAMPLE
    .\scripts\test_natal_simplified_reading.ps1 -NegativeOnly
    .\scripts\test_natal_simplified_reading.ps1

.EXAMPLE
    .\scripts\test_natal_simplified_reading.ps1 -ForceFake

.EXAMPLE
    .\scripts\test_natal_simplified_reading.ps1 -UseReal -TimeoutSec 900

.EXAMPLE
    .\scripts\test_natal_simplified_reading.ps1 -Case complete_birth_data -SaveOutputs
#>
param(
    [string]$LlmBase = "",
    [string[]]$Case = @(),
    [switch]$UseReal,
    [switch]$ForceFake,
    [switch]$SubmitProfile,
    [switch]$PositiveOnly,
    [switch]$NegativeOnly,
    [switch]$SaveOutputs,
    [string]$OutputDir = "",
    [string]$QualityMetricsPath = "",
    [int]$MinWordsPerChapter = 30,
    [int]$WaitReadySec = 120,
    [int]$TimeoutSec = 180
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "lib\astral_http_auth.ps1")
. (Join-Path $PSScriptRoot "lib\simplified_natal_cases.ps1")
. (Join-Path $PSScriptRoot "lib\simplified_natal_assertions.ps1")
. (Join-Path $PSScriptRoot "lib\simplified_e2e_llm_provider.ps1")
Import-AstralDotEnv -RepoRoot $repoRoot

$fakeProviderArmed = $false
if ($ForceFake -and -not $UseReal) {
    Enable-SimplifiedE2eFakeLlmProvider -RepoRoot $repoRoot
    $fakeProviderArmed = $true
}

try {
$LlmBase = Resolve-AstralLlmBaseForHost -LlmBase $LlmBase

if (-not $UseReal) {
    if ($env:ASTRAL_LLM_ENABLE_FAKE -eq "false") {
        throw "ASTRAL_LLM_ENABLE_FAKE=false - activez fake ou passez -UseReal."
    }
} else {
    $TimeoutSec = [Math]::Max($TimeoutSec, 900)
    if ([string]::IsNullOrWhiteSpace($env:OPENAI_API_KEY)) {
        throw "OPENAI_API_KEY requis avec -UseReal"
    }
}

$orchIssue = Test-AstralOrchestrationEnvIssue -LlmBase $LlmBase
if ($orchIssue) {
    Write-Host $orchIssue -ForegroundColor Red
    exit 1
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "output\natal_simplified\reading"
}
if ($SaveOutputs) {
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
}

if ($SubmitProfile) {
    $profilePath = Join-Path $repoRoot "config\natal_interpretation_profiles\natal_simplified.json"
    Write-Host "Soumission profil natal_simplified..."
    & (Join-Path $repoRoot "scripts\manage_natal_interpretation_profiles.ps1") -Submit -Path $profilePath
}

$casesKind = "positive"
if ($PositiveOnly) { $casesKind = "positive" }
if ($NegativeOnly) { $casesKind = "negative" }
$cases = Select-SimplifiedNatalCases -Labels $Case -Kind $casesKind

Write-SimplifiedTestBanner -Title "Test lecture natal simplifiee (orchestration)" -CalculatorBase "(via LLM)" -LlmBase $LlmBase

$headers = New-AstralAuthHeaders -Service llm
Write-Host "Attente readiness LLM..."
Test-AstralServiceReady -BaseUrl $LlmBase -Headers $headers -TimeoutSec $WaitReadySec | Out-Null

if (-not $UseReal) {
    $providerIssue = Test-LlmFakeProviderReady -LlmBase $LlmBase -Headers $headers
    if ($providerIssue) {
        Write-Host $providerIssue -ForegroundColor Red
        exit 1
    }
}

$calcBaseForProbe = Resolve-AstralCalculatorBaseForHost
$calcHeaders = New-AstralAuthHeaders -Service calculator
$catalogIssue = Test-SimplifiedCatalogReady -CalculatorBase $calcBaseForProbe -Headers $calcHeaders
if ($catalogIssue) {
    Write-Host $catalogIssue -ForegroundColor Red
    exit 1
}

$passed = 0
$failed = 0
$qualityMetrics = $null
if ($UseReal -and -not $NegativeOnly -and -not [string]::IsNullOrWhiteSpace($QualityMetricsPath)) {
    $qualityMetrics = New-SimplifiedOpenAiQualityMetrics -LlmBase $LlmBase -Headers $headers
}

foreach ($testCase in $cases) {
    $script:SimplifiedLastStrictWarnings = @()
    $body = $testCase.Request | ConvertTo-Json -Depth 20 | ConvertFrom-Json
    $body | Add-Member -NotePropertyName user_language -NotePropertyValue "fr" -Force
    $body | Add-Member -NotePropertyName audience_level -NotePropertyValue "beginner" -Force

    $uri = "$($LlmBase.TrimEnd('/'))/v1/readings/natal/simplified"
    $result = Invoke-AstralHttpWithStatus -Method Post -Uri $uri -Headers $headers -Body $body -TimeoutSec $TimeoutSec

    if ($testCase.ExpectedOrchestrationStatus) {
        if ($SaveOutputs) {
            $outPath = Join-Path $OutputDir "$($testCase.Label).error.json"
            $payload = if ($null -ne $result.Body) { $result.Body } else { [ordered]@{ status_code = $result.StatusCode; raw = $result.Raw } }
            $payload | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $outPath -Encoding utf8
        }
        $caseFailures = Assert-SimplifiedOrchestrationRejected -Result $result -Case $testCase
        if ($caseFailures.Count -eq 0) {
            $passed++
            Write-SimplifiedCaseResult -Label "$($testCase.Label) ($($testCase.Description))" -Passed $true
        } else {
            $failed++
            Write-SimplifiedCaseResult -Label "$($testCase.Label)" -Passed $false -Failures $caseFailures
        }
        continue
    }

    $apiResponse = $result.Body
    $hasSimplifiedEnvelope = ($null -ne $apiResponse) -and ($null -ne $apiResponse.calculation) -and ($null -ne $apiResponse.reading)

    if (-not $result.Ok -and -not $hasSimplifiedEnvelope) {
        if ($SaveOutputs) {
            $outPath = Join-Path $OutputDir "$($testCase.Label).error.json"
            $payload = if ($null -ne $apiResponse) { $apiResponse } else { [ordered]@{ status_code = $result.StatusCode; raw = $result.Raw } }
            $payload | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $outPath -Encoding utf8
        }
        $failed++
        $msg = "HTTP $($result.StatusCode)"
        if ($apiResponse.error.message) { $msg += " - $($apiResponse.error.message)" }
        if ($apiResponse.error.code -eq "CALCULATOR_UNAVAILABLE") {
            $msg += " (verifier ASTRAL_CALCULATOR_HOST/PORT et calculateur up)"
        }
        if ($qualityMetrics) {
            Add-SimplifiedOpenAiQualityCaseResult `
                -Metrics $qualityMetrics `
                -Label $testCase.Label `
                -RunId "" `
                -Success $false `
                -Failures @($msg) `
                -Warnings @()
        }
        Write-SimplifiedCaseResult -Label "$($testCase.Label)" -Passed $false -Failures @($msg)
        continue
    }

    if ($SaveOutputs) {
        $outPath = Join-Path $OutputDir "$($testCase.Label).json"
        if (-not $result.Ok) {
            $outPath = Join-Path $OutputDir "$($testCase.Label).error.json"
        }
        $apiResponse | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $outPath -Encoding utf8
    }

    $caseFailures = Assert-SimplifiedReadingResponse -ApiResponse $apiResponse -Case $testCase -MinWordsPerChapter $MinWordsPerChapter -StrictOpenAiQuality:($UseReal -and -not $NegativeOnly)
    if ($qualityMetrics) {
        $strictWarnings = @()
        if ($script:SimplifiedLastStrictWarnings) {
            $strictWarnings = @($script:SimplifiedLastStrictWarnings)
        }
        Add-SimplifiedOpenAiQualityCaseResult `
            -Metrics $qualityMetrics `
            -Label $testCase.Label `
            -RunId ([string]$apiResponse.run_id) `
            -Success ($caseFailures.Count -eq 0) `
            -Failures @($caseFailures) `
            -Warnings $strictWarnings
    }
    if ($caseFailures.Count -eq 0) {
        $passed++
        $content = Get-SimplifiedReadingContent -ApiResponse $apiResponse
        $words = 0
        if ($content.chapters) {
            $words = Get-SimplifiedWordCount -Text $content.chapters[0].body
        }
        Write-SimplifiedCaseResult -Label "$($testCase.Label) - run_id=$($apiResponse.run_id) mots=$words" -Passed $true
    } else {
        $failed++
        Write-SimplifiedCaseResult -Label "$($testCase.Label)" -Passed $false -Failures $caseFailures
    }
}

Write-Host ""
Write-Host "Resultat : $passed OK, $failed FAIL sur $($cases.Count) cas" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })
if ($qualityMetrics) {
    $qualityMetrics.generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    $qualityMetrics | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $QualityMetricsPath -Encoding utf8
}
if ($failed -gt 0) { exit 1 }
Write-Host "Reading simplified OK." -ForegroundColor Green
} finally {
    if ($fakeProviderArmed) {
        Restore-SimplifiedE2eLlmProvider -RepoRoot $repoRoot
    }
}
