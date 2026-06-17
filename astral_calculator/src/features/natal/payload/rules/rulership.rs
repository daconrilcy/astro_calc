use std::collections::{BTreeMap, HashMap, HashSet};

use crate::domain::{
    BasicChartEmphasis, BasicDispositorLink, BasicFinalDispositor, BasicMutualReception,
    BasicRulerContext, BasicRulerSource, BasicRulershipChain, BasicRulershipContext, BasicSignal,
    DomicileRulerReference, ObjectPositionFact,
};

const MAX_CHAIN_DEPTH: usize = 6;

pub(crate) fn build_rulership_context(
    positions: &[ObjectPositionFact],
    chart_emphasis: &BasicChartEmphasis,
    rulers: &[DomicileRulerReference],
    signals: &[BasicSignal],
) -> BasicRulershipContext {
    let rules_by_sign = rules_by_sign(rulers);
    let positions_by_object = positions
        .iter()
        .map(|position| (position.object_code.as_str(), position))
        .collect::<HashMap<_, _>>();
    let signal_keys = signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect::<HashSet<_>>();

    let ascendant_ruler = angle_ruler(
        "ascendant",
        "identity_ruler",
        positions,
        &rules_by_sign,
        &positions_by_object,
        &signal_keys,
    );
    let mc_ruler = angle_ruler(
        "mc",
        "public_direction_ruler",
        positions,
        &rules_by_sign,
        &positions_by_object,
        &signal_keys,
    );
    let descendant_ruler = angle_ruler(
        "descendant",
        "relationship_angle_ruler",
        positions,
        &rules_by_sign,
        &positions_by_object,
        &signal_keys,
    );
    let dominant_sign_rulers = chart_emphasis
        .dominant_signs
        .iter()
        .filter_map(|sign| {
            ruler_context_for_sign(
                "dominant_sign",
                &sign.sign_code,
                &sign.sign_code,
                "dominant_sign_ruler",
                &rules_by_sign,
                &positions_by_object,
                &signal_keys,
            )
        })
        .collect();
    let dominant_house_rulers = chart_emphasis
        .dominant_houses
        .iter()
        .filter_map(|house| {
            dominant_house_sign(positions, house.house_number).and_then(|sign_code| {
                ruler_context_for_sign(
                    "dominant_house",
                    &format!("house_{}", house.house_number),
                    sign_code,
                    "dominant_house_ruler",
                    &rules_by_sign,
                    &positions_by_object,
                    &signal_keys,
                )
            })
        })
        .collect();
    let dispositor_links = dispositor_links(
        positions,
        &rules_by_sign,
        &positions_by_object,
        &signal_keys,
    );
    let rulership_chains = rulership_chains(positions, &rules_by_sign);
    let final_dispositors = final_dispositors(&rulership_chains);
    let mutual_receptions = mutual_receptions(&rulership_chains);

    BasicRulershipContext {
        ascendant_ruler,
        mc_ruler,
        descendant_ruler,
        dominant_house_rulers,
        dominant_sign_rulers,
        dispositor_links,
        rulership_chains,
        final_dispositors,
        mutual_receptions,
    }
}

pub(crate) fn has_current_rulership_context(context: &BasicRulershipContext) -> bool {
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

pub(crate) fn matches_domicile_ruler_references(
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

pub(crate) fn is_mobile_position(position: &ObjectPositionFact) -> bool {
    let role_is_angle = position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("object_context"))
        .and_then(|context| context.get("role"))
        .and_then(|value| value.as_str())
        == Some("angle");
    let has_angle_context = position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("angle_context"))
        .is_some();

    !role_is_angle && !has_angle_context
}

fn rules_by_sign(rulers: &[DomicileRulerReference]) -> HashMap<&str, Vec<&DomicileRulerReference>> {
    let mut map: HashMap<&str, Vec<&DomicileRulerReference>> = HashMap::new();
    for ruler in rulers {
        map.entry(ruler.sign_code.as_str()).or_default().push(ruler);
    }
    map
}

fn angle_ruler(
    angle_code: &str,
    interpretive_role: &str,
    positions: &[ObjectPositionFact],
    rules_by_sign: &HashMap<&str, Vec<&DomicileRulerReference>>,
    positions_by_object: &HashMap<&str, &ObjectPositionFact>,
    signal_keys: &HashSet<&str>,
) -> Option<BasicRulerContext> {
    let angle = positions.iter().find(|position| {
        position.object_code == angle_code
            || position
                .facts_json
                .as_ref()
                .and_then(|facts| facts.get("angle_context"))
                .and_then(|context| context.get("angle_point_code"))
                .and_then(|value| value.as_str())
                == Some(angle_code)
    })?;

    ruler_context_for_sign(
        "angle",
        angle_code,
        &angle.sign_code,
        interpretive_role,
        rules_by_sign,
        positions_by_object,
        signal_keys,
    )
}

