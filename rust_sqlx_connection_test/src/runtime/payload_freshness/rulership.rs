use std::collections::BTreeMap;

use crate::domain::{BasicRulerContext, BasicRulershipChain, BasicRulershipContext};

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
