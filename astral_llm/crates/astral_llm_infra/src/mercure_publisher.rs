//! Publication d'événements Mercure pour jobs d'intégration (Phase 4).

use reqwest::Client;
use serde_json::json;
use tracing::warn;

/// Client Mercure optionnel — no-op si URL non configurée.
#[derive(Clone)]
pub struct MercurePublisher {
    client: Client,
    hub_url: Option<String>,
    jwt: Option<String>,
}

impl MercurePublisher {
    pub fn from_env() -> Self {
        let hub_url = std::env::var("ASTRAL_LLM_MERCURE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let jwt = std::env::var("ASTRAL_LLM_MERCURE_PUBLISHER_JWT")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Self {
            client: Client::new(),
            hub_url,
            jwt,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.hub_url.is_some()
    }

    /// Topic : `tenants/{tenant_id}/jobs/{run_id}`
    pub async fn publish_job_status(&self, tenant_id: &str, run_id: &str, status: &str) {
        let Some(hub_url) = self.hub_url.as_ref() else {
            return;
        };
        let topic = format!("tenants/{tenant_id}/jobs/{run_id}");
        let poll_url = format!("/v1/jobs/{run_id}");
        let data = json!({
            "run_id": run_id,
            "status": status,
            "poll_url": poll_url,
        });
        let mut req = self
            .client
            .post(hub_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[("topic", topic.as_str()), ("data", &data.to_string())]);
        if let Some(token) = &self.jwt {
            req = req.bearer_auth(token);
        }
        if let Err(err) = req.send().await {
            warn!(error = %err, run_id, status, "mercure publish failed");
        }
    }
}

impl Default for MercurePublisher {
    fn default() -> Self {
        Self::from_env()
    }
}
