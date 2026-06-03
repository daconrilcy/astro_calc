param(
    [Parameter(Mandatory = $true)]
    [string]$RunId,
    [string]$BaseUrl = "",
    [string]$ApiKey = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    $llmHost = if ($env:ASTRAL_LLM_HOST) { $env:ASTRAL_LLM_HOST } else { "127.0.0.1" }
    $llmPort = if ($env:ASTRAL_LLM_PORT) { $env:ASTRAL_LLM_PORT } else { "8081" }
    $BaseUrl = "http://${llmHost}:${llmPort}"
}

if ([string]::IsNullOrWhiteSpace($ApiKey)) {
    $ApiKey = $env:ASTRAL_LLM_API_KEY
}

$headers = @{}
if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
    $headers["Authorization"] = "Bearer $ApiKey"
}

$uri = "$($BaseUrl.TrimEnd('/'))/v1/runs/$RunId"
Write-Host "GET $uri"

$audit = Invoke-RestMethod -Uri $uri -Headers $headers
$audit | ConvertTo-Json -Depth 20
