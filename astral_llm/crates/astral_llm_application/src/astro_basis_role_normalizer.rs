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
