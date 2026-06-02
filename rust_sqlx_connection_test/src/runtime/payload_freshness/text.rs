pub(super) fn has_text(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.trim().is_empty())
}

pub(super) fn has_current_aspect_hint(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|text| {
        !text.contains(" by a opposition") && !text.contains(" are connected by ")
    })
}

pub(super) fn has_current_drafting_language(item: &crate::domain::BasicDraftingPlanItem) -> bool {
    let fields = std::iter::once(item.section_title.as_str())
        .chain(std::iter::once(item.writing_objective.as_str()))
        .chain(item.avoid.iter().map(String::as_str));

    fields
        .into_iter()
        .all(|field| !contains_non_ascii_letter(field))
}

fn contains_non_ascii_letter(text: &str) -> bool {
    text.chars()
        .any(|character| character.is_alphabetic() && !character.is_ascii())
}
