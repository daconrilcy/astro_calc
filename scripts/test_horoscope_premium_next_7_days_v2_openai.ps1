<#
.SYNOPSIS
    Runs real OpenAI certification for Premium 7-day horoscope V2.

.DESCRIPTION
    Loads .env, requires OPENAI_API_KEY, submits horoscope_premium_next_7_days_natal
    with target_language_code, and validates the V2 debug/writer contract plus the
    public reading envelope. Services must already be running with a real provider.

.EXAMPLE
    .\scripts\test_horoscope_premium_next_7_days_v2_openai.ps1

.EXAMPLE
    .\scripts\test_horoscope_premium_next_7_days_v2_openai.ps1 -Languages fr,en
#>
param(
    [string]$BaseUrl = "http://127.0.0.1:8081",
    [string]$CalculatorUrl = "http://127.0.0.1:8080",
    [string[]]$Languages = @("fr", "en", "es", "de"),
    [int]$TimeoutSec = 600,
    [int]$WaitReadySec = 90,
    [string]$AnchorDate = "2026-06-07",
    [string]$Timezone = "Europe/Paris",
    [string]$UseExistingChartCalculationId = "",
    [string]$OutputDir = "",
    [int]$PollSeconds = 5,
    [switch]$SubmitCatalogue
)

$ErrorActionPreference = "Stop"

. "$PSScriptRoot\lib\astral_http_auth.ps1"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Import-AstralDotEnv -RepoRoot $repoRoot

$supportedLanguages = @("fr", "en", "es", "de")
foreach ($language in $Languages) {
    if ($supportedLanguages -notcontains $language) {
        throw "Unsupported target language '$language'. Supported: $($supportedLanguages -join ', ')"
    }
}

if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("OPENAI_API_KEY"))) {
    throw "OPENAI_API_KEY is required in .env for real Premium 7-day horoscope V2 OpenAI test."
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $stamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHHmmssZ")
    $OutputDir = Join-Path $repoRoot "output\horoscope_premium_period_v2_openai\$stamp"
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ($SubmitCatalogue) {
    & (Join-Path $repoRoot "scripts\manage_integration_services.ps1") -Submit
    if ($LASTEXITCODE -ne 0) {
        throw "Catalogue submission failed."
    }
}

$headers = New-AstralAuthHeaders -Service llm
$calcHeaders = New-AstralAuthHeaders -Service calculator

function ConvertFrom-JsonPreserveDates {
    param([string]$Json)
    if ((Get-Command ConvertFrom-Json).Parameters.ContainsKey("DateKind")) {
        return $Json | ConvertFrom-Json -DateKind String
    }
    return $Json | ConvertFrom-Json
}

function Test-HttpReady {
    param(
        [string]$Url,
        [hashtable]$Headers,
        [int]$TimeoutSec
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $lastError = $null
    while ((Get-Date) -lt $deadline) {
        try {
            Invoke-WebRequest -Uri $Url -Headers $Headers -UseBasicParsing -TimeoutSec 10 | Out-Null
            return
        } catch {
            $lastError = $_.Exception.Message
            Start-Sleep -Seconds 2
        }
    }
    throw "Service not ready at $Url after $TimeoutSec seconds. Last error: $lastError"
}

function Assert-RealProvider {
    param(
        [string]$BaseUrl,
        [hashtable]$Headers
    )
    $providers = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/providers" -Headers $Headers -TimeoutSec 15
    $defaultProvider = [string]$providers.default_provider
    $defaultModel = [string]$providers.default_model
    if ([string]::IsNullOrWhiteSpace($defaultProvider) -or $defaultProvider -eq "fake") {
        throw "Premium V2 OpenAI test requires a real provider. Current default_provider='$defaultProvider', default_model='$defaultModel'. Restart/configure LLM API from .env."
    }
    Write-Host "Provider: $defaultProvider / $defaultModel" -ForegroundColor DarkCyan
}

function New-ChartCalculationId {
    param(
        [string]$CalculatorUrl,
        [hashtable]$Headers
    )
    $natalRequestPath = Join-Path $repoRoot "contracts\integration\examples\natal_calculation_request_v1.paris_1990.json"
    if (-not (Test-Path -LiteralPath $natalRequestPath)) {
        throw "Missing natal fixture: $natalRequestPath"
    }
    $natalRequest = Get-Content -LiteralPath $natalRequestPath -Raw | ConvertFrom-Json
    $natalResponse = Invoke-AstralJson -Method Post -Uri "$CalculatorUrl/v1/internal/calculations/natal" -Headers $Headers -Body $natalRequest
    if ($natalResponse.calculation_result.status -ne "completed") {
        throw "Natal calculation did not complete."
    }
    $chartCalculationId = [string]$natalResponse.calculation_result.chart_calculation_id
    if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
        throw "Natal calculation did not return chart_calculation_id."
    }
    return $chartCalculationId
}

function Wait-HoroscopeJob {
    param(
        [string]$BaseUrl,
        [hashtable]$Headers,
        [string]$RunId,
        [int]$TimeoutSec,
        [int]$PollSeconds,
        [string]$StatusPath = ""
    )
    $startedAt = Get-Date
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $lastRaw = ""
    $attempt = 0
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds $PollSeconds
        $attempt += 1
        $statusResponse = Invoke-WebRequest -Method Get -Uri "$BaseUrl/v1/jobs/$RunId" -Headers $Headers -UseBasicParsing -TimeoutSec 30
        $lastRaw = [string]$statusResponse.Content
        if (-not [string]::IsNullOrWhiteSpace($StatusPath)) {
            $lastRaw | Set-Content -LiteralPath $StatusPath -Encoding UTF8
        }
        $status = ConvertFrom-JsonPreserveDates $lastRaw
        $elapsed = [int]((Get-Date) - $startedAt).TotalSeconds
        $provider = [string]$status.result.reading.quality.provider
        $model = [string]$status.result.reading.quality.model
        $suffix = ""
        if (-not [string]::IsNullOrWhiteSpace($provider) -or -not [string]::IsNullOrWhiteSpace($model)) {
            $suffix = " provider=$provider model=$model"
        }
        Write-Host "  poll#$attempt status=$($status.status) elapsed=${elapsed}s$suffix" -ForegroundColor DarkGray
        if ($status.status -eq "completed") {
            return @{ Parsed = $status; Raw = $lastRaw }
        }
        if ($status.status -eq "failed" -or $status.status -eq "safety_rejected") {
            throw "Job $RunId ended with $($status.status): $lastRaw"
        }
    }
    throw "Timeout waiting for job $RunId. Last response: $lastRaw"
}

