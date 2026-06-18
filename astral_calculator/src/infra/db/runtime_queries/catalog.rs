//! Requetes SQL runtime specialisees.

use super::*;

impl RuntimeQueries {
    pub async fn basic_payload_catalog(
        &self,
        product_code: &str,
        payload_contract_version: &str,
        reference_version_id: i32,
    ) -> Result<BasicPayloadCatalog, RuntimeError> {
        let product_scoring = self
            .basic_product_scoring_profile(product_code, payload_contract_version)
            .await?;
        let essential_dignity_rules = self
            .essential_dignity_rule_references(
                reference_version_id,
                product_scoring.essential_dignity_score_profile_id,
            )
            .await?;
        let accidental_triggers = self.accidental_condition_triggers().await?;
        let accidental_scoring = self
            .accidental_scoring_params(product_scoring.accidental_scoring_params_id)
            .await?;
        let accidental_polarity_bands = self
            .accidental_overall_polarity_bands(product_scoring.accidental_scoring_params_id)
            .await?;
        Ok(BasicPayloadCatalog::build(
            product_scoring,
            essential_dignity_rules,
            accidental_triggers,
            accidental_scoring,
            accidental_polarity_bands,
        ))
    }

    /// Fonction house_system.
    pub async fn house_system(&self, id: i32) -> Result<HouseSystem, RuntimeError> {
        Ok(sqlx::query_as::<_, HouseSystem>(
            r#"
            SELECT id, code, name, calculation_engine_code
            FROM astral_house_systems
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Fonction house_systems.
    pub async fn house_systems(&self) -> Result<Vec<HouseSystem>, RuntimeError> {
        Ok(sqlx::query_as::<_, HouseSystem>(
            r#"
            SELECT id, code, name, calculation_engine_code
            FROM astral_house_systems
            WHERE is_active = true
            ORDER BY id
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }
}
