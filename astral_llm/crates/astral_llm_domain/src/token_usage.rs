use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenUsageType {
    Input,
    Output,
    Cache,
    Reasoning,
}

impl TokenUsageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
            Self::Cache => "cache",
            Self::Reasoning => "reasoning",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenUsageItem {
    pub usage_type: TokenUsageType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_subtype: Option<String>,
    pub token_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_metric_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_price_usd_per_mtok: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct TokenUsage {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<TokenUsageItem>,
}

impl TokenUsage {
    pub fn simple(input_tokens: u32, output_tokens: u32) -> Self {
        let mut usage = Self::default();
        if input_tokens > 0 {
            usage.push(TokenUsageItem {
                usage_type: TokenUsageType::Input,
                usage_subtype: None,
                token_count: input_tokens,
                provider_metric_name: Some("input_tokens".into()),
                unit_price_usd_per_mtok: None,
                estimated_cost_usd: None,
            });
        }
        if output_tokens > 0 {
            usage.push(TokenUsageItem {
                usage_type: TokenUsageType::Output,
                usage_subtype: None,
                token_count: output_tokens,
                provider_metric_name: Some("output_tokens".into()),
                unit_price_usd_per_mtok: None,
                estimated_cost_usd: None,
            });
        }
        usage
    }

    pub fn push(&mut self, item: TokenUsageItem) {
        self.items.push(item);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn tokens_for(&self, usage_type: TokenUsageType) -> Option<u32> {
        self.items
            .iter()
            .filter(|item| item.usage_type == usage_type)
            .map(|item| item.token_count)
            .reduce(|a, b| a.saturating_add(b))
    }

    pub fn cost_for(&self, usage_type: TokenUsageType) -> Option<f64> {
        self.items
            .iter()
            .filter(|item| item.usage_type == usage_type)
            .filter_map(|item| item.estimated_cost_usd)
            .reduce(|a, b| a + b)
    }

    pub fn with_pricing(
        &self,
        pricing: &TokenPricing,
        pricing_source: Option<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> PublicTokenUsage {
        PublicTokenUsage {
            summary: TokenUsageSummary {
                input_tokens: self.tokens_for(TokenUsageType::Input),
                output_tokens: self.tokens_for(TokenUsageType::Output),
                cache_tokens: self.tokens_for(TokenUsageType::Cache),
                reasoning_tokens: self.tokens_for(TokenUsageType::Reasoning),
            },
            cost: TokenCostSummary {
                currency: pricing.currency.clone(),
                estimated_total: self
                    .items
                    .iter()
                    .filter_map(|item| item.estimated_cost_usd)
                    .reduce(|a, b| a + b),
                input_cost: self.cost_for(TokenUsageType::Input),
                output_cost: self.cost_for(TokenUsageType::Output),
                cache_cost: self.cost_for(TokenUsageType::Cache),
                reasoning_cost: self.cost_for(TokenUsageType::Reasoning),
            },
            engine: TokenUsageEngine {
                provider: provider.into(),
                model: model.into(),
                pricing_source,
            },
            details: self.items.clone(),
        }
    }

    pub fn priced(&self, pricing: &TokenPricing) -> Self {
        let mut priced = self.clone();
        for item in &mut priced.items {
            let unit_price = match item.usage_type {
                TokenUsageType::Input => pricing.input_price_usd_per_mtok,
                TokenUsageType::Output => pricing.output_price_usd_per_mtok,
                TokenUsageType::Reasoning => pricing.reasoning_price_usd_per_mtok,
                TokenUsageType::Cache => match item.usage_subtype.as_deref() {
                    Some("write") => pricing.cache_write_price_usd_per_mtok,
                    _ => pricing.cache_read_price_usd_per_mtok,
                },
            };
            item.unit_price_usd_per_mtok = unit_price;
            item.estimated_cost_usd = unit_price.map(|price| {
                (item.token_count as f64 / 1_000_000.0) * price
            });
        }
        priced
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenPricing {
    pub currency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_price_usd_per_mtok: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_price_usd_per_mtok: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_price_usd_per_mtok: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_price_usd_per_mtok: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_price_usd_per_mtok: Option<f64>,
}

impl Default for TokenPricing {
    fn default() -> Self {
        Self {
            currency: "USD".into(),
            input_price_usd_per_mtok: None,
            output_price_usd_per_mtok: None,
            cache_read_price_usd_per_mtok: None,
            cache_write_price_usd_per_mtok: None,
            reasoning_price_usd_per_mtok: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenUsageSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenCostSummary {
    pub currency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_total: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenUsageEngine {
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PublicTokenUsage {
    pub summary: TokenUsageSummary,
    pub cost: TokenCostSummary,
    pub engine: TokenUsageEngine,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<TokenUsageItem>,
}
