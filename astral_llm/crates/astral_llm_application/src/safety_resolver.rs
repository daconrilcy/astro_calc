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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_cannot_weaken_mandatory_rules() {
        let weakened = SafetyPolicy {
            forbid_medical_advice: false,
            forbid_legal_advice: false,
            forbid_financial_advice: false,
            forbid_death_prediction: false,
            forbid_pregnancy_prediction: false,
            forbid_deterministic_claims: false,
            require_symbolic_framing: false,
            require_disclaimer: false,
            custom_forbidden_topics: vec![],
        };

        let effective = SafetyResolver::resolve(
            &SafetyResolver::product_default_for("natal_prompter", None),
            Some(&weakened),
        );

        assert!(effective.forbid_medical_advice);
        assert!(effective.require_symbolic_framing);
    }

    #[test]
    fn override_can_strengthen_rules() {
        let stronger = SafetyPolicy {
            custom_forbidden_topics: vec!["politique".to_string()],
            ..SafetyPolicy::mandatory()
        };

        let effective = SafetyResolver::resolve(
            &SafetyResolver::product_default_for("natal_prompter", None),
            Some(&stronger),
        );

        assert!(effective
            .custom_forbidden_topics
            .iter()
            .any(|t| t == "politique"));
    }
}
