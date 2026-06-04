use astral_llm_domain::{
    generation_response::ReadingChapter,
    interpretive_evidence::{ChapterEvidencePack, InterpretiveEvidence},
};

use crate::evidence_fact_parse::{
    compute_semantic_fact_key, fact_id_role_bucket, object_code_from_fact_id,
};

pub struct AstroBasisRoleNormalizer;

impl AstroBasisRoleNormalizer {
    pub fn normalize_chapter(chapter: &mut ReadingChapter, pack: Option<&ChapterEvidencePack>) {
        let Some(pack) = pack else {
            for item in &mut chapter.astro_basis {
                Self::coerce_role_string(&mut item.interpretive_role);
            }
            return;
        };

        for item in &mut chapter.astro_basis {
            let Some(ref fact_id) = item.fact_id else {
                Self::coerce_role_string(&mut item.interpretive_role);
                continue;
            };
            if fact_id.starts_with("domain_score:") {
                item.interpretive_role = "domain_score".into();
                continue;
            }
            if let Some(role) = Self::role_from_pack_fact_id(pack, fact_id) {
                item.interpretive_role = role;
            } else {
                Self::coerce_role_string(&mut item.interpretive_role);
            }
        }
    }

    fn role_from_pack_fact_id(pack: &ChapterEvidencePack, fact_id: &str) -> Option<String> {
        let semantic_key = compute_semantic_fact_key(
            fact_id,
            &serde_json::json!({}),
            &std::collections::HashMap::new(),
        );
        if let Some(role) = pack.role_for_fact_id(fact_id, &semantic_key) {
            return Some(role.to_string());
        }
        let object = object_code_from_fact_id(fact_id)?;
        let bucket = fact_id_role_bucket(fact_id);
        Self::role_for_pack_object_same_bucket(pack, &object, bucket)
    }

    fn role_for_pack_object_same_bucket(
        pack: &ChapterEvidencePack,
        object: &str,
        bucket: &str,
    ) -> Option<String> {
        let matches = |e: &InterpretiveEvidence| {
            fact_id_role_bucket(&e.fact_id) == bucket
                && (e.object_code.as_deref() == Some(object)
                    || object_code_from_fact_id(&e.fact_id).as_deref() == Some(object))
        };
        if pack.core.iter().any(matches) {
            return Some("core".into());
        }
        if pack.supporting.iter().any(matches) {
            return Some("supporting".into());
        }
        if pack.nuance.iter().any(matches) {
            return Some("nuance".into());
        }
        None
    }

    fn coerce_role_string(role: &mut String) {
        let normalized = role.trim().to_lowercase();
        let code = match normalized.as_str() {
            "core" | "principal" | "fondement" | "fondement principal" => "core",
            "supporting" | "soutien" | "support" => "supporting",
            "nuance" => "nuance",
            "domain_score" | "signal de selection du domaine" => "domain_score",
            _ if normalized.contains("nuance") => "nuance",
            _ if normalized.contains("support") || normalized.contains("soutien") => "supporting",
            _ if normalized.contains("core") || normalized.contains("principal") => "core",
            _ => "supporting",
        };
        *role = code.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        generation_response::{ConfidenceLevel, ReadingChapter},
        interpretive_evidence::InterpretiveEvidence,
        ChapterEvidencePack, EvidenceKindFamily, SlotEligibility,
    };

    fn sample_evidence(id: &str) -> InterpretiveEvidence {
        sample_evidence_with_object(id, None)
    }

    fn sample_evidence_with_object(id: &str, object_code: Option<&str>) -> InterpretiveEvidence {
        InterpretiveEvidence {
            fact_id: id.into(),
            semantic_fact_key: id.into(),
            kind_code: "placement".into(),
            family: EvidenceKindFamily::Placement,
            label: id.into(),
            interpretive_hint: String::new(),
            chapter_affinity: vec![],
            weight: 1.0,
            slot_eligibility: SlotEligibility::default(),
            object_code: object_code.map(str::to_string),
            sign_code: None,
            house_number: None,
        }
    }

