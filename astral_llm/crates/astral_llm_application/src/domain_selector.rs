use astral_llm_domain::{GenerateReadingRequest, ProductGenerationPolicy, ServiceLimits};
use astral_llm_infra::SharedCanonicalCatalog;

use crate::domain_resolver::DomainResolver;

/// Alias historique — delegue a `DomainResolver`.
pub fn select_domains(
    request: &GenerateReadingRequest,
    catalog: &SharedCanonicalCatalog,
    limits: &ServiceLimits,
) -> Vec<String> {
    let policy = catalog
        .product_policy(&request.product_context.product_code)
        .cloned()
        .unwrap_or_else(ProductGenerationPolicy::bootstrap_natal_prompter);
    DomainResolver::resolve(request, catalog, limits, &policy, None)
}
