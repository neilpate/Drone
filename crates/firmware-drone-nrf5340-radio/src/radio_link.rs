//! Radio link parameters shared by drone TX and remote RX.
//!
//! Using the embassy IEEE 802.15.4 driver. Only the channel needs to match
//! on both sides; CRC, whitening and framing are fixed by the standard.
//!
//! Duplicated in firmware-drone and firmware-remote until a shared crate
//! is warranted — see issue #5.

/// IEEE 802.15.4 channel. Valid range 11..=26 (2405..2480 MHz, 5 MHz steps).
/// Channel 20 = 2450 MHz, middle of the band.
pub const CHANNEL: u8 = 20;
