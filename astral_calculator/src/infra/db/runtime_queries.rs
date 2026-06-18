//! Module astral_calculator\src\infra\db\runtime_queries.rs du moteur astral_calculator.

use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domain::{
    AccidentalConditionTrigger, AccidentalPolarityBand, AccidentalScoringParams,
    BasicProductScoringProfile, EssentialDignityRuleReference,
};
use crate::domain::{
    AspectFact, BasicPayload, CalculatedChartFacts, HouseCuspFact, InterpretationSignalDraft,
    NatalChartInput, ObjectPositionFact, RuntimeOptions,
};
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::infra::db::models::{
    AccidentalConditionTriggerRow, AccidentalDignityConditionReferenceRow,
    AccidentalPolarityBandRow, AccidentalScoringParamsRow, AnglePointReference, AspectDefinition,
    AstralTimePeriodProfileRow, BasicProductScoringProfileRow, ChartCalculationRow, ChartObject,
    DomicileRulerReference, EssentialDignityRuleReferenceRow, HorizonPositionReference,
    HoroscopeOrbWeightBandRow, HoroscopeScanProfileRow, HoroscopeServiceRow,
    HoroscopeSignalThemeMappingRow, HoroscopeSupportedObjectRow, HoroscopeTimeSlotProfileRow,
    HouseAxisReferenceRow, HouseReference, HouseSystem, InterpretationSignalRow,
    LlmProjectionProfileRow, LunarPhaseReferenceRow, MajorAspectFamilyReference,
    MotionStateReference, ObjectSectAffinityReferenceRow, PersistedAspectFact,
    PersistedObjectPositionFact, SignReference,
};
use crate::infra::db::runtime_repository::parse_existing_basic_payload_value;
use crate::shared::error::RuntimeError;

#[derive(Clone)]
/// Structure RuntimeQueries.
pub struct RuntimeQueries {
    pool: PgPool,
}

impl RuntimeQueries {
    /// Fonction new.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Fonction pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

mod calculation;
mod catalog;
mod horoscope;
mod mappers;
mod projection;
mod reference;
