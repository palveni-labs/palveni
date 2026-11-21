use std::io::IsTerminal;

use anyhow::{Context, anyhow};
use tracing_subscriber::EnvFilter;

/// Initialises structured logging with a `tracing` filter directive.
/// Colours are only used on a terminal, so container and service logs stay
/// plain text.
pub fn init(filter: &str) -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_new(filter).with_context(|| format!("invalid log level `{filter}`"))?;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_ansi(std::io::stdout().is_terminal())
        .try_init()
        .map_err(|e| anyhow!("cannot initialise logging: {e}"))
}
