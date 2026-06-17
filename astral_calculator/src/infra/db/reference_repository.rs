use sqlx::PgPool;

use super::models::{
    AnglePointReference, AspectDefinition, ChartObject, DomicileRulerReference, HouseReference,
    HouseSystem, HorizonPositionReference, MajorAspectFamilyReference, MotionStateReference,
    SignReference,
};
use super::runtime_repository::RuntimeRepository;
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
        self.inner.coordinate_reference_system_display_name(id).await
    }

    pub async fn house_system(&self, id: i32) -> Result<HouseSystem, RuntimeError> {
        self.inner.house_system(id).await
    }

    pub async fn active_chart_objects(
        &self,
        reference_version_id: i32,
    ) -> Result<Vec<ChartObject>, RuntimeError> {
        self.inner.active_chart_objects(reference_version_id).await
    }

    pub async fn aspect_definitions(&self) -> Result<Vec<AspectDefinition>, RuntimeError> {
        self.inner.aspect_definitions().await
    }

    pub async fn major_aspect_family_reference(
        &self,
    ) -> Result<MajorAspectFamilyReference, RuntimeError> {
        self.inner.major_aspect_family_reference().await
    }

    pub async fn sign_references(&self) -> Result<Vec<SignReference>, RuntimeError> {
        self.inner.sign_references().await
    }

    pub async fn house_references(&self) -> Result<Vec<HouseReference>, RuntimeError> {
        self.inner.house_references().await
    }

    pub async fn motion_state_references(&self) -> Result<Vec<MotionStateReference>, RuntimeError> {
        self.inner.motion_state_references().await
    }

    pub async fn horizon_position_references(
        &self,
    ) -> Result<Vec<HorizonPositionReference>, RuntimeError> {
        self.inner.horizon_position_references().await
    }

    pub async fn angle_point_references(&self) -> Result<Vec<AnglePointReference>, RuntimeError> {
        self.inner.angle_point_references().await
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
        self.inner.house_systems().await
    }

    pub async fn domicile_ruler_references(
        &self,
        reference_version_id: i32,
    ) -> Result<Vec<DomicileRulerReference>, RuntimeError> {
        self.inner.domicile_ruler_references(reference_version_id).await
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
