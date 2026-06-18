//! Traits applicatifs orientés usage.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    AccidentalDignityConditionReference, AspectDefinition, AspectFact, BasicPayload,
    CalculatedChartFacts, ChartObject, DomicileRulerReference, EssentialDignityRuleReference,
    HouseAxisReference, HouseSystem, InterpretationSignalDraft, InterpretationSignalRow,
    LunarPhaseReference, NatalChartInput, ObjectPositionFact, ObjectSectAffinityReference,
    ProjectionReasonDefinition, RuntimeOptions,
};
use crate::engine::projection::LlmProjectionProfile;
use crate::features::horoscope::{HoroscopeSignalThemeMapping, HoroscopeSupportedObject};
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::features::simplified::catalog::{ProfileFeatureExclusion, SimplifiedCatalog};
use crate::shared::error::RuntimeError;

#[derive(Debug, Clone)]
pub struct MajorAspectFamilyReference {
    pub expected_aspect_count: i32,
    pub max_default_orb_deg: f64,
}

#[derive(Debug, Clone)]
pub struct HoroscopeOrbWeightBand {
    pub max_orb_deg: f64,
}

/// Ligne minimale nécessaire aux règles d'idempotence applicatives.
#[derive(Debug, Clone)]
pub struct CalculationAttempt {
    pub id: i32,
    pub status: String,
    pub execution_attempt: i32,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub stale_after_seconds: Option<i32>,
}

#[async_trait]
pub trait ReferenceCatalog: Send + Sync {
    async fn default_reference_version_id(&self) -> Result<i32, RuntimeError>;
    async fn zodiacal_reference_system_id_by_key(&self, key: &str) -> Result<i32, RuntimeError>;
    async fn coordinate_reference_system_id_by_key(&self, key: &str) -> Result<i32, RuntimeError>;
    async fn house_system_id_by_code(&self, code: &str) -> Result<i32, RuntimeError>;
    async fn zodiacal_reference_system_display_name(&self, id: i32)
        -> Result<String, RuntimeError>;
    async fn coordinate_reference_system_display_name(
        &self,
        id: i32,
    ) -> Result<String, RuntimeError>;
    async fn house_system(&self, id: i32) -> Result<HouseSystem, RuntimeError>;
    async fn active_chart_objects(
        &self,
        reference_version_id: i32,
    ) -> Result<Vec<ChartObject>, RuntimeError>;
    async fn aspect_definitions(&self) -> Result<Vec<AspectDefinition>, RuntimeError>;
    async fn major_aspect_family_reference(
        &self,
    ) -> Result<MajorAspectFamilyReference, RuntimeError>;
    async fn sign_references(&self) -> Result<Vec<crate::domain::SignReference>, RuntimeError>;
    async fn house_references(&self) -> Result<Vec<crate::domain::HouseReference>, RuntimeError>;
    async fn motion_state_references(
        &self,
    ) -> Result<Vec<crate::domain::MotionStateReference>, RuntimeError>;
    async fn horizon_position_references(
        &self,
    ) -> Result<Vec<crate::domain::HorizonPositionReference>, RuntimeError>;
    async fn angle_point_references(
        &self,
    ) -> Result<Vec<crate::domain::AnglePointReference>, RuntimeError>;
    async fn domicile_ruler_references(
        &self,
        reference_version_id: i32,
    ) -> Result<Vec<DomicileRulerReference>, RuntimeError>;
    async fn house_axis_references(&self) -> Result<Vec<HouseAxisReference>, RuntimeError>;
    async fn lunar_phase_references(&self) -> Result<Vec<LunarPhaseReference>, RuntimeError>;
    async fn accidental_dignity_condition_references(
        &self,
    ) -> Result<Vec<AccidentalDignityConditionReference>, RuntimeError>;
    async fn object_sect_affinity_references(
        &self,
    ) -> Result<Vec<ObjectSectAffinityReference>, RuntimeError>;
    async fn language_id_for_code(&self, code: &str) -> Result<i32, RuntimeError>;
}

