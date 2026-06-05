use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use astral_calculator::config::load_dotenv;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub allow_public_bind: bool,
    pub api_key: Option<String>,
    pub max_body_bytes: usize,
    pub request_timeout_ms: u64,
    pub schemas_dir: PathBuf,
    pub openapi_path: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Self {
        load_dotenv();

        let host = env_var("ASTRAL_CALCULATOR_HOST").unwrap_or_else(|| "127.0.0.1".into());
        let port = parse_env_u16("ASTRAL_CALCULATOR_PORT", 8080);
        let schemas_dir = env_var("ASTRAL_CALCULATOR_SCHEMAS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(default_schemas_dir);
        let openapi_path = env_var("ASTRAL_CALCULATOR_OPENAPI_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| schemas_dir.join("openapi.yaml"));

        Self {
            bind_addr: format!("{host}:{port}")
                .parse()
                .expect("valid ASTRAL_CALCULATOR bind address"),
            allow_public_bind: env_bool("ASTRAL_CALCULATOR_ALLOW_PUBLIC_BIND", false),
            api_key: env_var("ASTRAL_CALCULATOR_API_KEY"),
            max_body_bytes: parse_env_usize("ASTRAL_CALCULATOR_MAX_BODY_BYTES", 262_144),
            request_timeout_ms: parse_env_u64("ASTRAL_CALCULATOR_REQUEST_TIMEOUT_MS", 60_000),
            schemas_dir,
            openapi_path,
        }
    }

    pub fn requires_auth(&self) -> bool {
        self.api_key
            .as_ref()
            .is_some_and(|key| !key.trim().is_empty())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.bind_addr.ip().is_unspecified() && !self.allow_public_bind {
            return Err(format!(
                "binding to {} requires ASTRAL_CALCULATOR_ALLOW_PUBLIC_BIND=true",
                self.bind_addr
            ));
        }
        if !self.schemas_dir.is_dir() {
            return Err(format!(
                "schemas directory not found: {}",
                self.schemas_dir.display()
            ));
        }
        validate_path_within(&self.openapi_path, &self.schemas_dir)?;
        if !self.openapi_path.is_file() {
            return Err(format!(
                "OpenAPI file not found: {}",
                self.openapi_path.display()
            ));
        }
        Ok(())
    }
}

fn default_schemas_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(&dir)
            .join("..")
            .join("contracts")
            .join("calculator");
        if candidate.is_dir() {
            return candidate;
        }
        let fallback = PathBuf::from(dir)
            .join("..")
            .join("astral_calculator")
            .join("schemas");
        if fallback.is_dir() {
            return fallback;
        }
    }
    PathBuf::from("contracts/calculator")
}

pub fn validate_path_within(path: &Path, allowed_dir: &Path) -> Result<(), String> {
    let allowed = allowed_dir
        .canonicalize()
        .map_err(|e| format!("invalid schemas dir {}: {e}", allowed_dir.display()))?;
    let resolved = if path.exists() {
        path.canonicalize()
    } else {
        path.parent()
            .unwrap_or(path)
            .canonicalize()
            .and_then(|parent| Ok(parent.join(path.file_name().unwrap_or_default())))
    }
    .map_err(|e| format!("invalid path {}: {e}", path.display()))?;

    if !resolved.starts_with(&allowed) {
        return Err(format!(
            "path {} escapes allowed directory {}",
            path.display(),
            allowed.display()
        ));
    }
    Ok(())
}

fn parse_env_u16(name: &str, default: u16) -> u16 {
    env_var(name)
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            if std::env::var(name).is_ok() {
                tracing::warn!(name, "invalid u16 env value; using default");
            }
            default
        })
}

fn parse_env_u64(name: &str, default: u64) -> u64 {
    env_var(name)
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            if std::env::var(name).is_ok() {
                tracing::warn!(name, "invalid u64 env value; using default");
            }
            default
        })
}

fn parse_env_usize(name: &str, default: usize) -> usize {
    env_var(name)
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            if std::env::var(name).is_ok() {
                tracing::warn!(name, "invalid usize env value; using default");
            }
            default
        })
}

fn env_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn env_bool(name: &str, default: bool) -> bool {
    env_var(name)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_path_within_accepts_child_file() {
        let dir = std::env::temp_dir().join("astral_schema_test");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("openapi.yaml");
        std::fs::write(&file, "openapi: 3.1.0").unwrap();
        validate_path_within(&file, &dir).expect("child path allowed");
        let _ = std::fs::remove_dir_all(dir);
    }
}
