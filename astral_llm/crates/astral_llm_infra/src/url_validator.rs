//! Validation des URLs provider (anti-SSRF).

const ALLOWED_OPENAI_HOSTS: &[&str] = &["api.openai.com"];
const ALLOWED_ANTHROPIC_HOSTS: &[&str] = &["api.anthropic.com"];
const ALLOWED_MISTRAL_HOSTS: &[&str] = &["api.mistral.ai"];

pub fn validate_provider_base_url(
    label: &str,
    url: &str,
    allowed_hosts: &[&str],
) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("{label} URL invalid: {e}"))?;

    if parsed.scheme() != "https" {
        return Err(format!("{label} URL must use HTTPS"));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| format!("{label} URL missing host"))?;

    if is_private_or_local_host(host) {
        return Err(format!(
            "{label} URL must not target private or local addresses"
        ));
    }

    if !allowed_hosts
        .iter()
        .any(|allowed| host.eq_ignore_ascii_case(allowed))
    {
        return Err(format!(
            "{label} URL host `{host}` not in allowlist: {allowed_hosts:?}"
        ));
    }

    Ok(())
}

pub fn validate_openai_base_url(url: &str) -> Result<(), String> {
    validate_provider_base_url("OPENAI_BASE_URL", url, ALLOWED_OPENAI_HOSTS)
}

pub fn validate_anthropic_base_url(url: &str) -> Result<(), String> {
    validate_provider_base_url("ANTHROPIC_BASE_URL", url, ALLOWED_ANTHROPIC_HOSTS)
}

pub fn validate_mistral_base_url(url: &str) -> Result<(), String> {
    validate_provider_base_url("MISTRAL_BASE_URL", url, ALLOWED_MISTRAL_HOSTS)
}

fn is_private_or_local_host(host: &str) -> bool {
    let lower = host.to_lowercase();
    if lower == "localhost" || lower.ends_with(".local") {
        return true;
    }
    if lower.starts_with("127.") || lower.starts_with("10.") || lower.starts_with("192.168.") {
        return true;
    }
    if lower.starts_with("169.254.") || lower == "0.0.0.0" {
        return true;
    }
    lower.starts_with("172.")
        && lower
            .split('.')
            .nth(1)
            .and_then(|p| p.parse::<u8>().ok())
            .is_some_and(|p| (16..=31).contains(&p))
}
