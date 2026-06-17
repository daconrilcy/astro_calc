use sqlx::PgPool;

use super::models::{
    AnglePointReference as AnglePointReferenceRow, AspectDefinition as AspectDefinitionRow,
    ChartObject as ChartObjectRow, DomicileRulerReference as DomicileRulerReferenceRow,
    HorizonPositionReference as HorizonPositionReferenceRow, HouseReference as HouseReferenceRow,
    HouseSystem as HouseSystemRow, MajorAspectFamilyReference,
    MotionStateReference as MotionStateReferenceRow, SignReference as SignReferenceRow,
};
use super::runtime_repository::RuntimeRepository;
use crate::domain::{
    AnglePointReference, AspectDefinition, ChartObject, DomicileRulerReference,
    HorizonPositionReference, HouseReference, HouseSystem, MotionStateReference, SignReference,
};
use crate::shared::error::RuntimeError;

#[derive(Clone)]
pub struct ReferenceRepository {
    inner: RuntimeRepository,
}

impl ReferenceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeRepository::new(pool),
        }
    }

    pub fn pool(&self) -> &PgPool {
        self.inner.pool()
    }

    pub async fn default_reference_version_id(&self) -> Result<i32, RuntimeError> {
        self.inner.default_reference_version_id().await
    }

    pub async fn zodiacal_reference_system_id_by_key(
        &self,
        key: &str,
    ) -> Result<i32, RuntimeError> {
        self.inner.zodiacal_reference_system_id_by_key(key).await
    }

    pub async fn coordinate_reference_system_id_by_key(
        &self,
        key: &str,
    ) -> Result<i32, RuntimeError> {
        self.inner.coordinate_reference_system_id_by_key(key).await
    }

    pub async fn house_system_id_by_code(&self, code: &str) -> Result<i32, RuntimeError> {
        self.inner.house_system_id_by_code(code).await
    }

    pub async fn zodiacal_reference_system_display_name(
        &self,
        id: i32,
    ) -> Result<String, RuntimeError> {
        self.inner.zodiacal_reference_system_display_name(id).await
    }

    pub async fn coordinate_reference_system_display_name(
        &self,
        id: i32,
    ) -> Result<String, RuntimeError> {
        self.inner
            .coordinate_reference_system_display_name(id)
            .await
    }

    pub async fn house_system(&self, id: i32) -> Result<HouseSystem, RuntimeError> {
        let row = self.inner.house_system(id).await?;
        Ok(map_house_system(row))
    }

    pub async fn active_chart_objects(
        &self,
        reference_version_id: i32,
    ) -> Result<Vec<ChartObject>, RuntimeError> {
        Ok(self
            .inner
            .active_chart_objects(reference_version_id)
            .await?
            .into_iter()
            .map(map_chart_object)
            .collect())
    }

    pub async fn aspect_definitions(&self) -> Result<Vec<AspectDefinition>, RuntimeError> {
        Ok(self
            .inner
            .aspect_definitions()
            .await?
            .into_iter()
            .map(map_aspect_definition)
            .collect())
    }

    pub async fn major_aspect_family_reference(
        &self,
    ) -> Result<MajorAspectFamilyReference, RuntimeError> {
        self.inner.major_aspect_family_reference().await
    }

    pub async fn sign_references(&self) -> Result<Vec<SignReference>, RuntimeError> {
        Ok(self
            .inner
            .sign_references()
            .await?
            .into_iter()
            .map(map_sign_reference)
            .collect())
    }

    pub async fn house_references(&self) -> Result<Vec<HouseReference>, RuntimeError> {
        Ok(self
            .inner
            .house_references()
            .await?
            .into_iter()
            .map(map_house_reference)
            .collect())
    }

    pub async fn motion_state_references(&self) -> Result<Vec<MotionStateReference>, RuntimeError> {
        Ok(self
            .inner
            .motion_state_references()
            .await?
            .into_iter()
            .map(map_motion_state_reference)
            .collect())
    }

    pub async fn horizon_position_references(
        &self,
    ) -> Result<Vec<HorizonPositionReference>, RuntimeError> {
        Ok(self
            .inner
            .horizon_position_references()
            .await?
            .into_iter()
            .map(map_horizon_position_reference)
            .collect())
    }

    pub async fn angle_point_references(&self) -> Result<Vec<AnglePointReference>, RuntimeError> {
        Ok(self
            .inner
            .angle_point_references()
            .await?
            .into_iter()
            .map(map_angle_point_reference)
            .collect())
    }

    pub async fn zodiacal_reference_systems(
        &self,
    ) -> Result<Vec<super::models::ZodiacalReferenceSystemRow>, RuntimeError> {
        self.inner.zodiacal_reference_systems().await
    }

    pub async fn coordinate_reference_systems(
        &self,
    ) -> Result<Vec<super::models::CoordinateReferenceSystemRow>, RuntimeError> {
        self.inner.coordinate_reference_systems().await
    }

    pub async fn house_systems(&self) -> Result<Vec<HouseSystem>, RuntimeError> {
        Ok(self
            .inner
            .house_systems()
            .await?
            .into_iter()
            .map(map_house_system)
            .collect())
    }

    pub async fn domicile_ruler_references(
        &self,
        reference_version_id: i32,
    ) -> Result<Vec<DomicileRulerReference>, RuntimeError> {
        Ok(self
            .inner
            .domicile_ruler_references(reference_version_id)
            .await?
            .into_iter()
            .map(map_domicile_ruler_reference)
            .collect())
    }

    pub async fn house_axis_references(
        &self,
    ) -> Result<Vec<crate::domain::HouseAxisReference>, RuntimeError> {
        self.inner.house_axis_references().await
    }

    pub async fn lunar_phase_references(
        &self,
    ) -> Result<Vec<crate::domain::LunarPhaseReference>, RuntimeError> {
        self.inner.lunar_phase_references().await
    }

    pub async fn accidental_dignity_condition_references(
        &self,
    ) -> Result<Vec<crate::domain::AccidentalDignityConditionReference>, RuntimeError> {
        self.inner.accidental_dignity_condition_references().await
    }

    pub async fn object_sect_affinity_references(
        &self,
    ) -> Result<Vec<crate::domain::ObjectSectAffinityReference>, RuntimeError> {
        self.inner.object_sect_affinity_references().await
    }

    pub async fn language_id_for_code(&self, code: &str) -> Result<i32, RuntimeError> {
        self.inner.language_id_for_code(code).await
    }
}

