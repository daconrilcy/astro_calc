param(
    [string]$RequestPath = "",
    [string]$OutputPath = "",
    [string]$IdempotencyKey = "",
    [string]$BaseUrl = "",
    [string]$ApiKey = "",
    [string]$Model = "",
    [string]$SummaryModel = "",
    [string]$Provider = "",
    [int]$TimeoutSec = 900,
    [int]$EngineTimeoutMs = 120000,
    [int]$WaitApiSec = 120,
    [switch]$UseFake,
    [switch]$SkipGenerate,
    [switch]$SubmitProfile,
    [int]$MinWordsPerChapter = 0,
    [int]$MinAstroBasisPerChapter = 0
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$profileJsonPath = Join-Path $repoRoot "config\natal_interpretation_profiles\natal_premium.json"

function Import-DotEnv {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }

    Get-Content -LiteralPath $Path | ForEach-Object {
        $line = $_.Trim()
        if ($line -eq "" -or $line.StartsWith("#")) {
            return
        }
        $eq = $line.IndexOf("=")
        if ($eq -lt 1) {
            return
        }
        $name = $line.Substring(0, $eq).Trim()
        $value = $line.Substring($eq + 1).Trim()
        if ($value.StartsWith('"') -and $value.EndsWith('"')) {
            $value = $value.Substring(1, $value.Length - 2)
        }
        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name, "Process"))) {
            [Environment]::SetEnvironmentVariable($name, $value, "Process")
        }
    }
}

function Get-DotEnvVariable {
    param(
        [string]$Path,
        [string]$Name
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }

    foreach ($raw in Get-Content -LiteralPath $Path) {
        $line = $raw.Trim()
        if ($line -eq "" -or $line.StartsWith("#")) {
            continue
        }
        $eq = $line.IndexOf("=")
        if ($eq -lt 1) {
            continue
        }
        $key = $line.Substring(0, $eq).Trim()
        if ($key -ne $Name) {
            continue
        }
        $value = $line.Substring($eq + 1).Trim()
        if ($value.StartsWith('"') -and $value.EndsWith('"')) {
            $value = $value.Substring(1, $value.Length - 2)
        }
        return $value
    }

    return $null
}

function Get-WordCount {
    param([string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) {
        return 0
    }
    return [regex]::Matches($Text, '\S+').Count
}

function Get-ReadingContent {
    param($Response)

    if ($null -eq $Response) {
        return $null
    }
    if ($Response.reading) {
        return $Response.reading
    }
    return $Response
}

function Test-ApiHealth {
    param(
        [string]$Url,
        [string]$ApiKey = ""
    )

    $healthUri = "{0}/health" -f $Url.TrimEnd("/")
    $headers = @{}
    if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
        $headers["Authorization"] = "Bearer $ApiKey"
    }

    try {
        $params = @{
            Uri             = $healthUri
            UseBasicParsing = $true
            TimeoutSec      = 10
        }
        if ($headers.Count -gt 0) {
            $params["Headers"] = $headers
        }
        $resp = Invoke-WebRequest @params
        return ($resp.StatusCode -ge 200 -and $resp.StatusCode -lt 300)
    } catch {
        return $false
    }
}

