use astral_llm_domain::{SafetyPolicy, SafetyPolicyOverride};

pub struct SafetyResolver;

impl SafetyResolver {
    pub fn resolve(
        product_default: &SafetyPolicy,
        request_override: Option<&SafetyPolicyOverride>,
    ) -> SafetyPolicy {
        let mandatory = SafetyPolicy::mandatory();
        let with_product = SafetyPolicy::merge(&mandatory, product_default);
        match request_override {
            Some(override_policy) => SafetyPolicy::merge(&with_product, override_policy),
            None => with_product,
        }
    }

    pub fn product_default_for(
        _product_code: &str,
        interpretation: Option<
            &crate::interpretation_profile_resolver::ResolvedInterpretationContext,
        >,
    ) -> SafetyPolicy {
        let mut policy = SafetyPolicy::mandatory();
        if interpretation
            .map(|ctx| ctx.profile.require_disclaimer())
            .unwrap_or(false)
        {
            policy.require_disclaimer = true;
        }
        policy
    }
}
