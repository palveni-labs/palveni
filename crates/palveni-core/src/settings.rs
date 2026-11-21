use serde::{Deserialize, Deserializer, Serialize};

use crate::PerfTier;

/// Administrator overrides. Null restores boot configuration or host detection.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RuntimeSettings {
    #[schema(required = true)]
    #[serde(deserialize_with = "nullable")]
    pub server_name: Option<String>,
    #[schema(required = true)]
    #[serde(deserialize_with = "nullable")]
    pub perf_tier: Option<PerfTier>,
}

// Both fields must be present, including when PUT clears an override.
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}
