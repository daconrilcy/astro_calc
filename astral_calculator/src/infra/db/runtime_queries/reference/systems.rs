//! Requetes des systemes de reference astrologiques.

use super::*;

impl RuntimeQueries {
    /// Fonction house_system_id_by_code.
    pub async fn house_system_id_by_code(&self, code: &str) -> Result<i32, RuntimeError> {
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM astral_house_systems
            WHERE code = $1 AND is_active = true
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;
        id.ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(format!("unknown house_system: {code}"))
        })
    }

    /// Fonction zodiacal_reference_system_id_by_key.
    pub async fn zodiacal_reference_system_id_by_key(
        &self,
        key: &str,
    ) -> Result<i32, RuntimeError> {
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM astral_zodiacal_reference_systems
            WHERE key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        id.ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(format!("unknown zodiacal_reference_system: {key}"))
        })
    }

    /// Fonction zodiacal_reference_systems.
    pub async fn zodiacal_reference_systems(
        &self,
    ) -> Result<Vec<crate::infra::db::models::ZodiacalReferenceSystemRow>, RuntimeError> {
        Ok(
            sqlx::query_as::<_, crate::infra::db::models::ZodiacalReferenceSystemRow>(
                r#"
            SELECT id,
                   key,
                   display_name,
                   category_id,
                   description,
                   requires_ayanamsha,
                   usage_note
            FROM astral_zodiacal_reference_systems
            ORDER BY id
            "#,
            )
            .fetch_all(&self.pool)
            .await?,
        )
    }

    /// Fonction coordinate_reference_system_id_by_key.
    pub async fn coordinate_reference_system_id_by_key(
        &self,
        key: &str,
    ) -> Result<i32, RuntimeError> {
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM astral_coordinate_reference_systems
            WHERE key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        id.ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(format!(
                "unknown coordinate_reference_system: {key}"
            ))
        })
    }

    /// Fonction coordinate_reference_systems.
    pub async fn coordinate_reference_systems(
        &self,
    ) -> Result<Vec<crate::infra::db::models::CoordinateReferenceSystemRow>, RuntimeError> {
        Ok(
            sqlx::query_as::<_, crate::infra::db::models::CoordinateReferenceSystemRow>(
                r#"
            SELECT id,
                   key,
                   display_name,
                   category_id,
                   description,
                   usage_note
            FROM astral_coordinate_reference_systems
            ORDER BY id
            "#,
            )
            .fetch_all(&self.pool)
            .await?,
        )
    }

    /// Fonction zodiacal_reference_system_display_name.
    pub async fn zodiacal_reference_system_display_name(
        &self,
        id: i32,
    ) -> Result<String, RuntimeError> {
        let name = sqlx::query_scalar::<_, String>(
            r#"
            SELECT display_name
            FROM astral_zodiacal_reference_systems
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        name.ok_or_else(|| RuntimeError::InvalidRuntimeTable(format!("zodiac system {id} missing")))
    }

    /// Fonction coordinate_reference_system_display_name.
    pub async fn coordinate_reference_system_display_name(
        &self,
        id: i32,
    ) -> Result<String, RuntimeError> {
        let name = sqlx::query_scalar::<_, String>(
            r#"
            SELECT display_name
            FROM astral_coordinate_reference_systems
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        name.ok_or_else(|| {
            RuntimeError::InvalidRuntimeTable(format!("coordinate system {id} missing"))
        })
    }
}
