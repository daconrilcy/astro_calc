use std::sync::{Mutex, OnceLock};

use astral_llm_infra::AppConfig;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(pairs: &[(&'static str, Option<&str>)]) -> Self {
        let saved = pairs
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect::<Vec<_>>();

        for (key, value) in pairs {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }

        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..) {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

#[test]
fn app_config_reports_invalid_openai_base_url() {
    let _guard = env_lock().lock().expect("env lock");
    let _env = EnvGuard::set(&[
        ("OPENAI_BASE_URL", Some("notaurl")),
        ("ANTHROPIC_BASE_URL", Some("https://api.anthropic.com")),
        ("MISTRAL_BASE_URL", Some("https://api.mistral.ai")),
        ("ASTRAL_LLM_HOST", Some("127.0.0.1")),
        ("ASTRAL_LLM_PORT", Some("8081")),
    ]);

    let err = AppConfig::try_from_env().expect_err("invalid OPENAI_BASE_URL must fail");
    assert!(err.to_string().contains("invalid OPENAI_BASE_URL"));
}

#[test]
fn app_config_from_env_reports_invalid_openai_base_url() {
    let _guard = env_lock().lock().expect("env lock");
    let _env = EnvGuard::set(&[
        ("OPENAI_BASE_URL", Some("notaurl")),
        ("ANTHROPIC_BASE_URL", Some("https://api.anthropic.com")),
        ("MISTRAL_BASE_URL", Some("https://api.mistral.ai")),
        ("ASTRAL_LLM_HOST", Some("127.0.0.1")),
        ("ASTRAL_LLM_PORT", Some("8081")),
    ]);

    let err = AppConfig::from_env().expect_err("invalid OPENAI_BASE_URL must fail");
    assert!(err.to_string().contains("invalid OPENAI_BASE_URL"));
}

#[test]
fn app_config_reports_invalid_bind_address() {
    let _guard = env_lock().lock().expect("env lock");
    let _env = EnvGuard::set(&[
        ("OPENAI_BASE_URL", Some("https://api.openai.com")),
        ("ANTHROPIC_BASE_URL", Some("https://api.anthropic.com")),
        ("MISTRAL_BASE_URL", Some("https://api.mistral.ai")),
        ("ASTRAL_LLM_HOST", Some("not a host")),
        ("ASTRAL_LLM_PORT", Some("8081")),
    ]);

    let err = AppConfig::try_from_env().expect_err("invalid bind address must fail");
    assert!(err.to_string().contains("invalid ASTRAL_LLM bind address"));
}
