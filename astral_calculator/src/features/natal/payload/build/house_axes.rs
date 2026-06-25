//! Module astral_calculator\src\features\natal\payload\build\house_axes.rs du moteur astral_calculator.

use std::collections::{HashMap, HashSet};

use crate::domain::{
    BasicAngleFact, BasicChartEmphasis, BasicDignity, BasicHouseAxisEmphasis, BasicHouseAxisScore,
    BasicProjectionReason, BasicRulershipContext, BasicSignal, HouseAxisReference,
    ObjectPositionFact,
};
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::features::natal::payload::rules::chart_context::is_angle_role;

use super::projection_reasons::{
    reason_active_signal, reason_angle_in_house, reason_cross_axis_aspect,
    reason_essential_dignity, reason_luminary_in_house, reason_object_in_house,
    reason_rulership_context, reason_simple, reason_theme_emphasis,
};

#[derive(Default)]
/// Structure HouseScoreDraft.
struct HouseScoreDraft {
    raw_score: f64,
    reason_details: Vec<BasicProjectionReason>,
    source_signal_keys: Vec<String>,
    source_context_keys: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_house_axis_emphasis(
    references: &[HouseAxisReference],
    positions: &[ObjectPositionFact],
    angles: &[BasicAngleFact],
    dignities: &[BasicDignity],
    chart_emphasis: &BasicChartEmphasis,
    rulership_context: &BasicRulershipContext,
    signals: &[BasicSignal],
    catalog: &BasicPayloadCatalog,
    locale: &str,
) -> Vec<BasicHouseAxisEmphasis> {
    let scoring = &catalog.product_scoring;
    if references.is_empty() {
        return Vec::new();
    }

    let signal_keys: HashSet<&str> = signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();
    let position_house_by_object: HashMap<&str, i32> = positions
        .iter()
        .filter_map(|position| {
            position
                .house_number
                .map(|house| (position.object_code.as_str(), house))
        })
        .collect();

    let mut axes = references
        .iter()
        .map(|reference| {
            build_axis(
                reference,
                positions,
                angles,
                dignities,
                chart_emphasis,
                rulership_context,
                signals,
                &signal_keys,
                &position_house_by_object,
                catalog,
                locale,
            )
        })
        .collect::<Vec<_>>();

    axes.retain(|axis| {
        axis.axis_score >= scoring.axis_min_score && axis.polarity_balance != "weak_axis"
    });
    axes.sort_by(|left, right| {
        right
            .axis_score
            .partial_cmp(&left.axis_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.axis_code.cmp(&right.axis_code))
    });
    axes.truncate(scoring.max_house_axis_emphasis);
    axes
}

#[allow(clippy::too_many_arguments)]
fn build_axis(
    reference: &HouseAxisReference,
    positions: &[ObjectPositionFact],
    angles: &[BasicAngleFact],
    dignities: &[BasicDignity],
    chart_emphasis: &BasicChartEmphasis,
    rulership_context: &BasicRulershipContext,
    signals: &[BasicSignal],
    signal_keys: &HashSet<&str>,
    position_house_by_object: &HashMap<&str, i32>,
    catalog: &BasicPayloadCatalog,
    locale: &str,
) -> BasicHouseAxisEmphasis {
    let scoring = &catalog.product_scoring;
    let mut first = score_house(
        reference.house_a_number,
        &reference.theme_a_code,
        positions,
        angles,
        dignities,
        chart_emphasis,
        rulership_context,
        signals,
        signal_keys,
        position_house_by_object,
    );
    let mut second = score_house(
        reference.house_b_number,
        &reference.theme_b_code,
        positions,
        angles,
        dignities,
        chart_emphasis,
        rulership_context,
        signals,
        signal_keys,
        position_house_by_object,
    );
    add_cross_axis_aspects(
        reference,
        signals,
        position_house_by_object,
        &mut first,
        &mut second,
    );
    let first_score = normalized_house_score(first.raw_score, scoring.house_axis_full_score);
    let second_score = normalized_house_score(second.raw_score, scoring.house_axis_full_score);
    let axis_score = round4(
        (first_score.max(second_score)
            + scoring.axis_secondary_weight * first_score.min(second_score))
        .clamp(0.0, 1.0),
    );
    let (primary_house, secondary_house) = if first_score >= second_score {
        (reference.house_a_number, reference.house_b_number)
    } else {
        (reference.house_b_number, reference.house_a_number)
    };

    let mut source_signal_keys = first.source_signal_keys.clone();
    push_unique_all(&mut source_signal_keys, &second.source_signal_keys);
    source_signal_keys.retain(|key| signal_keys.contains(key.as_str()));
    let mut source_context_keys = first.source_context_keys.clone();
    push_unique_all(&mut source_context_keys, &second.source_context_keys);
    let mut reason_details = first.reason_details.clone();
    push_unique_reasons(&mut reason_details, &second.reason_details);

    let polarity_balance = polarity_balance(first_score, second_score, scoring);

    BasicHouseAxisEmphasis {
        axis_code: reference.axis_code.clone(),
        houses: vec![reference.house_a_number, reference.house_b_number],
        theme_codes: vec![
            reference.theme_a_code.clone(),
            reference.theme_b_code.clone(),
        ],
        house_scores: vec![
            BasicHouseAxisScore {
                house_number: reference.house_a_number,
                theme_code: reference.theme_a_code.clone(),
                score: first_score,
                reason_details: first.reason_details,
            },
            BasicHouseAxisScore {
                house_number: reference.house_b_number,
                theme_code: reference.theme_b_code.clone(),
                score: second_score,
                reason_details: second.reason_details,
            },
        ],
        primary_house,
        secondary_house,
        axis_score,
        polarity_balance: polarity_balance.clone(),
        source_signal_keys,
        source_context_keys,
        reason_details,
        interpretive_hint: interpretive_hint(reference, &polarity_balance, locale),
    }
}

#[allow(clippy::too_many_arguments)]
fn score_house(
    house_number: i32,
    theme_code: &str,
    positions: &[ObjectPositionFact],
    angles: &[BasicAngleFact],
    dignities: &[BasicDignity],
    chart_emphasis: &BasicChartEmphasis,
    rulership_context: &BasicRulershipContext,
    signals: &[BasicSignal],
    signal_keys: &HashSet<&str>,
    position_house_by_object: &HashMap<&str, i32>,
) -> HouseScoreDraft {
    let mut draft = HouseScoreDraft::default();

    if let Some(dominant) = chart_emphasis
        .dominant_houses
        .iter()
        .find(|entry| entry.house_number == house_number)
    {
        add_score(
            &mut draft,
            dominant.score * 0.75,
            reason_simple("dominant_house"),
        );
    }

    for position in positions
        .iter()
        .filter(|position| position.house_number == Some(house_number))
    {
        let object_code = position.object_code.as_str();
        add_score(
            &mut draft,
            object_source_weight(position) * 0.35,
            reason_object_in_house(object_code, house_number, Some(theme_code)),
        );
        if is_luminary(position) {
            add_score(
                &mut draft,
                0.25,
                reason_luminary_in_house(object_code, house_number, theme_code),
            );
        }
        if is_angle(position) {
            add_score(
                &mut draft,
                0.35,
                reason_angle_in_house(object_code, house_number, theme_code),
            );
        }
        push_signal_if_exists(
            &mut draft.source_signal_keys,
            signal_keys,
            &format!("object_position:{object_code}"),
        );
    }

    for angle in angles
        .iter()
        .filter(|angle| angle.house_number == house_number)
    {
        add_score(
            &mut draft,
            0.25,
            reason_angle_in_house(&angle.angle_code, house_number, theme_code),
        );
        push_signal_prefix(
            &mut draft.source_signal_keys,
            signals,
            &format!("angle:{}:sign:", angle.angle_code),
        );
    }

    for signal in signals {
        if cluster_house_number(signal) == Some(house_number) {
            add_score(
                &mut draft,
                signal.priority_score / 100.0 * 0.45,
                reason_simple("cluster"),
            );
            push_unique(&mut draft.source_signal_keys, signal.signal_key.clone());
        } else if signal_matches_house(signal, house_number, position_house_by_object) {
            add_score(
                &mut draft,
                signal.source_weight.unwrap_or(0.0).min(1.0) * 0.2,
                reason_active_signal(&signal.signal_key),
            );
            push_unique(&mut draft.source_signal_keys, signal.signal_key.clone());
        }
    }

    for dignity in dignities {
        if position_house_by_object
            .get(dignity.object_code.as_str())
            .copied()
            == Some(house_number)
        {
            add_score(
                &mut draft,
                dignity_weight(&dignity.dignity_type),
                reason_essential_dignity(&dignity.object_code, &dignity.dignity_type),
            );
            if let Some(signal_key) = &dignity.signal_key {
                push_signal_if_exists(&mut draft.source_signal_keys, signal_keys, signal_key);
            }
        }
    }

    add_rulership_context(house_number, rulership_context, &mut draft);

    if !draft.reason_details.is_empty() {
        add_reason(&mut draft, reason_theme_emphasis(theme_code));
    }

    draft
}

fn add_rulership_context(
    house_number: i32,
    rulership_context: &BasicRulershipContext,
    draft: &mut HouseScoreDraft,
) {
    for context in rulership_context
        .ascendant_ruler
        .iter()
        .chain(rulership_context.mc_ruler.iter())
        .chain(rulership_context.dominant_house_rulers.iter())
        .chain(rulership_context.dominant_sign_rulers.iter())
    {
        if context.ruler_house_number == Some(house_number)
            || (context.source_kind == "dominant_house"
                && context.source_code == format!("house_{house_number}"))
        {
            add_score(draft, 0.2, reason_rulership_context(&context.context_key));
            push_unique(&mut draft.source_context_keys, context.context_key.clone());
            if let Some(signal_key) = &context.ruler_position_signal_key {
                push_unique(&mut draft.source_signal_keys, signal_key.clone());
            }
        }
    }
}

fn add_cross_axis_aspects(
    reference: &HouseAxisReference,
    signals: &[BasicSignal],
    position_house_by_object: &HashMap<&str, i32>,
    first: &mut HouseScoreDraft,
    second: &mut HouseScoreDraft,
) {
    for signal in signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("aspect:"))
    {
        let object_houses = signal_object_codes(signal)
            .iter()
            .filter_map(|object_code| position_house_by_object.get(object_code.as_str()).copied())
            .collect::<HashSet<_>>();

        if object_houses.contains(&reference.house_a_number)
            && object_houses.contains(&reference.house_b_number)
        {
            add_reason(first, reason_cross_axis_aspect(&signal.signal_key));
            add_reason(second, reason_cross_axis_aspect(&signal.signal_key));
            push_unique(&mut first.source_signal_keys, signal.signal_key.clone());
            push_unique(&mut second.source_signal_keys, signal.signal_key.clone());
        }
    }
}

