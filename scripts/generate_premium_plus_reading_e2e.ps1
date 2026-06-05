param(
    [string]$RequestPath = "",
    [string]$OutputPath = "",
    [string]$IdempotencyKey = "",
    [string]$BaseUrl = "",
    [string]$ApiKey = "",
    [string]$Model = "",
    [string]$SummaryModel = "",
    [string]$Provider = "",
    [int]$TimeoutSec = 1800,
    [int]$EngineTimeoutMs = 300000
)

$repoRoot = Split-Path -Parent $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($RequestPath)) {
    $RequestPath = Join-Path $repoRoot "request-premium-plus-rich.json"
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot "output\premium_plus_reading_e2e.json"
}
if ([string]::IsNullOrWhiteSpace($IdempotencyKey)) {
    $IdempotencyKey = "e2e-premium-plus-$(Get-Date -Format 'yyyyMMddHHmmss')"
}

& (Join-Path $PSScriptRoot "generate_premium_reading_e2e.ps1") `
    -RequestPath $RequestPath `
    -OutputPath $OutputPath `
    -IdempotencyKey $IdempotencyKey `
    -BaseUrl $BaseUrl `
    -ApiKey $ApiKey `
    -Model $Model `
    -SummaryModel $SummaryModel `
    -Provider $Provider `
    -TimeoutSec $TimeoutSec `
    -EngineTimeoutMs $EngineTimeoutMs
