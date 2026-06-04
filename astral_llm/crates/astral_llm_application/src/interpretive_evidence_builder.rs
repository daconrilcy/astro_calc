use astral_llm_domain::{
    astro_fact::NormalizedAstroFacts,
    interpretive_evidence::{
        EvidenceKindFamily, InterpretiveEvidence, InterpretiveEvidencePool, SlotEligibility,
        KIND_DOMAIN_SCORE,
    },
    GenerationError, GenerationErrorCode,
};
use astral_llm_infra::EvidenceCanonicalCatalog;

use crate::evidence_fact_parse::{house_number_from_fact, object_code_from_fact_id};

pub struct InterpretiveEvidenceBuilder;

impl InterpretiveEvidenceBuilder {
    pub fn build(
        facts: &NormalizedAstroFacts,
        _evidence_catalog: &EvidenceCanonicalCatalog,
    ) -> Result<InterpretiveEvidencePool, GenerationError> {
        let evidence: Vec<InterpretiveEvidence> = facts
            .facts
            .iter()
            .map(Self::from_fact)
            .collect();

        Ok(InterpretiveEvidencePool {
            contract_version: facts.contract_version.clone(),
            evidence,
        })
    }

    fn from_fact(fact: &astral_llm_domain::NormalizedAstroFact) -> InterpretiveEvidence {
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

        let (can_core, can_supporting, can_nuance) = if kind_code == KIND_DOMAIN_SCORE {
            (false, false, false)
        } else {
            (true, true, true)
        };

        InterpretiveEvidence {
            fact_id: fact.id.clone(),
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
            house_number,
        }
    }
}

pub fn is_premium_product(product_code: &str) -> bool {
    product_code.contains("premium")
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_infra::bootstrap_evidence_catalog;

    fn minimal_facts() -> NormalizedAstroFacts {
        let payload = astral_llm_domain::AstroCalculationPayload {
            contract_version: "natal_structured_v13".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({
                "planets": {
                    "sun": { "house": 2, "sign": "capricorn" },
                    "moon": { "house": 4, "sign": "pisces" },
                    "ascendant": { "house": 1, "sign": "scorpio" }
                }
            }),
        };
        crate::AstroPayloadNormalizer::normalize(
            &payload,
            &astral_llm_domain::PrivacyPolicy::default(),
            &astral_llm_infra::CanonicalCatalog::default(),
            "fr",
        )
        .unwrap()
    }

    #[test]
    fn minimal_pool_not_rich_enough() {
        let facts = minimal_facts();
        let pool = InterpretiveEvidenceBuilder::build(&facts, &bootstrap_evidence_catalog()).unwrap();
        let policy = bootstrap_evidence_catalog().premium_policy;
        assert!(pool_richness_check(&pool, &policy).is_err());
    }
}

pub fn pool_richness_check(
    pool: &InterpretiveEvidencePool,
    policy: &astral_llm_domain::PremiumEvidencePolicy,
) -> Result<(), GenerationError> {
    if !pool.is_rich_enough_for_premium(policy.min_evidence_per_chapter) {
        return Err(GenerationError::with_details(
            GenerationErrorCode::PremiumEvidenceDiversityFailed,
            "The provided astrology payload does not contain enough interpretive evidence for a Premium reading.",
            serde_json::json!({
                "missing": ["aspects", "rulers", "angles", "dignities", "extended_placements"],
                "interpretive_count": pool.interpretive_evidence().count(),
                "minimum_required": format!(
                    "{} evidence items per chapter when pool is rich",
                    policy.min_evidence_per_chapter
                ),
            }),
        ));
    }
    Ok(())
}
