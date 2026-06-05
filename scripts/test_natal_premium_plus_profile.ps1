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
    [int]$EngineTimeoutMs = 300000,
    [int]$WaitApiSec = 120,
    [switch]$UseFake,
    [switch]$SkipGenerate,
    [switch]$SubmitProfile,
    [int]$MinWordsPerChapter = 0,
    [int]$MinAstroBasisPerChapter = 0
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$profileJsonPath = Join-Path $repoRoot "config\natal_interpretation_profiles\natal_premium_plus.json"

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
        $healthUri = "{0}/health" -f $Url.TrimEnd("/")
        try {
            $params = @{
                Uri             = $healthUri
                UseBasicParsing = $true
                TimeoutSec      = 5
            }
            if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
                $params["Headers"] = @{ Authorization = "Bearer $ApiKey" }
            }
            $resp = Invoke-WebRequest @params
            if ($resp.StatusCode -ge 200 -and $resp.StatusCode -lt 300) {
                return $true
            }
            $lastError = "HTTP $($resp.StatusCode)"
        } catch {
            $lastError = $_.Exception.Message
        }
        Write-Host "  API pas encore prete ($lastError) — nouvel essai dans ${IntervalSec}s..."
        Start-Sleep -Seconds $IntervalSec
    }

    throw "API indisponible sur $Url/health apres ${TimeoutSec}s (derniere erreur : $lastError). Verifiez cargo run -p astral_llm_api et ASTRAL_LLM_PORT."
}

function Assert-PremiumPlusReading {
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

    if ($content.chapters.Count -ne $ExpectedChapterOrder.Count) {
        $failures.Add(
            "Nombre de chapitres : attendu $($ExpectedChapterOrder.Count), recu $($content.chapters.Count)."
        )
    }

    for ($i = 0; $i -lt [Math]::Min($content.chapters.Count, $ExpectedChapterOrder.Count); $i++) {
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

    $synthesis = $content.chapters | Where-Object { $_.code -eq "synthesis" } | Select-Object -First 1
    if (-not $synthesis) {
        $failures.Add("Chapitre 'synthesis' absent.")
    }

    if (-not $content.summary -or [string]::IsNullOrWhiteSpace($content.summary.short_text)) {
        $failures.Add("summary.short_text manquant ou vide.")
    }

    if ($content.quality.generation_mode -and $content.quality.generation_mode -ne "chapter_orchestrated") {
        $failures.Add(
            "generation_mode attendu chapter_orchestrated, recu $($content.quality.generation_mode)."
        )
    }

    return ,$failures
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
    $RequestPath = Join-Path $repoRoot "request-premium-plus-rich.json"
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot "output\premium_plus_reading_e2e.json"
}
if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    $llmHost = if ($env:ASTRAL_LLM_HOST) { $env:ASTRAL_LLM_HOST } else { "127.0.0.1" }
    $llmPort = if ($env:ASTRAL_LLM_PORT) { $env:ASTRAL_LLM_PORT } else { "8081" }
    $BaseUrl = "http://${llmHost}:${llmPort}"
}
if ([string]::IsNullOrWhiteSpace($ApiKey)) {
    $ApiKey = $env:ASTRAL_LLM_API_KEY
}

