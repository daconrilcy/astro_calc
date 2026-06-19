use crate::application::ports::{
    CalculationReferenceLoader, ReferenceSystemLookup, ReferenceVersionProvider,
};
use crate::domain::CalculationReferenceData;
use crate::shared::error::RuntimeError;

pub async fn load_calculation_reference_data<R>(
    repository: &R,
) -> Result<CalculationReferenceData, RuntimeError>
where
    R: CalculationReferenceLoader + ReferenceSystemLookup + ?Sized,
{
    Ok(CalculationReferenceData {
        tropical_zodiacal_reference_system_id: repository
            .zodiacal_reference_system_id_by_key("tropical")
            .await?,
        geocentric_coordinate_reference_system_id: repository
            .coordinate_reference_system_id_by_key("geocentric")
            .await?,
        signs: repository.sign_references().await?,
        houses: repository.house_references().await?,
        motion_states: repository.motion_state_references().await?,
        horizon_positions: repository.horizon_position_references().await?,
        angle_points: repository.angle_point_references().await?,
    })
}

pub async fn load_default_calculation_reference_data<R>(
    repository: &R,
) -> Result<(i32, CalculationReferenceData), RuntimeError>
where
    R: CalculationReferenceLoader + ReferenceSystemLookup + ReferenceVersionProvider + ?Sized,
{
    Ok((
        repository.default_reference_version_id().await?,
        load_calculation_reference_data(repository).await?,
    ))
}
