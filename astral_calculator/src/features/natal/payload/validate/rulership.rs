//! Module astral_calculator\src\features\natal\payload\validate\rulership.rs du moteur astral_calculator.

use crate::domain::BasicRulershipContext;
use crate::domain::DomicileRulerReference;

pub(super) fn has_current_rulership_context(context: &BasicRulershipContext) -> bool {
    crate::features::natal::payload::rules::rulership::has_current_rulership_context(context)
}

pub(super) fn matches_domicile_ruler_references(
    context: &BasicRulershipContext,
    domicile_rulers: &[DomicileRulerReference],
) -> bool {
    crate::features::natal::payload::rules::rulership::matches_domicile_ruler_references(
        context,
        domicile_rulers,
    )
}
