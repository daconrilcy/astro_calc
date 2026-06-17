pub(crate) struct CanonicalAxis {
    pub houses: [i32; 2],
    pub theme_codes: [&'static str; 2],
}

pub(crate) fn canonical_axis(axis_code: &str) -> Option<CanonicalAxis> {
    let (houses, theme_codes) = match axis_code {
        "self_relationship" => ([1, 7], ["identity", "relationships"]),
        "resources_sharing" => ([2, 8], ["resources", "shared_resources"]),
        "local_distant" => ([3, 9], ["communication", "beliefs"]),
        "private_public" => ([4, 10], ["roots", "career"]),
        "creation_collective" => ([5, 11], ["creativity", "community"]),
        "control_surrender" => ([6, 12], ["work_health", "inner_world"]),
        _ => return None,
    };

    Some(CanonicalAxis {
        houses,
        theme_codes,
    })
}

pub(crate) fn axis_label(axis_code: &str) -> &'static str {
    match axis_code {
        "self_relationship" => "Self and Relationship",
        "resources_sharing" => "Resources and Sharing",
        "local_distant" => "Local and Distant",
        "private_public" => "Private and Public",
        "creation_collective" => "Creation and Collective",
        "control_surrender" => "Control and Surrender",
        _ => "",
    }
}
