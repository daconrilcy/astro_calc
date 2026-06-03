use std::collections::{HashMap, HashSet};

use crate::domain::{
    BasicChartEmphasis, BasicDispositorLink, BasicFinalDispositor, BasicRulerContext,
    BasicRulerSource, BasicRulershipChain, BasicRulershipContext, BasicSignal,
    DomicileRulerReference, ObjectPositionFact,
};

const MAX_CHAIN_DEPTH: usize = 6;

pub(super) fn build_rulership_context(
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

    BasicRulershipContext {
        ascendant_ruler,
        mc_ruler,
        dominant_house_rulers,
        dominant_sign_rulers,
        dispositor_links,
        rulership_chains,
        final_dispositors,
    }
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
    let mut grouped: HashMap<(String, String), Vec<String>> = HashMap::new();
    for chain in chains {
        match chain.termination.as_str() {
            "final_dispositor" | "mutual_reception" | "cycle" => {
                if let Some(last) = chain.chain.last() {
                    grouped
                        .entry((last.clone(), chain.termination.clone()))
                        .or_default()
                        .push(chain.object_code.clone());
                }
            }
            _ => {}
        }
    }

    let mut values = grouped
        .into_iter()
        .map(|((object_code, disposition_type), mut source_objects)| {
            source_objects.sort();
            source_objects.dedup();
            BasicFinalDispositor {
                object_code,
                disposition_type,
                source_objects,
            }
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.object_code.cmp(&right.object_code));
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
            position
                .house_number
                .map(|number| number.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ),
        None => format!(
            "The {source_kind} {source_code} in {sign_code} is routed through {} by domicile rulership.",
            ruler.object_name
        ),
    }
}

fn is_mobile_position(position: &ObjectPositionFact) -> bool {
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
