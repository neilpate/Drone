use crate::board::Radio;
use crate::radio_link;
use embassy_nrf::radio::ieee802154::Packet;

#[embassy_executor::task]
pub async fn receive(mut radio: Radio) -> ! {
    defmt::info!("receive task: started");

    radio.set_channel(radio_link::CHANNEL);

    let mut packet = Packet::new();

    loop {
        match radio.receive(&mut packet).await {
            Ok(()) => {
                let bytes: &[u8] = &packet;
                if bytes.len() == 4 {
                    let count = u32::from_le_bytes(bytes.try_into().unwrap());
                    defmt::info!("receive task: count={} lqi={}", count, packet.lqi());
                } else {
                    defmt::info!("receive task: len={} bytes={=[u8]:x}", bytes.len(), bytes);
                }
            }
            Err(e) => {
                defmt::warn!("receive task: error: {:?}", e);
            }
        }
    }
}