Write-Host "=== Test profil natal_premium_plus ==="
Write-Host "Profil   : $($profileDoc.profile_code)"
Write-Host "Chapitres: $($expectedChapters -join ', ')"
Write-Host "Seuils   : min $MinWordsPerChapter mots/ch., min $MinAstroBasisPerChapter astro_basis/ch."
Write-Host "API      : $BaseUrl"
Write-Host "Timeouts : client ${TimeoutSec}s, engine.timeout_ms ${EngineTimeoutMs} ms par appel LLM"
$dotEnvPath = Join-Path $repoRoot ".env"
$serverTimeoutMs = 120000
$timeoutFromFile = Get-DotEnvVariable -Path $dotEnvPath -Name "ASTRAL_LLM_REQUEST_TIMEOUT_MS"
$timeoutSource = "defaut"
if (-not [string]::IsNullOrWhiteSpace($timeoutFromFile)) {
    $parsed = 0
    if ([int]::TryParse($timeoutFromFile, [ref]$parsed)) {
        $serverTimeoutMs = $parsed
        $timeoutSource = ".env"
    }
} elseif ($env:ASTRAL_LLM_REQUEST_TIMEOUT_MS) {
    $parsed = 0
    if ([int]::TryParse($env:ASTRAL_LLM_REQUEST_TIMEOUT_MS, [ref]$parsed)) {
        $serverTimeoutMs = $parsed
        $timeoutSource = "process"
    }
}
Write-Host ('Serveur  : ASTRAL_LLM_REQUEST_TIMEOUT_MS={0} ms ({1}, coupe HTTP a {2} ms)' -f $serverTimeoutMs, $timeoutSource, ($serverTimeoutMs + 5000))
if ($timeoutSource -eq "process" -and -not [string]::IsNullOrWhiteSpace($timeoutFromFile) -and $timeoutFromFile -ne $env:ASTRAL_LLM_REQUEST_TIMEOUT_MS) {
    Write-Warning ('Variable process ASTRAL_LLM_REQUEST_TIMEOUT_MS={0} differ de .env ({1}). Redemarrer le terminal ou l''API depuis .env.' -f $env:ASTRAL_LLM_REQUEST_TIMEOUT_MS, $timeoutFromFile)
}
if ($serverTimeoutMs -lt 600000) {
    Write-Warning 'ASTRAL_LLM_REQUEST_TIMEOUT_MS trop court pour premium_plus - 10 appels LLM longs.'
    Write-Warning 'Recommande : ASTRAL_LLM_REQUEST_TIMEOUT_MS=900000 dans .env puis redemarrer astral_llm_api.'
}
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
        RequestPath      = $RequestPath
        OutputPath       = $OutputPath
        BaseUrl          = $BaseUrl
        TimeoutSec       = $TimeoutSec
        EngineTimeoutMs  = $EngineTimeoutMs
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
        if (-not $env:ASTRAL_LLM_ENABLE_FAKE -or $env:ASTRAL_LLM_ENABLE_FAKE -ne "true") {
            Write-Warning "UseFake : definir ASTRAL_LLM_ENABLE_FAKE=true et redemarrer astral_llm_api."
        }
        Write-Warning "UseFake : la gate qualite (min $MinWordsPerChapter mots) echouera souvent avec FakeProvider."
    } elseif (-not [string]::IsNullOrWhiteSpace($Provider)) {
        $e2eArgs["Provider"] = $Provider
    } elseif (-not $env:OPENAI_API_KEY) {
        Write-Warning "OPENAI_API_KEY absent - utilisez -UseFake ou configurez .env"
    }

    Write-Host "Generation E2E..."
    & (Join-Path $PSScriptRoot "generate_premium_plus_reading_e2e.ps1") @e2eArgs
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
$failures = Assert-PremiumPlusReading `
    -Reading $apiResponse `
    -ExpectedChapterOrder $expectedChapters `
    -MinWords $MinWordsPerChapter `
    -MinBasis $MinAstroBasisPerChapter

Write-Host "=== Validation structure premium_plus ==="
Write-Host "Fichier : $OutputPath"
if ($apiResponse.run_id) {
    Write-Host "Run ID  : $($apiResponse.run_id)"
    Write-Host "Audit   : .\scripts\show_generation_run.ps1 -RunId $($apiResponse.run_id)"
}

$totalWords = 0
$totalBasis = 0
foreach ($ch in $readingContent.chapters) {
    $w = Get-WordCount -Text $ch.body
    $b = if ($ch.astro_basis) { $ch.astro_basis.Count } else { 0 }
    $totalWords += $w
    $totalBasis += $b
    $okW = if ($w -ge $MinWordsPerChapter) { "OK" } else { "!!" }
    $okB = if ($b -ge $MinAstroBasisPerChapter) { "OK" } else { "!!" }
    Write-Host ("  {0,-22} {1,4} mots [{2}]  {3,2} basis [{4}]" -f $ch.code, $w, $okW, $b, $okB)
}
Write-Host ("  {0,-22} {1,4} mots (corps)  {2,2} basis (total)" -f "TOTAL", $totalWords, $totalBasis)
if ($readingContent.summary.short_text) {
    $summaryWords = Get-WordCount -Text $readingContent.summary.short_text
    Write-Host ("  summary.short_text     {0,4} mots" -f $summaryWords)
}

Write-Host ""
if ($failures.Count -eq 0) {
    Write-Host "PASS - profil natal_premium_plus conforme aux criteres du JSON profil."
    exit 0
}

Write-Host "FAIL - $($failures.Count) ecart(s) :"
foreach ($f in $failures) {
    Write-Host "  - $f"
}
exit 4
