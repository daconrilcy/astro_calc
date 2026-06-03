use std::collections::BTreeMap;

use crate::domain::{
    BasicDispositorLink, BasicRulerContext, BasicRulerSource, BasicRulershipChain,
    BasicRulershipContext,
};
use crate::models::DomicileRulerReference;

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
                && !dispositor.source_objects.is_empty()
                && dispositor
                    .source_objects
                    .iter()
                    .all(|object_code| !object_code.trim().is_empty())
        })
        && final_dispositors_match_chains(context)
        && context.mutual_receptions.iter().all(|reception| {
            reception.object_codes.len() == 2
                && reception
                    .object_codes
                    .iter()
                    .all(|object_code| !object_code.trim().is_empty())
                && !reception.source_objects.is_empty()
                && reception
                    .source_objects
                    .iter()
                    .all(|object_code| !object_code.trim().is_empty())
        })
        && mutual_receptions_match_chains(context)
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

pub(super) fn matches_domicile_ruler_references(
    context: &BasicRulershipContext,
    domicile_rulers: &[DomicileRulerReference],
) -> bool {
    context
        .ascendant_ruler
        .as_ref()
        .is_none_or(|ruler| ruler_context_matches_references(ruler, domicile_rulers))
        && context
            .mc_ruler
            .as_ref()
            .is_none_or(|ruler| ruler_context_matches_references(ruler, domicile_rulers))
        && context
            .dominant_house_rulers
            .iter()
            .all(|ruler| ruler_context_matches_references(ruler, domicile_rulers))
        && context
            .dominant_sign_rulers
            .iter()
            .all(|ruler| ruler_context_matches_references(ruler, domicile_rulers))
        && context
            .dispositor_links
            .iter()
            .all(|link| dispositor_link_matches_references(link, domicile_rulers))
}

fn ruler_context_matches_references(
    context: &BasicRulerContext,
    domicile_rulers: &[DomicileRulerReference],
) -> bool {
    source_signatures(&context.ruler_sources)
        == reference_signatures(context.sign_code.as_str(), domicile_rulers)
}

fn dispositor_link_matches_references(
    link: &BasicDispositorLink,
    domicile_rulers: &[DomicileRulerReference],
) -> bool {
    source_signatures(&link.ruler_sources)
        == reference_signatures(link.object_sign_code.as_str(), domicile_rulers)
}

fn source_signatures(sources: &[BasicRulerSource]) -> Vec<RulerSourceSignature> {
    let mut signatures = sources
        .iter()
        .map(|source| RulerSourceSignature {
            reference_version_id: source.reference_version_id,
            astral_system_id: source.astral_system_id,
            astral_system_code: source.astral_system_code.clone(),
            dignity_type: source.dignity_type.clone(),
            object_code: source.object_code.clone(),
            weight_bits: source.weight.to_bits(),
            is_primary: source.is_primary,
        })
        .collect::<Vec<_>>();
    signatures.sort();
    signatures
}

fn reference_signatures(
    sign_code: &str,
    domicile_rulers: &[DomicileRulerReference],
) -> Vec<RulerSourceSignature> {
    let mut signatures = domicile_rulers
        .iter()
        .filter(|ruler| ruler.sign_code == sign_code)
        .map(|ruler| RulerSourceSignature {
            reference_version_id: ruler.reference_version_id,
            astral_system_id: ruler.astral_system_id,
            astral_system_code: ruler.astral_system_code.clone(),
            dignity_type: ruler.dignity_type.clone(),
            object_code: ruler.object_code.clone(),
            weight_bits: ruler.weight.to_bits(),
            is_primary: ruler.is_primary,
        })
        .collect::<Vec<_>>();
    signatures.sort();
    signatures
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RulerSourceSignature {
    reference_version_id: Option<i32>,
    astral_system_id: i32,
    astral_system_code: String,
    dignity_type: String,
    object_code: String,
    weight_bits: u64,
    is_primary: bool,
}

fn final_dispositors_match_chains(context: &BasicRulershipContext) -> bool {
    let mut expected: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for chain in context
        .rulership_chains
        .iter()
        .filter(|chain| chain.termination == "final_dispositor")
    {
        let Some(last) = chain.chain.last() else {
            return false;
        };
        expected
            .entry(last.clone())
            .or_default()
            .push(chain.object_code.clone());
    }
    normalize_map_values(&mut expected);

    let mut actual = BTreeMap::new();
    for final_dispositor in &context.final_dispositors {
        actual.insert(
            final_dispositor.object_code.clone(),
            final_dispositor.source_objects.clone(),
        );
    }
    normalize_map_values(&mut actual);

    actual == expected
}

fn mutual_receptions_match_chains(context: &BasicRulershipContext) -> bool {
    let mut expected: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    for chain in context
        .rulership_chains
        .iter()
        .filter(|chain| chain.termination == "mutual_reception")
    {
        let Some(pair) = mutual_reception_pair(chain) else {
            return false;
        };
        expected
            .entry(pair)
            .or_default()
            .push(chain.object_code.clone());
    }
    normalize_map_values(&mut expected);

    let mut actual = BTreeMap::new();
    for reception in &context.mutual_receptions {
        let mut pair = reception.object_codes.clone();
        pair.sort();
        pair.dedup();
        if pair.len() != 2 {
            return false;
        }
        actual.insert(pair, reception.source_objects.clone());
    }
    normalize_map_values(&mut actual);

    actual == expected
}

fn mutual_reception_pair(chain: &BasicRulershipChain) -> Option<Vec<String>> {
    let len = chain.chain.len();
    if len < 3 || chain.chain[len - 1] != chain.chain[len - 3] {
        return None;
    }
    let mut pair = vec![chain.chain[len - 2].clone(), chain.chain[len - 1].clone()];
    pair.sort();
    pair.dedup();
    (pair.len() == 2).then_some(pair)
}

fn normalize_map_values<K: Ord>(map: &mut BTreeMap<K, Vec<String>>) {
    for values in map.values_mut() {
        values.sort();
        values.dedup();
    }
}
