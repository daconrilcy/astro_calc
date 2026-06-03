use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AstralLlmEnv {
    Local,
    Test,
    Production,
}

impl AstralLlmEnv {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_lowercase().as_str() {
            "production" | "prod" => Self::Production,
            "test" => Self::Test,
            _ => Self::Local,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Test => "test",
            Self::Production => "production",
        }
    }

    pub fn is_production(&self) -> bool {
        matches!(self, Self::Production)
    }

    pub fn allows_fake_by_default(&self) -> bool {
        !self.is_production()
    }

    pub fn requires_api_key(&self) -> bool {
        self.is_production()
    }
}
