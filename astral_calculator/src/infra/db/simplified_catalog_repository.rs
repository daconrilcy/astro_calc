//! Module astral_calculator\src\infra\db\simplified_catalog_repository.rs du moteur astral_calculator.

use sqlx::PgPool;

use async_trait::async_trait;

use crate::application::ports::SimplifiedCatalogStore;
use crate::domain::{
    CalculationScope, InputPrecisionLevel, LimitationCode, ProfileFeatureExclusion,
    ReliabilityLevel, SimplifiedCatalog, SimplifiedPolicy,
};
use crate::infra::db::models::{
    CalculationScopeRow, InputPrecisionLevelRow, LimitationCodeRow, ProfileFeatureExclusionRow,
    ReliabilityLevelRow, SimplifiedPolicyRow,
};
use crate::shared::error::RuntimeError;

#[derive(Clone)]
/// Repository SQL du catalogue natal simplifié.
pub struct SimplifiedCatalogRepository {
    pool: PgPool,
}

impl SimplifiedCatalogRepository {
    /// Fonction new.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SimplifiedCatalogStore for SimplifiedCatalogRepository {
    async fn simplified_catalog(&self) -> Result<SimplifiedCatalog, RuntimeError> {
        load_simplified_catalog(&self.pool).await
    }

    async fn profile_feature_exclusions(
        &self,
    ) -> Result<Vec<ProfileFeatureExclusion>, RuntimeError> {
        load_profile_feature_exclusions(&self.pool).await
    }
}

/// Fonction map_catalog_db_error.
fn map_catalog_db_error(err: sqlx::Error) -> RuntimeError {
    let msg = err.to_string();
    if msg.contains("does not exist") {
        RuntimeError::InvalidRuntimeTable(format!(
            "simplified natal catalog tables missing in database â€” run: python scripts/import_json_db_to_postgres.py ({msg})"
        ))
    } else {
        RuntimeError::Database(err)
    }
}

/// Fonction load_simplified_catalog.
pub async fn load_simplified_catalog(pool: &PgPool) -> Result<SimplifiedCatalog, RuntimeError> {
    let policy = sqlx::query_as::<_, SimplifiedPolicyRow>(
        r#"
        SELECT code, reference_time_utc, date_only_uncertainty_mode,
               uncertainty_sampling_minutes, default_timezone_strategy,
               cusp_warning_orb_deg::float8 AS cusp_warning_orb_deg, stable_fact_strategy
        FROM astral_simplified_calculation_policies
        WHERE is_active = true
        ORDER BY id
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(map_catalog_db_error)?
    .ok_or_else(|| {
        RuntimeError::Ephemeris("missing active astral_simplified_calculation_policies".into())
    })
    .map(map_simplified_policy_row)?;

    let limitation_codes = sqlx::query_as::<_, LimitationCodeRow>(
        r#"
        SELECT code, severity, affected_features_json
        FROM astral_simplified_limitation_codes
        WHERE is_active = true
        ORDER BY sort_order, id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_catalog_db_error)?
    .into_iter()
    .map(map_limitation_code_row)
    .collect();

    let reliability_levels = sqlx::query_as::<_, ReliabilityLevelRow>(
        r#"
        SELECT code, allows_interpretive_affirmation
        FROM astral_fact_reliability_levels
        WHERE is_active = true
        ORDER BY sort_order, id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_catalog_db_error)?
    .into_iter()
    .map(map_reliability_level_row)
    .collect();

    let calculation_scopes = sqlx::query_as::<_, CalculationScopeRow>(
        r#"
        SELECT code, min_input_precision_code, supports_angles, supports_houses,
               supports_aspects, supports_object_sign_facts, supports_ambiguous_facts
        FROM astral_calculation_scopes
        WHERE is_active = true
        ORDER BY sort_order, id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_catalog_db_error)?
    .into_iter()
    .map(map_calculation_scope_row)
    .collect();

    let input_precision_levels = sqlx::query_as::<_, InputPrecisionLevelRow>(
        r#"
        SELECT code
        FROM astral_birth_input_precision_levels
        WHERE is_active = true
        ORDER BY sort_order, id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_catalog_db_error)?
    .into_iter()
    .map(map_input_precision_level_row)
    .collect();

    Ok(SimplifiedCatalog {
        policy,
        limitation_codes,
        reliability_levels,
        calculation_scopes,
        input_precision_levels,
    })
}

/// Fonction load_profile_feature_exclusions.
pub async fn load_profile_feature_exclusions(
    pool: &PgPool,
) -> Result<Vec<ProfileFeatureExclusion>, RuntimeError> {
    Ok(sqlx::query_as::<_, ProfileFeatureExclusionRow>(
        r#"
        SELECT profile_code, computed_scope_code, feature_code, exclusion_kind, sort_order
        FROM astral_simplified_profile_feature_exclusions
        WHERE is_active = true
        ORDER BY sort_order, id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_catalog_db_error)?
    .into_iter()
    .map(map_profile_feature_exclusion_row)
    .collect())
}

fn map_simplified_policy_row(row: SimplifiedPolicyRow) -> SimplifiedPolicy {
    SimplifiedPolicy {
        code: row.code,
        reference_time_utc: row.reference_time_utc,
        date_only_uncertainty_mode: row.date_only_uncertainty_mode,
        uncertainty_sampling_minutes: row.uncertainty_sampling_minutes,
        default_timezone_strategy: row.default_timezone_strategy,
        cusp_warning_orb_deg: row.cusp_warning_orb_deg,
        stable_fact_strategy: row.stable_fact_strategy,
    }
}

fn map_limitation_code_row(row: LimitationCodeRow) -> LimitationCode {
    LimitationCode {
        code: row.code,
        severity: row.severity,
        affected_features_json: row.affected_features_json,
    }
}

fn map_reliability_level_row(row: ReliabilityLevelRow) -> ReliabilityLevel {
    ReliabilityLevel {
        code: row.code,
        allows_interpretive_affirmation: row.allows_interpretive_affirmation,
    }
}

fn map_calculation_scope_row(row: CalculationScopeRow) -> CalculationScope {
    CalculationScope {
        code: row.code,
        min_input_precision_code: row.min_input_precision_code,
        supports_angles: row.supports_angles,
        supports_houses: row.supports_houses,
        supports_aspects: row.supports_aspects,
        supports_object_sign_facts: row.supports_object_sign_facts,
        supports_ambiguous_facts: row.supports_ambiguous_facts,
    }
}

fn map_input_precision_level_row(row: InputPrecisionLevelRow) -> InputPrecisionLevel {
    InputPrecisionLevel { code: row.code }
}

fn map_profile_feature_exclusion_row(row: ProfileFeatureExclusionRow) -> ProfileFeatureExclusion {
    ProfileFeatureExclusion {
        profile_code: row.profile_code,
        computed_scope_code: row.computed_scope_code,
        feature_code: row.feature_code,
        exclusion_kind: row.exclusion_kind,
        sort_order: row.sort_order,
    }
}
