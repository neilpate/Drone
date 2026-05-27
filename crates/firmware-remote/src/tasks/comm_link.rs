use crate::board::Radio;
use crate::radio_link;
use embassy_nrf::radio::ieee802154::Packet;
use embassy_time::{Duration, Timer, with_timeout};

pub struct ControlState {
    pub count: u32,
}

impl ControlState {
    pub fn new(count: u32) -> Self {
        Self { count }
    }

    pub fn to_packet(&self, packet: &mut Packet) {
        packet.copy_from_slice(&self.count.to_le_bytes());
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, defmt::Format)]
pub struct TelemetryState {
    pub count: u32,
}

impl TelemetryState {
    pub fn from_packet(packet: &Packet) -> Self {
        let bytes: &[u8] = packet;
        if bytes.len() == 4 {
            Self {
                count: u32::from_le_bytes(bytes.try_into().unwrap()),
            }
        } else {
            Self { count: 0 }
        }
    }
}

#[embassy_executor::task]
pub async fn comm_link(mut radio: Radio) -> ! {
    defmt::info!("comm_link task: started");

    radio.set_channel(radio_link::CHANNEL);

    let mut tx_packet = Packet::new();
    let mut rx_packet = Packet::new();

    let mut count = 0_u32;

    loop {
        let state = ControlState::new(count);
        state.to_packet(&mut tx_packet);
        let _ = radio.try_send(&mut tx_packet).await;

        match with_timeout(Duration::from_millis(100), radio.receive(&mut rx_packet)).await {
            Ok(Ok(())) => {
                let telemetry_state = TelemetryState::from_packet(&rx_packet);

                defmt::info!(
                    "telemetry received: {} lqi={}",
                    telemetry_state,
                    rx_packet.lqi()
                );
            }

            Ok(Err(e)) => {
                defmt::warn!("comm_link receive: error: {:?}", e);
            }
            Err(e) => {
                defmt::warn!("comm_link receive: timeout    : {:?}", e);
            }
        }

        count += 1;
        Timer::after_millis(10).await;
    }
}