function Get-HttpErrorBody {
    param($ErrorRecord)
    try {
        if ($ErrorRecord.ErrorDetails -and -not [string]::IsNullOrWhiteSpace([string]$ErrorRecord.ErrorDetails.Message)) {
            return [string]$ErrorRecord.ErrorDetails.Message
        }
    } catch {
    }
    try {
        $response = $ErrorRecord.Exception.Response
        if ($response -and $response.Content -and $response.Content.ReadAsStringAsync) {
            return $response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        }
    } catch {
    }
    try {
        $response = $ErrorRecord.Exception.Response
        if ($response -and $response.GetResponseStream) {
            $reader = [System.IO.StreamReader]::new($response.GetResponseStream())
            return $reader.ReadToEnd()
        }
    } catch {
    }
    return ($ErrorRecord | Out-String)
}

function Submit-HoroscopePremiumV2Job {
    param(
        [string]$BaseUrl,
        [hashtable]$Headers,
        [string]$Language,
        [string]$AnchorDate,
        [string]$Timezone,
        [string]$ChartCalculationId,
        [string]$OutputDir
    )

    $payloadV2 = @{
        anchor_date = $AnchorDate
        timezone = $Timezone
        target_language_code = $Language
        chart_calculation_id = $ChartCalculationId
        audience_level = "general"
    }
    $bodyObject = @{
        service_code = "horoscope_premium_next_7_days_natal"
        payload = $payloadV2
    }
    $body = $bodyObject | ConvertTo-Json -Depth 30
    $requestPath = Join-Path $OutputDir "request_$Language.json"
    $body | Set-Content -LiteralPath $requestPath -Encoding UTF8

    try {
        Write-Host "  POST /v1/jobs payload=target_language_code request=$requestPath" -ForegroundColor DarkGray
        $submit = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/jobs" -Headers $Headers -ContentType "application/json" -Body $body -TimeoutSec 30
        Write-Host "  submitted run_id=$($submit.run_id) payload=target_language_code" -ForegroundColor DarkGray
        return @{
            Submit = $submit
            RequestPath = $requestPath
            PayloadMode = "target_language_code"
            FallbackReason = $null
        }
    } catch {
        $errorBody = Get-HttpErrorBody $_
        $isLegacyPublicSchema = $errorBody -match "target_language_code" -and $errorBody -match "target_language"
        if (-not $isLegacyPublicSchema) {
            throw
        }

        Write-Warning "Running API rejected target_language_code in public payload. Retrying with legacy target_language='$Language'; writer_request.target_language_code will still be validated after completion."
        $payloadLegacy = @{
            anchor_date = $AnchorDate
            timezone = $Timezone
            target_language = $Language
            chart_calculation_id = $ChartCalculationId
            audience_level = "general"
        }
        $legacyBodyObject = @{
            service_code = "horoscope_premium_next_7_days_natal"
            payload = $payloadLegacy
        }
        $legacyBody = $legacyBodyObject | ConvertTo-Json -Depth 30
        $legacyRequestPath = Join-Path $OutputDir "request_${Language}_legacy_target_language.json"
        $legacyBody | Set-Content -LiteralPath $legacyRequestPath -Encoding UTF8
        Write-Host "  POST /v1/jobs payload=target_language request=$legacyRequestPath" -ForegroundColor DarkGray
        $submit = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/jobs" -Headers $Headers -ContentType "application/json" -Body $legacyBody -TimeoutSec 30
        Write-Host "  submitted run_id=$($submit.run_id) payload=target_language" -ForegroundColor DarkGray
        return @{
            Submit = $submit
            RequestPath = $legacyRequestPath
            PayloadMode = "target_language"
            FallbackReason = "running_api_rejected_target_language_code"
        }
    }
}

