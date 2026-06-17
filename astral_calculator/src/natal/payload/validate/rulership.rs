use crate::domain::BasicRulershipContext;
use crate::domain::DomicileRulerReference;

pub(super) fn has_current_rulership_context(context: &BasicRulershipContext) -> bool {
    crate::natal::payload::rules::rulership::has_current_rulership_context(context)
}

pub(super) fn matches_domicile_ruler_references(
    context: &BasicRulershipContext,
    domicile_rulers: &[DomicileRulerReference],
) -> bool {
    crate::natal::payload::rules::rulership::matches_domicile_ruler_references(
        context,
        domicile_rulers,
    )
}
