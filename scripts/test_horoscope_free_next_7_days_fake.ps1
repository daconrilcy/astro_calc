param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$CalculatorUrl = "http://127.0.0.1:8080"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    cargo test -p astral_llm_api --test horoscope_v1_tests horoscope_free_next_7_days
} finally {
    Pop-Location
}
