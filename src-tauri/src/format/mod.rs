//! Format rules v2 — pure data transforms over transcribed text.
//!
//! Each rule module exposes a single public entry point; orchestration
//! happens in `pipeline::format::run_format`.

pub mod fillers;
pub mod commands;
