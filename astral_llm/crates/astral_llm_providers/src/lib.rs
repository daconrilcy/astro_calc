//! Adaptateurs fournisseurs LLM.

pub mod anthropic_adapter;
pub mod fake_provider;
pub mod http;
pub mod mistral_adapter;
pub mod openai_adapter;
pub mod provider_trait;
pub mod response_json;
pub mod types;

pub use anthropic_adapter::AnthropicProvider;
pub use fake_provider::FakeProvider;
pub use mistral_adapter::MistralProvider;
pub use openai_adapter::OpenAiProvider;
pub use provider_trait::{LlmProvider, SharedLlmProvider};
pub use types::*;