function ConvertTo-StableJson {
    param($Value)
    return ($Value | ConvertTo-Json -Depth 80 -Compress)
}

function Add-IfString {
    param(
        [System.Collections.Generic.List[string]]$Items,
        $Value
    )
    $text = [string]$Value
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        $Items.Add($text) | Out-Null
    }
}

function Get-PublicReadingTexts {
    param($Reading)
    $items = [System.Collections.Generic.List[string]]::new()
    Add-IfString $items $Reading.week_overview.title
    Add-IfString $items $Reading.week_overview.text
    Add-IfString $items $Reading.week_overview.trajectory
    Add-IfString $items $Reading.advice.main
    Add-IfString $items $Reading.advice.best_use
    Add-IfString $items $Reading.advice.avoid
    Add-IfString $items $Reading.watch_summary.text
    foreach ($day in @($Reading.daily_timeline)) {
        foreach ($field in @("day_label", "theme", "tone", "text", "advice")) {
            Add-IfString $items $day.$field
        }
    }
    foreach ($marker in @($Reading.key_days) + @($Reading.best_days) + @($Reading.watch_days)) {
        foreach ($field in @("title", "reason")) {
            if ($marker.PSObject.Properties.Name -contains $field) {
                Add-IfString $items $marker.$field
            }
        }
    }
    foreach ($section in @($Reading.domain_sections)) {
        foreach ($field in @("domain", "title", "text")) {
            Add-IfString $items $section.$field
        }
    }
    foreach ($window in @($Reading.best_windows)) {
        foreach ($field in @("title", "theme", "tone", "reason")) {
            Add-IfString $items $window.$field
        }
    }
    foreach ($window in @($Reading.watch_windows)) {
        foreach ($field in @("title", "theme", "tone", "watch_point")) {
            Add-IfString $items $window.$field
        }
    }
    foreach ($field in @("title", "text", "best_use", "recovery")) {
        Add-IfString $items $Reading.strategy.$field
    }
    foreach ($evidence in @($Reading.evidence_summary)) {
        Add-IfString $items $evidence.label
    }
    return $items
}

