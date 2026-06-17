use std::path::Path;

use super::catalog::SimplifiedCatalog;
use super::facts::{collect_declared_sign_facts, collect_window_sign_facts, CollectedSignFacts};
use super::payload::build_response;
use super::repository::{load_profile_feature_exclusions, load_simplified_catalog};
use super::request::AstroSimplifiedNatalRequest;
use super::resolve::{build_uncertainty_window, declared_datetime_utc, validate_and_resolve};
use super::response::{AstroSimplifiedNatalResponse, RECOMMENDED_SIMPLIFIED_PROFILE_CODE};
use crate::domain::{AspectDefinition, ChartObject};
use crate::natal::aspects::detect_aspects;
use crate::domain::{CalculatedChartFacts, NatalChartInput, ObjectPositionFact};
use crate::natal::ephemeris::EphemerisEngine;
use crate::infra::db::reference_repository::ReferenceRepository;
use crate::runtime::validate_calculation_references;
use crate::shared::error::RuntimeError;

pub async fn calculate_simplified_natal<E: EphemerisEngine>(
    repository: &ReferenceRepository,
    ephemeris: &E,
    ephemeris_path: &Path,
    request: AstroSimplifiedNatalRequest,
) -> Result<AstroSimplifiedNatalResponse, RuntimeError> {
    if request.birth.time.is_some() && request.birth.timezone.is_none() {
        return Err(RuntimeError::InvalidEngineRequest(
            "birth.time requires birth.timezone".into(),
        ));
    }

    let catalog = load_simplified_catalog(repository.pool()).await?;
    let profile_feature_exclusions = load_profile_feature_exclusions(repository.pool()).await?;
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

    let reference_version_id = repository.default_reference_version_id().await?;
    let zodiacal_id = repository
        .zodiacal_reference_system_id_by_key(&resolved.zodiac_key)
        .await?;
    let coordinate_id = repository
        .coordinate_reference_system_id_by_key("geocentric")
        .await?;
    let house_system_id = repository
        .house_system_id_by_code(&resolved.house_system_code)
        .await?;

    let chart_objects = repository
        .active_chart_objects(reference_version_id)
        .await?;
    let signs = repository.sign_references().await?;
    let houses = repository.house_references().await?;
    let motion_states = repository.motion_state_references().await?;
    let horizon_positions = repository.horizon_position_references().await?;
    let angle_points = repository.angle_point_references().await?;
    validate_calculation_references(&crate::domain::CalculationReferenceData {
        signs: signs.clone(),
        houses: houses.clone(),
        motion_states: motion_states.clone(),
        horizon_positions: horizon_positions.clone(),
        angle_points: angle_points.clone(),
    })?;

    let aspect_definitions = repository.aspect_definitions().await?;

    let collected = match resolved.computed_scope.as_str() {
        "planetary_positions" | "angular_chart" => {
            let instant = declared_datetime_utc(&resolved)?.ok_or_else(|| {
                RuntimeError::InvalidEngineRequest("missing declared datetime".into())
            })?;
            collect_declared_sign_facts(ephemeris_path, instant, &chart_objects, &signs, &catalog)?
        }
        _ => {
            let (start, end) = build_uncertainty_window(&resolved, &catalog)?;
            collect_window_sign_facts(
                ephemeris_path,
                &resolved,
                &catalog,
                &chart_objects,
                &signs,
                start,
                end,
            )?
        }
    };

    let angular_facts = match resolved.computed_scope.as_str() {
        "angular_chart" => {
            let instant = declared_datetime_utc(&resolved)?.unwrap();
            let input = NatalChartInput {
                subject_label: None,
                birth_datetime_utc: instant,
                latitude_deg: resolved.latitude.unwrap(),
                longitude_deg: resolved.longitude.unwrap(),
                altitude_m: Some(0.0),
                reference_version_id,
                calculation_profile_id: None,
                zodiacal_reference_system_id: zodiacal_id,
                coordinate_reference_system_id: coordinate_id,
                house_system_id,
                product_code: Some("simplified".to_string()),
                client_idempotency_key: None,
            };
            let house_system = repository.house_system(house_system_id).await?;
            let references = crate::domain::CalculationReferenceData {
                signs,
                houses,
                motion_states,
                horizon_positions,
                angle_points,
            };
            Some(ephemeris.calculate_natal(
                &input,
                &chart_objects,
                &aspect_definitions,
                &house_system,
                &references,
            )?)
        }
        "planetary_positions" => Some(build_planetary_only_facts(
            &collected,
            &chart_objects,
            &aspect_definitions,
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

fn build_planetary_only_facts(
    collected: &CollectedSignFacts,
    chart_objects: &[ChartObject],
    aspect_definitions: &[AspectDefinition],
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
                zodiacal_reference_system_id: 1,
                coordinate_reference_system_id: 1,
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
