use crate::domain::HouseAxisReference;

pub fn house_axis_label(axis_code: &str, refs: &[HouseAxisReference]) -> String {
    refs.iter()
        .find(|axis| axis.axis_code == axis_code)
        .map(|axis| axis.label.clone())
        .unwrap_or_else(|| axis_code.replace('_', " "))
}
