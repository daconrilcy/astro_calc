use dotenvy::dotenv;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: String,
    pub calculator_base_url: String,
    pub calculator_api_key: Option<String>,
    pub llm_base_url: String,
    pub llm_api_key: Option<String>,
    pub request_timeout_ms: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let _ = dotenv();
        let calculator_host =
            std::env::var("ASTRAL_CALCULATOR_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let calculator_port = std::env::var("ASTRAL_CALCULATOR_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(8080);
        let llm_host = std::env::var("ASTRAL_LLM_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let llm_port = std::env::var("ASTRAL_LLM_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(8081);

        Self {
            bind_addr: std::env::var("ASTRAL_GATEWAY_BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8082".into()),
            calculator_base_url: format!("http://{calculator_host}:{calculator_port}"),
            calculator_api_key: std::env::var("ASTRAL_CALCULATOR_API_KEY").ok(),
            llm_base_url: format!("http://{llm_host}:{llm_port}"),
            llm_api_key: std::env::var("ASTRAL_LLM_API_KEY").ok(),
            request_timeout_ms: std::env::var("ASTRAL_GATEWAY_REQUEST_TIMEOUT_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(180_000),
        }
    }
}
