//! Inject stage — copy formatted text to the clipboard and simulate Ctrl+V.
//!
//! Phase 2 mirrors the behaviour of the legacy `crate::paste::paste_text`
//! helper but returns a structured [`InjectMethod`] so the orchestrator can
//! record telemetry on whether the simulated keystroke worked. The legacy
//! `paste.rs` module stays untouched in T14 and is removed in T15 as part
//! of the Tauri command cutover.
//!
//! Behaviour notes:
//! - Empty `text` is a no-op that returns `Enigo` — there is nothing to
//!   paste, but the call is still considered successful.
//! - Clipboard write failures are hard errors (`Err(String)`).
//! - Enigo keystroke failures are soft: we already succeeded at putting
//!   the text on the clipboard, so we degrade to `ClipboardOnly` and let
//!   the user paste manually. This matches Wispr's "best effort" inject.
//! - On non-Windows platforms the keystroke path returns an error, which
//!   degrades to `ClipboardOnly`. Typr is Windows-only, but keeping the
//!   conditional lets the crate compile under `cargo test` in mixed-OS
//!   sandboxes.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectMethod {
    /// Clipboard write succeeded AND the simulated Ctrl+V keystroke fired.
    Enigo,
    /// Clipboard write succeeded but the simulated keystroke failed; the
    /// user must paste manually.
    ClipboardOnly,
}

/// Copy `text` to the clipboard and (best effort) simulate Ctrl+V into the
/// foreground window.
///
/// Returns the [`InjectMethod`] actually used. A clipboard-write failure
/// surfaces as `Err(String)`; a keystroke-only failure is logged at warn and
/// downgraded to `Ok(InjectMethod::ClipboardOnly)`.
pub fn paste(text: &str) -> Result<InjectMethod, String> {
    if text.is_empty() {
        // Nothing to inject. Treat as a successful no-op so the orchestrator
        // can keep going to the persist stage.
        return Ok(InjectMethod::Enigo);
    }

    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| format!("clipboard init: {e}"))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("clipboard set: {e}"))?;

    // Tiny delay so the OS commits the new clipboard contents before we
    // synthesise the Ctrl+V keystroke. Mirrors `paste.rs` (50ms there;
    // 30ms is enough in practice and keeps perceived latency low).
    std::thread::sleep(std::time::Duration::from_millis(30));

    match enigo_paste() {
        Ok(()) => Ok(InjectMethod::Enigo),
        Err(e) => {
            tracing::warn!(error = %e, "enigo paste failed; user must paste manually");
            Ok(InjectMethod::ClipboardOnly)
        }
    }
}

#[cfg(target_os = "windows")]
fn enigo_paste() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut e = Enigo::new(&Settings::default()).map_err(|err| err.to_string())?;
    e.key(Key::Control, Direction::Press)
        .map_err(|err| err.to_string())?;
    e.key(Key::Unicode('v'), Direction::Click)
        .map_err(|err| err.to_string())?;
    e.key(Key::Control, Direction::Release)
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn enigo_paste() -> Result<(), String> {
    Err("enigo paste unsupported on this OS in Phase 2 (Typr is Windows-only)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_is_noop_returning_enigo() {
        // Empty input must not touch the clipboard and must succeed; this
        // keeps the orchestrator's persist stage reachable for sessions
        // where formatting drops every token.
        let m = paste("").unwrap();
        assert_eq!(m, InjectMethod::Enigo);
    }

    #[test]
    fn inject_method_is_clone_and_eq() {
        let a = InjectMethod::ClipboardOnly;
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(a, InjectMethod::Enigo);
    }
}
