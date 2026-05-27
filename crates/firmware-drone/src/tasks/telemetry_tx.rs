//! Drone -> remote telemetry TX over BLE 1Mbit raw mode.
//!
//! EXPERIMENT BRANCH (see ADR 0014). Sends a length-prefixed packet whose
//! payload is a little-endian u32 counter.

use crate::board::Radio;
use crate::radio_link;
use embassy_nrf::radio::ble::Mode;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn transmit(mut radio: Radio) -> ! {
    defmt::info!("transmit task (BLE): started");

    radio.set_mode(Mode::BLE_1MBIT);
    radio.set_header_expansion(false);
    radio.set_frequency(radio_link::FREQUENCY);
    radio.set_access_address(radio_link::ACCESS_ADDRESS);
    radio.set_whitening_init(radio_link::WHITENING_INIT);
    radio.set_crc_poly(radio_link::CRC_POLY);
    radio.set_crc_init(radio_link::CRC_INIT);

    // BLE-mode raw frame, header_expansion=false:
    //   [0] = LENGTH byte (payload bytes that follow)
    //   [1..] = payload
    let mut buf = [0u8; 1 + 4];
    buf[0] = 4;

    let mut count = 0_u32;

    loop {
        buf[1..5].copy_from_slice(&count.to_le_bytes());

        if let Err(e) = radio.transmit(&buf).await {
            defmt::error!("transmit task: error: {:?}", e);
            continue;
        }
        defmt::info!("transmit task: sent {}", count);
        count = count.wrapping_add(1);

        Timer::after_millis(100).await;
    }
}
