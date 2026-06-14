use astral_llm_domain::{
    domain_selection::{DomainSelection, DomainSelectionStrategy},
    GenerateReadingRequest, ProductGenerationPolicy, ServiceLimits,
};
use astral_llm_infra::SharedCanonicalCatalog;

use crate::interpretation_profile_resolver::ResolvedInterpretationContext;

pub struct DomainResolver;

impl DomainResolver {
    pub fn resolve(
        request: &GenerateReadingRequest,
        catalog: &SharedCanonicalCatalog,
        limits: &ServiceLimits,
        product_policy: &ProductGenerationPolicy,
        interpretation: Option<&ResolvedInterpretationContext>,
    ) -> Vec<String> {
        let mut allowed = catalog.domains_or_fallback(&[]);
        if let Some(ctx) = interpretation {
            let chapter_types = ctx.profile.astrological_chapter_types();
            if !chapter_types.is_empty() {
                allowed.retain(|d| chapter_types.contains(d));
            }
        }
        let fixed_sequence = interpretation
            .map(|ctx| ctx.profile.uses_fixed_chapter_sequence())
            .unwrap_or(false);

        let default_count = interpretation
            .map(|c| c.profile.default_domain_count())
            .unwrap_or(3);
        let requested_count = if fixed_sequence {
            default_count
        } else {
            request.engine.domain_count.unwrap_or(default_count)
        };
        let count = requested_count
            .max(1)
            .min(product_policy.max_domains)
            .min(limits.max_domain_count) as usize;
        let max_count = count;

        if !request.astrologer_profile.preferred_domains.is_empty() && !fixed_sequence {
            return request
                .astrologer_profile
                .preferred_domains
                .iter()
                .filter(|d| allowed.is_empty() || allowed.contains(d))
                .take(count)
                .cloned()
                .collect();
        }
        let strategy = if fixed_sequence || request.engine.domain_count.is_none() {
            DomainSelectionStrategy::ProductDefault
        } else {
            DomainSelectionStrategy::TopWeightedAstroSignals
        };

        let selection = DomainSelection {
            domain_count: count as u8,
            allowed_domains: allowed,
            selected_domains: None,
            selection_strategy: strategy,
        };

        match selection.selection_strategy {
            DomainSelectionStrategy::Explicit => selection
                .selected_domains
                .unwrap_or_default()
                .into_iter()
                .take(max_count)
                .collect(),
            DomainSelectionStrategy::TopWeightedAstroSignals => {
                rank_by_astro_signals(request, &selection, max_count)
            }
            DomainSelectionStrategy::ProductDefault => selection
                .allowed_domains
                .iter()
                .take(count)
                .cloned()
                .collect(),
        }
    }
}

fn rank_by_astro_signals(
    request: &GenerateReadingRequest,
    selection: &DomainSelection,
    max_count: usize,
) -> Vec<String> {
    if let Some(scores) = request
        .astro_result
        .data
        .get("domain_scores")
        .and_then(|v| v.as_object())
    {
        let mut ranked: Vec<(String, f64)> = scores
            .iter()
            .filter_map(|(k, v)| v.as_f64().map(|score| (k.clone(), score)))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let domains: Vec<String> = ranked
            .into_iter()
            .map(|(k, _)| k)
            .filter(|d| {
                selection.allowed_domains.is_empty() || selection.allowed_domains.contains(d)
            })
            .take(selection.domain_count as usize)
            .collect();

        if !domains.is_empty() {
            return domains;
        }
    }

    selection
        .allowed_domains
        .iter()
        .take(max_count)
        .cloned()
        .collect()
}
