<#
.SYNOPSIS
    E2E Docker premium + premium_plus avec OpenAI reel (stack :8080/:8081).
#>
param(
    [string]$CalculatorUrl = "http://localhost:8080",
    [string]$LlmUrl = "http://localhost:8081",
    [string]$LlmApiKey = "",
    [switch]$SkipBootstrap,
    [switch]$SkipPremium,
    [switch]$SkipPremiumPlus,
    [switch]$SkipGenerate,
    [switch]$ValidateOnly,
    [string]$PremiumRequestPath = "",
    [string]$PremiumPlusRequestPath = "",
    [string]$PremiumOutputPath = "",
    [string]$PremiumPlusOutputPath = "",
    [int]$TimeoutSecPremium = 900,
    [int]$TimeoutSecPremiumPlus = 1800,
    [int]$EngineTimeoutMsPremium = 120000,
    [int]$EngineTimeoutMsPremiumPlus = 300000,
    [int]$WaitApiSec = 120,
    [switch]$AllowFake
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "lib\astral_http_auth.ps1")
Import-AstralDotEnv -RepoRoot $repoRoot

if ([string]::IsNullOrWhiteSpace($LlmApiKey)) {
    $LlmApiKey = $env:ASTRAL_LLM_API_KEY
}

if (-not $AllowFake -and [string]::IsNullOrWhiteSpace($env:OPENAI_API_KEY)) {
    throw "OPENAI_API_KEY requis pour E2E OpenAI reel (ou -AllowFake pour tests fake)."
}

if ([string]::IsNullOrWhiteSpace($PremiumRequestPath)) {
    $PremiumRequestPath = Join-Path $repoRoot "request-premium-rich.json"
}
if ([string]::IsNullOrWhiteSpace($PremiumPlusRequestPath)) {
    $PremiumPlusRequestPath = Join-Path $repoRoot "request-premium-plus-rich.json"
}
if ([string]::IsNullOrWhiteSpace($PremiumOutputPath)) {
    $PremiumOutputPath = Join-Path $repoRoot "output\premium_reading_e2e_docker.json"
}
if ([string]::IsNullOrWhiteSpace($PremiumPlusOutputPath)) {
    $PremiumPlusOutputPath = Join-Path $repoRoot "output\premium_plus_reading_e2e_docker.json"
}

$skipGen = $SkipGenerate -or $ValidateOnly
$stamp = Get-Date -Format "yyyyMMddHHmmss"

Write-Host "== E2E Docker Premium / Premium Plus (OpenAI) ==" -ForegroundColor Cyan
Write-Host "LLM URL : $LlmUrl"
if ($AllowFake) {
    Write-Host "Mode    : fake autorise (-AllowFake)" -ForegroundColor Yellow
} else {
    Write-Host "Mode    : OpenAI reel"
}
Write-Host ""

if (-not $SkipBootstrap) {
    Write-Host "[bootstrap] docker_bootstrap.ps1"
    & (Join-Path $PSScriptRoot "docker_bootstrap.ps1") `
        -CalculatorUrl $CalculatorUrl `
        -LlmUrl $LlmUrl
    Write-Host ""
}

$overallExit = 0

if (-not $SkipPremium) {
    Write-Host "[premium] test_natal_premium_profile.ps1" -ForegroundColor Cyan
    $premiumArgs = @{
        BaseUrl         = $LlmUrl
        RequestPath     = $PremiumRequestPath
        OutputPath      = $PremiumOutputPath
        TimeoutSec      = $TimeoutSecPremium
        EngineTimeoutMs = $EngineTimeoutMsPremium
        WaitApiSec      = $WaitApiSec
        IdempotencyKey  = "docker-e2e-premium-$stamp"
    }
    if (-not [string]::IsNullOrWhiteSpace($LlmApiKey)) {
        $premiumArgs["ApiKey"] = $LlmApiKey
    }
    if ($skipGen) {
        $premiumArgs["SkipGenerate"] = $true
    }
    if ($AllowFake) {
        $premiumArgs["UseFake"] = $true
    }

    & (Join-Path $PSScriptRoot "test_natal_premium_profile.ps1") @premiumArgs
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Host "Echec premium (exit $code)." -ForegroundColor Red
        exit $code
    }
    Write-Host "Premium OK." -ForegroundColor Green
    Write-Host ""
}

if (-not $SkipPremiumPlus) {
    Write-Host "[premium_plus] test_natal_premium_plus_profile.ps1" -ForegroundColor Cyan
    $plusArgs = @{
        BaseUrl         = $LlmUrl
        RequestPath     = $PremiumPlusRequestPath
        OutputPath      = $PremiumPlusOutputPath
        TimeoutSec      = $TimeoutSecPremiumPlus
        EngineTimeoutMs = $EngineTimeoutMsPremiumPlus
        WaitApiSec      = $WaitApiSec
        IdempotencyKey  = "docker-e2e-premium-plus-$stamp"
    }
    if (-not [string]::IsNullOrWhiteSpace($LlmApiKey)) {
        $plusArgs["ApiKey"] = $LlmApiKey
    }
    if ($skipGen) {
        $plusArgs["SkipGenerate"] = $true
    }
    if ($AllowFake) {
        $plusArgs["UseFake"] = $true
    }

    & (Join-Path $PSScriptRoot "test_natal_premium_plus_profile.ps1") @plusArgs
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Host "Echec premium_plus (exit $code)." -ForegroundColor Red
        exit $code
    }
    Write-Host "Premium Plus OK." -ForegroundColor Green
    Write-Host ""
}

Write-Host "E2E Docker Premium / Premium Plus termine." -ForegroundColor Green
exit $overallExit
