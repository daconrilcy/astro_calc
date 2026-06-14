use astral_llm_domain::{
    ChapterEvidenceExclusion, EvidenceKindFamily, InterpretiveEvidence, SlotEligibility,
    KIND_PLACEMENT,
};

fn placement_sun() -> InterpretiveEvidence {
    InterpretiveEvidence {
        fact_id: "placement:sun:capricorn:house:2".into(),
        semantic_fact_key: "placement:sun:capricorn:house:2".into(),
        kind_code: KIND_PLACEMENT.into(),
        family: EvidenceKindFamily::Placement,
        label: String::new(),
        interpretive_hint: String::new(),
        chapter_affinity: vec![],
        weight: 1.0,
        slot_eligibility: SlotEligibility {
            can_be_core: true,
            can_be_supporting: true,
            can_be_nuance: false,
        },
        object_code: Some("sun".into()),
        sign_code: None,
        house_number: None,
    }
}

#[test]
fn chapter_exclusion_rules_remain_effective_after_test_migration() {
    let rule = ChapterEvidenceExclusion {
        rule_code: "identity_no_sun".into(),
        chapter_code: "identity".into(),
        kind_code: None,
        object_code: Some("sun".into()),
        fact_id_contains: None,
        global_filler_only: false,
        global_filler_allow_contains: vec![],
    };
    assert!(rule.excludes("identity", &placement_sun()));
    assert!(!rule.excludes("career", &placement_sun()));
}
