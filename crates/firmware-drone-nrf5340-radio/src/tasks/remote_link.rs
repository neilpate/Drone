use embassy_futures::select::{Either, select};
use embassy_nrf::peripherals;
use embassy_nrf::radio::ieee802154::{Packet, Radio};
use embassy_time::{Duration, with_timeout};
use nrf5340_ipc::{Endpoint, MAX_FRAME_SIZE, RADIO_CHANNEL, STARTUP_PROBE};

const _: () = assert!(MAX_FRAME_SIZE == Packet::CAPACITY as usize);

#[embassy_executor::task]
pub async fn remote_link(mut radio: Radio<'static, peripherals::RADIO>) -> ! {
    let mut endpoint = Endpoint::attach().expect("invalid IPC shared-memory header");
    let mut payload = [0; MAX_FRAME_SIZE];
    let length = with_timeout(Duration::from_secs(2), endpoint.receive(&mut payload))
        .await
        .expect("IPC startup probe timeout")
        .expect("IPC startup probe receive failed");
    assert_eq!(
        &payload[..length],
        STARTUP_PROBE,
        "IPC startup probe mismatch"
    );
    endpoint
        .try_send(&payload[..length])
        .expect("IPC startup probe echo failed");
    defmt::info!("radio-core IPC loopback passed; channel {}", RADIO_CHANNEL);

    radio.set_channel(RADIO_CHANNEL);
    let mut rx_packet = Packet::new();
    let mut tx_packet = Packet::new();
    loop {
        // Radio::receive is cancellation-safe; an outgoing IPC frame preempts RX.
        match select(
            radio.receive(&mut rx_packet),
            endpoint.receive(&mut payload),
        )
        .await
        {
            Either::First(Ok(())) => {
                if let Err(error) = endpoint.try_send(&rx_packet) {
                    defmt::warn!("radio RX dropped by IPC: {:?}", error);
                }
            }
            Either::First(Err(error)) => defmt::trace!("radio RX error: {:?}", error),
            Either::Second(Ok(length)) => {
                tx_packet.copy_from_slice(&payload[..length]);
                if let Err(error) = radio.try_send(&mut tx_packet).await {
                    defmt::warn!("radio TX failed: {:?}", error);
                }
            }
            Either::Second(Err(error)) => defmt::panic!("IPC receive failed: {:?}", error),
        }
    }
}
