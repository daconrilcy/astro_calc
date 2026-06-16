use crate::domain::BasicRulershipContext;
use crate::models::DomicileRulerReference;

pub(super) fn has_current_rulership_context(context: &BasicRulershipContext) -> bool {
    crate::payload_rules::rulership::has_current_rulership_context(context)
}

pub(super) fn matches_domicile_ruler_references(
    context: &BasicRulershipContext,
    domicile_rulers: &[DomicileRulerReference],
) -> bool {
    crate::payload_rules::rulership::matches_domicile_ruler_references(context, domicile_rulers)
}
