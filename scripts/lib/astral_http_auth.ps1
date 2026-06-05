function Import-AstralDotEnv {
    param([string]$RepoRoot)
    $envFile = Join-Path $RepoRoot ".env"
    if (-not (Test-Path -LiteralPath $envFile)) { return }
    Get-Content -LiteralPath $envFile | ForEach-Object {
        if ($_ -match '^\s*([^#=]+)=(.*)$') {
            $name = $matches[1].Trim()
            $value = $matches[2].Trim()
            if ($name) { Set-Item -Path "Env:$name" -Value $value }
        }
    }
}

function New-AstralAuthHeaders {
    param(
        [ValidateSet("calculator", "llm")]
        [string]$Service = "llm"
    )
    $headers = @{ "Content-Type" = "application/json" }
    $keyName = if ($Service -eq "calculator") { "ASTRAL_CALCULATOR_API_KEY" } else { "ASTRAL_LLM_API_KEY" }
    $apiKey = [Environment]::GetEnvironmentVariable($keyName)
    if (-not [string]::IsNullOrWhiteSpace($apiKey)) {
        $headers["Authorization"] = "Bearer $apiKey"
        $headers["X-API-Key"] = $apiKey
    }
    return $headers
}

function Invoke-AstralJson {
    param(
        [string]$Method,
        [string]$Uri,
        [hashtable]$Headers,
        $Body = $null
    )
    $params = @{
        Method      = $Method
        Uri         = $Uri
        Headers     = $Headers
        ContentType = "application/json"
    }
    if ($null -ne $Body) {
        if ($Body -is [string]) {
            $params["Body"] = $Body
        } else {
            $params["Body"] = ($Body | ConvertTo-Json -Depth 40)
        }
    }
    return Invoke-RestMethod @params
}
