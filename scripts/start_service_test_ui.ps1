param(
    [int]$Port = 8099,
    [string]$LlmBaseUrl = "http://localhost:8081",
    [string]$CalculatorBaseUrl = "http://localhost:8080"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$uiRoot = Join-Path $repoRoot "tests\service_test_ui"
if (-not (Test-Path $uiRoot)) {
    throw "UI directory not found: $uiRoot"
}
$uiRootFull = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $uiRoot).Path)
$uiRootPrefix = $uiRootFull.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar

$listener = [System.Net.HttpListener]::new()
$prefix = "http://localhost:$Port/"
$listener.Prefixes.Add($prefix)
$listener.Start()

$httpClient = [System.Net.Http.HttpClient]::new()
$httpClient.Timeout = [TimeSpan]::FromMinutes(20)
$geocodeCache = @{}
$lastGeocodeAt = [DateTimeOffset]::MinValue

function Read-DotEnv {
    param([string]$Path)
    $values = @{}
    if (-not (Test-Path -LiteralPath $Path)) {
        return $values
    }
    foreach ($line in Get-Content -LiteralPath $Path) {
        $trimmed = $line.Trim()
        if (-not $trimmed -or $trimmed.StartsWith("#")) {
            continue
        }
        $idx = $trimmed.IndexOf("=")
        if ($idx -lt 1) {
            continue
        }
        $name = $trimmed.Substring(0, $idx).Trim()
        $value = $trimmed.Substring($idx + 1).Trim()
        if (($value.StartsWith('"') -and $value.EndsWith('"')) -or ($value.StartsWith("'") -and $value.EndsWith("'"))) {
            $value = $value.Substring(1, $value.Length - 2)
        }
        if ($name -and $value) {
            $values[$name] = $value
        }
    }
    return $values
}

$envFileValues = Read-DotEnv -Path (Join-Path $repoRoot ".env")
$defaultLlmApiKey = $envFileValues["ASTRAL_LLM_API_KEY"]
$defaultCalculatorApiKey = $envFileValues["ASTRAL_CALCULATOR_API_KEY"]

Write-Host "Astral service test UI: $prefix"
Write-Host "LLM proxy        : $LlmBaseUrl"
Write-Host "Calculator proxy : $CalculatorBaseUrl"
Write-Host "LLM API key      : $(if ($defaultLlmApiKey) { 'loaded from .env' } else { 'not configured' })"
Write-Host "Calculator key   : $(if ($defaultCalculatorApiKey) { 'loaded from .env' } else { 'not configured' })"
Write-Host "Stop with Ctrl+C"

function Send-Bytes {
    param(
        [System.Net.HttpListenerContext]$Context,
        [int]$StatusCode,
        [byte[]]$Bytes,
        [string]$ContentType
    )
    $Context.Response.StatusCode = $StatusCode
    $Context.Response.ContentType = $ContentType
    $Context.Response.ContentLength64 = $Bytes.Length
    $Context.Response.OutputStream.Write($Bytes, 0, $Bytes.Length)
    $Context.Response.OutputStream.Close()
}

function Send-Json {
    param(
        [System.Net.HttpListenerContext]$Context,
        [int]$StatusCode,
        [object]$Body
    )
    $json = $Body | ConvertTo-Json -Depth 100
    Send-Bytes -Context $Context -StatusCode $StatusCode -Bytes ([Text.Encoding]::UTF8.GetBytes($json)) -ContentType "application/json; charset=utf-8"
}

function Send-Text {
    param(
        [System.Net.HttpListenerContext]$Context,
        [int]$StatusCode,
        [string]$Body
    )
    Send-Bytes -Context $Context -StatusCode $StatusCode -Bytes ([Text.Encoding]::UTF8.GetBytes($Body)) -ContentType "text/plain; charset=utf-8"
}

function Get-ContentType {
    param([string]$Path)
    switch ([IO.Path]::GetExtension($Path).ToLowerInvariant()) {
        ".html" { "text/html; charset=utf-8" }
        ".css" { "text/css; charset=utf-8" }
        ".js" { "application/javascript; charset=utf-8" }
        ".json" { "application/json; charset=utf-8" }
        default { "application/octet-stream" }
    }
}

