//! Settings module. v1 shim today; v2 struct, keyring backend, migrator land in later tasks.

mod legacy_v1;

pub use legacy_v1::Settings;
