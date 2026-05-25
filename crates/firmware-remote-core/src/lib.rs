#![no_std]

//! # firmware-remote-core
//!
//! Pure logic for the remote/ground-station firmware. Host-testable per ADR 0007.
//!
//! ## Rules
//!
//! - `#![no_std]` — runs on the target.
//! - Host-runnable — `cargo test -p firmware-remote-core` from the repo root
//!   must work on the developer's workstation with no extra flags.
//! - **No HAL, no async runtime, no device logging.** Enforced by the absence
//!   of those crates from this crate's `Cargo.toml` (see ADR 0009).
//!
//! Submodules land here as the firmware grows: input mapping, telemetry
//! parsing, framing, display formatting. Each submodule is a pure function
//! of its inputs; the matching `firmware-remote` task does the I/O.
