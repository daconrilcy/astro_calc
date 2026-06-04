use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AstroFactKind {
    PlanetPosition,
    HousePlacement,
    Aspect,
    Dignity,
    DomainScore,
    Angle,
    Ruler,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AstroFactUsage {
    /// Score ou signal de selection de domaine — ne suffit pas seul en Premium.
    DomainSelection,
    /// Placement, aspect, dignite, angle, etc. — base interpretative requise en Premium.
    InterpretiveBasis,
}

impl AstroFactKind {
    pub fn default_usage(self) -> AstroFactUsage {
        match self {
            Self::DomainScore => AstroFactUsage::DomainSelection,
            _ => AstroFactUsage::InterpretiveBasis,
        }
    }

    pub fn as_kind_code(self) -> &'static str {
        match self {
            Self::PlanetPosition => crate::interpretive_evidence::KIND_PLACEMENT,
            Self::Angle => crate::interpretive_evidence::KIND_ANGLE,
            Self::Aspect => crate::interpretive_evidence::KIND_ASPECT,
            Self::Ruler => crate::interpretive_evidence::KIND_HOUSE_RULER,
            Self::Dignity => "essential_dignity",
            Self::HousePlacement => "house_axis",
            Self::DomainScore => crate::interpretive_evidence::KIND_DOMAIN_SCORE,
            Self::Other => "other",
        }
    }

    pub fn from_kind_code(code: &str) -> Self {
        match code {
            "placement" => Self::PlanetPosition,
            "angle" => Self::Angle,
            "aspect" => Self::Aspect,
            "house_ruler" => Self::Ruler,
            "essential_dignity" | "accidental_dignity" => Self::Dignity,
            "house_axis" | "house_emphasis" => Self::HousePlacement,
            "domain_score" => Self::DomainScore,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NormalizedAstroFact {
    pub id: String,
    pub kind: AstroFactKind,
    #[serde(default = "default_empty_string")]
    pub kind_code: String,
    #[serde(default = "default_usage_for_kind")]
    pub usage: AstroFactUsage,
    pub label: String,
    pub value: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpretive_weight: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub domains: Vec<String>,
}

fn default_usage_for_kind() -> AstroFactUsage {
    AstroFactUsage::InterpretiveBasis
}

fn default_empty_string() -> String {
    String::new()
}

impl NormalizedAstroFact {
    pub fn effective_kind_code(&self) -> &str {
        if !self.kind_code.is_empty() {
            return self.kind_code.as_str();
        }
        self.kind.as_kind_code()
    }

    pub fn with_kind_code(mut self) -> Self {
        if self.kind_code.is_empty() {
            self.kind_code = self.kind.as_kind_code().to_string();
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NormalizedAstroFacts {
    pub contract_version: String,
    pub facts: Vec<NormalizedAstroFact>,
}

impl NormalizedAstroFacts {
    pub fn fact_ids(&self) -> Vec<&str> {
        self.facts.iter().map(|f| f.id.as_str()).collect()
    }

    pub fn contains_fact(&self, fact_id: &str) -> bool {
        self.facts.iter().any(|f| f.id == fact_id)
    }

    pub fn fact_by_id(&self, fact_id: &str) -> Option<&NormalizedAstroFact> {
        self.facts.iter().find(|f| f.id == fact_id)
    }

    pub fn is_interpretive_fact_id(&self, fact_id: &str) -> bool {
        self.fact_by_id(fact_id)
            .is_some_and(|f| f.usage == AstroFactUsage::InterpretiveBasis)
    }

    /// Facts destines au prompt d'un chapitre : signaux interpretatifs du domaine + globaux + score domaine.
    pub fn facts_for_chapter_prompt(&self, chapter_code: &str) -> Vec<&NormalizedAstroFact> {
        let mut selected: Vec<&NormalizedAstroFact> = self
            .facts
            .iter()
            .filter(|f| match f.usage {
                AstroFactUsage::DomainSelection => {
                    f.domains.iter().any(|d| d == chapter_code)
                }
                AstroFactUsage::InterpretiveBasis => {
                    f.domains.is_empty() || f.domains.iter().any(|d| d == chapter_code)
                }
            })
            .collect();

        selected.sort_by(|a, b| interpretive_priority(a).cmp(&interpretive_priority(b)));
        selected.dedup_by_key(|f| f.id.as_str());
        selected
    }

    pub fn interpretive_fact_ids(&self) -> Vec<&str> {
        self.facts
            .iter()
            .filter(|f| f.usage == AstroFactUsage::InterpretiveBasis)
            .map(|f| f.id.as_str())
            .collect()
    }
}

fn interpretive_priority(fact: &NormalizedAstroFact) -> u8 {
    match fact.kind {
        AstroFactKind::PlanetPosition | AstroFactKind::Angle => 0,
        AstroFactKind::Aspect => 1,
        AstroFactKind::Dignity | AstroFactKind::Ruler => 2,
        AstroFactKind::HousePlacement => 3,
        AstroFactKind::DomainScore => 9,
        AstroFactKind::Other => 4,
    }
}