#[async_trait]
pub trait ProjectionCatalog: Send + Sync {
    async fn llm_projection_profile(
        &self,
        contract_version: &str,
        level: &str,
    ) -> Result<LlmProjectionProfile, RuntimeError>;
}

#[async_trait]
pub trait HoroscopeCatalog: Send + Sync {
    async fn horoscope_orb_weight_bands(&self)
        -> Result<Vec<HoroscopeOrbWeightBand>, RuntimeError>;
    async fn horoscope_supported_objects(
        &self,
    ) -> Result<Vec<HoroscopeSupportedObject>, RuntimeError>;
    async fn horoscope_signal_theme_mappings(
        &self,
    ) -> Result<Vec<HoroscopeSignalThemeMapping>, RuntimeError>;
}

#[async_trait]
pub trait PayloadCatalogStore: Send + Sync {
    async fn basic_payload_catalog(
        &self,
        product_code: &str,
        payload_contract_version: &str,
        reference_version_id: i32,
    ) -> Result<BasicPayloadCatalog, RuntimeError>;
    async fn basic_product_scoring_profile(
        &self,
        product_code: &str,
        payload_contract_version: &str,
    ) -> Result<crate::domain::BasicProductScoringProfile, RuntimeError>;
    async fn essential_dignity_rule_references(
        &self,
        reference_version_id: i32,
        score_profile_id: i32,
    ) -> Result<Vec<EssentialDignityRuleReference>, RuntimeError>;
    async fn projection_reason_definitions(
        &self,
    ) -> Result<Vec<ProjectionReasonDefinition>, RuntimeError>;
}

#[async_trait]
pub trait SimplifiedCatalogStore: Send + Sync {
    async fn simplified_catalog(&self) -> Result<SimplifiedCatalog, RuntimeError>;
    async fn profile_feature_exclusions(
        &self,
    ) -> Result<Vec<ProfileFeatureExclusion>, RuntimeError>;
}

#[async_trait]
pub trait NatalCalculationStore: Send + Sync {
    type Tx: Send;

    async fn begin(&self) -> Result<Self::Tx, RuntimeError>;
    async fn commit(&self, tx: Self::Tx) -> Result<(), RuntimeError>;
    async fn existing_basic_payload(
        &self,
        chart_calculation_id: i32,
        product_code: &str,
        language_id: Option<i32>,
    ) -> Result<Option<BasicPayload>, RuntimeError>;
    async fn positions_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<ObjectPositionFact>, RuntimeError>;
    async fn aspects_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError>;
    async fn natal_input_for_calculation(
        &self,
        chart_calculation_id: i32,
    ) -> Result<NatalChartInput, RuntimeError>;
    async fn lock_idempotency(&self, tx: &mut Self::Tx, lock_key: i64) -> Result<(), RuntimeError>;
    async fn calculations_for_key(
        &self,
        tx: &mut Self::Tx,
        idempotency_key: &str,
    ) -> Result<Vec<CalculationAttempt>, RuntimeError>;
    async fn persist_signals(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
        reference_version_id: i32,
        signals: &[InterpretationSignalDraft],
    ) -> Result<Vec<InterpretationSignalRow>, RuntimeError>;
    async fn persist_basic_payload(
        &self,
        tx: &mut Self::Tx,
        input: &NatalChartInput,
        payload_language_id: Option<i32>,
        payload: &BasicPayload,
    ) -> Result<(), RuntimeError>;
    async fn mark_stale_failed(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError>;
    async fn insert_running_calculation(
        &self,
        tx: &mut Self::Tx,
        input: &NatalChartInput,
        options: &RuntimeOptions,
        input_hash: &str,
        idempotency_key: &str,
        next_attempt: i32,
    ) -> Result<i32, RuntimeError>;
    async fn heartbeat(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
        progress_state: &str,
    ) -> Result<(), RuntimeError>;
    async fn mark_failed(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
        error: &RuntimeError,
    ) -> Result<(), RuntimeError>;
    async fn persist_facts(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
        facts: &CalculatedChartFacts,
    ) -> Result<(), RuntimeError>;
    async fn aspects_for_payload_in_tx(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError>;
    async fn mark_completed(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError>;
}
