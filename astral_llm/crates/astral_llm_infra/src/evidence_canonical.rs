//! Referentiel evidence Premium (bootstrap + chargement DB optionnel).

use astral_llm_domain::{
    ChapterEvidenceSlot, EvidenceKindFamily, EvidenceRequirement, EvidenceRequirementSeverity,
    EvidenceSlotRole, PremiumEvidencePolicy,
};

impl Default for EvidenceCanonicalCatalog {
    fn default() -> Self {
        bootstrap_evidence_catalog()
    }
}

#[derive(Debug, Clone)]
pub struct EvidenceCanonicalCatalog {
    pub premium_policy: PremiumEvidencePolicy,
    pub chapter_slots: Vec<ChapterEvidenceSlot>,
    pub requirements: Vec<EvidenceRequirement>,
}

impl EvidenceCanonicalCatalog {
    pub fn slots_for_chapter(&self, chapter_code: &str) -> Vec<&ChapterEvidenceSlot> {
        let mut slots: Vec<_> = self
            .chapter_slots
            .iter()
            .filter(|s| s.chapter_code == chapter_code)
            .collect();
        slots.sort_by_key(|s| s.priority);
        slots
    }

    pub fn requirements_for_chapter(&self, chapter_code: &str) -> Vec<&EvidenceRequirement> {
        self.requirements
            .iter()
            .filter(|r| r.chapter_code == chapter_code)
            .collect()
    }
}

pub fn bootstrap_evidence_catalog() -> EvidenceCanonicalCatalog {
    EvidenceCanonicalCatalog {
        premium_policy: PremiumEvidencePolicy::default(),
        chapter_slots: bootstrap_chapter_slots(),
        requirements: bootstrap_requirements(),
    }
}

fn bootstrap_chapter_slots() -> Vec<ChapterEvidenceSlot> {
    let rows: &[(&str, EvidenceSlotRole, Option<&str>, Option<&str>, Option<u8>, i32, u8, bool)] = &[
        ("identity", EvidenceSlotRole::Core, Some("angle"), Some("ascendant"), Some(1), 10, 1, true),
        ("identity", EvidenceSlotRole::Core, Some("house_ruler"), None, Some(1), 20, 1, true),
        ("identity", EvidenceSlotRole::Core, Some("placement"), Some("sun"), None, 30, 1, false),
        ("identity", EvidenceSlotRole::Supporting, Some("aspect"), None, None, 40, 2, false),
        ("identity", EvidenceSlotRole::Nuance, Some("essential_dignity"), None, None, 50, 1, false),
        ("emotional_life", EvidenceSlotRole::Core, Some("placement"), Some("moon"), None, 10, 1, true),
        ("emotional_life", EvidenceSlotRole::Core, Some("aspect"), None, None, 20, 2, true),
        ("emotional_life", EvidenceSlotRole::Supporting, Some("placement"), None, Some(4), 30, 1, true),
        ("emotional_life", EvidenceSlotRole::Supporting, Some("house_ruler"), None, Some(4), 40, 1, true),
        ("emotional_life", EvidenceSlotRole::Nuance, Some("lunar_phase"), None, None, 50, 1, false),
        ("relationships", EvidenceSlotRole::Core, Some("placement"), Some("venus"), None, 10, 1, true),
        ("relationships", EvidenceSlotRole::Core, Some("placement"), None, Some(7), 20, 1, true),
        ("relationships", EvidenceSlotRole::Core, Some("house_ruler"), None, Some(7), 30, 1, true),
        ("relationships", EvidenceSlotRole::Supporting, Some("aspect"), None, None, 40, 2, false),
        ("relationships", EvidenceSlotRole::Nuance, Some("placement"), Some("moon"), None, 50, 1, false),
        ("career", EvidenceSlotRole::Core, Some("angle"), Some("mc"), Some(10), 10, 1, true),
        ("career", EvidenceSlotRole::Core, Some("placement"), None, Some(10), 20, 1, true),
        ("career", EvidenceSlotRole::Core, Some("house_ruler"), None, Some(10), 30, 1, true),
        ("career", EvidenceSlotRole::Supporting, Some("placement"), Some("saturn"), None, 40, 1, false),
        ("career", EvidenceSlotRole::Supporting, Some("placement"), Some("jupiter"), None, 50, 1, false),
        ("career", EvidenceSlotRole::Supporting, Some("placement"), None, Some(2), 60, 1, false),
        ("career", EvidenceSlotRole::Supporting, Some("placement"), None, Some(6), 70, 1, false),
        ("growth_path", EvidenceSlotRole::Core, Some("placement"), Some("north_node"), None, 10, 1, false),
        ("growth_path", EvidenceSlotRole::Core, Some("placement"), Some("saturn"), None, 20, 1, false),
        ("growth_path", EvidenceSlotRole::Supporting, Some("aspect"), None, None, 30, 2, false),
        ("growth_path", EvidenceSlotRole::Supporting, Some("placement"), None, Some(8), 40, 1, false),
        ("growth_path", EvidenceSlotRole::Supporting, Some("placement"), None, Some(9), 50, 1, false),
        ("growth_path", EvidenceSlotRole::Supporting, Some("placement"), None, Some(12), 60, 1, false),
    ];
    rows.iter()
        .map(|(chapter, role, kind, object, house, priority, max_items, req)| ChapterEvidenceSlot {
            chapter_code: chapter.to_string(),
            slot_role: *role,
            kind_code: kind.map(str::to_string),
            object_code: object.map(str::to_string),
            house_number: *house,
            domain_code: None,
            priority: *priority,
            min_weight: 0.0,
            max_items: *max_items,
            required_if_available: *req,
        })
        .collect()
}

