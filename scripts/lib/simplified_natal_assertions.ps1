# Assertions partagees pour les tests natal simplifie (calculateur + lecture).

function Get-SimplifiedWordCount {
    param([string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) { return 0 }
    return [regex]::Matches($Text, '\S+').Count
}

function Invoke-AstralHttpWithStatus {
    param(
        [string]$Method,
        [string]$Uri,
        [hashtable]$Headers,
        $Body = $null,
        [int]$TimeoutSec = 120
    )

    $jsonBody = $null
    if ($null -ne $Body) {
        $jsonBody = if ($Body -is [string]) { $Body } else { $Body | ConvertTo-Json -Depth 40 }
    }

    $iwrParams = @{
        Method          = $Method
        Uri             = $Uri
        Headers         = $Headers
        ContentType     = "application/json"
        TimeoutSec      = $TimeoutSec
        UseBasicParsing = $true
    }
    if ($null -ne $jsonBody) { $iwrParams.Body = $jsonBody }

    $supportsSkip = (Get-Command Invoke-WebRequest).Parameters.ContainsKey("SkipHttpErrorCheck")
    if ($supportsSkip) {
        $iwrParams.SkipHttpErrorCheck = $true
    }

    try {
        $response = Invoke-WebRequest @iwrParams
        $parsed = $null
        if ($response.Content) {
            $parsed = $response.Content | ConvertFrom-Json
        }
        return [ordered]@{
            Ok         = ([int]$response.StatusCode -ge 200 -and [int]$response.StatusCode -lt 300)
            StatusCode = [int]$response.StatusCode
            Body       = $parsed
            Raw        = $response.Content
        }
    } catch {
        $status = 0
        $raw = if ($_.ErrorDetails.Message) { $_.ErrorDetails.Message } else { $_.Exception.Message }
        if ($_.Exception.Response) {
            try { $status = [int]$_.Exception.Response.StatusCode } catch { }
        }
        $parsed = $null
        if (-not [string]::IsNullOrWhiteSpace($raw)) {
            try { $parsed = $raw | ConvertFrom-Json } catch { $parsed = $raw }
        }
        return [ordered]@{
            Ok         = $false
            StatusCode = $status
            Body       = $parsed
            Raw        = $raw
        }
    }
}

function Test-AstralServiceReady {
    param(
        [string]$BaseUrl,
        [hashtable]$Headers,
        [string]$ReadyPath = "/health/ready",
        [int]$TimeoutSec = 120,
        [int]$IntervalSec = 2
    )

    $uri = "{0}{1}" -f $BaseUrl.TrimEnd("/"), $ReadyPath
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $null = Invoke-RestMethod -Method Get -Uri $uri -Headers $Headers -TimeoutSec 10
            return $true
        } catch {
            Start-Sleep -Seconds $IntervalSec
        }
    }
    throw "Service non pret : $uri (${TimeoutSec}s)"
}

function Get-SimplifiedReadingContent {
    param($ApiResponse)

    if ($null -eq $ApiResponse) { return $null }
    if ($ApiResponse.reading.reading) { return $ApiResponse.reading.reading }
    if ($ApiResponse.reading.chapters) { return $ApiResponse.reading }
    return $null
}

