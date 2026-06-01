use crate::board::Radio;
use crate::radio_link;
use crate::tasks::pilot_command;
use embassy_nrf::radio;
use embassy_nrf::radio::ieee802154::Packet;
use embassy_time::{Duration, with_timeout};
use firmware_types::{PilotCommand, TelemetryState};

const MAX_SEND_BUFFER_SIZE: usize = 32;
const RECEIVE_TIMEOUT: Duration = Duration::from_millis(50); //5× the 10ms remote period — generous for early bring-up

async fn receive(radio: &mut Radio) -> Option<PilotCommand> {
    let mut rx_packet = Packet::new();

    match with_timeout(RECEIVE_TIMEOUT, radio.receive(&mut rx_packet)).await {
        // Outer match is for the timeout; inner match is for the radio receive result
        Ok(Ok(())) => {} //Received a packet successfully within the timeout
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
        Ok(control_state) => Some(control_state),
        Err(e) => {
            defmt::warn!("postcard decode error: {:?}", e);
            None
        }
    }
}

async fn send(radio: &mut Radio, telemetry: TelemetryState) -> Result<(), radio::Error> {
    let mut scratch = [0u8; MAX_SEND_BUFFER_SIZE]; //Working space for serialization

    //bytes_to_send is a subslice of scratch which contains the serialized TelemetryState
    let bytes_to_send = postcard::to_slice(&telemetry, &mut scratch)
        .expect("scratch is large enough for TelemetryState");

    let mut tx_packet = Packet::new();

    tx_packet.copy_from_slice(bytes_to_send);

    radio.try_send(&mut tx_packet).await
}

#[embassy_executor::task]
pub async fn comm_link(mut radio: Radio) -> ! {
    defmt::info!("comm_link task: started");

    let mut telemetry_state = TelemetryState { count: 0 };

    radio.set_channel(radio_link::CHANNEL);

    loop {
        let Some(command) = receive(&mut radio).await else {
            continue;
        };

        // This will only run if control data was received from the remote
        defmt::info!("received: {}", command);

        pilot_command::set(command); //Publish the received command to any subscribers

        telemetry_state.count = command.sequence_count; //In this scaffold, just echo back the count from the received PilotCommand

        if let Err(e) = send(&mut radio, telemetry_state).await {
            defmt::warn!("failed to send telemetry: {:?}", e);
        }
    }
}
