//! Board Support Package — physical → logical peripheral mapping.
//!
//! See [ADR 0010](../../../../doc/decisions/0010-board-support-package.md) for the
//! design rationale and contract every board module must satisfy.
//!
//! Exactly one `board-*` Cargo feature must be active per build. Add a new
//! board by creating a sibling file (e.g. `drone_rev_a.rs`), exposing the
//! same public surface (`NAME`, `Board`, `Board::new`), and gating it under
//! a new `board-<name>` feature.

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
///     /// Heartbeat LED row.
///     heartbeat_row: P0_21,
///     /// Heartbeat LED column.
///     heartbeat_col: P0_28,
/// }
/// ```
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