function Wait-ApiHealth {
    param(
        [string]$Url,
        [string]$ApiKey = "",
        [int]$TimeoutSec = 120,
        [int]$IntervalSec = 2
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $lastError = ""

    while ((Get-Date) -lt $deadline) {
        if (Test-ApiHealth -Url $Url -ApiKey $ApiKey) {
            return $true
        }
        $lastError = "health check failed"
        Write-Host "  API pas encore prete ($lastError) — nouvel essai dans ${IntervalSec}s..."
        Start-Sleep -Seconds $IntervalSec
    }

    throw "API indisponible sur $Url/health apres ${TimeoutSec}s."
}

function Assert-PremiumReading {
    param(
        $Reading,
        [string[]]$ExpectedChapterOrder,
        [int]$MinWords,
        [int]$MinBasis
    )

    $failures = [System.Collections.Generic.List[string]]::new()

    if ($null -eq $Reading) {
        $failures.Add("Reponse vide ou illisible.")
        return ,$failures
    }

    if ($Reading.status -and $Reading.status -ne "success") {
        $code = if ($Reading.error.code) { $Reading.error.code } else { "?" }
        $msg = if ($Reading.error.message) { $Reading.error.message } else { "" }
        $failures.Add("status=$($Reading.status) error=$code $msg")
        return ,$failures
    }

    $content = Get-ReadingContent -Response $Reading
    if ($null -eq $content) {
        $failures.Add("Reponse sans contenu reading.")
        return ,$failures
    }

    if (-not $content.chapters -or $content.chapters.Count -eq 0) {
        $failures.Add("Aucun chapitre dans la reponse.")
        return ,$failures
    }

    $minRequired = $ExpectedChapterOrder.Count
    if ($content.chapters.Count -lt $minRequired) {
        $failures.Add(
            "Nombre de chapitres insuffisant : min $minRequired, recu $($content.chapters.Count)."
        )
    }

    $synthesis = $content.chapters | Where-Object { $_.code -eq "synthesis" } | Select-Object -First 1
    if ($synthesis) {
        $failures.Add("Chapitre 'synthesis' inattendu pour natal_premium.")
    }

    for ($i = 0; $i -lt $minRequired; $i++) {
        if ($i -ge $content.chapters.Count) {
            break
        }
        $expected = $ExpectedChapterOrder[$i]
        $ch = $content.chapters[$i]
        if ($ch.code -ne $expected) {
            $failures.Add("Ordre chapitre index $i : attendu '$expected', recu '$($ch.code)'.")
        }

        $words = Get-WordCount -Text $ch.body
        if ($words -lt $MinWords) {
            $failures.Add("Chapitre '$($ch.code)' trop court : $words mots (min $MinWords).")
        }

        $basisCount = 0
        if ($ch.astro_basis) {
            $basisCount = @($ch.astro_basis | Where-Object {
                    $_.factor -and -not [string]::IsNullOrWhiteSpace($_.factor)
                }).Count
        }
        if ($basisCount -lt $MinBasis) {
            $failures.Add(
                "Chapitre '$($ch.code)' astro_basis insuffisant : $basisCount (min $MinBasis)."
            )
        }
    }

    if ($content.quality.generation_mode -and $content.quality.generation_mode -ne "chapter_orchestrated") {
        $failures.Add(
            "generation_mode attendu chapter_orchestrated, recu $($content.quality.generation_mode)."
        )
    }

    if (-not $content.summary -or [string]::IsNullOrWhiteSpace($content.summary.short_text)) {
        $failures.Add("summary.short_text manquant ou vide.")
    }

    return ,$failures
}

function Get-RunAuditFailures {
    param(
        [string]$RunId,
        [string]$BaseUrl,
        [string]$ApiKey = ""
    )

    $result = [System.Collections.Generic.List[string]]::new()
    if ([string]::IsNullOrWhiteSpace($RunId)) {
        Write-Warning "run_id absent — verification repair_too_short ignoree."
        return ,$result
    }

    $uri = "{0}/v1/runs/{1}" -f $BaseUrl.TrimEnd("/"), $RunId
    $headers = @{}
    if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
        $headers["Authorization"] = "Bearer $ApiKey"
    }

    try {
        $params = @{
            Uri             = $uri
            UseBasicParsing = $true
            TimeoutSec      = 30
        }
        if ($headers.Count -gt 0) {
            $params["Headers"] = $headers
        }
        $audit = Invoke-RestMethod @params
    } catch {
        $result.Add("Audit run introuvable pour run_id=$RunId : $($_.Exception.Message)")
        return ,$result
    }

    if (-not $audit.steps) {
        return ,$result
    }

    $repairTooShort = @($audit.steps | Where-Object { $_.step_type -eq "repair_too_short" })
    if ($repairTooShort.Count -gt 0) {
        $result.Add("Audit : $($repairTooShort.Count) step(s) repair_too_short detecte(s).")
    }

    return ,$result
}

Import-DotEnv (Join-Path $repoRoot ".env")

