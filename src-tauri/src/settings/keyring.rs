//! Groq API key storage — Windows Credential Manager via `keyring` v3.
//!
//! All callers go through `KeyringBackend` so the migrator + loader are testable
//! without touching the OS. Service/user strings are hard-coded per spec.

use std::sync::Mutex;

pub const SERVICE: &str = "com.typr.app";
pub const USER: &str = "groq_api_key";

#[derive(thiserror::Error, Debug)]
pub enum KeyringError {
    #[error("keyring entry not found")]
    NotFound,
    #[error("keyring access denied")]
    AccessDenied,
    #[error("keyring backend failure: {0}")]
    Other(String),
}

pub trait KeyringBackend: Send + Sync {
    fn get(&self) -> Result<Option<String>, KeyringError>;
    fn set(&self, secret: &str) -> Result<(), KeyringError>;
    fn delete(&self) -> Result<(), KeyringError>;
}

pub struct SystemBackend;

impl KeyringBackend for SystemBackend {
    fn get(&self) -> Result<Option<String>, KeyringError> {
        let entry =
            ::keyring::Entry::new(SERVICE, USER).map_err(|e| KeyringError::Other(e.to_string()))?;
        match entry.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(::keyring::Error::NoEntry) => Ok(None),
            Err(::keyring::Error::PlatformFailure(e)) => Err(KeyringError::Other(e.to_string())),
            Err(e) => Err(KeyringError::Other(e.to_string())),
        }
    }

    fn set(&self, secret: &str) -> Result<(), KeyringError> {
        let entry =
            ::keyring::Entry::new(SERVICE, USER).map_err(|e| KeyringError::Other(e.to_string()))?;
        entry
            .set_password(secret)
            .map_err(|e| KeyringError::Other(e.to_string()))
    }

    fn delete(&self) -> Result<(), KeyringError> {
        let entry =
            ::keyring::Entry::new(SERVICE, USER).map_err(|e| KeyringError::Other(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(::keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(KeyringError::Other(e.to_string())),
        }
    }
}

/// In-memory backend used by unit + integration tests.
#[derive(Default)]
pub struct MockBackend {
    inner: Mutex<Option<String>>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_secret(secret: &str) -> Self {
        Self {
            inner: Mutex::new(Some(secret.to_string())),
        }
    }

    pub fn peek(&self) -> Option<String> {
        self.inner.lock().unwrap().clone()
    }
}

impl KeyringBackend for MockBackend {
    fn get(&self) -> Result<Option<String>, KeyringError> {
        Ok(self.inner.lock().unwrap().clone())
    }

    fn set(&self, secret: &str) -> Result<(), KeyringError> {
        *self.inner.lock().unwrap() = Some(secret.to_string());
        Ok(())
    }

    fn delete(&self) -> Result<(), KeyringError> {
        *self.inner.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_get_returns_none_when_empty() {
        let m = MockBackend::new();
        assert_eq!(m.get().unwrap(), None);
    }

    #[test]
    fn mock_set_then_get() {
        let m = MockBackend::new();
        m.set("sk-test-123").unwrap();
        assert_eq!(m.get().unwrap().as_deref(), Some("sk-test-123"));
    }

    #[test]
    fn mock_delete_removes_secret() {
        let m = MockBackend::with_secret("sk-test-123");
        m.delete().unwrap();
        assert_eq!(m.get().unwrap(), None);
    }

    #[test]
    fn mock_overwrites() {
        let m = MockBackend::with_secret("old");
        m.set("new").unwrap();
        assert_eq!(m.get().unwrap().as_deref(), Some("new"));
    }

    #[test]
    fn service_user_constants_match_spec() {
        assert_eq!(SERVICE, "com.typr.app");
        assert_eq!(USER, "groq_api_key");
    }
}