fn signal_matches_house(
    signal: &BasicSignal,
    house_number: i32,
    position_house_by_object: &HashMap<&str, i32>,
) -> bool {
    if signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("placement_context"))
        .and_then(|context| context.get("house_context"))
        .and_then(|context| context.get("house_number"))
        .and_then(|value| value.as_i64())
        == Some(i64::from(house_number))
    {
        return true;
    }

    signal_object_codes(signal).iter().any(|object_code| {
        position_house_by_object.get(object_code.as_str()) == Some(&house_number)
    })
}

fn signal_object_codes(signal: &BasicSignal) -> Vec<String> {
    let mut object_codes = Vec::new();
    if let Some(evidence) = &signal.evidence {
        for key in [
            "object_code",
            "chart_object",
            "source_object_code",
            "target_object_code",
        ] {
            if let Some(object_code) = evidence.get(key).and_then(|value| value.as_str()) {
                push_unique(&mut object_codes, object_code.to_string());
            }
        }
    }

    let parts: Vec<&str> = signal.signal_key.split(':').collect();
    if (signal.signal_key.starts_with("object_position:")
        || signal.signal_key.starts_with("dignity:"))
        && parts.len() >= 2
    {
        push_unique(&mut object_codes, parts[1].to_string());
    } else if signal.signal_key.starts_with("aspect:") && parts.len() >= 4 {
        push_unique(&mut object_codes, parts[1].to_string());
        push_unique(&mut object_codes, parts[2].to_string());
    }

    object_codes
}

