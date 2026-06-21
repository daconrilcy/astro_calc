//! Module astral_calculator\src\domain\chart_facts.rs du moteur astral_calculator.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "swisseph-engine")]
use crate::domain::{
    AnglePointReference, ChartObject, HouseReference, MotionStateReference, SignReference,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignContext {
    pub element: Option<String>,
    pub element_label: Option<String>,
    pub modality: Option<String>,
    pub modality_label: Option<String>,
    pub polarity: Option<String>,
    pub polarity_label: Option<String>,
    pub keywords: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HouseContext {
    pub theme_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HouseModalityContext {
    pub code: Option<String>,
    pub label: Option<String>,
    pub accidental_strength: Option<f64>,
    pub priority_delta: Option<f64>,
    pub interpretation_weight: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectContext {
    pub role: Option<String>,
    pub role_label: Option<String>,
    pub nature: Option<Value>,
    pub is_luminary: Option<bool>,
    pub is_planet_symbolic: Option<bool>,
    pub is_visible_to_naked_eye: Option<bool>,
    pub signal_scoring: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AngleContext {
    pub angle_point_code: Option<String>,
    pub short_label: Option<String>,
    pub full_name: Option<String>,
    pub axis: Option<String>,
    pub opposite_angle_code: Option<String>,
    pub associated_house_number: Option<i32>,
    pub house_theme_code: Option<String>,
    pub description: Option<String>,
    pub chart_object_sort_order: Option<i32>,
    pub house_cusp_longitude_deg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionVisibilityContext {
    pub horizon_position_id: Option<i32>,
    pub horizon_position: Option<String>,
    pub altitude_deg: Option<f64>,
    pub is_visible: Option<bool>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionFactContext {
    pub sign_context: Option<SignContext>,
    pub house_context: Option<HouseContext>,
    pub house_modality: Option<HouseModalityContext>,
    pub object_context: Option<ObjectContext>,
    pub motion_context: Option<MotionContext>,
    pub angle_context: Option<AngleContext>,
    pub visibility_context: Option<PositionVisibilityContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MotionContext {
    pub motion_state: Option<String>,
    pub label: Option<String>,
    pub motion_family: Option<String>,
}

#[derive(Debug, Clone)]
/// Structure ObjectPositionFact.
pub struct ObjectPositionFact {
    pub chart_object_id: i32,
    pub object_code: String,
    pub object_name: String,
    pub zodiacal_reference_system_id: i32,
    pub coordinate_reference_system_id: i32,
    pub sign_id: i32,
    pub sign_code: String,
    pub sign_name: String,
    pub house_id: Option<i32>,
    pub house_number: Option<i32>,
    pub house_name: Option<String>,
    pub motion_state_id: Option<i32>,
    pub horizon_position_id: Option<i32>,
    pub longitude_deg: f64,
    pub latitude_deg: Option<f64>,
    pub apparent_speed_deg_per_day: Option<f64>,
    pub altitude_deg: Option<f64>,
    pub is_visible: Option<bool>,
    pub facts_json: Option<Value>,
}

impl ObjectPositionFact {
    pub fn context(&self) -> Option<PositionFactContext> {
        PositionFactContext::from_facts_json(self.facts_json.as_ref())
    }

    pub fn object_context(&self) -> Option<ObjectContext> {
        self.context().and_then(|context| context.object_context)
    }

    pub fn angle_context(&self) -> Option<AngleContext> {
        self.context().and_then(|context| context.angle_context)
    }

    pub fn visibility_context(&self) -> Option<PositionVisibilityContext> {
        self.context()
            .and_then(|context| context.visibility_context)
    }
}

impl PositionFactContext {
    pub fn from_calculated_position(
        sign_context: Option<SignContext>,
        house_context: Option<HouseContext>,
        house_modality: Option<HouseModalityContext>,
        object_context: Option<ObjectContext>,
        motion_context: Option<MotionContext>,
        visibility_context: Option<PositionVisibilityContext>,
    ) -> Self {
        Self {
            sign_context,
            house_context,
            house_modality,
            object_context,
            motion_context,
            angle_context: None,
            visibility_context,
        }
    }

    pub fn from_angle_position(
        sign_context: Option<SignContext>,
        house_context: Option<HouseContext>,
        house_modality: Option<HouseModalityContext>,
        object_context: Option<ObjectContext>,
        angle_context: Option<AngleContext>,
        visibility_context: Option<PositionVisibilityContext>,
    ) -> Self {
        Self {
            sign_context,
            house_context,
            house_modality,
            object_context,
            motion_context: None,
            angle_context,
            visibility_context,
        }
    }

    pub fn from_facts_json(facts_json: Option<&Value>) -> Option<Self> {
        let facts = facts_json?;
        let sign_context = facts
            .get("sign_context")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        let house_context = facts
            .get("house_context")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        let house_modality = facts
            .get("house_modality")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        let object_context = facts
            .get("object_context")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        let motion_context = facts
            .get("motion_context")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        let angle_context = facts
            .get("angle_context")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        let visibility_context = facts
            .get("visibility_context")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());

        if sign_context.is_none()
            && house_context.is_none()
            && house_modality.is_none()
            && object_context.is_none()
            && motion_context.is_none()
            && angle_context.is_none()
            && visibility_context.is_none()
        {
            None
        } else {
            Some(Self {
                sign_context,
                house_context,
                house_modality,
                object_context,
                motion_context,
                angle_context,
                visibility_context,
            })
        }
    }

    pub fn to_facts_json(&self) -> Value {
        let mut facts = serde_json::Map::new();
        facts.insert(
            "sign_context".to_string(),
            self.sign_context
                .as_ref()
                .map(sign_context_json)
                .unwrap_or(Value::Null),
        );
        facts.insert(
            "house_context".to_string(),
            self.house_context
                .as_ref()
                .map(house_context_json)
                .unwrap_or(Value::Null),
        );
        facts.insert(
            "house_modality".to_string(),
            self.house_modality
                .as_ref()
                .map(house_modality_json)
                .unwrap_or(Value::Null),
        );
        facts.insert(
            "object_context".to_string(),
            self.object_context
                .as_ref()
                .map(object_context_json)
                .unwrap_or(Value::Null),
        );
        facts.insert(
            "motion_context".to_string(),
            self.motion_context
                .as_ref()
                .map(motion_context_json)
                .unwrap_or(Value::Null),
        );
        if let Some(angle_context) = self.angle_context.as_ref() {
            facts.insert(
                "angle_context".to_string(),
                angle_context_json(angle_context),
            );
        }
        facts.insert(
            "visibility_context".to_string(),
            self.visibility_context
                .as_ref()
                .map(visibility_context_json)
                .unwrap_or(Value::Null),
        );
        Value::Object(facts)
    }

    #[cfg(feature = "swisseph-engine")]
    pub(crate) fn facts_json_for_calculated_position(
        sign: &SignReference,
        house: Option<&HouseReference>,
        object: &ChartObject,
        motion_state: Option<&MotionStateReference>,
        horizon_position_id: i32,
        horizon_position_code: &str,
        altitude_deg: f64,
    ) -> Option<Value> {
        Some(
            Self::from_calculated_position(
                Some(sign_context(sign)),
                house.map(house_context),
                house.and_then(house_modality),
                Some(object_context(object)),
                motion_state.map(motion_context),
                Some(PositionVisibilityContext {
                    horizon_position_id: Some(horizon_position_id),
                    horizon_position: Some(horizon_position_code.to_string()),
                    altitude_deg: Some(altitude_deg),
                    is_visible: Some(is_visible_horizon_position(horizon_position_code)),
                    source: Some("calculated_altitude".to_string()),
                }),
            )
            .to_facts_json(),
        )
    }

    #[cfg(feature = "swisseph-engine")]
    pub(crate) fn facts_json_for_angle_position(
        sign: &SignReference,
        house: &HouseReference,
        object: &ChartObject,
        angle: &AnglePointReference,
        house_cusp_longitude_deg: Option<f64>,
        horizon_position_id: i32,
        horizon_position_code: &str,
    ) -> Option<Value> {
        Some(
            Self::from_angle_position(
                Some(sign_context(sign)),
                Some(house_context(house)),
                house_modality(house),
                Some(object_context(object)),
                Some(AngleContext {
                    angle_point_code: Some(angle.code.clone()),
                    short_label: Some(angle.short_label.clone()),
                    full_name: Some(angle.full_name.clone()),
                    axis: Some(angle.axis.clone()),
                    opposite_angle_code: angle.opposite_angle_code.clone(),
                    associated_house_number: Some(angle.associated_house),
                    house_theme_code: Some(house.theme_code.clone()),
                    description: Some(angle.description.clone()),
                    chart_object_sort_order: Some(angle.chart_object_sort_order),
                    house_cusp_longitude_deg,
                }),
                Some(PositionVisibilityContext {
                    horizon_position_id: Some(horizon_position_id),
                    horizon_position: Some(horizon_position_code.to_string()),
                    altitude_deg: None,
                    is_visible: Some(is_visible_horizon_position(horizon_position_code)),
                    source: Some("angle_context".to_string()),
                }),
            )
            .to_facts_json(),
        )
    }
}

fn sign_context_json(context: &SignContext) -> Value {
    serde_json::json!({
        "element": context.element,
        "element_label": context.element_label,
        "modality": context.modality,
        "modality_label": context.modality_label,
        "polarity": context.polarity,
        "polarity_label": context.polarity_label,
        "keywords": context.keywords,
    })
}

fn house_context_json(context: &HouseContext) -> Value {
    serde_json::json!({
        "theme_code": context.theme_code,
    })
}

fn house_modality_json(context: &HouseModalityContext) -> Value {
    serde_json::json!({
        "code": context.code,
        "label": context.label,
        "accidental_strength": context.accidental_strength,
        "priority_delta": context.priority_delta,
        "interpretation_weight": context.interpretation_weight,
    })
}

fn object_context_json(context: &ObjectContext) -> Value {
    serde_json::json!({
        "role": context.role,
        "role_label": context.role_label,
        "nature": context.nature,
        "is_luminary": context.is_luminary,
        "is_planet_symbolic": context.is_planet_symbolic,
        "is_visible_to_naked_eye": context.is_visible_to_naked_eye,
        "signal_scoring": context.signal_scoring,
    })
}

fn motion_context_json(context: &MotionContext) -> Value {
    serde_json::json!({
        "motion_state": context.motion_state,
        "label": context.label,
        "motion_family": context.motion_family,
    })
}

fn angle_context_json(context: &AngleContext) -> Value {
    serde_json::json!({
        "angle_point_code": context.angle_point_code,
        "short_label": context.short_label,
        "full_name": context.full_name,
        "axis": context.axis,
        "opposite_angle_code": context.opposite_angle_code,
        "associated_house_number": context.associated_house_number,
        "house_theme_code": context.house_theme_code,
        "description": context.description,
        "chart_object_sort_order": context.chart_object_sort_order,
        "house_cusp_longitude_deg": context.house_cusp_longitude_deg,
    })
}

fn visibility_context_json(context: &PositionVisibilityContext) -> Value {
    serde_json::json!({
        "horizon_position_id": context.horizon_position_id,
        "horizon_position": context.horizon_position,
        "altitude_deg": context.altitude_deg,
        "is_visible": context.is_visible,
        "source": context.source,
    })
}

#[cfg(feature = "swisseph-engine")]
fn sign_context(sign: &SignReference) -> SignContext {
    SignContext {
        element: sign.element_code.clone(),
        element_label: sign.element_label.clone(),
        modality: sign.modality_code.clone(),
        modality_label: sign.modality_name.clone(),
        polarity: sign.polarity_code.clone(),
        polarity_label: sign.polarity_name.clone(),
        keywords: sign.keywords_json.clone(),
    }
}

#[cfg(feature = "swisseph-engine")]
fn house_modality(house: &HouseReference) -> Option<HouseModalityContext> {
    house
        .modality_code
        .as_ref()
        .map(|code| HouseModalityContext {
            code: Some(code.clone()),
            label: house.modality_label.clone(),
            accidental_strength: house
                .accidental_strength
                .as_deref()
                .and_then(|value| value.parse::<f64>().ok()),
            priority_delta: house.modality_priority_delta,
            interpretation_weight: house
                .interpretation_weight
                .as_deref()
                .and_then(|value| value.parse::<f64>().ok()),
        })
}

#[cfg(feature = "swisseph-engine")]
fn house_context(house: &HouseReference) -> HouseContext {
    HouseContext {
        theme_code: Some(house.theme_code.clone()),
    }
}

#[cfg(feature = "swisseph-engine")]
fn object_context(object: &ChartObject) -> ObjectContext {
    ObjectContext {
        role: object.role_code.clone(),
        role_label: object.role_label.clone(),
        nature: object.nature_codes.clone(),
        is_luminary: object.is_luminary,
        is_planet_symbolic: object.is_planet_symbolic,
        is_visible_to_naked_eye: object.is_visible_to_naked_eye,
        signal_scoring: Some(serde_json::json!({
            "position_priority_base": object.position_priority_base,
            "angle_priority_base": object.angle_priority_base,
            "source_weight": object.source_weight
        })),
    }
}

#[cfg(feature = "swisseph-engine")]
fn motion_context(motion_state: &MotionStateReference) -> MotionContext {
    MotionContext {
        motion_state: Some(motion_state.code.clone()),
        label: Some(motion_state.label.clone()),
        motion_family: Some(motion_state.motion_family.clone()),
    }
}

#[cfg(feature = "swisseph-engine")]
fn is_visible_horizon_position(horizon_position_code: &str) -> bool {
    matches!(horizon_position_code, "above_horizon" | "on_horizon")
}

#[derive(Debug, Clone)]
/// Structure HouseCuspFact.
pub struct HouseCuspFact {
    pub house_id: i32,
    pub house_number: i32,
    pub sign_id: i32,
    pub longitude_deg: f64,
}

#[derive(Debug, Clone)]
/// Structure AspectFact.
pub struct AspectFact {
    pub source_chart_object_id: i32,
    pub source_object_code: String,
    pub source_object_name: String,
    pub target_chart_object_id: i32,
    pub target_object_code: String,
    pub target_object_name: String,
    pub aspect_id: i32,
    pub aspect_code: String,
    pub aspect_name: String,
    pub aspect_family: String,
    pub orb_deg: f64,
    pub phase_state: String,
    pub is_applying: bool,
    pub is_exact: bool,
    pub strength_score: Option<f64>,
    pub primary_valence: Option<String>,
    pub intensity_modifier: Option<String>,
    pub secondary_effect: Option<String>,
    pub valence_family: Option<String>,
    pub valence_is_tonal: Option<bool>,
    pub valence_is_intensity_modifier: Option<bool>,
    pub calculation_notes_json: Option<Value>,
}

#[derive(Debug, Clone)]
/// Structure CalculatedChartFacts.
pub struct CalculatedChartFacts {
    pub positions: Vec<ObjectPositionFact>,
    pub house_cusps: Vec<HouseCuspFact>,
    pub aspects: Vec<AspectFact>,
}

#[derive(Debug, Clone)]
/// Structure InterpretationSignalDraft.
pub struct InterpretationSignalDraft {
    pub signal_key: String,
    pub signal_type_id: Option<i32>,
    pub theme_code: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub priority_score: f64,
    pub confidence_score: Option<f64>,
    pub suppression_state: String,
    pub payload_json: Option<Value>,
}