function Assert-SimplifiedCalculatorResponse {
    param(
        $Response,
        $Case
    )

    $failures = [System.Collections.Generic.List[string]]::new()

    if ($null -eq $Response) {
        $failures.Add("Reponse calculateur vide.")
        return ,$failures
    }

    if ($Response.response_contract_version -ne "astro_simplified_natal_response_v1") {
        $failures.Add("response_contract_version=$($Response.response_contract_version)")
    }
    if ($Response.input_precision.level -ne $Case.ExpectedInputPrecision) {
        $failures.Add("input_precision=$($Response.input_precision.level) attendu=$($Case.ExpectedInputPrecision)")
    }
    if ($Response.computed_scope -ne $Case.ExpectedScope) {
        $failures.Add("computed_scope=$($Response.computed_scope) attendu=$($Case.ExpectedScope)")
    }
    if ($Response.reading_hint.recommended_profile_code -ne "natal_simplified") {
        $failures.Add("recommended_profile_code=$($Response.reading_hint.recommended_profile_code)")
    }
    if ($Response.reading_hint.reading_completeness -notin @("partial", "simplified")) {
        $failures.Add("reading_completeness=$($Response.reading_hint.reading_completeness)")
    }

    $limitationCodes = @($Response.limitations | ForEach-Object { $_.code })
    foreach ($code in $Case.ExpectedLimitations) {
        if ($limitationCodes -notcontains $code) {
            $failures.Add("limitation manquante: $code")
        }
    }

    $actualExcluded = @($Response.excluded_features)
    foreach ($feature in $Case.ExpectedExcluded) {
        if ($actualExcluded -notcontains $feature) {
            $failures.Add("excluded_features manque: $feature")
        }
    }
    foreach ($feature in $actualExcluded) {
        if ($Case.ExpectedExcluded -notcontains $feature) {
            $failures.Add("excluded_features inattendu: $feature")
        }
    }

    if ($Response.simplified_payload.payload_contract -ne "natal_simplified_structured_v1") {
        $failures.Add("payload_contract=$($Response.simplified_payload.payload_contract)")
    }

    if ($Response.llm_payload.profile_code -ne "natal_simplified") {
        $failures.Add("llm_payload.profile_code=$($Response.llm_payload.profile_code)")
    }

    foreach ($fact in @($Response.facts)) {
        $code = "$($fact.object_code).sign"
        if ($Response.llm_payload.allowed_fact_codes -notcontains $code) {
            $failures.Add("allowed_fact_codes manque stable: $code")
        }
        $planet = $Response.simplified_payload.payload.planets.($fact.object_code)
        if ($null -eq $planet -or -not $planet.sign) {
            $failures.Add("planets mirror manque signe stable: $($fact.object_code)")
        }
    }

    foreach ($ambig in @($Response.ambiguous_facts)) {
        $code = "$($ambig.object_code).sign"
        if ($Response.llm_payload.blocked_interpretation_fact_codes -notcontains $code) {
            $failures.Add("blocked_interpretation_fact_codes manque: $code")
        }
        $planet = $Response.simplified_payload.payload.planets.($ambig.object_code)
        if ($null -ne $planet -and $planet.sign) {
            $failures.Add("planets mirror ne doit pas affirmer ambigu: $($ambig.object_code)")
        }
    }

    if ($Case.ExpectCounts) {
        $payload = $Response.simplified_payload.payload
        if ($Case.MinPositionCount -gt 0 -and [int]$payload.position_count -lt $Case.MinPositionCount) {
            $failures.Add("position_count=$($payload.position_count)")
        }
        if ($Case.MinHouseCuspCount -gt 0 -and [int]$payload.house_cusp_count -lt $Case.MinHouseCuspCount) {
            $failures.Add("house_cusp_count=$($payload.house_cusp_count)")
        }
        if ($null -ne $Case.MinAspectCount -and [int]$payload.aspect_count -lt $Case.MinAspectCount) {
            $failures.Add("aspect_count=$($payload.aspect_count)")
        }
    }

    if ($Case.AssertMoonAmbiguity) {
        $moonAmbig = @($Response.ambiguous_facts) | Where-Object { $_.object_code -eq "moon" }
        if ($moonAmbig.Count -gt 0) {
            if ($Response.llm_payload.blocked_interpretation_fact_codes -notcontains "moon.sign") {
                $failures.Add("moon ambigu mais moon.sign non bloque")
            }
        }
    }

    return ,$failures
}

function Assert-SimplifiedReadingResponse {
    param(
        $ApiResponse,
        $Case,
        [int]$MinWordsPerChapter = 30
    )

    $failures = [System.Collections.Generic.List[string]]::new()

    if ($null -eq $ApiResponse) {
        $failures.Add("Reponse lecture vide.")
        return ,$failures
    }

    if ($ApiResponse.reading_completeness -notin @("partial", "simplified")) {
        $failures.Add("reading_completeness=$($ApiResponse.reading_completeness)")
    }
    if (-not $ApiResponse.run_id) {
        $failures.Add("run_id manquant")
    }

    if ($ApiResponse.reading.status -ne "success") {
        $code = if ($ApiResponse.reading.error.code) { $ApiResponse.reading.error.code } else { "?" }
        $msg = if ($ApiResponse.reading.error.message) { $ApiResponse.reading.error.message } else { "" }
        $failures.Add("reading.status=$($ApiResponse.reading.status) $code $msg")
        return ,$failures
    }

    $calcFailures = Assert-SimplifiedCalculatorResponse -Response $ApiResponse.calculation -Case $Case
    foreach ($f in $calcFailures) { $failures.Add("calculation: $f") }

    $content = Get-SimplifiedReadingContent -ApiResponse $ApiResponse
    if ($null -eq $content) {
        $failures.Add("contenu reading illisible")
        return ,$failures
    }

    if ($content.schema_version -ne "natal_reading_v1") {
        $failures.Add("schema_version=$($content.schema_version)")
    }
    if (@($content.chapters).Count -lt 1) {
        $failures.Add("chapters vide")
    }

    $allText = @()
    foreach ($ch in @($content.chapters)) {
        $words = Get-SimplifiedWordCount -Text $ch.body
        if ($words -lt $MinWordsPerChapter) {
            $failures.Add("$($ch.code) mots=$words min=$MinWordsPerChapter")
        }
        $allText += $ch.body
        $allText += $ch.title
    }
    if ($content.summary) {
        $allText += $content.summary.short_text
        $allText += $content.summary.title
    }

    $joined = ($allText -join " ").ToLowerInvariant()
    if ($joined -match "degraded|d[eé]grad[eé]e") {
        $failures.Add("wording interdit degraded detecte")
    }

    if ($Case.ExpectedExcluded -contains "ascendant") {
        if ($joined -match "ascendant (en|est|du |de la |:) (b[eé]lier|taureau|g[eé]meaux|cancer|lion|vierge|balance|scorpion|sagittaire|capricorne|verseau|poissons)") {
            $failures.Add("affirmation ascendant par signe detectee (scope sans angles)")
        }
    }

    if ($Case.AssertMoonAmbiguity) {
        $moonBlocked = $ApiResponse.calculation.llm_payload.blocked_interpretation_fact_codes -contains "moon.sign"
        if ($moonBlocked -and $joined -match "lune (en|est) (b[eé]lier|taureau|g[eé]meaux|cancer|lion|vierge|balance|scorpion|sagittaire|capricorne|verseau|poissons)") {
            $failures.Add("signe lunaire affirme alors que moon.sign est bloque")
        }
    }

    if ($content.legal -and -not $content.legal.disclaimer) {
        $failures.Add("disclaimer legal manquant")
    }

    return ,$failures
}

