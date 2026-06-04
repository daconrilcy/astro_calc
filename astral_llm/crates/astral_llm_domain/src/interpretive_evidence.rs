use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const KIND_PLACEMENT: &str = "placement";
pub const KIND_ASPECT: &str = "aspect";
pub const KIND_HOUSE_RULER: &str = "house_ruler";
pub const KIND_ANGLE: &str = "angle";
pub const KIND_DOMAIN_SCORE: &str = "domain_score";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKindFamily {
    Placement,
    Aspect,
    Rulership,
    Dignity,
    Condition,
    Balance,
    Pattern,
    DomainScore,
    Other,
}

impl EvidenceKindFamily {
    pub fn from_kind_code(kind_code: &str) -> Self {
        match kind_code {
            "placement" | "angle" => Self::Placement,
            "aspect" => Self::Aspect,
            "house_ruler" => Self::Rulership,
            "essential_dignity" | "accidental_dignity" => Self::Dignity,
            "planetary_condition" | "sect_condition" | "lunar_phase" => Self::Condition,
            "dominant_planet" | "element_balance" | "modality_balance" => Self::Balance,
            "house_emphasis" | "house_axis" => Self::Pattern,
            "domain_score" => Self::DomainScore,
            _ => Self::Other,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Placement => "placement",
            Self::Aspect => "aspect",
            Self::Rulership => "rulership",
            Self::Dignity => "dignity",
            Self::Condition => "condition",
            Self::Balance => "balance",
            Self::Pattern => "pattern",
            Self::DomainScore => "domain_score",
            Self::Other => "other",
        }
    }

