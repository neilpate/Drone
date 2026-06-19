use embassy_nrf::radio;
use embassy_nrf::radio::ieee802154::Packet;
use embassy_time::{Duration, Ticker, with_timeout};
use firmware_types::{PilotCommand, TelemetryState};

use crate::board::Radio;
use crate::radio_link;
use crate::signals::telemetry;
use crate::signals::throttle_command;

const MAX_SEND_BUFFER_SIZE: usize = 32;
const LOOP_PERIOD: Duration = Duration::from_millis(10);
const RECEIVE_TIMEOUT: Duration = Duration::from_millis(8); //Needs to be shorter than the LOOP_PERIOD

async fn send(radio: &mut Radio, state: PilotCommand) -> Result<(), radio::Error> {
    let mut scratch = [0u8; MAX_SEND_BUFFER_SIZE]; //Working space for serialization

    //bytes_to_send is a subslice of scratch which contains the serialized PilotCommand
    let bytes_to_send =
        postcard::to_slice(&state, &mut scratch).expect("scratch is large enough for PilotCommand");

    let mut tx_packet = Packet::new();

    tx_packet.copy_from_slice(bytes_to_send);

    radio.try_send(&mut tx_packet).await
}

async fn receive(radio: &mut Radio) -> Option<TelemetryState> {
    let mut rx_packet = Packet::new();

    match with_timeout(RECEIVE_TIMEOUT, radio.receive(&mut rx_packet)).await {
        // Outer match is for the timeout; inner match is for the radio receive result
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            defmt::warn!("drone_link receive: error: {:?}", e);
            return None;
        }
        Err(_) => {
            // defmt::warn!("drone_link receive: timeout");
            return None;
        }
    }

    match postcard::from_bytes(&rx_packet) {
        Ok(telemetry) => Some(telemetry),
        Err(e) => {
            defmt::warn!("postcard decode error: {:?}", e);
            None
        }
    }
}

#[embassy_executor::task]
pub async fn drone_link(mut radio: Radio) -> ! {
    defmt::info!("drone_link task: started");

    let mut ticker = Ticker::every(LOOP_PERIOD);

    let mut sequence_count: u32 = 0;

    radio.set_channel(radio_link::CHANNEL);

    let mut throttle_command_receiver = throttle_command::subscribe();

    loop {
        ticker.next().await; // Wait for the next tick before sending the next control state

        let throttle = throttle_command_receiver.get().await;

        defmt::debug!("drone_link received: {}", throttle);

        let state = PilotCommand {
            sequence_count,
            throttle,
        };
        if let Err(e) = send(&mut radio, state).await {
            defmt::error!("drone_link transmit: error: {:?}", e);
            continue;
        }

        // This will only run if the send succeeded, so now we wait for a response from the drone
        if let Some(telemetry) = receive(&mut radio).await {
            // defmt::info!("received: {}", telemetry);
            telemetry::set(telemetry);
        }

        sequence_count = sequence_count.wrapping_add(1);
    }
}