function Test-SimplifiedCatalogReady {
    param(
        [string]$CalculatorBase,
        [hashtable]$Headers
    )

    $probe = [ordered]@{
        request_contract_version = "astro_simplified_natal_request_v1"
        birth                    = [ordered]@{ date = "1990-06-15" }
    }
    $uri = "$($CalculatorBase.TrimEnd('/'))/v1/calculations/natal/simplified"
    $result = Invoke-AstralHttpWithStatus -Method Post -Uri $uri -Headers $Headers -Body $probe -TimeoutSec 30

    if ($result.Ok) { return $null }

    $message = if ($result.Body.error.message) { [string]$result.Body.error.message } else { $result.Raw }
    $code = if ($result.Body.error.code) { [string]$result.Body.error.code } else { "HTTP $($result.StatusCode)" }

    if ($code -eq "REFERENCE_DATA_MISSING" -or $message -match "does not exist|import_json_db") {
        return @"
Catalogue natal simplifie absent en base PostgreSQL.
  python scripts/import_json_db_to_postgres.py
  ou : .\scripts\docker_bootstrap.ps1
"@
    }
    if ($code -eq "SERVICE_NOT_READY" -or $result.StatusCode -eq 503) {
        return "Calculateur non pret (reference ou ephemerides). Lancez .\scripts\docker_bootstrap.ps1"
    }
    return $null
}

function Test-LlmFakeProviderReady {
    param(
        [string]$LlmBase,
        [hashtable]$Headers
    )

    try {
        $providers = Invoke-RestMethod -Method Get -Uri "$($LlmBase.TrimEnd('/'))/v1/providers" -Headers $Headers -TimeoutSec 10
        $default = [string]$providers.default_provider
        if ($default -eq "fake") { return $null }
        $hasFakeModel = @($providers.models) | Where-Object { $_.provider -eq "fake" }
        if ($providers.fake_enabled -and $hasFakeModel.Count -gt 0 -and [string]::IsNullOrWhiteSpace($default)) {
            return $null
        }
        return @"
Le gateway LLM n'utilise pas le provider fake (default=$default).
Pour les tests E2E sans cout OpenAI :
  docker compose up -d --build astral_llm_api
  (docker-compose force ASTRAL_LLM_DEFAULT_PROVIDER=fake)
Ou lancez avec -UseReal si vous acceptez les appels OpenAI.
"@
    } catch {
        return "Impossible de lire /v1/providers sur $LlmBase"
    }
}

function Write-SimplifiedTestBanner {
    param(
        [string]$Title,
        [string]$CalculatorBase,
        [string]$LlmBase = ""
    )
    Write-Host ""
    Write-Host "=== $Title ===" -ForegroundColor Cyan
    Write-Host "Calculateur : $CalculatorBase"
    if ($LlmBase) { Write-Host "LLM         : $LlmBase" }
}

function Write-SimplifiedCaseResult {
    param(
        [string]$Label,
        [bool]$Passed,
        [string[]]$Failures = @()
    )
    if ($Passed) {
        Write-Host "  [OK] $Label" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] $Label" -ForegroundColor Red
        foreach ($f in $Failures) {
            Write-Host "         - $f" -ForegroundColor Yellow
        }
    }
}
