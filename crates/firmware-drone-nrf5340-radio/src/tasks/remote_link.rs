use embassy_futures::select::{Either, select};
use embassy_nrf::peripherals;
use embassy_nrf::radio::ieee802154::{Packet, Radio};
use nrf5340_ipc::{MAX_FRAME_SIZE, RADIO_CHANNEL};

use crate::board::AppCoreInterface;

const _: () = assert!(MAX_FRAME_SIZE == Packet::CAPACITY as usize);

#[embassy_executor::task]
pub async fn remote_link(
    mut radio: Radio<'static, peripherals::RADIO>,
    mut app_core_interface: AppCoreInterface,
) -> ! {
    defmt::info!("radio-core bridge started; channel {}", RADIO_CHANNEL);

    let mut payload = [0; MAX_FRAME_SIZE];
    radio.set_channel(RADIO_CHANNEL);
    let mut rx_packet = Packet::new();
    let mut tx_packet = Packet::new();
    loop {
        // Radio::receive is cancellation-safe; an outgoing IPC frame preempts RX.
        match select(
            radio.receive(&mut rx_packet),
            app_core_interface.receive(&mut payload),
        )
        .await
        {
            Either::First(Ok(())) => {
                if let Err(error) = app_core_interface.try_send(&rx_packet) {
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