if (-not (Test-Path -LiteralPath $profileJsonPath)) {
    throw "Profil introuvable : $profileJsonPath"
}

$profileDoc = Get-Content -LiteralPath $profileJsonPath -Raw | ConvertFrom-Json
$expectedChapters = @($profileDoc.chapter_types)
if ($MinWordsPerChapter -le 0) {
    $MinWordsPerChapter = [int]$profileDoc.quality.min_words_per_chapter
}
if ($MinAstroBasisPerChapter -le 0) {
    $minQuality = [int]$profileDoc.quality.min_astro_basis_refs_per_chapter
    $minEvidence = [int]$profileDoc.evidence.policy.min_evidence_per_chapter
    $MinAstroBasisPerChapter = [Math]::Max($minQuality, $minEvidence)
}

if ([string]::IsNullOrWhiteSpace($RequestPath)) {
    $RequestPath = Join-Path $repoRoot "request-premium-rich.json"
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot "output\premium_reading_e2e.json"
}

if (-not (Test-Path -LiteralPath $RequestPath)) {
    if (-not $SkipGenerate) {
        throw "Fixture introuvable : $RequestPath"
    }
} else {
    $requestDoc = Get-Content -LiteralPath $RequestPath -Raw | ConvertFrom-Json
    $domainCount = 0
    if ($requestDoc.engine -and $requestDoc.engine.domain_count) {
        $domainCount = [int]$requestDoc.engine.domain_count
    }
    if ($domainCount -le 0 -and $requestDoc.astrologer_profile.preferred_domains) {
        $domainCount = @($requestDoc.astrologer_profile.preferred_domains).Count
    }
    if ($domainCount -gt 0 -and $requestDoc.astrologer_profile.preferred_domains) {
        $preferred = @($requestDoc.astrologer_profile.preferred_domains)
        if ($preferred.Count -ge $domainCount) {
            $expectedChapters = $preferred[0..($domainCount - 1)]
        }
    }
}

if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    $llmHost = if ($env:ASTRAL_LLM_HOST) { $env:ASTRAL_LLM_HOST } else { "127.0.0.1" }
    $llmPort = if ($env:ASTRAL_LLM_PORT) { $env:ASTRAL_LLM_PORT } else { "8081" }
    $BaseUrl = "http://${llmHost}:${llmPort}"
}
if ([string]::IsNullOrWhiteSpace($ApiKey)) {
    $ApiKey = $env:ASTRAL_LLM_API_KEY
}

Write-Host "=== Test profil natal_premium ==="
Write-Host "Profil   : $($profileDoc.profile_code)"
Write-Host "Chapitres: $($expectedChapters -join ', ')"
Write-Host "Seuils   : min $MinWordsPerChapter mots, $MinAstroBasisPerChapter basis par chapitre"
Write-Host "API      : $BaseUrl"
Write-Host "Timeouts : client ${TimeoutSec}s, engine.timeout_ms ${EngineTimeoutMs} ms par appel LLM"
Write-Host ""

