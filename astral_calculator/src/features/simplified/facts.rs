//! Module astral_calculator\src\features\simplified\facts.rs du moteur astral_calculator.

use std::path::Path;

use chrono::{DateTime, Utc};

use super::catalog::SimplifiedCatalog;
use super::ephemeris_calc::{
    dedupe_preserve_order, distance_to_sign_boundary_deg, julian_day_utc, sign_code_at_jd,
};
use super::resolve::ResolvedSimplifiedInput;
use super::response::{AmbiguousSignFactResponse, CuspWarningResponse, SignFactResponse};
use super::uncertainty_window::build_sampling_schedule;
use crate::domain::{ChartObject, SignReference};
use crate::shared::error::RuntimeError;

pub const RELIABILITY_STABLE: &str = "stable_across_uncertainty_window";
pub const RELIABILITY_AMBIGUOUS: &str = "ambiguous_across_uncertainty_window";
pub const RELIABILITY_DECLARED: &str = "calculated_from_declared_datetime";

#[derive(Debug, Clone)]
/// Structure CollectedSignFacts.
pub struct CollectedSignFacts {
    pub facts: Vec<SignFactResponse>,
    pub ambiguous_facts: Vec<AmbiguousSignFactResponse>,
    pub cusp_warnings: Vec<CuspWarningResponse>,
}

/// Fonction collect_window_sign_facts.
pub fn collect_window_sign_facts(
    ephemeris_path: &Path,
    _resolved: &ResolvedSimplifiedInput,
    catalog: &SimplifiedCatalog,
    chart_objects: &[ChartObject],
    signs: &[SignReference],
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Result<CollectedSignFacts, RuntimeError> {
    let samples = build_sampling_schedule(window_start, window_end, catalog);
    let planets: Vec<_> = chart_objects
        .iter()
        .filter(|object| object.swe_id.is_some())
        .collect();

    let mut facts = Vec::new();
    let mut ambiguous_facts = Vec::new();
    let mut cusp_warnings = Vec::new();

    for object in planets {
        let swe_id = object.swe_id.unwrap();
        let mut observed_signs = Vec::new();
        let mut last_longitude = None;

        for sample in &samples {
            let jd = julian_day_utc(*sample);
            let (sign_code, longitude) = sign_code_at_jd(ephemeris_path, jd, swe_id, signs)?;
            observed_signs.push(sign_code);
            last_longitude = Some(longitude);
        }

        let unique = dedupe_preserve_order(&observed_signs);
        if unique.len() == 1 {
            facts.push(SignFactResponse {
                object_code: object.code.clone(),
                fact_type: "sign".to_string(),
                sign_code: unique[0].clone(),
                reliability: RELIABILITY_STABLE.to_string(),
                longitude_deg: last_longitude,
            });
        } else {
            ambiguous_facts.push(AmbiguousSignFactResponse {
                object_code: object.code.clone(),
                fact_type: "sign".to_string(),
                possible_sign_codes: unique,
                reliability: RELIABILITY_AMBIGUOUS.to_string(),
            });
        }

        if let Some(longitude) = last_longitude {
            let orb = distance_to_sign_boundary_deg(longitude);
            if orb <= catalog.policy.cusp_warning_orb_deg {
                cusp_warnings.push(CuspWarningResponse {
                    object_code: object.code.clone(),
                    message_code: "near_zodiac_boundary".to_string(),
                    orb_deg: orb,
                });
            }
        }
    }

    Ok(CollectedSignFacts {
        facts,
        ambiguous_facts,
        cusp_warnings,
    })
}

/// Fonction collect_declared_sign_facts.
pub fn collect_declared_sign_facts(
    ephemeris_path: &Path,
    instant: DateTime<Utc>,
    chart_objects: &[ChartObject],
    signs: &[SignReference],
    catalog: &SimplifiedCatalog,
) -> Result<CollectedSignFacts, RuntimeError> {
    let jd = julian_day_utc(instant);
    let mut facts = Vec::new();
    let mut cusp_warnings = Vec::new();

    for object in chart_objects.iter().filter(|o| o.swe_id.is_some()) {
        let (sign_code, longitude) =
            sign_code_at_jd(ephemeris_path, jd, object.swe_id.unwrap(), signs)?;
        facts.push(SignFactResponse {
            object_code: object.code.clone(),
            fact_type: "sign".to_string(),
            sign_code,
            reliability: RELIABILITY_DECLARED.to_string(),
            longitude_deg: Some(longitude),
        });

        let orb = distance_to_sign_boundary_deg(longitude);
        if orb <= catalog.policy.cusp_warning_orb_deg {
            cusp_warnings.push(CuspWarningResponse {
                object_code: object.code.clone(),
                message_code: "near_zodiac_boundary".to_string(),
                orb_deg: orb,
            });
        }
    }

    Ok(CollectedSignFacts {
        facts,
        ambiguous_facts: Vec::new(),
        cusp_warnings,
    })
}
