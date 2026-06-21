//! Module astral_calculator\src\features\simplified\service.rs du moteur astral_calculator.

use std::path::Path;

use super::catalog::SimplifiedCatalog;
use super::facts::{collect_declared_sign_facts, collect_window_sign_facts, CollectedSignFacts};
use super::payload::build_response;
use super::request::AstroSimplifiedNatalRequest;
use super::resolve::{
    build_uncertainty_window, declared_datetime_utc, validate_and_resolve, ANGULAR_SCOPE,
    PLANETARY_SCOPE,
};
use super::response::{AstroSimplifiedNatalResponse, RECOMMENDED_SIMPLIFIED_PROFILE_CODE};
use crate::application::chart_context::load_default_chart_context;
use crate::application::ports::{
    NatalReferenceStore, ReferenceSystemResolver, ReferenceVersionProvider, SimplifiedCatalogStore,
};
use crate::astrology::aspects::detect_aspects;
use crate::astrology::ephemeris::EphemerisEngine;
use crate::astrology::validation::validate_calculation_references;
use crate::domain::{AspectDefinition, ChartObject};
use crate::domain::{CalculatedChartFacts, NatalChartInput, ObjectPositionFact};
use crate::shared::error::RuntimeError;

/// Fonction calculate_simplified_natal.
pub async fn calculate_simplified_natal<R, S, E>(
    repository: &R,
    simplified_catalogs: &S,
    ephemeris: &E,
    ephemeris_path: &Path,
    request: AstroSimplifiedNatalRequest,
) -> Result<AstroSimplifiedNatalResponse, RuntimeError>
where
    R: ReferenceSystemResolver + ReferenceVersionProvider + NatalReferenceStore,
    S: SimplifiedCatalogStore,
    E: EphemerisEngine,
{
    if request.birth.time.is_some() && request.birth.timezone.is_none() {
        return Err(RuntimeError::InvalidEngineRequest(
            "birth.time requires birth.timezone".into(),
        ));
    }

    let catalog = simplified_catalogs.simplified_catalog().await?;
    let profile_feature_exclusions = simplified_catalogs.profile_feature_exclusions().await?;
    let resolved = validate_and_resolve(&request, &catalog)?;
    if SimplifiedCatalog::profile_feature_exclusions_for(
        &profile_feature_exclusions,
        RECOMMENDED_SIMPLIFIED_PROFILE_CODE,
        &resolved.computed_scope,
    )
    .is_empty()
    {
        return Err(RuntimeError::InvalidRuntimeTable(
            "missing active astral_simplified_profile_feature_exclusions for natal_simplified"
                .into(),
        ));
    }

    let house_system_id = repository
        .house_system_id_by_code(&resolved.house_system_code)
        .await?;
    let chart_context = load_default_chart_context(repository, house_system_id).await?;
    let zodiacal_id = repository
        .zodiacal_reference_system_id_by_key(&resolved.zodiac_key)
        .await?;
    let coordinate_id = chart_context
        .references
        .geocentric_coordinate_reference_system_id;
    let reference_version_id = chart_context.reference_version_id;
    let chart_objects = chart_context.chart_objects;
    let aspect_definitions = chart_context.aspect_definitions;
    let house_system = chart_context.house_system;
    let references = chart_context.references;
    validate_calculation_references(&references)?;

    let collected = match resolved.computed_scope.as_str() {
        PLANETARY_SCOPE | ANGULAR_SCOPE => {
            let instant = declared_datetime_utc(&resolved)?.ok_or_else(|| {
                RuntimeError::InvalidEngineRequest("missing declared datetime".into())
            })?;
            collect_declared_sign_facts(
                ephemeris_path,
                instant,
                &chart_objects,
                &references.signs,
                &catalog,
            )?
        }
        _ => {
            let (start, end) = build_uncertainty_window(&resolved, &catalog)?;
            collect_window_sign_facts(
                ephemeris_path,
                &resolved,
                &catalog,
                &chart_objects,
                &references.signs,
                start,
                end,
            )?
        }
    };

    let angular_facts = match resolved.computed_scope.as_str() {
        ANGULAR_SCOPE => {
            let instant = required_declared_datetime(&resolved)?;
            let input = NatalChartInput {
                subject_label: None,
                birth_datetime_utc: instant,
                latitude_deg: required_latitude(&resolved)?,
                longitude_deg: required_longitude(&resolved)?,
                altitude_m: Some(0.0),
                reference_version_id,
                calculation_profile_id: None,
                zodiacal_reference_system_id: zodiacal_id,
                coordinate_reference_system_id: coordinate_id,
                house_system_id,
                product_code: Some("simplified".to_string()),
                client_idempotency_key: None,
            };
            Some(ephemeris.calculate_chart(
                &input,
                &chart_objects,
                &aspect_definitions,
                &house_system,
                &references,
            )?)
        }
        PLANETARY_SCOPE => Some(build_planetary_only_facts(
            &collected,
            &chart_objects,
            &aspect_definitions,
            zodiacal_id,
            coordinate_id,
        )),
        _ => None,
    };

    Ok(build_response(
        &resolved,
        &catalog,
        &profile_feature_exclusions,
        collected,
        angular_facts.as_ref(),
    ))
}

/// Fonction build_planetary_only_facts.
fn build_planetary_only_facts(
    collected: &CollectedSignFacts,
    chart_objects: &[ChartObject],
    aspect_definitions: &[AspectDefinition],
    zodiacal_reference_system_id: i32,
    coordinate_reference_system_id: i32,
) -> CalculatedChartFacts {
    let positions: Vec<ObjectPositionFact> = collected
        .facts
        .iter()
        .filter_map(|fact| {
            let longitude_deg = fact.longitude_deg?;
            let object = chart_objects.iter().find(|o| o.code == fact.object_code)?;
            Some(ObjectPositionFact {
                chart_object_id: object.id,
                object_code: fact.object_code.clone(),
                object_name: object.name.clone(),
                zodiacal_reference_system_id,
                coordinate_reference_system_id,
                sign_id: 0,
                sign_code: fact.sign_code.clone(),
                sign_name: fact.sign_code.clone(),
                house_id: None,
                house_number: None,
                house_name: None,
                motion_state_id: None,
                horizon_position_id: None,
                longitude_deg,
                latitude_deg: None,
                apparent_speed_deg_per_day: None,
                altitude_deg: None,
                is_visible: None,
                facts_json: None,
            })
        })
        .collect();

    CalculatedChartFacts {
        aspects: detect_aspects(&positions, aspect_definitions),
        positions,
        house_cusps: Vec::new(),
    }
}

fn required_declared_datetime(
    resolved: &super::resolve::ResolvedSimplifiedInput,
) -> Result<chrono::DateTime<chrono::Utc>, RuntimeError> {
    declared_datetime_utc(resolved)?
        .ok_or_else(|| RuntimeError::InvalidEngineRequest("missing declared datetime".into()))
}

fn required_latitude(
    resolved: &super::resolve::ResolvedSimplifiedInput,
) -> Result<f64, RuntimeError> {
    resolved
        .latitude
        .ok_or_else(|| RuntimeError::InvalidEngineRequest("missing latitude".into()))
}

fn required_longitude(
    resolved: &super::resolve::ResolvedSimplifiedInput,
) -> Result<f64, RuntimeError> {
    resolved
        .longitude
        .ok_or_else(|| RuntimeError::InvalidEngineRequest("missing longitude".into()))
}