if ($SubmitProfile) {
    Write-Host "Soumission profil en base..."
    & (Join-Path $PSScriptRoot "manage_natal_interpretation_profiles.ps1") `
        -Submit -Path $profileJsonPath
    Write-Host "Redemarrer astral_llm_api pour recharger le catalogue."
    Write-Host ""
}

if (-not $SkipGenerate) {
    Write-Host "Attente API ($BaseUrl/health)..."
    Wait-ApiHealth -Url $BaseUrl -ApiKey $ApiKey -TimeoutSec $WaitApiSec | Out-Null
    Write-Host "API OK."
    Write-Host ""

    $e2eArgs = @{
        RequestPath     = $RequestPath
        OutputPath      = $OutputPath
        BaseUrl         = $BaseUrl
        TimeoutSec      = $TimeoutSec
        EngineTimeoutMs = $EngineTimeoutMs
    }
    if (-not [string]::IsNullOrWhiteSpace($IdempotencyKey)) {
        $e2eArgs["IdempotencyKey"] = $IdempotencyKey
    }
    if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
        $e2eArgs["ApiKey"] = $ApiKey
    }
    if (-not [string]::IsNullOrWhiteSpace($Model)) {
        $e2eArgs["Model"] = $Model
    }
    if (-not [string]::IsNullOrWhiteSpace($SummaryModel)) {
        $e2eArgs["SummaryModel"] = $SummaryModel
    }

    if ($UseFake) {
        $e2eArgs["Provider"] = "fake"
        $e2eArgs["Model"] = if ($Model) { $Model } else { "fake-model" }
        Write-Warning "UseFake : la gate qualite (min $MinWordsPerChapter mots) echouera souvent avec FakeProvider."
    } elseif (-not [string]::IsNullOrWhiteSpace($Provider)) {
        $e2eArgs["Provider"] = $Provider
    } elseif (-not $env:OPENAI_API_KEY) {
        Write-Warning "OPENAI_API_KEY absent - utilisez -UseFake ou configurez .env"
    }

    Write-Host "Generation E2E..."
    & (Join-Path $PSScriptRoot "generate_premium_reading_e2e.ps1") @e2eArgs
    $exitGen = $LASTEXITCODE
    if ($exitGen -ne 0) {
        Write-Host ""
        Write-Host "Echec generation (code $exitGen). Validation structure ignoree."
        exit $exitGen
    }
    Write-Host ""
}

if (-not (Test-Path -LiteralPath $OutputPath)) {
    throw "Fichier resultat introuvable : $OutputPath (lancez sans -SkipGenerate)"
}

$apiResponse = Get-Content -LiteralPath $OutputPath -Raw | ConvertFrom-Json
$readingContent = Get-ReadingContent -Response $apiResponse
$failures = Assert-PremiumReading `
    -Reading $apiResponse `
    -ExpectedChapterOrder $expectedChapters `
    -MinWords $MinWordsPerChapter `
    -MinBasis $MinAstroBasisPerChapter

if ($apiResponse.run_id) {
    $auditFailures = Get-RunAuditFailures `
        -RunId $apiResponse.run_id `
        -BaseUrl $BaseUrl `
        -ApiKey $ApiKey
    foreach ($af in $auditFailures) {
        $failures.Add($af)
    }
}

Write-Host "=== Validation structure premium ==="
Write-Host "Fichier : $OutputPath"
if ($apiResponse.run_id) {
    Write-Host "Run ID  : $($apiResponse.run_id)"
}
$minRequired = $expectedChapters.Count
if ($readingContent.chapters.Count -gt $minRequired) {
    Write-Host "Info    : $($readingContent.chapters.Count) chapitres (min requis $minRequired) — surplus accepte."
}

$totalWords = 0
$totalBasis = 0
for ($i = 0; $i -lt $readingContent.chapters.Count; $i++) {
    $ch = $readingContent.chapters[$i]
    $w = Get-WordCount -Text $ch.body
    $b = if ($ch.astro_basis) { $ch.astro_basis.Count } else { 0 }
    $totalWords += $w
    $totalBasis += $b
    $required = ($i -lt $minRequired)
    if ($required) {
        $okW = if ($w -ge $MinWordsPerChapter) { "OK" } else { "!!" }
        $okB = if ($b -ge $MinAstroBasisPerChapter) { "OK" } else { "!!" }
    } else {
        $okW = "—"
        $okB = "—"
    }
    $tag = if ($required) { "" } else { " (extra)" }
    Write-Host ("  {0,-22}{5} {1,4} mots [{2}]  {3,2} basis [{4}]" -f $ch.code, $w, $okW, $b, $okB, $tag)
}
Write-Host ("  {0,-22} {1,4} mots (corps)  {2,2} basis (total)" -f "TOTAL", $totalWords, $totalBasis)

Write-Host ""
if ($failures.Count -eq 0) {
    Write-Host "PASS - profil natal_premium conforme aux criteres du JSON profil."
    exit 0
}

Write-Host "FAIL - $($failures.Count) ecart(s) :"
foreach ($f in $failures) {
    Write-Host "  - $f"
}
exit 4
