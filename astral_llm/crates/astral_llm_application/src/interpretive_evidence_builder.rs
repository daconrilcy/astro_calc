use astral_llm_domain::{
    astro_fact::NormalizedAstroFacts,
    interpretive_evidence::{
        EvidenceKindFamily, InterpretiveEvidence, InterpretiveEvidencePool, SlotEligibility,
        KIND_DOMAIN_SCORE,
    },
    GenerationError, GenerationErrorCode,
};
use astral_llm_infra::EvidenceCanonicalCatalog;

use crate::evidence_fact_parse::{
    compute_semantic_fact_key, house_number_from_fact, object_code_from_fact_id,
    placement_index_by_object, sign_code_from_fact,
};

pub struct InterpretiveEvidenceBuilder;

impl InterpretiveEvidenceBuilder {
    pub fn build(
        facts: &NormalizedAstroFacts,
        _evidence_catalog: &EvidenceCanonicalCatalog,
    ) -> Result<InterpretiveEvidencePool, GenerationError> {
        let placement_by_object = placement_index_by_object(&facts.facts);
        let evidence: Vec<InterpretiveEvidence> = facts
            .facts
            .iter()
            .map(|f| Self::from_fact(f, &placement_by_object))
            .collect();

        Ok(InterpretiveEvidencePool {
            contract_version: facts.contract_version.clone(),
            evidence,
        })
    }

    fn from_fact(
        fact: &astral_llm_domain::NormalizedAstroFact,
        placement_by_object: &std::collections::HashMap<String, String>,
    ) -> InterpretiveEvidence {
        let kind_code = fact.effective_kind_code().to_string();
        let family = EvidenceKindFamily::from_kind_code(&kind_code);
        let hint = fact
            .value
            .get("interpretive_hint")
            .or_else(|| fact.value.get("summary"))
            .and_then(|v| v.as_str())
            .unwrap_or(&fact.label)
            .to_string();

        let weight = fact.interpretive_weight.unwrap_or(0.5);
        let object_code = object_code_from_fact_id(&fact.id);
        let house_number = house_number_from_fact(&fact.id, &fact.value);
        let sign_code = sign_code_from_fact(&fact.id, &fact.value);
        let semantic_fact_key =
            compute_semantic_fact_key(&fact.id, &fact.value, placement_by_object);

        let (can_core, can_supporting, can_nuance) = if kind_code == KIND_DOMAIN_SCORE {
            (false, false, false)
        } else {
            (true, true, true)
        };

        InterpretiveEvidence {
            fact_id: fact.id.clone(),
            semantic_fact_key,
            kind_code,
            family,
            label: fact.label.clone(),
            interpretive_hint: hint,
            chapter_affinity: fact.domains.clone(),
            weight,
            slot_eligibility: SlotEligibility {
                can_be_core: can_core,
                can_be_supporting: can_supporting,
                can_be_nuance: can_nuance,
            },
            object_code,
            sign_code,
            house_number,
        }
    }
}

use crate::interpretation_profile_resolver::ResolvedInterpretationContext;

pub fn evidence_enabled_for_request(
    interpretation: Option<&ResolvedInterpretationContext>,
) -> bool {
    interpretation
        .map(|ctx| ctx.profile.evidence_enabled())
        .unwrap_or(false)
}
pub fn pool_richness_check(
    pool: &InterpretiveEvidencePool,
    policy: &astral_llm_domain::PremiumEvidencePolicy,
    chapter_count: usize,
) -> Result<(), GenerationError> {
    if !pool.is_rich_enough_for_premium(policy.min_evidence_per_chapter, chapter_count) {
        let minimum =
            (policy.min_evidence_per_chapter as usize) * chapter_count.clamp(1, 12).min(6).max(3);
        return Err(GenerationError::with_details(
            GenerationErrorCode::PremiumEvidenceDiversityFailed,
            "The provided astrology payload does not contain enough interpretive evidence for a Premium reading.",
            serde_json::json!({
                "missing": ["aspects", "rulers", "angles", "dignities", "extended_placements"],
                "interpretive_count": pool.interpretive_evidence().count(),
                "minimum_required": minimum,
                "chapter_count": chapter_count,
                "min_evidence_per_chapter": policy.min_evidence_per_chapter,
            }),
        ));
    }
    Ok(())
}
