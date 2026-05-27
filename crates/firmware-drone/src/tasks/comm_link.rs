use crate::board::Radio;
use crate::radio_link;
use embassy_nrf::radio::ieee802154::Packet;
use embassy_time::{Duration, with_timeout};

#[derive(Clone, Copy, Eq, PartialEq, Debug, defmt::Format)]
pub struct ControlState {
    pub count: u32,
}

impl ControlState {
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

pub struct TelemetryState {
    pub count: u32,
}

impl TelemetryState {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn to_packet(&self, packet: &mut Packet) {
        packet.copy_from_slice(&self.count.to_le_bytes());
    }
}

#[embassy_executor::task]
pub async fn comm_link(mut radio: Radio) -> ! {
    defmt::info!("comm_link task: started");

    let mut telemetry_state = TelemetryState::new();

    radio.set_channel(radio_link::CHANNEL);

    let mut rx_packet = Packet::new();
    let mut tx_packet = Packet::new();

    loop {
        match with_timeout(Duration::from_millis(100), radio.receive(&mut rx_packet)).await {
            Ok(Ok(())) => {
                let control_state = ControlState::from_packet(&rx_packet);

                defmt::info!(
                    "comm_link received: {} lqi={}",
                    control_state,
                    rx_packet.lqi()
                );
                telemetry_state.count = control_state.count;
            }
            Ok(Err(e)) => {
                defmt::warn!("comm_link receive: error: {:?}", e);
            }
            Err(e) => {
                defmt::warn!("comm_link receive: timeout    : {:?}", e);
            }
        }

        telemetry_state.to_packet(&mut tx_packet);

        if let Err(e) = radio.try_send(&mut tx_packet).await {
            defmt::error!("comm_link transmit: error: {:?}", e);
            continue;
        }
        // defmt::info!("comm_link transmit: sent {}", telemetry_state.count);
        telemetry_state.count += 1;
    }
}