function Read-RequestBytes {
    param([System.Net.HttpListenerRequest]$Request)
    if (-not $Request.HasEntityBody) {
        return [byte[]]::new(0)
    }
    $memory = [IO.MemoryStream]::new()
    $Request.InputStream.CopyTo($memory)
    return $memory.ToArray()
}

function Invoke-Proxy {
    param(
        [System.Net.HttpListenerContext]$Context,
        [string]$BaseUrl,
        [string]$PrefixToRemove,
        [string]$DefaultApiKey
    )

    $request = $Context.Request
    $relative = $request.RawUrl.Substring($PrefixToRemove.Length)
    if (-not $relative.StartsWith("/")) {
        $relative = "/$relative"
    }
    $target = "$BaseUrl$relative"
    $message = [System.Net.Http.HttpRequestMessage]::new([System.Net.Http.HttpMethod]::new($request.HttpMethod), $target)

    foreach ($name in @("Authorization", "X-API-Key", "Idempotency-Key", "X-Tenant-Id")) {
        $value = $request.Headers[$name]
        if ($value) {
            [void]$message.Headers.TryAddWithoutValidation($name, $value)
        }
    }
    if ($DefaultApiKey -and -not $request.Headers["Authorization"] -and -not $request.Headers["X-API-Key"]) {
        [void]$message.Headers.TryAddWithoutValidation("X-API-Key", $DefaultApiKey)
    }

    $bodyBytes = Read-RequestBytes -Request $request
    if ($bodyBytes.Length -gt 0) {
        $message.Content = [System.Net.Http.ByteArrayContent]::new($bodyBytes)
        if ($request.ContentType) {
            $message.Content.Headers.TryAddWithoutValidation("Content-Type", $request.ContentType) | Out-Null
        }
    }

    try {
        $response = $httpClient.SendAsync($message).GetAwaiter().GetResult()
        $bytes = $response.Content.ReadAsByteArrayAsync().GetAwaiter().GetResult()
        $contentType = "application/json; charset=utf-8"
        if ($response.Content.Headers.ContentType) {
            $contentType = $response.Content.Headers.ContentType.ToString()
        }
        Send-Bytes -Context $Context -StatusCode ([int]$response.StatusCode) -Bytes $bytes -ContentType $contentType
    } catch {
        Send-Json -Context $Context -StatusCode 502 -Body @{
            error = @{
                code = "PROXY_ERROR"
                message = $_.Exception.Message
            }
        }
    }
}

function Invoke-Geocode {
    param([System.Net.HttpListenerContext]$Context)

    if ($Context.Request.HttpMethod -ne "GET") {
        Send-Json -Context $Context -StatusCode 405 -Body @{ error = @{ code = "METHOD_NOT_ALLOWED"; message = "GET required" } }
        return
    }

    $query = [System.Web.HttpUtility]::ParseQueryString($Context.Request.Url.Query)
    $city = [string]$query["city"]
    $country = [string]$query["country"]
    if ($null -eq $city) { $city = "" }
    if ($null -eq $country) { $country = "" }
    $city = $city.Trim()
    $country = $country.Trim()
    if (-not $city -or -not $country) {
        Send-Json -Context $Context -StatusCode 400 -Body @{ error = @{ code = "INVALID_INPUT"; message = "city and country are required" } }
        return
    }

    $cacheKey = "$city|$country".ToLowerInvariant()
    if ($geocodeCache.ContainsKey($cacheKey)) {
        Send-Json -Context $Context -StatusCode 200 -Body $geocodeCache[$cacheKey]
        return
    }

    $now = [DateTimeOffset]::UtcNow
    $elapsedMs = ($now - $script:lastGeocodeAt).TotalMilliseconds
    if ($elapsedMs -lt 1000) {
        Start-Sleep -Milliseconds ([int](1000 - $elapsedMs))
    }

    $builder = [System.UriBuilder]::new("https://nominatim.openstreetmap.org/search")
    $nvc = [System.Web.HttpUtility]::ParseQueryString("")
    $nvc["format"] = "jsonv2"
    $nvc["limit"] = "1"
    $nvc["addressdetails"] = "1"
    $nvc["city"] = $city
    $nvc["country"] = $country
    $builder.Query = $nvc.ToString()

    $message = [System.Net.Http.HttpRequestMessage]::new([System.Net.Http.HttpMethod]::Get, $builder.Uri)
    $message.Headers.UserAgent.ParseAdd("astral-calculation-service-test-ui/1.0 (local-dev)")
    $message.Headers.Referrer = [Uri]"http://localhost:$Port/"

    try {
        $script:lastGeocodeAt = [DateTimeOffset]::UtcNow
        $response = $httpClient.SendAsync($message).GetAwaiter().GetResult()
        $raw = $response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        if (-not $response.IsSuccessStatusCode) {
            Send-Json -Context $Context -StatusCode ([int]$response.StatusCode) -Body @{ error = @{ code = "GEOCODE_FAILED"; message = $raw } }
            return
        }
        $items = $raw | ConvertFrom-Json
        if (-not $items -or $items.Count -lt 1) {
            Send-Json -Context $Context -StatusCode 404 -Body @{ error = @{ code = "GEOCODE_NOT_FOUND"; message = "No location found" } }
            return
        }
        $item = @($items)[0]
        $body = @{
            latitude = [double]$item.lat
            longitude = [double]$item.lon
            label = $item.display_name
            country_code = $item.address.country_code
        }
        $geocodeCache[$cacheKey] = $body
        Send-Json -Context $Context -StatusCode 200 -Body $body
    } catch {
        Send-Json -Context $Context -StatusCode 502 -Body @{ error = @{ code = "GEOCODE_PROXY_ERROR"; message = $_.Exception.Message } }
    }
}