fn cluster_house_number(signal: &BasicSignal) -> Option<i32> {
    if !signal.signal_key.starts_with("cluster:") {
        return None;
    }
    signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("house_number"))
        .and_then(|value| value.as_i64())
        .and_then(|value| i32::try_from(value).ok())
}

fn object_source_weight(position: &ObjectPositionFact) -> f64 {
    position
        .object_context()
        .and_then(|context| context.signal_scoring)
        .and_then(|scoring| {
            scoring
                .get("source_weight")
                .and_then(|value| value.as_f64())
        })
        .unwrap_or(0.0)
}

fn is_luminary(position: &ObjectPositionFact) -> bool {
    position
        .object_context()
        .and_then(|context| context.is_luminary)
        .unwrap_or(false)
}

fn is_angle(position: &ObjectPositionFact) -> bool {
    let role = position.object_context().and_then(|context| context.role);
    let role_label = position
        .object_context()
        .and_then(|context| context.role_label);
    is_angle_role(role.as_deref(), role_label.as_deref()) || position.angle_context().is_some()
}

fn dignity_weight(dignity_type: &str) -> f64 {
    match dignity_type {
        "domicile" => 0.35,
        "exaltation" => 0.3,
        "detriment" => 0.2,
        "fall" => 0.18,
        _ => 0.15,
    }
}

