use serde::{Deserialize, Serialize};

use crate::ProviderKind;

/// Valeurs par defaut du moteur LLM (source : `.env` au demarrage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineDefaults {
    pub provider: ProviderKind,
    pub model: String,
}
