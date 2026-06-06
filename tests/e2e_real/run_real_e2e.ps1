param(
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [string]$LlmUrl = "http://127.0.0.1:8081",
    [int]$ReadyTimeoutSec = 120,
    [int]$PollTimeoutSec = 300,
    [string]$ReportPath = "",
    [string]$DiagnosticsDir = "",
    [int]$DockerLogTail = 250
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHHmmssZ")

if ([string]::IsNullOrWhiteSpace($ReportPath)) {
    $reportDir = Join-Path $repoRoot "output\e2e_real_reports"
    New-Item -ItemType Directory -Force -Path $reportDir | Out-Null
    $ReportPath = Join-Path $reportDir "real_e2e_report_$timestamp.md"
} else {
    $reportParent = Split-Path -Parent $ReportPath
    if (-not [string]::IsNullOrWhiteSpace($reportParent)) {
        New-Item -ItemType Directory -Force -Path $reportParent | Out-Null
    }
}

if ([string]::IsNullOrWhiteSpace($DiagnosticsDir)) {
    $DiagnosticsDir = Join-Path $repoRoot "output\e2e_real_reports\diagnostics_$timestamp"
}
New-Item -ItemType Directory -Force -Path $DiagnosticsDir | Out-Null
$transcriptPath = Join-Path $DiagnosticsDir "powershell_transcript.log"
$dockerLogPaths = New-Object System.Collections.Generic.List[string]
$diagnosticPaths = New-Object System.Collections.Generic.List[string]
$transcriptStarted = $false

function Add-ReportLine {
    param([string]$Line = "")
    Add-Content -LiteralPath $ReportPath -Value $Line -Encoding UTF8
}

function Save-DockerLogs {
    param(
        [string]$Reason,
        [string[]]$Services = @("astral_llm_worker", "astral_llm_api", "astral_calculator_api", "postgres")
    )
    foreach ($service in $Services) {
        $path = Join-Path $DiagnosticsDir "$service.log"
        try {
            docker compose logs --no-color --tail $DockerLogTail $service 2>&1 | Out-File -LiteralPath $path -Encoding UTF8
            $dockerLogPaths.Add($path)
            $diagnosticPaths.Add($path)
        } catch {
            "Failed to collect docker logs for $service after $Reason`: $($_.Exception.Message)" |
                Out-File -LiteralPath $path -Encoding UTF8
            $dockerLogPaths.Add($path)
            $diagnosticPaths.Add($path)
        }
    }
}

function Save-JobQueueSnapshot {
    param([string]$Reason)
    $path = Join-Path $DiagnosticsDir "llm_jobs_snapshot.txt"
    try {
        $dbUser = (docker compose exec -T postgres printenv POSTGRES_USER).Trim()
        $dbName = (docker compose exec -T postgres printenv POSTGRES_DB).Trim()
        $sql = "select run_id, service_code, status, submitted_at, started_at, heartbeat_at, stale_after, completed_at, attempt_count from llm_jobs order by submitted_at desc limit 40;"
        docker compose exec -T postgres psql -U $dbUser -d $dbName -c $sql 2>&1 |
            Out-File -LiteralPath $path -Encoding UTF8
    } catch {
        "Failed to collect llm_jobs snapshot after $Reason`: $($_.Exception.Message)" |
            Out-File -LiteralPath $path -Encoding UTF8
    }
    $diagnosticPaths.Add($path)
}

function Get-CatalogueServicesForReport {
    param([string]$BaseUrl)
    try {
        $services = Invoke-RestMethod -Uri "$BaseUrl/v1/services" -Method Get -TimeoutSec 10
        return @($services.services | Where-Object { $_.availability -in @("active", "beta") })
    } catch {
        return @()
    }
}

$scripts = @(
    @{
        Path = Join-Path $PSScriptRoot "01_calculator_services.e2e.ps1"
        Name = "Calculator services"
        Args = @{
            CalculatorUrl = $CalculatorUrl
            ReadyTimeoutSec = $ReadyTimeoutSec
        }
    },
    @{
        Path = Join-Path $PSScriptRoot "02_llm_sync_services.e2e.ps1"
        Name = "LLM sync services"
        Args = @{
            CalculatorUrl = $CalculatorUrl
            LlmUrl = $LlmUrl
            ReadyTimeoutSec = $ReadyTimeoutSec
        }
    },
    @{
        Path = Join-Path $PSScriptRoot "03_integration_catalog_services.e2e.ps1"
        Name = "Integration catalogue services"
        Args = @{
            CalculatorUrl = $CalculatorUrl
            LlmUrl = $LlmUrl
            ReadyTimeoutSec = $ReadyTimeoutSec
            PollTimeoutSec = $PollTimeoutSec
        }
    }
)

Write-Host "=== Real Docker E2E suite ===" -ForegroundColor Cyan
Write-Host "Calculator: $CalculatorUrl"
Write-Host "LLM:        $LlmUrl"
Write-Host "Report:     $ReportPath"
Write-Host "Diagnostics:$DiagnosticsDir"

try {
    Start-Transcript -LiteralPath $transcriptPath -Force | Out-Null
    $transcriptStarted = $true
} catch {
    Write-Warning "Unable to start transcript: $($_.Exception.Message)"
}

$suiteStartedAt = Get-Date
$scriptResults = New-Object System.Collections.Generic.List[object]
$overallStatus = "PASSED"

Set-Content -LiteralPath $ReportPath -Value "# Real Docker E2E Report" -Encoding UTF8
Add-ReportLine ""
Add-ReportLine "- Generated at UTC: $($suiteStartedAt.ToUniversalTime().ToString("o"))"
Add-ReportLine "- Calculator URL: $CalculatorUrl"
Add-ReportLine "- LLM URL: $LlmUrl"
Add-ReportLine "- Ready timeout: $ReadyTimeoutSec s"
Add-ReportLine "- Job poll timeout: $PollTimeoutSec s"
Add-ReportLine "- Diagnostics dir: $DiagnosticsDir"
Add-ReportLine "- Transcript: $transcriptPath"
Add-ReportLine ""

foreach ($item in $scripts) {
    Write-Host ""
    Write-Host ">>> $($item.Path)" -ForegroundColor Cyan
    $scriptArgs = $item.Args
    $started = Get-Date
    try {
        $global:LASTEXITCODE = 0
        & $item.Path @scriptArgs
        $status = "PASSED"
        $errorMessage = ""
    } catch {
        $status = "FAILED"
        $errorMessage = $_.Exception.Message
        $overallStatus = "FAILED"
    }
    $finished = Get-Date
    $scriptResults.Add([ordered]@{
        Name = $item.Name
        Path = $item.Path
        Status = $status
        DurationSec = [Math]::Round(($finished - $started).TotalSeconds, 1)
        Error = $errorMessage
    })
    if ($status -eq "FAILED") {
        Write-Host "FAILED $($item.Name): $errorMessage" -ForegroundColor Red
        Save-JobQueueSnapshot -Reason $item.Name
        Save-DockerLogs -Reason $item.Name
        break
    }
}

$suiteFinishedAt = Get-Date
$catalogueServices = Get-CatalogueServicesForReport -BaseUrl $LlmUrl

Add-ReportLine "## Summary"
Add-ReportLine ""
Add-ReportLine "| Status | Started UTC | Finished UTC | Duration |"
Add-ReportLine "|---|---:|---:|---:|"
Add-ReportLine "| $overallStatus | $($suiteStartedAt.ToUniversalTime().ToString("o")) | $($suiteFinishedAt.ToUniversalTime().ToString("o")) | $([Math]::Round(($suiteFinishedAt - $suiteStartedAt).TotalSeconds, 1)) s |"
Add-ReportLine ""
Add-ReportLine "## Script Results"
Add-ReportLine ""
Add-ReportLine "| Test group | Status | Duration | Script |"
Add-ReportLine "|---|---:|---:|---|"
foreach ($result in $scriptResults) {
    $resultName = $result["Name"]
    $resultStatus = $result["Status"]
    $resultDurationSec = $result["DurationSec"]
    $resultPath = $result["Path"]
    Add-ReportLine "| $resultName | $resultStatus | $resultDurationSec s | $resultPath |"
}
Add-ReportLine ""

$failed = @($scriptResults | Where-Object { $_.Status -eq "FAILED" })
if ($failed.Count -gt 0) {
    Add-ReportLine "## Failures"
    Add-ReportLine ""
    foreach ($failure in $failed) {
        $failurePath = $failure["Path"]
        $failureError = $failure["Error"]
        Add-ReportLine "- $failurePath"
        Add-ReportLine ""
        Add-ReportLine '```text'
        Add-ReportLine $failureError
        Add-ReportLine '```'
        Add-ReportLine ""
    }
    Add-ReportLine ""
}

Add-ReportLine "## Diagnostics"
Add-ReportLine ""
Add-ReportLine "- Transcript: $transcriptPath"
if ($diagnosticPaths.Count -gt 0) {
    foreach ($path in $diagnosticPaths) {
        Add-ReportLine "- Diagnostic: $path"
    }
} else {
    Add-ReportLine "- Extra diagnostics: not collected because the suite passed."
}
Add-ReportLine ""

Add-ReportLine "## Services Covered"
Add-ReportLine ""
Add-ReportLine "### Calculator endpoints"
Add-ReportLine ""
Add-ReportLine "- GET /v1/contracts"
Add-ReportLine "- GET /v1/schemas/{version}"
Add-ReportLine "- POST /v1/calculations/validate"
Add-ReportLine "- POST /v1/calculations/natal"
Add-ReportLine "- POST /v1/calculations/natal/simplified"
Add-ReportLine "- POST /v1/calculations/horoscope/daily-natal"
Add-ReportLine ""
Add-ReportLine "### LLM endpoints"
Add-ReportLine ""
Add-ReportLine "- GET /v1/contracts"
Add-ReportLine "- GET /v1/providers"
Add-ReportLine "- GET /v1/schemas/{schema_version}"
Add-ReportLine "- POST /v1/readings/generate"
Add-ReportLine "- POST /v1/readings/validate"
Add-ReportLine "- POST /v1/readings/natal/simplified"
Add-ReportLine "- GET /v1/services"
Add-ReportLine "- GET /v1/services/{service_code}/contract"
Add-ReportLine "- POST /v1/jobs"
Add-ReportLine "- GET /v1/jobs/{run_id}"
Add-ReportLine ""
Add-ReportLine "### Active/beta integration services"
Add-ReportLine ""
if ($catalogueServices.Count -eq 0) {
    Add-ReportLine "- Catalogue unavailable during report finalization."
} else {
    foreach ($service in $catalogueServices) {
        $serviceCode = $service.service_code
        $availability = $service.availability
        $calculationMode = $service.calculation_mode
        Add-ReportLine "- $serviceCode ($availability, $calculationMode)"
    }
}
Add-ReportLine ""

Write-Host ""
if ($overallStatus -eq "PASSED") {
    if ($transcriptStarted) {
        Stop-Transcript | Out-Null
    }
    Write-Host "=== Real Docker E2E suite PASSED ===" -ForegroundColor Green
    Write-Host "Report written: $ReportPath" -ForegroundColor Green
    exit 0
}

if ($transcriptStarted) {
    Stop-Transcript | Out-Null
}
Write-Host "=== Real Docker E2E suite FAILED ===" -ForegroundColor Red
Write-Host "Report written: $ReportPath" -ForegroundColor Yellow
exit 1
