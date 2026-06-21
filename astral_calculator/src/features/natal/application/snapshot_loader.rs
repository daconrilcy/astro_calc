use crate::application::calculation_references::load_calculation_reference_data;
use crate::application::ports::{
    NatalReferenceStore, PayloadCatalogStore, ReferenceSystemResolver,
};
use crate::domain::CalculationReferenceData;
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::features::natal::validate::{
    validate_accidental_condition_triggers, validate_accidental_dignity_condition_references,
    validate_accidental_polarity_bands, validate_accidental_scoring_params,
    validate_aspect_definitions, validate_calculation_references,
    validate_chart_object_signal_profiles, validate_house_axis_references,
    validate_lunar_phase_references, validate_object_sect_affinity_references,
};
use crate::shared::error::RuntimeError;

pub(super) struct NatalReferenceSnapshot {
    pub(super) chart_objects: Vec<crate::domain::ChartObject>,
    pub(super) aspect_definitions: Vec<crate::domain::AspectDefinition>,
    pub(super) house_system: crate::domain::HouseSystem,
    pub(super) references: CalculationReferenceData,
    pub(super) domicile_rulers: Vec<crate::domain::DomicileRulerReference>,
    pub(super) house_axes: Vec<crate::domain::HouseAxisReference>,
    pub(super) lunar_phases: Vec<crate::domain::LunarPhaseReference>,
    pub(super) accidental_conditions: Vec<crate::domain::AccidentalDignityConditionReference>,
    pub(super) sect_affinities: Vec<crate::domain::ObjectSectAffinityReference>,
    pub(super) catalog: BasicPayloadCatalog,
}

pub(super) struct NatalReferenceSnapshotLoader<'a, P, R> {
    catalogs: &'a P,
    references: &'a R,
}

impl<'a, P, R> NatalReferenceSnapshotLoader<'a, P, R>
where
    P: PayloadCatalogStore,
    R: NatalReferenceStore + ReferenceSystemResolver,
{
    pub(super) fn new(catalogs: &'a P, references: &'a R) -> Self {
        Self {
            catalogs,
            references,
        }
    }

    pub(super) async fn load(
        &self,
        input: &crate::domain::NatalChartInput,
        product_code: &str,
    ) -> Result<NatalReferenceSnapshot, RuntimeError> {
        let chart_objects = self
            .references
            .active_chart_objects(input.reference_version_id)
            .await?;
        validate_chart_object_signal_profiles(&chart_objects)?;
        let aspect_definitions = self.references.aspect_definitions().await?;
        let major_aspect_family = self.references.major_aspect_family_reference().await?;
        let catalog = self
            .catalogs
            .basic_payload_catalog(
                product_code,
                "natal_structured_v14",
                input.reference_version_id,
            )
            .await?;
        validate_aspect_definitions(
            &aspect_definitions,
            catalog.product_scoring.default_major_orb_deg,
            major_aspect_family.expected_aspect_count as usize,
            major_aspect_family.max_default_orb_deg,
        )?;
        let house_system = self.references.house_system(input.house_system_id).await?;
        let references = load_calculation_reference_data(self.references).await?;
        validate_calculation_references(&references)?;
        let domicile_rulers = self
            .references
            .domicile_ruler_references(input.reference_version_id)
            .await?;
        let house_axes = self.references.house_axis_references().await?;
        validate_house_axis_references(&house_axes, &references.houses)?;
        let lunar_phases = self.references.lunar_phase_references().await?;
        validate_lunar_phase_references(&lunar_phases)?;
        let accidental_conditions = self
            .references
            .accidental_dignity_condition_references()
            .await?;
        validate_accidental_dignity_condition_references(
            &accidental_conditions,
            &catalog.accidental_triggers,
        )?;
        validate_accidental_condition_triggers(&catalog.accidental_triggers)?;
        validate_accidental_scoring_params(&catalog.accidental_scoring)?;
        validate_accidental_polarity_bands(&catalog.accidental_polarity_bands)?;
        let sect_affinities = self.references.object_sect_affinity_references().await?;
        validate_object_sect_affinity_references(&sect_affinities)?;

        Ok(NatalReferenceSnapshot {
            chart_objects,
            aspect_definitions,
            house_system,
            references,
            domicile_rulers,
            house_axes,
            lunar_phases,
            accidental_conditions,
            sect_affinities,
            catalog,
        })
    }
}
