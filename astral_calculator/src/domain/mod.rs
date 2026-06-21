//! Module astral_calculator\src\domain\mod.rs du moteur astral_calculator.

mod catalogs;
mod chart_facts;
/// Module natal_input.
mod natal_input;
/// Module payload.
mod payload;
/// Module references.
mod references;
/// Module scoring.
mod scoring;

pub use catalogs::{
    accidental_polarity_bands_are_valid, overall_polarity_for_score_with_bands,
    BasicPayloadCatalog, CalculationScope, EssentialDignityScoringWeight,
    HoroscopeSignalThemeMapping, HoroscopeSupportedObject, InputPrecisionLevel, LimitationCode,
    ProfileFeatureExclusion, ReliabilityLevel, SimplifiedCatalog, SimplifiedPolicy,
};
pub use chart_facts::{
    AngleContext, AspectFact, CalculatedChartFacts, HouseContext, HouseCuspFact,
    HouseModalityContext, InterpretationSignalDraft, MotionContext, ObjectContext,
    ObjectPositionFact, PositionFactContext, PositionVisibilityContext, SignContext,
};
pub use natal_input::{NatalChartInput, RuntimeOptions};
pub use payload::{
    BasicAccidentalDignityCondition, BasicAccidentalDignityContextSummary,
    BasicAccidentalDignityEvaluation, BasicAngleFact, BasicCalculationReliability,
    BasicChartContext, BasicChartEmphasis, BasicDignity, BasicDispositorLink, BasicDominantHouse,
    BasicDominantObject, BasicDominantSign, BasicFinalDispositor, BasicHemisphereEmphasis,
    BasicHouseAxisEmphasis, BasicHouseAxisScore, BasicLunarPhaseContext, BasicMutualReception,
    BasicObjectPosition, BasicPayload, BasicPayloadContract, BasicProjectionReason,
    BasicReadingPlanItem, BasicRulerContext, BasicRulerSource, BasicRulershipChain,
    BasicRulershipContext, BasicSecondarySlotCandidate, BasicSectContext, BasicSignal,
};
pub use references::{
    AccidentalConditionTrigger, AccidentalDignityConditionReference, AccidentalPolarityBand,
    AccidentalScoringParams, AnglePointReference, AspectDefinition, BasicProductScoringProfile,
    CalculationReferenceData, ChartObject, DomicileRulerReference, EssentialDignityRuleReference,
    HorizonPositionReference, HouseAxisReference, HouseReference, HouseSystem,
    InterpretationSignalRow, LunarPhaseReference, MotionStateReference,
    ObjectSectAffinityReference, ProjectionLabelDefinition, ProjectionReasonDefinition,
    SignReference,
};
pub use scoring::{BasicAccidentalScoringSnapshot, BasicProductScoringSnapshot};
