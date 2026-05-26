//! Radio link parameters shared by drone TX and remote RX.
//!
//! Both sides must initialise their `Radio` with these identical values
//! or packets are silently dropped (CRC fails, address mismatch, or
//! whitening goes out of phase). Mismatches manifest as RX-side silence.
//!
//! Duplicated in firmware-drone and firmware-remote until a shared
//! crate is warranted — see issue #5.

/// Centre frequency in MHz. Mid-band, away from common WiFi traffic.
pub const FREQUENCY_MHZ: u32 = 2450;

/// 32-bit access address. Acts as the network ID — packets with a
/// different address are filtered out by hardware before reaching the
/// CPU. Arbitrary chosen value; no relation to BLE advertising address.
pub const ACCESS_ADDRESS: u32 = 0xDEAD_BEEF;

/// Initial state of the data-whitening LFSR. Must match on both sides;
/// see AGENTS.md / issue #5 explanation of whitening.
pub const WHITENING_INIT: u8 = 0x42;

/// Maximum payload size (excluding the leading length byte and the
/// hardware-appended CRC). The counter payload is 4 bytes; we round up
/// to give a bit of headroom without committing to a real protocol.
pub const MAX_PAYLOAD_BYTES: usize = 8;
