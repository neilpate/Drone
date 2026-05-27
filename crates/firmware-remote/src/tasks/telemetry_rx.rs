//! Remote <- drone telemetry RX over BLE 1Mbit raw mode.
//!
//! EXPERIMENT BRANCH (see ADR 0014). `main` uses IEEE 802.15.4; this branch
//! is a reference implementation proving BLE-mode works on the same hardware
//! once the embassy-nrf BLE driver's defaults are corrected:
//!
//!   1. RXADDRESSES = 0x01 — the driver's set_access_address() enables
//!      logical addresses 0..4 but only programs address 0. Addresses 1..4
//!      match zeros in RF noise and deliver garbage frames. We override
//!      RXADDRESSES *after* set_access_address() to enable only addr0.
//!
//!   2. CRCSTATUS gating — the driver's receive() completes on END, not
//!      CRCOK, so it returns Ok(()) for CRC-failed frames. We read
//!      CRCSTATUS ourselves and drop bad frames.
//!
//! Also requires HfclkSource::ExternalXtal in the board init — without it
//! the RADIO PLL cannot lock on a 2.4 GHz carrier reliably.

use crate::board::Radio;
use crate::radio_link;
use embassy_nrf::radio::ble::Mode;

#[embassy_executor::task]
pub async fn receive(mut radio: Radio) -> ! {
    defmt::info!("receive task (BLE): started");

    radio.set_mode(Mode::BLE_1MBIT);
    radio.set_header_expansion(false);
    radio.set_frequency(radio_link::FREQUENCY);
    radio.set_access_address(radio_link::ACCESS_ADDRESS);
    radio.set_whitening_init(radio_link::WHITENING_INIT);
    radio.set_crc_poly(radio_link::CRC_POLY);
    radio.set_crc_init(radio_link::CRC_INIT);

    // Fix #1: restrict RX address filter to addr0 only.
    // Safety: we hold the only handle to RADIO via Board. The peripheral is
    // in DISABLED state after the configuration calls above and no receive
    // is in flight yet, so this register write is race-free.
    {
        let r = unsafe { &*embassy_nrf::pac::RADIO::ptr() };
        r.rxaddresses.write(|w| w.addr0().enabled());
    }

    // Frame buffer: [0]=LENGTH, [1..]=payload. With header_expansion=false the
    // length byte can be up to 255, so size the buffer accordingly. We only
    // look at the first 5 bytes for our 4-byte counter payload.
    let mut buf = [0u8; 1 + 255];
    let mut ok = 0u32;
    let mut crc_bad = 0u32;

    loop {
        match radio.receive(&mut buf).await {
            Ok(()) => {
                // Fix #2: read CRCSTATUS ourselves. Driver completes on END,
                // not CRCOK, so a CRC-failed frame still shows up as Ok(()).
                let r = unsafe { &*embassy_nrf::pac::RADIO::ptr() };
                let crc_ok = r.crcstatus.read().crcstatus().is_crcok();

                if !crc_ok {
                    crc_bad = crc_bad.wrapping_add(1);
                    if crc_bad.is_multiple_of(50) {
                        defmt::warn!("receive task: CRC fails so far: {}", crc_bad);
                    }
                    continue;
                }

                let len = buf[0] as usize;
                if len == 4 {
                    let payload: [u8; 4] = buf[1..5].try_into().unwrap();
                    let count = u32::from_le_bytes(payload);
                    ok = ok.wrapping_add(1);
                    defmt::info!(
                        "receive task: count={} (ok={} crc_bad={})",
                        count,
                        ok,
                        crc_bad
                    );
                } else {
                    defmt::info!("receive task: unexpected len={}", len);
                }
            }
            Err(e) => {
                defmt::warn!("receive task: error: {:?}", e);
            }
        }
    }
}
