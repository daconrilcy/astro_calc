use super::*;
pub(crate) fn horoscope_writer_engine_defaults(
    use_case: &GenerateReadingUseCase,
) -> EngineDefaults {
    let mut defaults = use_case.engine_defaults().clone();
    let Some(policy) = use_case.catalog().product_policy(HOROSCOPE_PRODUCT_CODE) else {
        return defaults;
    };
    if let Some(provider) = policy.default_provider.clone() {
        defaults.provider = provider;
    }
    if let Some(model) = policy
        .default_model
        .as_ref()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
    {
        defaults.model = model.to_string();
    }
    defaults
}
