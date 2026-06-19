//! Module astral_calculator\src\features\natal\payload\rules\house_axes.rs du moteur astral_calculator.

use crate::domain::HouseAxisReference;

pub(crate) fn canonical_axis_reference<'a>(
    axis_code: &str,
    references: &'a [HouseAxisReference],
) -> Option<&'a HouseAxisReference> {
    references.iter().find(|reference| reference.axis_code == axis_code)
}

pub(crate) fn axis_label(axis_code: &str, references: &[HouseAxisReference]) -> String {
    canonical_axis_reference(axis_code, references)
        .map(|reference| reference.label.clone())
        .unwrap_or_default()
}
