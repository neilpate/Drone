//! Radio link parameters shared by drone TX and remote RX.
//!
//! EXPERIMENT BRANCH: BLE 1Mbit mode. See ADR 0014 — main uses IEEE 802.15.4.
//! This branch verifies the hypothesis that BLE-mode failure was driver
//! multi-address + missing CRC gating + missing HFCLK xtal, not RF.
//!
//! Duplicated in firmware-drone and firmware-remote until a shared crate
//! is warranted — see issue #5.

/// Carrier frequency in MHz (driver expects actual MHz, range 2400..=2500).
/// 2440 MHz = middle of 2.4 GHz ISM, clear of BLE adv channels.
pub const FREQUENCY: u32 = 2440;

/// 4-byte access address. Arbitrary; avoids BLE adv address 0x8E89BED6.
pub const ACCESS_ADDRESS: u32 = 0x7176_4129;

/// CRC polynomial (standard BLE CRC-24).
pub const CRC_POLY: u32 = 0x0000_065B;

/// CRC seed (arbitrary 24-bit; must match on both ends).
pub const CRC_INIT: u32 = 0x0055_5555;

/// Data-whitening init value (arbitrary 7-bit; must match on both ends).
pub const WHITENING_INIT: u8 = 0x40;
