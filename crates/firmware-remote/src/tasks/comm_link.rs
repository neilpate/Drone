use crate::board::Radio;
use crate::radio_link;
use embassy_nrf::radio;
use embassy_nrf::radio::ieee802154::Packet;
use embassy_time::{Duration, Ticker, with_timeout};

use firmware_types::{PilotCommand, TelemetryState, Throttle};

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
            defmt::warn!("comm_link receive: error: {:?}", e);
            return None;
        }
        Err(_) => {
            defmt::warn!("comm_link receive: timeout");
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
pub async fn comm_link(mut radio: Radio) -> ! {
    defmt::info!("comm_link task: started");

    let mut ticker = Ticker::every(LOOP_PERIOD);

    radio.set_channel(radio_link::CHANNEL);

    let mut count = 0_u32;

    let mut throttle_counter = 0_f32;

    loop {
        ticker.next().await; // Wait for the next tick before sending the next control state

        let throttle = Throttle::from_normalised(throttle_counter);

        let state = PilotCommand {
            sequence_count: count,
            throttle,
        };
        if let Err(e) = send(&mut radio, state).await {
            defmt::error!("comm_link transmit: error: {:?}", e);
            continue;
        }

        // This will only run if the send succeeded, so now we wait for a response from the drone
        if let Some(telemetry) = receive(&mut radio).await {
            defmt::info!("received: {}", telemetry);
        }

        count += 1;
        throttle_counter += 0.0002;
        if throttle_counter > 1.0 {
            throttle_counter = 0.0;
        }
    }
}
