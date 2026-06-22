use astral_llm_domain::{integration::IntegrationService, ProductGenerationPolicy};
use astral_llm_infra::SharedCanonicalCatalog;

#[derive(Clone)]
pub struct ReadingCatalog {
    shared: SharedCanonicalCatalog,
}

impl ReadingCatalog {
    pub fn new(shared: SharedCanonicalCatalog) -> Self {
        Self { shared }
    }

    pub(crate) fn as_shared(&self) -> &SharedCanonicalCatalog {
        &self.shared
    }

    pub fn integration_service(&self, service_code: &str) -> Option<&IntegrationService> {
        self.shared.integration_service(service_code)
    }

    pub fn list_integration_services(&self, include_planned: bool) -> Vec<&IntegrationService> {
        self.shared.list_integration_services(include_planned)
    }

    pub fn product_policy(&self, product_code: &str) -> Option<&ProductGenerationPolicy> {
        self.shared.product_policy(product_code)
    }
}

impl From<SharedCanonicalCatalog> for ReadingCatalog {
    fn from(shared: SharedCanonicalCatalog) -> Self {
        Self::new(shared)
    }
}
