use crate::domain::{BasicRulerContext, BasicRulershipContext};

pub(super) fn has_current_rulership_context(context: &BasicRulershipContext) -> bool {
    context
        .ascendant_ruler
        .as_ref()
        .is_some_and(has_current_ruler_context)
        && context
            .mc_ruler
            .as_ref()
            .is_some_and(has_current_ruler_context)
        && context
            .dominant_house_rulers
            .iter()
            .all(has_current_ruler_context)
        && context
            .dominant_sign_rulers
            .iter()
            .all(has_current_ruler_context)
        && context.dispositor_links.iter().all(|link| {
            !link.object_code.trim().is_empty()
                && !link.object_sign_code.trim().is_empty()
                && !link.dispositor_object_code.trim().is_empty()
                && !link.dispositor_signal_key.trim().is_empty()
                && !link.ruler_sources.is_empty()
        })
        && context.rulership_chains.iter().all(|chain| {
            !chain.object_code.trim().is_empty()
                && !chain.chain.is_empty()
                && chain.chain.len() <= 7
                && !chain.termination.trim().is_empty()
        })
        && context.final_dispositors.iter().all(|dispositor| {
            !dispositor.object_code.trim().is_empty()
                && matches!(
                    dispositor.disposition_type.as_str(),
                    "final_dispositor" | "mutual_reception" | "cycle"
                )
        })
}

fn has_current_ruler_context(context: &BasicRulerContext) -> bool {
    !context.context_key.trim().is_empty()
        && !context.source_kind.trim().is_empty()
        && !context.source_code.trim().is_empty()
        && !context.sign_code.trim().is_empty()
        && !context.ruler_object_codes.is_empty()
        && context
            .ruler_object_codes
            .iter()
            .all(|object_code| !object_code.trim().is_empty())
        && context
            .ruler_object_codes
            .contains(&context.ruler_object_code)
        && !context.ruler_object_code.trim().is_empty()
        && !context.interpretive_role.trim().is_empty()
        && !context.interpretive_hint.trim().is_empty()
        && !context.ruler_sources.is_empty()
        && context.ruler_sources.iter().all(|source| {
            !source.object_code.trim().is_empty()
                && context.ruler_object_codes.contains(&source.object_code)
                && source.astral_system_id > 0
                && !source.astral_system_code.trim().is_empty()
                && source.dignity_type == "domicile"
                && source.weight.is_finite()
        })
}
