use crate::application::calculation_references::load_calculation_reference_data;
use crate::application::ports::{
    NatalReferenceStore, ReferenceSystemResolver, ReferenceVersionProvider,
};
use crate::domain::{AspectDefinition, CalculationReferenceData, ChartObject, HouseSystem};
use crate::shared::error::RuntimeError;

pub struct ChartContextData {
    pub reference_version_id: i32,
    pub chart_objects: Vec<ChartObject>,
    pub aspect_definitions: Vec<AspectDefinition>,
    pub house_system: HouseSystem,
    pub references: CalculationReferenceData,
}

pub async fn load_chart_context<R>(
    repository: &R,
    reference_version_id: i32,
    house_system_id: i32,
) -> Result<ChartContextData, RuntimeError>
where
    R: NatalReferenceStore + ReferenceSystemResolver + ?Sized,
{
    Ok(ChartContextData {
        reference_version_id,
        chart_objects: repository
            .active_chart_objects(reference_version_id)
            .await?,
        aspect_definitions: repository.aspect_definitions().await?,
        house_system: repository.house_system(house_system_id).await?,
        references: load_calculation_reference_data(repository).await?,
    })
}

pub async fn load_default_chart_context<R>(
    repository: &R,
    house_system_id: i32,
) -> Result<ChartContextData, RuntimeError>
where
    R: NatalReferenceStore + ReferenceSystemResolver + ReferenceVersionProvider + ?Sized,
{
    let reference_version_id = repository.default_reference_version_id().await?;
    load_chart_context(repository, reference_version_id, house_system_id).await
}
