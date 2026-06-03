pub(super) fn has_text(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.trim().is_empty())
}

pub(super) fn has_current_aspect_hint(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|text| {
        !text.contains(" by a opposition") && !text.contains(" are connected by ")
    })
}
