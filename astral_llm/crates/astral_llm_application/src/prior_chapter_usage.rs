//! Suivi des cles semantiques deja utilisees dans les chapitres precedents.

use std::collections::{HashMap, HashSet};

use astral_llm_domain::{
    interpretive_evidence::{ChapterEvidencePack, EvidenceKindFamily, InterpretiveEvidence},
    PremiumEvidencePolicy,
};

#[derive(Debug, Default)]
pub struct PriorChapterUsage {
    pub core_keys: Vec<String>,
    aspect_keys: HashSet<String>,
    dignity_keys: HashSet<String>,
    /// Nombre de chapitres precedents ayant deja cite cette cle en supporting.
    supporting_semantic_chapter_count: HashMap<String, u8>,
}

impl PriorChapterUsage {
    pub fn record_pack(&mut self, pack: &ChapterEvidencePack) {
        for ev in pack
            .core
            .iter()
            .chain(pack.supporting.iter())
            .chain(pack.nuance.iter())
        {
            if Self::is_aspect(ev) {
                self.aspect_keys.insert(ev.semantic_fact_key.clone());
            }
            if Self::is_dignity(ev) {
                self.dignity_keys.insert(ev.semantic_fact_key.clone());
            }
        }
        for ev in &pack.core {
            self.core_keys.push(ev.semantic_fact_key.clone());
        }
        let mut seen_supporting = HashSet::new();
        for ev in &pack.supporting {
            if seen_supporting.insert(ev.semantic_fact_key.clone()) {
                *self
                    .supporting_semantic_chapter_count
                    .entry(ev.semantic_fact_key.clone())
                    .or_insert(0) += 1;
            }
        }
    }

    pub fn supporting_semantic_chapters_used(&self, semantic_key: &str) -> u8 {
        self.supporting_semantic_chapter_count
            .get(semantic_key)
            .copied()
            .unwrap_or(0)
    }

    pub fn exceeds_supporting_semantic_cap(&self, semantic_key: &str, max_chapters: u8) -> bool {
        max_chapters > 0 && self.supporting_semantic_chapters_used(semantic_key) >= max_chapters
    }

    pub fn planner_avoid_keys(&self) -> HashSet<String> {
        let mut keys: HashSet<String> = self.core_keys.iter().cloned().collect();
        keys.extend(self.aspect_keys.iter().cloned());
        keys.extend(self.dignity_keys.iter().cloned());
        keys
    }

    pub fn build_avoid_repeating(&self, policy: &PremiumEvidencePolicy) -> Vec<String> {
        let skip = self
            .core_keys
            .len()
            .saturating_sub(policy.max_avoid_repeating as usize);
        let mut out: Vec<String> = self.core_keys.iter().skip(skip).cloned().collect();
        for key in &self.aspect_keys {
            if !out.contains(key) {
                out.push(key.clone());
            }
        }
        for key in &self.dignity_keys {
            if !out.contains(key) {
                out.push(key.clone());
            }
        }
        out
    }

    fn is_aspect(ev: &InterpretiveEvidence) -> bool {
        ev.kind_code == "aspect" || ev.family == EvidenceKindFamily::Aspect
    }

    fn is_dignity(ev: &InterpretiveEvidence) -> bool {
        matches!(
            ev.kind_code.as_str(),
            "essential_dignity" | "accidental_dignity"
        ) || matches!(ev.family, EvidenceKindFamily::Dignity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::interpretive_evidence::SlotEligibility;

    fn ev(id: &str, kind: &str, family: EvidenceKindFamily, key: &str) -> InterpretiveEvidence {
        InterpretiveEvidence {
            fact_id: id.into(),
            semantic_fact_key: key.into(),
            kind_code: kind.into(),
            family,
            label: id.into(),
            interpretive_hint: String::new(),
            chapter_affinity: vec![],
            weight: 1.0,
            slot_eligibility: SlotEligibility::default(),
            object_code: None,
            sign_code: None,
            house_number: None,
        }
    }

    #[test]
    fn avoid_list_includes_prior_aspects_and_dignities() {
        let mut prior = PriorChapterUsage::default();
        prior.record_pack(&ChapterEvidencePack {
            chapter_code: "identity".into(),
            core: vec![ev(
                "placement:ascendant:scorpio:house:1",
                "placement",
                EvidenceKindFamily::Placement,
                "placement:ascendant:scorpio:house:1",
            )],
            supporting: vec![ev(
                "signal:aspect:jupiter:uranus:opposition",
                "aspect",
                EvidenceKindFamily::Aspect,
                "aspect:jupiter:uranus:opposition",
            )],
            nuance: vec![ev(
                "signal:dignity:saturn:domicile:capricorn",
                "essential_dignity",
                EvidenceKindFamily::Dignity,
                "signal:dignity:saturn:domicile:capricorn",
            )],
            avoid_repeating: vec![],
        });
        let avoid = prior.build_avoid_repeating(&PremiumEvidencePolicy::default());
        assert!(avoid.contains(&"aspect:jupiter:uranus:opposition".to_string()));
        assert!(avoid.contains(&"signal:dignity:saturn:domicile:capricorn".to_string()));
        assert!(prior
            .planner_avoid_keys()
            .contains("aspect:jupiter:uranus:opposition"));
    }

    #[test]
    fn supporting_semantic_cap_counts_distinct_chapters() {
        let mut prior = PriorChapterUsage::default();
        let policy = PremiumEvidencePolicy::default();
        let key = "placement:sun:capricorn:house:2";
        for ch in ["emotional_life", "relationships", "career"] {
            prior.record_pack(&ChapterEvidencePack {
                chapter_code: ch.into(),
                core: vec![],
                supporting: vec![ev(
                    "placement:sun",
                    "placement",
                    EvidenceKindFamily::Placement,
                    key,
                )],
                nuance: vec![],
                avoid_repeating: vec![],
            });
        }
        assert_eq!(prior.supporting_semantic_chapters_used(key), 3);
        assert!(prior.exceeds_supporting_semantic_cap(key, policy.max_supporting_semantic_chapters));
    }
}