    #[test]
    fn maps_free_text_to_core_from_pack() {
        let pack = ChapterEvidencePack {
            chapter_code: "identity".into(),
            core: vec![sample_evidence("placement:sun:capricorn:house:2")],
            supporting: vec![],
            nuance: vec![],
            avoid_repeating: vec![],
        };
        let mut chapter = ReadingChapter {
            code: "identity".into(),
            title: "T".into(),
            body: "B".into(),
            astro_basis: vec![astral_llm_domain::AstroBasisItem {
                fact_id: Some("placement:sun:capricorn:house:2".into()),
                label: None,
                factor: "x".into(),
                interpretive_role: "Fondement principal du chemin".into(),
            }],
            confidence: ConfidenceLevel::High,
            safety_flags: vec![],
        };
        AstroBasisRoleNormalizer::normalize_chapter(&mut chapter, Some(&pack));
        assert_eq!(chapter.astro_basis[0].interpretive_role, "core");
    }

    #[test]
    fn signal_sun_keeps_supporting_when_placement_sun_is_core() {
        let pack = ChapterEvidencePack {
            chapter_code: "identity".into(),
            core: vec![sample_evidence_with_object(
                "placement:sun:capricorn:house:2",
                Some("sun"),
            )],
            supporting: vec![InterpretiveEvidence {
                fact_id: "signal:object_position:sun".into(),
                semantic_fact_key: "placement:sun:capricorn:house:2".into(),
                kind_code: "signal".into(),
                family: EvidenceKindFamily::Other,
                label: "Sun signal".into(),
                interpretive_hint: String::new(),
                chapter_affinity: vec![],
                weight: 1.0,
                slot_eligibility: SlotEligibility::default(),
                object_code: Some("sun".into()),
                sign_code: None,
                house_number: None,
            }],
            nuance: vec![],
            avoid_repeating: vec![],
        };
        let mut chapter = ReadingChapter {
            code: "identity".into(),
            title: "T".into(),
            body: "B".into(),
            astro_basis: vec![astral_llm_domain::AstroBasisItem {
                fact_id: Some("signal:object_position:sun".into()),
                label: None,
                factor: "x".into(),
                interpretive_role: "core".into(),
            }],
            confidence: ConfidenceLevel::High,
            safety_flags: vec![],
        };
        AstroBasisRoleNormalizer::normalize_chapter(&mut chapter, Some(&pack));
        assert_eq!(chapter.astro_basis[0].interpretive_role, "supporting");
    }

    #[test]
    fn placement_sun_stays_core_when_cited_as_placement() {
        let pack = ChapterEvidencePack {
            chapter_code: "identity".into(),
            core: vec![sample_evidence_with_object(
                "placement:sun:capricorn:house:2",
                Some("sun"),
            )],
            supporting: vec![InterpretiveEvidence {
                fact_id: "signal:object_position:sun".into(),
                semantic_fact_key: "placement:sun:capricorn:house:2".into(),
                kind_code: "signal".into(),
                family: EvidenceKindFamily::Other,
                label: "Sun signal".into(),
                interpretive_hint: String::new(),
                chapter_affinity: vec![],
                weight: 1.0,
                slot_eligibility: SlotEligibility::default(),
                object_code: Some("sun".into()),
                sign_code: None,
                house_number: None,
            }],
            nuance: vec![],
            avoid_repeating: vec![],
        };
        let mut chapter = ReadingChapter {
            code: "identity".into(),
            title: "T".into(),
            body: "B".into(),
            astro_basis: vec![astral_llm_domain::AstroBasisItem {
                fact_id: Some("placement:sun:capricorn:house:2".into()),
                label: None,
                factor: "x".into(),
                interpretive_role: "supporting".into(),
            }],
            confidence: ConfidenceLevel::High,
            safety_flags: vec![],
        };
        AstroBasisRoleNormalizer::normalize_chapter(&mut chapter, Some(&pack));
        assert_eq!(chapter.astro_basis[0].interpretive_role, "core");
    }
}
