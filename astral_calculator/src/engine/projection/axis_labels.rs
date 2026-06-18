//! Module astral_calculator\src\engine\projection\axis_labels.rs du moteur astral_calculator.

use crate::domain::HouseAxisReference;

/// Fonction house_axis_label.
pub fn house_axis_label(axis_code: &str, refs: &[HouseAxisReference]) -> String {
    refs.iter()
        .find(|axis| axis.axis_code == axis_code)
        .map(|axis| axis.label.clone())
        .unwrap_or_else(|| axis_code.replace('_', " "))
}
