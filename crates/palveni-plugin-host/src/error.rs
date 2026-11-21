#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    #[error("plugin.toml is not valid: {0}")]
    Parse(String),

    #[error("invalid `{field}`: {reason}")]
    Invalid { field: String, reason: String },

    #[error("plugin requires contract {required}, this server provides {available}")]
    Incompatible { required: String, available: String },
}

impl ManifestError {
    pub(crate) fn invalid(field: impl Into<String>, reason: impl Into<String>) -> Self {
        ManifestError::Invalid {
            field: field.into(),
            reason: reason.into(),
        }
    }
}
