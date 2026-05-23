//! BBC micro:bit v2 (nRF52833) board support.
//!
//! Phase 1–3 development target. Sensors and actuators are breadboarded onto
//! the edge connector; only the on-board 5x5 LED matrix is used directly.
//!
//! See [ADR 0010](../../../../doc/decisions/0010-board-support-package.md) for
//! the contract this module satisfies.

pub const NAME: &str = "BBC micro:bit v2";

super::board_pins! {
    /// Heartbeat LED — row 1 of the 5×5 matrix (P0.21). Held HIGH for the
    /// lifetime of the heartbeat task.
    heartbeat_row: P0_21,
    /// Heartbeat LED — col 1 of the 5×5 matrix (P0.28). Toggled to blink.
    heartbeat_col: P0_28,
}
