//! Inject stage: insert formatted text into the focused app.
//!
//! Dictation uses direct synthetic text input first because many games ignore
//! clipboard paste. Email drafts keep clipboard + Ctrl+V because long,
//! multiline text is safer in normal editors.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectMethod {
    /// Clipboard write succeeded AND the simulated Ctrl+V keystroke fired.
    Enigo,
    /// Text was typed directly with synthetic Unicode keyboard input.
    DirectText,
    /// Clipboard write succeeded but the simulated keystroke failed; the
    /// user must paste manually.
    ClipboardOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectStrategy {
    /// Clipboard + Ctrl+V. Best for long drafts in normal text editors.
    ClipboardPaste,
    /// Synthetic text input first, clipboard paste fallback. Best for games.
    DirectText,
}

/// Insert `text` using the requested strategy.
///
/// Clipboard write failure is fatal only if both direct text and clipboard
/// paste fail. Keystroke-only failures degrade to ClipboardOnly so the text is
/// still available for manual paste.
pub fn insert(text: &str, strategy: InjectStrategy) -> Result<InjectMethod, String> {
    if text.is_empty() {
        return Ok(InjectMethod::Enigo);
    }

    if strategy == InjectStrategy::DirectText {
        match enigo_type_text(text) {
            Ok(()) => return Ok(InjectMethod::DirectText),
            Err(e) => tracing::warn!(
                error = %e,
                "direct text input failed; falling back to clipboard paste"
            ),
        }
    }

    paste(text)
}

/// Copy `text` to the clipboard and best-effort simulate Ctrl+V.
pub fn paste(text: &str) -> Result<InjectMethod, String> {
    if text.is_empty() {
        return Ok(InjectMethod::Enigo);
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("clipboard init: {e}"))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("clipboard set: {e}"))?;

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

#[cfg(target_os = "windows")]
fn enigo_type_text(text: &str) -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut e = Enigo::new(&Settings::default()).map_err(|err| err.to_string())?;

    for ch in text.chars() {
        match ch {
            '\n' => {
                e.key(Key::Return, Direction::Click)
                    .map_err(|err| err.to_string())?;
            }
            '\r' => {}
            '\t' => {
                e.key(Key::Tab, Direction::Click)
                    .map_err(|err| err.to_string())?;
            }
            '\0' => return Err("text contains null byte".to_string()),
            _ => {
                if e.key(Key::Unicode(ch), Direction::Click).is_err() {
                    e.text(&ch.to_string()).map_err(|err| err.to_string())?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn enigo_paste() -> Result<(), String> {
    Err("enigo paste unsupported on this OS in Phase 2 (Typr is Windows-only)".to_string())
}

#[cfg(not(target_os = "windows"))]
fn enigo_type_text(_text: &str) -> Result<(), String> {
    Err("direct text input unsupported on this OS in Phase 2 (Typr is Windows-only)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_is_noop_returning_enigo() {
        let m = insert("", InjectStrategy::DirectText).unwrap();
        assert_eq!(m, InjectMethod::Enigo);
    }

    #[test]
    fn inject_method_is_clone_and_eq() {
        let a = InjectMethod::ClipboardOnly;
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(a, InjectMethod::Enigo);
    }

    #[test]
    fn inject_strategy_is_copy_and_eq() {
        let a = InjectStrategy::DirectText;
        let b = a;
        assert_eq!(a, b);
        assert_ne!(a, InjectStrategy::ClipboardPaste);
    }
}