function Assert-NoPublicTechnicalLeak {
    param(
        [string[]]$Texts,
        [string]$Language
    )
    $joined = ($Texts -join "`n")
    foreach ($pattern in @(
            "Vérifiez vérifier",
            "consiste à de",
            "donne une direction claire",
            "Situations associées",
            "public_role",
            "reader_situation",
            "narrative_function",
            "theme_code",
            "tone_code",
            "evidence_key",
            "source_snapshot",
            "semantic_brief",
            "scan_plan"
        )) {
        if ($joined -match [regex]::Escape($pattern)) {
            throw "Public reading text for language '$Language' leaks forbidden pattern '$pattern'."
        }
    }
}

function Assert-PremiumV2Result {
    param(
        $Status,
        [string]$Raw,
        [string]$Language
    )
    $result = $Status.result
    if (-not $result.reading -or -not $result.calculation -or -not $result.interpretation_request -or -not $result.writer_request) {
        $resultKeys = @()
        if ($result -and $result.PSObject.Properties) {
            $resultKeys = @($result.PSObject.Properties.Name)
        }
        $interpretationContract = [string]$result.interpretation_request.contract_version
        throw "Result for '$Language' must include calculation, interpretation_request, writer_request and reading. Got keys=[$($resultKeys -join ', ')], interpretation_contract='$interpretationContract'. If writer_request is missing or interpretation_contract is not horoscope_period_writer_request, restart astral_llm_api/worker with the latest code and re-import json_db so Premium 7 days uses semantic_brief_v2."
    }
    if ((ConvertTo-StableJson $result.interpretation_request) -ne (ConvertTo-StableJson $result.writer_request)) {
        throw "V2 debug alias mismatch for '$Language': interpretation_request != writer_request."
    }

    $writer = $result.writer_request
    if ($writer.contract_version -ne "horoscope_period_writer_request") {
        throw "Unexpected writer contract for '$Language': $($writer.contract_version)"
    }
    if ($writer.service_code -ne "horoscope_premium_next_7_days_natal") {
        throw "Unexpected writer service_code for '$Language': $($writer.service_code)"
    }
    if ($writer.generation_mode -ne "semantic_brief_v2") {
        throw "Unexpected writer generation_mode for '$Language': $($writer.generation_mode)"
    }
    if ($writer.target_language_code -ne $Language) {
        throw "Unexpected target_language_code for '$Language': $($writer.target_language_code)"
    }
    if ($writer.output_contract_version -ne "horoscope_period_response") {
        throw "Unexpected output contract for '$Language': $($writer.output_contract_version)"
    }
    if ($null -eq $writer.semantic_brief -or $writer.semantic_brief.PSObject.Properties.Name -contains "evidence") {
        throw "semantic_brief for '$Language' must exist and must not contain embedded evidence."
    }
    if (-not $writer.evidence -or @($writer.evidence).Count -lt 1) {
        throw "Top-level writer evidence is required for '$Language'."
    }

    $reading = $result.reading
    if ($reading.contract_version -ne "horoscope_period_response") {
        throw "Unexpected reading contract for '$Language': $($reading.contract_version)"
    }
    if ($reading.service_code -ne "horoscope_premium_next_7_days_natal") {
        throw "Unexpected reading service_code for '$Language': $($reading.service_code)"
    }
    if ([string]$reading.quality.provider -eq "fake" -or [string]::IsNullOrWhiteSpace([string]$reading.quality.provider)) {
        throw "Real OpenAI run for '$Language' used invalid provider: '$($reading.quality.provider)'."
    }
    if ([bool]$reading.quality.fallback_used) {
        throw "Real OpenAI run for '$Language' unexpectedly used fallback."
    }
    if (@($reading.daily_timeline).Count -ne 7) {
        throw "daily_timeline for '$Language' must contain 7 entries."
    }
    if (@($reading.best_windows).Count -lt 1) {
        throw "best_windows for '$Language' must be non-empty."
    }
    if (@($reading.domain_sections).Count -lt 3 -or @($reading.domain_sections).Count -gt 5) {
        throw "domain_sections for '$Language' must contain 3 to 5 entries."
    }

    $includedDates = @($writer.period_resolution.included_dates | ForEach-Object { [string]$_ })
    foreach ($day in @($reading.daily_timeline)) {
        if ($includedDates -notcontains [string]$day.date) {
            throw "daily_timeline date outside period for '$Language': $($day.date)"
        }
        if (-not $day.evidence_keys -or @($day.evidence_keys).Count -lt 1) {
            throw "daily_timeline entry missing evidence_keys for '$Language': $($day.date)"
        }
    }

    Assert-NoPublicTechnicalLeak -Texts (Get-PublicReadingTexts -Reading $reading) -Language $Language

    if ($Raw -notmatch '"writer_request"\s*:') {
        throw "Raw result for '$Language' must expose debug writer_request."
    }
}