fn normalized_house_score(raw_score: f64, house_axis_full_score: f64) -> f64 {
    round4((raw_score / house_axis_full_score).clamp(0.0, 1.0))
}

fn polarity_balance(
    first_score: f64,
    second_score: f64,
    scoring: &crate::domain::BasicProductScoringProfile,
) -> String {
    if first_score >= second_score + scoring.axis_polarity_dominance_delta {
        "primary_house_dominant".to_string()
    } else if second_score >= first_score + scoring.axis_polarity_dominance_delta {
        "secondary_house_dominant".to_string()
    } else if first_score >= scoring.axis_balanced_min_score
        && second_score >= scoring.axis_balanced_min_score
    {
        "balanced_axis".to_string()
    } else {
        "weak_axis".to_string()
    }
}

fn interpretive_hint(
    reference: &HouseAxisReference,
    polarity_balance: &str,
    locale: &str,
) -> String {
    match polarity_balance {
        "primary_house_dominant" => localized_axis_hint(
            locale,
            &reference.label,
            &format!(
                "is activated mainly through house {} ({}), with house {} ({}) present as a secondary counterpoint.",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
            &format!(
                "s'active principalement à travers la maison {} ({}), la maison {} ({}) apparaissant comme contrepoint secondaire.",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
            &format!(
                "se activa principalmente a través de la casa {} ({}), con la casa {} ({}) presente como contrapunto secundario.",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
            &format!(
                "wird vor allem durch Haus {} ({} ) aktiviert, während Haus {} ({}) als sekundärer Gegenpol präsent ist.",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
        ),
        "secondary_house_dominant" => localized_axis_hint(
            locale,
            &reference.label,
            &format!(
                "is activated mainly through house {} ({}), with house {} ({}) present as a secondary counterpoint.",
                reference.house_b_number,
                reference.theme_b_code,
                reference.house_a_number,
                reference.theme_a_code
            ),
            &format!(
                "s'active principalement à travers la maison {} ({}), la maison {} ({}) apparaissant comme contrepoint secondaire.",
                reference.house_b_number,
                reference.theme_b_code,
                reference.house_a_number,
                reference.theme_a_code
            ),
            &format!(
                "se activa principalmente a través de la casa {} ({}), con la casa {} ({}) presente como contrapunto secundario.",
                reference.house_b_number,
                reference.theme_b_code,
                reference.house_a_number,
                reference.theme_a_code
            ),
            &format!(
                "wird vor allem durch Haus {} ({}) aktiviert, während Haus {} ({}) als sekundärer Gegenpol präsent ist.",
                reference.house_b_number,
                reference.theme_b_code,
                reference.house_a_number,
                reference.theme_a_code
            ),
        ),
        "balanced_axis" => localized_axis_hint(
            locale,
            &reference.label,
            &format!(
                "is activated with both house {} ({}) and house {} ({}) strongly active.",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
            &format!(
                "s'active avec les maisons {} ({}) et {} ({}) toutes deux fortement actives.",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
            &format!(
                "se activa con la casa {} ({}) y la casa {} ({}) ambas fuertemente activas.",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
            &format!(
                "ist mit Haus {} ({}) und Haus {} ({}) gleichermaßen stark aktiviert.",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
        ),
        _ => localized_axis_hint(
            locale,
            &reference.label,
            &format!(
                "is weakly activated across house {} ({}) and house {} ({}).",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
            &format!(
                "est faiblement activé à travers les maisons {} ({}) et {} ({}).",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
            &format!(
                "está débilmente activado entre la casa {} ({}) y la casa {} ({}).",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
            &format!(
                "ist schwach über Haus {} ({}) und Haus {} ({}) aktiviert.",
                reference.house_a_number,
                reference.theme_a_code,
                reference.house_b_number,
                reference.theme_b_code
            ),
        ),
    }
}

fn localized_axis_hint(
    locale: &str,
    label: &str,
    en_tail: &str,
    fr_tail: &str,
    es_tail: &str,
    de_tail: &str,
) -> String {
    match locale {
        "fr" => format!("{label} {fr_tail}"),
        "es" => format!("{label} {es_tail}"),
        "de" => format!("{label} {de_tail}"),
        _ => format!("{label} {en_tail}"),
    }
}

fn add_score(draft: &mut HouseScoreDraft, score: f64, reason: BasicProjectionReason) {
    if score <= 0.0 {
        return;
    }
    draft.raw_score += score;
    add_reason(draft, reason);
}

fn add_reason(draft: &mut HouseScoreDraft, reason: BasicProjectionReason) {
    if !draft
        .reason_details
        .iter()
        .any(|existing| existing == &reason)
    {
        draft.reason_details.push(reason);
    }
}

fn push_signal_if_exists(target: &mut Vec<String>, signal_keys: &HashSet<&str>, signal_key: &str) {
    if signal_keys.contains(signal_key) {
        push_unique(target, signal_key.to_string());
    }
}

fn push_signal_prefix(target: &mut Vec<String>, signals: &[BasicSignal], prefix: &str) {
    for signal in signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with(prefix))
    {
        push_unique(target, signal.signal_key.clone());
    }
}

fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

fn push_unique_all(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        push_unique(target, value.clone());
    }
}

fn push_unique_reasons(target: &mut Vec<BasicProjectionReason>, values: &[BasicProjectionReason]) {
    for value in values {
        if !target.iter().any(|existing| existing == value) {
            target.push(value.clone());
        }
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
