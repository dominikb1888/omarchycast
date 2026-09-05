//! Omarchycast search daemon.
//!
//! Headless by design: the UI is a Quickshell overlay that runs inside the shell
//! process already on screen, so this process owns only the index, the matching
//! and the actions. That keeps the resident cost to a few megabytes.
//!
//! The binary in `main.rs` wires these modules together; this library crate
//! exists so integration tests can drive the providers directly.

pub mod clipboard;
pub mod config;
pub mod core;
pub mod hypr;
pub mod ipc;
pub mod launch;
pub mod limits;
pub mod safeio;
pub mod providers;