fn ruler_context_for_sign(
    source_kind: &str,
    source_code: &str,
    sign_code: &str,
    interpretive_role: &str,
    rules_by_sign: &HashMap<&str, Vec<&DomicileRulerReference>>,
    positions_by_object: &HashMap<&str, &ObjectPositionFact>,
    signal_keys: &HashSet<&str>,
) -> Option<BasicRulerContext> {
    let rules = rules_by_sign.get(sign_code)?;
    let primary = rules.first()?;
    let ruler_position = positions_by_object
        .get(primary.object_code.as_str())
        .copied();
    let ruler_position_signal_key =
        ruler_position_signal_key(&primary.object_code, signal_keys, ruler_position);

    Some(BasicRulerContext {
        context_key: format!("{source_kind}:{source_code}:ruler"),
        source_kind: source_kind.to_string(),
        source_code: source_code.to_string(),
        sign_code: sign_code.to_string(),
        ruler_object_codes: unique_ruler_object_codes(rules),
        ruler_object_code: primary.object_code.clone(),
        ruler_position_signal_key,
        ruler_house_number: ruler_position.and_then(|position| position.house_number),
        ruler_sign_code: ruler_position.map(|position| position.sign_code.clone()),
        interpretive_role: interpretive_role.to_string(),
        strength_context: strength_context(ruler_position),
        ruler_sources: rules.iter().map(|rule| ruler_source(rule)).collect(),
        interpretive_hint: interpretive_hint(
            source_kind,
            source_code,
            sign_code,
            primary,
            ruler_position,
        ),
    })
}

fn dispositor_links(
    positions: &[ObjectPositionFact],
    rules_by_sign: &HashMap<&str, Vec<&DomicileRulerReference>>,
    positions_by_object: &HashMap<&str, &ObjectPositionFact>,
    signal_keys: &HashSet<&str>,
) -> Vec<BasicDispositorLink> {
    positions
        .iter()
        .filter(|position| is_mobile_position(position))
        .filter_map(|position| {
            let rules = rules_by_sign.get(position.sign_code.as_str())?;
            let primary = rules.first()?;
            let dispositor_position = positions_by_object
                .get(primary.object_code.as_str())
                .copied();
            Some(BasicDispositorLink {
                object_code: position.object_code.clone(),
                object_sign_code: position.sign_code.clone(),
                dispositor_object_code: primary.object_code.clone(),
                dispositor_signal_key: dispositor_signal_key(
                    primary,
                    signal_keys,
                    dispositor_position,
                ),
                ruler_sources: rules.iter().map(|rule| ruler_source(rule)).collect(),
                interpretive_hint: format!(
                    "{} in {} is routed through {} by domicile rulership.",
                    position.object_name, position.sign_name, primary.object_name
                ),
            })
        })
        .collect()
}

fn rulership_chains(
    positions: &[ObjectPositionFact],
    rules_by_sign: &HashMap<&str, Vec<&DomicileRulerReference>>,
) -> Vec<BasicRulershipChain> {
    let position_sign_by_object = positions
        .iter()
        .filter(|position| is_mobile_position(position))
        .map(|position| (position.object_code.as_str(), position.sign_code.as_str()))
        .collect::<HashMap<_, _>>();

    positions
        .iter()
        .filter(|position| is_mobile_position(position))
        .map(|position| {
            let mut chain = vec![position.object_code.clone()];
            let mut seen = HashSet::from([position.object_code.clone()]);
            let mut current = position.object_code.as_str();
            let mut termination = "unresolved".to_string();

            for _ in 0..MAX_CHAIN_DEPTH {
                let Some(sign_code) = position_sign_by_object.get(current).copied() else {
                    break;
                };
                let Some(next) = rules_by_sign
                    .get(sign_code)
                    .and_then(|rules| rules.first())
                    .map(|rule| rule.object_code.as_str())
                else {
                    break;
                };
                chain.push(next.to_string());
                if next == current {
                    termination = "final_dispositor".to_string();
                    break;
                }
                if !seen.insert(next.to_string()) {
                    termination =
                        if chain.len() >= 3 && chain[chain.len() - 3] == chain[chain.len() - 1] {
                            "mutual_reception".to_string()
                        } else {
                            "cycle".to_string()
                        };
                    break;
                }
                current = next;
            }
            if termination == "unresolved" && chain.len() > MAX_CHAIN_DEPTH {
                termination = "max_depth".to_string();
            }

            BasicRulershipChain {
                object_code: position.object_code.clone(),
                chain,
                termination,
            }
        })
        .collect()
}