Write-Host "== Horoscope Premium Next 7 Days V2 OpenAI ==" -ForegroundColor Cyan
Write-Host "LLM API       : $BaseUrl"
Write-Host "Calculator API: $CalculatorUrl"
Write-Host "Languages     : $($Languages -join ', ')"
Write-Host "Output dir    : $OutputDir"
Write-Host "Poll interval : ${PollSeconds}s"

Test-HttpReady -Url "$CalculatorUrl/health/ready" -Headers $calcHeaders -TimeoutSec $WaitReadySec
Test-HttpReady -Url "$BaseUrl/health/ready" -Headers $headers -TimeoutSec $WaitReadySec
Assert-RealProvider -BaseUrl $BaseUrl -Headers $headers

$services = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v1/services" -Headers $headers -TimeoutSec 20
$service = @($services.services | Where-Object { $_.service_code -eq "horoscope_premium_next_7_days_natal" })[0]
if (-not $service) {
    throw "Service horoscope_premium_next_7_days_natal not listed."
}

$chartCalculationId = $UseExistingChartCalculationId
if ([string]::IsNullOrWhiteSpace($chartCalculationId)) {
    $chartCalculationId = New-ChartCalculationId -CalculatorUrl $CalculatorUrl -Headers $calcHeaders
}
Write-Host "Chart calculation: $chartCalculationId"

$summary = @()
foreach ($language in $Languages) {
    Write-Host "`n[$language] submitting Premium V2 real job..." -ForegroundColor Cyan
    $runKey = "horoscope-premium-v2-openai-$language-$([guid]::NewGuid().ToString('N'))"
    $runHeaders = @{}
    foreach ($key in $headers.Keys) {
        $runHeaders[$key] = $headers[$key]
    }
    $runHeaders["Idempotency-Key"] = $runKey

    $submitted = Submit-HoroscopePremiumV2Job `
        -BaseUrl $BaseUrl `
        -Headers $runHeaders `
        -Language $language `
        -AnchorDate $AnchorDate `
        -Timezone $Timezone `
        -ChartCalculationId $chartCalculationId `
        -OutputDir $OutputDir
    $submit = $submitted.Submit
    if (-not $submit.run_id) {
        throw "Missing run_id for '$language'."
    }
    $outputPath = Join-Path $OutputDir "response_$language.json"
    $statusPath = Join-Path $OutputDir "last_status_$language.json"
    $job = Wait-HoroscopeJob `
        -BaseUrl $BaseUrl `
        -Headers $runHeaders `
        -RunId $submit.run_id `
        -TimeoutSec $TimeoutSec `
        -PollSeconds $PollSeconds `
        -StatusPath $statusPath
    $job.Raw | Set-Content -LiteralPath $outputPath -Encoding UTF8
    Assert-PremiumV2Result -Status $job.Parsed -Raw $job.Raw -Language $language

    $reading = $job.Parsed.result.reading
    $wordCount = @((Get-PublicReadingTexts -Reading $reading) -join " " -split "\s+" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count
    $summary += [PSCustomObject]@{
        language = $language
        run_id = [string]$submit.run_id
        provider = [string]$reading.quality.provider
        model = [string]$reading.quality.model
        public_word_count = $wordCount
        payload_mode = [string]$submitted.PayloadMode
        fallback_reason = [string]$submitted.FallbackReason
        response_path = $outputPath
        request_path = [string]$submitted.RequestPath
    }
    Write-Host "[$language] OK run_id=$($submit.run_id) words=$wordCount provider=$($reading.quality.provider) model=$($reading.quality.model) payload=$($submitted.PayloadMode)" -ForegroundColor Green
}

$summaryPath = Join-Path $OutputDir "summary.json"
$summary | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $summaryPath -Encoding UTF8

Write-Host "`nPASS - Premium 7 days V2 OpenAI real certification completed." -ForegroundColor Green
Write-Host "Summary: $summaryPath"