    pub fn counts_as_non_placement(self) -> bool {
        !matches!(self, Self::Placement | Self::DomainScore)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSlotRole {
    Core,
    Supporting,
    Nuance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRequirementSeverity {
    Blocking,
    Warning,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SlotEligibility {
    pub can_be_core: bool,
    pub can_be_supporting: bool,
    pub can_be_nuance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InterpretiveEvidence {
    pub fact_id: String,
    /// Cle interpretative canonique (diversite, avoid_repeating, overlap).
    pub semantic_fact_key: String,
    pub kind_code: String,
    pub family: EvidenceKindFamily,
    pub label: String,
    pub interpretive_hint: String,
    pub chapter_affinity: Vec<String>,
    pub weight: f32,
    pub slot_eligibility: SlotEligibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_number: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChapterEvidencePack {
    pub chapter_code: String,
    pub core: Vec<InterpretiveEvidence>,
    pub supporting: Vec<InterpretiveEvidence>,
    pub nuance: Vec<InterpretiveEvidence>,
    pub avoid_repeating: Vec<String>,
}

impl ChapterEvidencePack {
    pub fn all_fact_ids(&self) -> Vec<&str> {
        self.core
            .iter()
            .chain(self.supporting.iter())
            .chain(self.nuance.iter())
            .map(|e| e.fact_id.as_str())
            .collect()
    }

    pub fn all_semantic_keys(&self) -> Vec<&str> {
        self.core
            .iter()
            .chain(self.supporting.iter())
            .chain(self.nuance.iter())
            .map(|e| e.semantic_fact_key.as_str())
            .collect()
    }

    pub fn total_count(&self) -> usize {
        self.core.len() + self.supporting.len() + self.nuance.len()
    }

    pub fn distinct_families(&self) -> usize {
        let mut families = std::collections::HashSet::new();
        for e in self.core.iter().chain(self.supporting.iter()).chain(self.nuance.iter()) {
            families.insert(e.family.as_str());
        }
        families.len()
    }

    pub fn has_non_placement(&self) -> bool {
        self.core
            .iter()
            .chain(self.supporting.iter())
            .chain(self.nuance.iter())
            .any(|e| e.family.counts_as_non_placement())
    }

    pub fn contains_fact_id(&self, fact_id: &str) -> bool {
        self.all_fact_ids().contains(&fact_id)
    }

    pub fn contains_semantic_key(&self, key: &str) -> bool {
        self.all_semantic_keys().contains(&key)
    }

    pub fn role_for_fact_id(&self, fact_id: &str, semantic_key: &str) -> Option<&'static str> {
        if self.core.iter().any(|e| e.fact_id == fact_id || e.semantic_fact_key == semantic_key) {
            return Some("core");
        }
        if self
            .supporting
            .iter()
            .any(|e| e.fact_id == fact_id || e.semantic_fact_key == semantic_key)
        {
            return Some("supporting");
        }
        if self
            .nuance
            .iter()
            .any(|e| e.fact_id == fact_id || e.semantic_fact_key == semantic_key)
        {
            return Some("nuance");
        }
        None
    }

    pub fn contains_fact_id_or_semantic(&self, fact_id: &str, semantic_key: &str) -> bool {
        self.contains_fact_id(fact_id) || self.contains_semantic_key(semantic_key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InterpretiveEvidencePool {
    pub contract_version: String,
    pub evidence: Vec<InterpretiveEvidence>,
}

impl InterpretiveEvidencePool {
    pub fn interpretive_evidence(&self) -> impl Iterator<Item = &InterpretiveEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.kind_code != KIND_DOMAIN_SCORE)
    }

    pub fn pool_has_aspects(&self) -> bool {
        self.evidence.iter().any(|e| e.kind_code == KIND_ASPECT)
    }

    pub fn pool_has_rulers(&self) -> bool {
        self.evidence.iter().any(|e| e.kind_code == KIND_HOUSE_RULER)
    }

    pub fn pool_has_non_placement(&self) -> bool {
        self.evidence
            .iter()
            .any(|e| e.family.counts_as_non_placement())
    }

    pub fn is_rich_enough_for_premium(&self, min_per_chapter: u8) -> bool {
        let interpretive_count = self.interpretive_evidence().count();
        interpretive_count >= min_per_chapter as usize * 3
    }

    /// Tous les faits interpretatifs, tries par affinite chapitre (pas de filtre dur).
    /// Les slots catalogue (ex. Venus pour `relationships`) peuvent ainsi puiser des
    /// placements dont le theme de maison pointe vers un autre domaine.
    pub fn matching_for_chapter(&self, chapter_code: &str) -> Vec<&InterpretiveEvidence> {
        let mut items: Vec<_> = self.interpretive_evidence().collect();
        items.sort_by(|a, b| {
            let rank = |e: &InterpretiveEvidence| {
                if e.chapter_affinity.is_empty() {
                    1
                } else if e.chapter_affinity.iter().any(|d| d == chapter_code) {
                    2
                } else {
                    0
                }
            };
            rank(b).cmp(&rank(a)).then(
                b.weight
                    .partial_cmp(&a.weight)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        items
    }

    pub fn fact_id_is_domain_score(&self, fact_id: &str) -> bool {
        fact_id.starts_with("domain_score:")
            || self
                .evidence
                .iter()
                .find(|e| e.fact_id == fact_id)
                .is_some_and(|e| e.kind_code == KIND_DOMAIN_SCORE)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PremiumEvidencePolicy {
    pub product_code: String,
    pub min_evidence_per_chapter: u8,
    pub min_distinct_kind_families: u8,
    pub min_non_placement_if_available: u8,
    pub max_core_overlap_ratio: f32,
    pub domain_score_counts_in_minimum: bool,
    pub max_core_evidence: u8,
    pub max_supporting_evidence: u8,
    pub max_nuance_evidence: u8,
    pub max_avoid_repeating: u8,
    /// Nombre max de chapitres ou la meme `semantic_fact_key` peut apparaitre en supporting.
    pub max_supporting_semantic_chapters: u8,
}

impl Default for PremiumEvidencePolicy {
    fn default() -> Self {
        // product_code = interpretation_profile_code (ex. natal_premium)
        Self {
            product_code: "natal_premium".into(),
            min_evidence_per_chapter: 4,
            min_distinct_kind_families: 2,
            min_non_placement_if_available: 1,
            max_core_overlap_ratio: 0.60,
            domain_score_counts_in_minimum: false,
            max_core_evidence: 3,
            max_supporting_evidence: 4,
            max_nuance_evidence: 2,
            max_avoid_repeating: 5,
            max_supporting_semantic_chapters: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterEvidenceSlot {
    pub chapter_code: String,
    pub slot_role: EvidenceSlotRole,
    pub kind_code: Option<String>,
    pub object_code: Option<String>,
    pub house_number: Option<u8>,
    pub domain_code: Option<String>,
    pub priority: i32,
    pub min_weight: f32,
    pub max_items: u8,
    pub required_if_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRequirement {
    pub requirement_code: String,
    pub chapter_code: String,
    pub accepted_kind_codes: Vec<String>,
    pub accepted_object_codes: Vec<String>,
    pub accepted_house_numbers: Vec<u8>,
    pub min_count: u8,
    pub required_if_available: bool,
    pub severity: EvidenceRequirementSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementAuditStatus {
    Applied,
    Failed,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementAuditEntry {
    pub requirement_code: String,
    pub chapter_code: String,
    pub status: RequirementAuditStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceMetrics {
    pub total_unique_facts: u32,
    pub total_unique_semantic_keys: u32,
    pub distinct_kind_families: u32,
    pub max_core_overlap_ratio: f32,
    pub domain_score_used_as_basis: bool,
    pub chapters_with_non_placement: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirement_audit: Vec<RequirementAuditEntry>,
}