fn map_chart_object(row: ChartObjectRow) -> ChartObject {
    ChartObject {
        id: row.id,
        code: row.code,
        name: row.name,
        swe_id: row.swe_id,
        role_code: row.role_code,
        role_label: row.role_label,
        is_luminary: row.is_luminary,
        is_planet_symbolic: row.is_planet_symbolic,
        is_visible_to_naked_eye: row.is_visible_to_naked_eye,
        nature_codes: row.nature_codes,
        position_priority_base: row.position_priority_base,
        angle_priority_base: row.angle_priority_base,
        source_weight: row.source_weight,
    }
}

fn map_aspect_definition(row: AspectDefinitionRow) -> AspectDefinition {
    AspectDefinition {
        id: row.id,
        code: row.code,
        name: row.name,
        angle: row.angle,
        family: row.family,
        default_orb_deg: row.default_orb_deg,
        max_default_orb_deg: row.max_default_orb_deg,
    }
}

fn map_house_system(row: HouseSystemRow) -> HouseSystem {
    HouseSystem {
        id: row.id,
        code: row.code,
        name: row.name,
        calculation_engine_code: row.calculation_engine_code,
    }
}

fn map_sign_reference(row: SignReferenceRow) -> SignReference {
    SignReference {
        id: row.id,
        code: row.code,
        name: row.name,
        element_code: row.element_code,
        element_label: row.element_label,
        modality_code: row.modality_code,
        modality_name: row.modality_name,
        polarity_code: row.polarity_code,
        polarity_name: row.polarity_name,
        keywords_json: row.keywords_json,
        shadow_keywords_json: row.shadow_keywords_json,
    }
}

fn map_house_reference(row: HouseReferenceRow) -> HouseReference {
    HouseReference {
        id: row.id,
        number: row.number,
        name: row.name,
        theme_code: row.theme_code,
        modality_code: row.modality_code,
        modality_label: row.modality_label,
        accidental_strength: row.accidental_strength,
        modality_priority_delta: row.modality_priority_delta,
        interpretation_weight: row.interpretation_weight,
    }
}

fn map_motion_state_reference(row: MotionStateReferenceRow) -> MotionStateReference {
    MotionStateReference {
        id: row.id,
        code: row.code,
        label: row.label,
        motion_family: row.motion_family,
    }
}

fn map_horizon_position_reference(row: HorizonPositionReferenceRow) -> HorizonPositionReference {
    HorizonPositionReference {
        id: row.id,
        code: row.code,
        label: row.label,
    }
}

fn map_angle_point_reference(row: AnglePointReferenceRow) -> AnglePointReference {
    AnglePointReference {
        id: row.id,
        code: row.code,
        short_label: row.short_label,
        full_name: row.full_name,
        axis: row.axis,
        opposite_angle_code: row.opposite_angle_code,
        associated_house: row.associated_house,
        description: row.description,
        chart_object_id: row.chart_object_id,
        chart_object_code: row.chart_object_code,
        chart_object_name: row.chart_object_name,
        chart_object_sort_order: row.chart_object_sort_order,
    }
}

fn map_domicile_ruler_reference(row: DomicileRulerReferenceRow) -> DomicileRulerReference {
    DomicileRulerReference {
        reference_version_id: row.reference_version_id,
        astral_system_id: row.astral_system_id,
        astral_system_code: row.astral_system_code,
        sign_id: row.sign_id,
        sign_code: row.sign_code,
        sign_name: row.sign_name,
        chart_object_id: row.chart_object_id,
        object_code: row.object_code,
        object_name: row.object_name,
        dignity_type: row.dignity_type,
        weight: row.weight,
        is_primary: row.is_primary,
    }
}
