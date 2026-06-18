//! Requetes SQL runtime specialisees.

use super::*;

impl RuntimeQueries {
    pub async fn horoscope_services(&self) -> Result<Vec<HoroscopeServiceRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeServiceRow>(
            r#"
            SELECT service_code,
                   product_level_code,
                   shortlist_profile_code,
                   time_slot_profile_code,
                   slot_mode,
                   requires_natal_chart,
                   requires_location,
                   requires_timezone,
                   requires_inline_birth_data,
                   house_system_code,
                   period_profile_code,
                   detail_profile_code,
                   scan_profile_code,
                   detail_level,
                   generation_mode,
                   max_words_target,
                   max_words_hard_limit
            FROM horoscope_services
            ORDER BY service_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction horoscope_time_slot_profiles.
    pub async fn horoscope_time_slot_profiles(
        &self,
    ) -> Result<Vec<HoroscopeTimeSlotProfileRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeTimeSlotProfileRow>(
            r#"
            SELECT service_code,
                   slot_code,
                   start_local_time,
                   end_local_time,
                   reference_local_time,
                   slot_label,
                   is_public,
                   sort_order
            FROM horoscope_time_slot_profiles
            ORDER BY service_code, sort_order, slot_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction astral_time_period_profiles.
    pub async fn astral_time_period_profiles(
        &self,
    ) -> Result<Vec<AstralTimePeriodProfileRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, AstralTimePeriodProfileRow>(
            r#"
            SELECT period_profile_code,
                   resolution_strategy,
                   duration_days,
                   week_offset,
                   included_days,
                   is_enabled,
                   sort_order
            FROM astral_time_period_profiles
            ORDER BY sort_order, period_profile_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction horoscope_scan_profiles.
    pub async fn horoscope_scan_profiles(
        &self,
    ) -> Result<Vec<HoroscopeScanProfileRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeScanProfileRow>(
            r#"
            SELECT scan_profile_code,
                   granularity,
                   reference_time_local,
                   expected_snapshots_per_day,
                   is_enabled,
                   sort_order
            FROM horoscope_scan_profiles
            ORDER BY sort_order, scan_profile_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction horoscope_orb_weight_bands.
    pub async fn horoscope_orb_weight_bands(
        &self,
    ) -> Result<Vec<HoroscopeOrbWeightBandRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeOrbWeightBandRow>(
            r#"
            SELECT band_code,
                   min_orb_deg::float8 AS min_orb_deg,
                   max_orb_deg::float8 AS max_orb_deg,
                   weight::float8 AS weight
            FROM horoscope_orb_weight_bands
            ORDER BY min_orb_deg, band_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn horoscope_supported_objects(
        &self,
    ) -> Result<Vec<HoroscopeSupportedObjectRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeSupportedObjectRow>(
            r#"
            SELECT supported.service_code,
                   objects.code AS object_code,
                   supported.min_product_level_code,
                   supported.is_enabled,
                   supported.sort_order
            FROM horoscope_supported_objects supported
            INNER JOIN astral_chart_objects objects ON objects.id = supported.chart_object_id
            WHERE supported.is_enabled = true
            ORDER BY supported.service_code, supported.sort_order, objects.code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn horoscope_signal_theme_mappings(
        &self,
    ) -> Result<Vec<HoroscopeSignalThemeMappingRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeSignalThemeMappingRow>(
            r#"
            SELECT service_code,
                   match_object,
                   match_aspect,
                   match_natal_target,
                   theme_code,
                   priority
            FROM horoscope_signal_theme_mappings
            ORDER BY service_code, priority DESC, match_object, theme_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }
}