fn final_dispositors(chains: &[BasicRulershipChain]) -> Vec<BasicFinalDispositor> {
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    for chain in chains {
        if chain.termination == "final_dispositor" {
            if let Some(last) = chain.chain.last() {
                grouped
                    .entry(last.clone())
                    .or_default()
                    .push(chain.object_code.clone());
            }
        }
    }

    let mut values = grouped
        .into_iter()
        .map(|(object_code, mut source_objects)| {
            source_objects.sort();
            source_objects.dedup();
            BasicFinalDispositor {
                object_code,
                source_objects,
            }
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.object_code.cmp(&right.object_code));
    values
}

fn mutual_receptions(chains: &[BasicRulershipChain]) -> Vec<BasicMutualReception> {
    let mut grouped: HashMap<String, BasicMutualReception> = HashMap::new();
    for chain in chains
        .iter()
        .filter(|chain| chain.termination == "mutual_reception")
    {
        let Some(pair) = mutual_reception_pair(chain) else {
            continue;
        };
        let key = pair.join(":");
        let entry = grouped.entry(key).or_insert_with(|| BasicMutualReception {
            object_codes: pair,
            source_objects: Vec::new(),
        });
        entry.source_objects.push(chain.object_code.clone());
    }

    let mut values = grouped.into_values().collect::<Vec<_>>();
    for value in &mut values {
        value.source_objects.sort();
        value.source_objects.dedup();
    }
    values.sort_by(|left, right| left.object_codes.cmp(&right.object_codes));
    values
}

fn dominant_house_sign(positions: &[ObjectPositionFact], house_number: i32) -> Option<&str> {
    positions
        .iter()
        .filter(|position| position.house_number == Some(house_number))
        .min_by(|left, right| left.longitude_deg.total_cmp(&right.longitude_deg))
        .map(|position| position.sign_code.as_str())
}

fn ruler_position_signal_key(
    object_code: &str,
    signal_keys: &HashSet<&str>,
    position: Option<&ObjectPositionFact>,
) -> Option<String> {
    let key = format!("object_position:{object_code}");
    if signal_keys.contains(key.as_str()) || position.is_some() {
        Some(key)
    } else {
        None
    }
}

fn dispositor_signal_key(
    rule: &DomicileRulerReference,
    signal_keys: &HashSet<&str>,
    position: Option<&ObjectPositionFact>,
) -> String {
    let dignity_key = format!(
        "dignity:{}:{}:{}",
        rule.object_code, rule.dignity_type, rule.sign_code
    );
    if signal_keys.contains(dignity_key.as_str()) {
        dignity_key
    } else {
        ruler_position_signal_key(&rule.object_code, signal_keys, position)
            .unwrap_or_else(|| format!("rulership:{}:{}", rule.sign_code, rule.object_code))
    }
}

fn ruler_source(rule: &DomicileRulerReference) -> BasicRulerSource {
    BasicRulerSource {
        object_code: rule.object_code.clone(),
        reference_version_id: rule.reference_version_id,
        astral_system_id: rule.astral_system_id,
        astral_system_code: rule.astral_system_code.clone(),
        dignity_type: rule.dignity_type.clone(),
        weight: rule.weight,
        is_primary: rule.is_primary,
    }
}

fn unique_ruler_object_codes(rules: &[&DomicileRulerReference]) -> Vec<String> {
    let mut object_codes = Vec::new();
    for rule in rules {
        if !object_codes.contains(&rule.object_code) {
            object_codes.push(rule.object_code.clone());
        }
    }
    object_codes
}

fn strength_context(position: Option<&ObjectPositionFact>) -> Vec<String> {
    let Some(position) = position else {
        return Vec::new();
    };
    let mut context = Vec::new();
    if matches!(position.house_number, Some(1 | 4 | 7 | 10)) {
        context.push("angular_house".to_string());
    }
    if position.house_number == Some(1) {
        context.push("identity_house".to_string());
    }
    if position.house_number == Some(2) {
        context.push("resources_house".to_string());
    }
    if position.house_number == Some(7) {
        context.push("partnership_house".to_string());
    }
    if matches!(position.object_code.as_str(), "sun" | "moon") {
        context.push("core_identity_signal".to_string());
    }
    context
}

fn interpretive_hint(
    source_kind: &str,
    source_code: &str,
    sign_code: &str,
    ruler: &DomicileRulerReference,
    ruler_position: Option<&ObjectPositionFact>,
) -> String {
    match ruler_position {
        Some(position) => format!(
            "The {source_kind} {source_code} in {sign_code} is routed through {}, placed in {} house {}.",
            ruler.object_name,
            position.sign_name,
            position.house_number.map(|number| number.to_string()).unwrap_or_else(|| "unknown".to_string())
        ),
        None => format!(
            "The {source_kind} {source_code} in {sign_code} is routed through {} by domicile rulership.",
            ruler.object_name
        ),
    }
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
