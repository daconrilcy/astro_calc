//! Requetes SQL runtime specialisees.

use super::*;

impl RuntimeQueries {
    pub async fn default_reference_version_id(&self) -> Result<i32, RuntimeError> {
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM astral_reference_versions
            WHERE status IN ('published', 'draft')
            ORDER BY CASE status WHEN 'published' THEN 0 ELSE 1 END, id ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        id.ok_or_else(|| {
            RuntimeError::InvalidRuntimeTable("no active astral_reference_versions row".to_string())
        })
    }

    /// Fonction llm_projection_profile.
    pub async fn llm_projection_profile(
        &self,
        contract_version: &str,
        level_code: &str,
    ) -> Result<crate::engine::projection::LlmProjectionProfile, RuntimeError> {
        let row = sqlx::query_as::<_, LlmProjectionProfileRow>(
            r#"
            SELECT id,
                   contract_version,
                   level_code,
                   max_keywords_per_item,
                   max_core_placements,
                   max_supporting_placements,
                   max_dominant_signs,
                   max_dominant_houses,
                   max_dominant_objects,
                   max_house_axes,
                   max_aspects,
                   include_accidental_conditions,
                   include_rulership_details,
                   include_minor_evidence,
                   include_degrees,
                   include_scores
            FROM astral_llm_projection_profiles
            WHERE contract_version = $1
              AND level_code = $2
              AND is_active = true
            "#,
        )
        .bind(contract_version)
        .bind(level_code)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Into::into).ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(format!(
                "unknown llm projection profile: {contract_version}/{level_code}"
            ))
        })
    }

    /// Fonction language_id_for_code.
    pub async fn language_id_for_code(&self, code: &str) -> Result<i32, RuntimeError> {
        Ok(sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM languages
            WHERE code = $1
            "#,
        )
        .bind(code)
        .fetch_one(&self.pool)
        .await?)
    }
}
