#![cfg_attr(not(test), no_std)]

//! # firmware-drone-shared
//!
//! Board-agnostic firmware layer shared across drone targets (micro:bit,
//! nRF5340). Holds the Embassy task bodies and the global signal statics that
//! sit between the pure `firmware-drone-core` logic and the per-board binaries.
//! See ADR 0029.
//!
//! ## Rules
//!
//! - Depends only on chip-agnostic crates (`embassy-executor`, `embassy-sync`,
//!   `embassy-time`) plus `firmware-drone-core` / `firmware-types` / `defmt`.
//!   Never `embassy-nrf`, a HAL/PAC, `defmt-rtt`, or `panic-probe` — those are
//!   per-binary. See ADR 0029 §4.
//! - Pure, host-testable logic belongs in `firmware-drone-core`, not here: this
//!   crate pulls in the async runtime and is not host-tested. See ADR 0029 §11.

pub mod signals {
    pub mod attitude;
    pub mod imu_data;
}

pub mod tasks {
    pub mod attitude_estimator;
}