try {
    $contextTask = $listener.GetContextAsync()
    while ($listener.IsListening) {
        try {
            if (-not $contextTask.Wait(250)) {
                continue
            }
            $context = $contextTask.GetAwaiter().GetResult()
            $contextTask = $listener.GetContextAsync()
        } catch [System.AggregateException] {
            $inner = $_.Exception.InnerException
            if ($inner -is [System.ObjectDisposedException] -or $inner -is [System.Net.HttpListenerException]) {
                break
            }
            throw
        } catch [System.ObjectDisposedException], [System.Net.HttpListenerException] {
            break
        }
        try {
            $path = $context.Request.Url.AbsolutePath
            if ($path -eq "/api/llm" -or $path.StartsWith("/api/llm/")) {
                Invoke-Proxy -Context $context -BaseUrl $LlmBaseUrl -PrefixToRemove "/api/llm" -DefaultApiKey $defaultLlmApiKey
                continue
            }
            if ($path -eq "/api/calculator" -or $path.StartsWith("/api/calculator/")) {
                Invoke-Proxy -Context $context -BaseUrl $CalculatorBaseUrl -PrefixToRemove "/api/calculator" -DefaultApiKey $defaultCalculatorApiKey
                continue
            }
            if ($path -eq "/api/geocode") {
                Invoke-Geocode -Context $context
                continue
            }

            $relative = [Uri]::UnescapeDataString($path).TrimStart("/")
            if (-not $relative) {
                $relative = "index.html"
            }
            $relative = $relative.Replace("/", [IO.Path]::DirectorySeparatorChar)
            $filePath = [IO.Path]::GetFullPath((Join-Path $uiRootFull $relative))
            if ($filePath -ne $uiRootFull -and -not $filePath.StartsWith($uiRootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
                Send-Text -Context $context -StatusCode 404 -Body "Not found"
                continue
            }
            if (-not [IO.File]::Exists($filePath)) {
                Send-Text -Context $context -StatusCode 404 -Body "Not found"
                continue
            }
            $bytes = [IO.File]::ReadAllBytes($filePath)
            Send-Bytes -Context $context -StatusCode 200 -Bytes $bytes -ContentType (Get-ContentType -Path $filePath)
        } catch {
            Send-Json -Context $context -StatusCode 500 -Body @{ error = @{ code = "SERVER_ERROR"; message = $_.Exception.Message } }
        }
    }
} finally {
    if ($listener.IsListening) {
        $listener.Stop()
    }
    $listener.Close()
    $httpClient.Dispose()
}
