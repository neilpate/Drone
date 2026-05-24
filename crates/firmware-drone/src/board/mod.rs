//! Board Support Package — physical → logical peripheral mapping.
//!
//! See [ADR 0010](../../../../doc/decisions/0010-board-support-package.md) for the
//! design rationale and contract every board module must satisfy.
//!
//! Exactly one `board-*` Cargo feature must be active per build. Add a new
//! board by creating a sibling file (e.g. `drone_rev_a.rs`) exposing the
//! same public surface as the existing boards (`NAME`, `Board`,
//! `Board::new`, plus every wrapper type referenced by a task signature —
//! currently `StatusLed`), and gating it under a new `board-<name>` feature.

/// Generate the `Board` struct and its `new(p)` constructor from a list of
/// `logical_name: PERIPHERAL_FIELD` pairs.
///
/// Each pair declares one logical role and its physical mapping. Doc
/// comments and other attributes on each entry are attached to the
/// generated field. The macro keeps every pin/peripheral name written
/// exactly once.
///
/// ```ignore
/// board_pins! {
///     /// IMU SPI clock.
///     imu_sck:  P0_17,
///     /// IMU SPI MOSI.
///     imu_mosi: P0_13,
///     /// IMU SPI MISO.
///     imu_miso: P0_01,
///     /// IMU chip-select.
///     imu_cs:   P0_02,
///     /// IMU INT1 (data-ready) line.
///     imu_int1: P0_03,
/// }
/// ```
//
// Currently unused: the only board (micro:bit v2) hand-rolls its `Board`
// struct because every logical role is a wrapper type (`StatusLed`), not a
// raw peripheral. Kept for the next batch of peripherals (SPI / I2C / INT
// pins for the IMU), which *will* be raw fields and benefit from the
// declarative form. Remove the `#[allow(...)]` attributes — and this note —
// the moment the first real invocation lands.
//
// Known limitation: the macro generates the `Board` struct itself, so it
// can't coexist with a hand-rolled `Board` in the same module. When the
// IMU pins land, either fold the macro's output into the hand-rolled
// `Board` (likely via a sub-struct shape: `board_pins!(struct ImuPins { ... })`)
// or drop the macro entirely.
#[allow(unused_macros)]
macro_rules! board_pins {
    ($(
        $(#[$attr:meta])*
        $field:ident : $pin:ident
    ),* $(,)?) => {
        /// Logical peripherals exposed by this board.
        ///
        /// Field names identify roles, not pins. Pin/peripheral types
        /// vary per board; field names do not. Tasks consume these by
        /// role and never see the physical mapping.
        pub struct Board {
            $(
                $(#[$attr])*
                pub $field: ::embassy_nrf::peripherals::$pin,
            )*
        }

        impl Board {
            /// Consume the raw `embassy_nrf` peripherals struct and
            /// surface only the logical roles this board provides.
            pub fn new(p: ::embassy_nrf::Peripherals) -> Self {
                Self { $( $field: p.$pin, )* }
            }
        }
    };
}
#[allow(unused_imports)]
pub(crate) use board_pins;

#[cfg(feature = "board-microbit-v2")]
mod microbit_v2;
#[cfg(feature = "board-microbit-v2")]
pub use microbit_v2::*;

#[cfg(not(any(feature = "board-microbit-v2")))]
compile_error!(
    "no board selected: enable exactly one `board-*` feature on the `firmware-drone` crate \
     (e.g. `--features board-microbit-v2`)"
);
