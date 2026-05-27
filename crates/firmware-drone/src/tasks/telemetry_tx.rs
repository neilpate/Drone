use crate::board::Radio;
use crate::radio_link;
use embassy_nrf::radio::ieee802154::Packet;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn transmit(mut radio: Radio) -> ! {
    defmt::info!("transmit task: started");

    radio.set_channel(radio_link::CHANNEL);

    let mut packet = Packet::new();
    let mut count = 0_u32;

    loop {
        packet.copy_from_slice(&count.to_le_bytes());

        if let Err(e) = radio.try_send(&mut packet).await {
            defmt::error!("transmit task: error: {:?}", e);
            continue;
        }
        defmt::info!("transmit task: sent {}", count);
        count += 1;

        Timer::after_millis(10).await;
    }
}