fn bootstrap_requirements() -> Vec<EvidenceRequirement> {
    vec![
        EvidenceRequirement {
            requirement_code: "career_mc_or_h10".into(),
            chapter_code: "career".into(),
            accepted_kind_codes: vec!["angle".into(), "placement".into()],
            accepted_object_codes: vec!["mc".into()],
            accepted_house_numbers: vec![10],
            min_count: 1,
            required_if_available: true,
            severity: EvidenceRequirementSeverity::Blocking,
        },
        EvidenceRequirement {
            requirement_code: "relationships_venus_or_h7".into(),
            chapter_code: "relationships".into(),
            accepted_kind_codes: vec!["placement".into(), "house_ruler".into()],
            accepted_object_codes: vec!["venus".into()],
            accepted_house_numbers: vec![7],
            min_count: 1,
            required_if_available: true,
            severity: EvidenceRequirementSeverity::Blocking,
        },
        EvidenceRequirement {
            requirement_code: "emotional_moon_aspects".into(),
            chapter_code: "emotional_life".into(),
            accepted_kind_codes: vec!["aspect".into()],
            accepted_object_codes: vec!["moon".into()],
            accepted_house_numbers: vec![],
            min_count: 1,
            required_if_available: true,
            severity: EvidenceRequirementSeverity::Blocking,
        },
        EvidenceRequirement {
            requirement_code: "identity_asc_ruler".into(),
            chapter_code: "identity".into(),
            accepted_kind_codes: vec!["angle".into(), "house_ruler".into()],
            accepted_object_codes: vec!["ascendant".into()],
            accepted_house_numbers: vec![1],
            min_count: 1,
            required_if_available: true,
            severity: EvidenceRequirementSeverity::Blocking,
        },
        EvidenceRequirement {
            requirement_code: "global_aspect_when_available".into(),
            chapter_code: "identity".into(),
            accepted_kind_codes: vec!["aspect".into()],
            accepted_object_codes: vec![],
            accepted_house_numbers: vec![],
            min_count: 1,
            required_if_available: true,
            severity: EvidenceRequirementSeverity::Warning,
        },
    ]
}

pub fn family_for_kind_code(kind_code: &str) -> EvidenceKindFamily {
    EvidenceKindFamily::from_kind_code(kind_code)
}
