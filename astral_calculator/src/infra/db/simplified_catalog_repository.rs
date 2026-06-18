//! Module astral_calculator\src\infra\db\simplified_catalog_repository.rs du moteur astral_calculator.

use sqlx::PgPool;

use async_trait::async_trait;

use crate::application::ports::SimplifiedCatalogStore;
use crate::features::simplified::catalog::{
    CalculationScope, InputPrecisionLevel, LimitationCode, ReliabilityLevel, SimplifiedCatalog,
    SimplifiedPolicy,
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
    ) -> Result<Vec<crate::features::simplified::catalog::ProfileFeatureExclusion>, RuntimeError>
    {
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
    let policy = sqlx::query_as::<_, SimplifiedPolicy>(
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
    })?;

    let limitation_codes = sqlx::query_as::<_, LimitationCode>(
        r#"
        SELECT code, severity, affected_features_json
        FROM astral_simplified_limitation_codes
        WHERE is_active = true
        ORDER BY sort_order, id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_catalog_db_error)?;

    let reliability_levels = sqlx::query_as::<_, ReliabilityLevel>(
        r#"
        SELECT code, allows_interpretive_affirmation
        FROM astral_fact_reliability_levels
        WHERE is_active = true
        ORDER BY sort_order, id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_catalog_db_error)?;

    let calculation_scopes = sqlx::query_as::<_, CalculationScope>(
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
    .map_err(map_catalog_db_error)?;

    let input_precision_levels = sqlx::query_as::<_, InputPrecisionLevel>(
        r#"
        SELECT code
        FROM astral_birth_input_precision_levels
        WHERE is_active = true
        ORDER BY sort_order, id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_catalog_db_error)?;

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
) -> Result<Vec<crate::features::simplified::catalog::ProfileFeatureExclusion>, RuntimeError> {
    sqlx::query_as::<_, crate::features::simplified::catalog::ProfileFeatureExclusion>(
        r#"
        SELECT profile_code, computed_scope_code, feature_code, exclusion_kind, sort_order
        FROM astral_simplified_profile_feature_exclusions
        WHERE is_active = true
        ORDER BY sort_order, id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_catalog_db_error)
}
