//! Tracing init + log sanitisation policy.
//!
//! **LOG SANITISATION — MANDATORY.** Never record any of these fields in a span:
//!   - `raw_text`, `final_text`, `command_selection`, `app_context` (user content / PII)
//!   - `groq_api_key` or any credential string
//!   - Absolute paths containing the Windows username (log relative paths or file names only)
//!
//! Instead, log numeric/categorical fields only: `duration_ms`, `byte_len`, `char_count`,
//! `stage`, `engine`, `mode`, error kind discriminants. Use
//! `#[tracing::instrument(skip(...))]` on every pipeline fn to enforce.
//!
//! Enforced by a presence test introduced in Task 11.

use std::path::Path;

pub fn init_tracing(log_dir: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    std::fs::create_dir_all(log_dir)?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "typr.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // Leak the guard on purpose — process-lifetime logger.
    Box::leak(Box::new(guard));

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("info,{}=debug", env!("CARGO_CRATE_NAME"))));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).with_writer(std::io::stdout))
        .with(
            fmt::layer()
                .with_target(false)
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .try_init()
        .ok(); // ignore double-init in tests
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn init_tracing_returns_ok_on_writable_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = super::init_tracing(tmp.path());
        assert!(result.is_ok(), "init_tracing should succeed");
    }
}
