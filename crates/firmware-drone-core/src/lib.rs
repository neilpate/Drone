#![cfg_attr(not(test), no_std)]

//! # firmware-drone-core
//!
//! Pure logic for the drone firmware. Host-testable per ADR 0007.
//!
//! ## Rules
//!
//! - `#![no_std]` on target, `std` only enabled for `cargo test` (per ADR 0015).
//! - Host-runnable — `cargo test -p firmware-drone-core` from the repo root
//!   must work on the developer's workstation with no extra flags.
//! - **No HAL, no async runtime, no device logging.** Enforced by the absence
//!   of those crates from this crate's `Cargo.toml` (see ADR 0009).
//!
//! Submodules land here as the firmware grows: attitude estimation, motor
//! mixing, control loops, framing, sensor models. Each submodule is a pure
//! function of its inputs; the matching `firmware-drone` task does the I/O.

pub mod control_system;
pub mod mixer;
pub mod sensor_fusion;
pub mod supervisor_core;
