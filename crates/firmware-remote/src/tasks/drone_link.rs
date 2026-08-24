use embassy_nrf::radio;
use embassy_nrf::radio::ieee802154::Packet;
use embassy_time::{Duration, Ticker, with_timeout};
use firmware_types::{
    Command, ControlMode, RADIO_MESSAGE_DESTINATION_ADDRESS, RadioMessage, TelemetryFrameHighRate,
};
use postcard::experimental::max_size::MaxSize;

use crate::board::Radio;
use crate::radio_link;
use crate::signals::{command, reset_imu, telemetry};

const MAX_SEND_BUFFER_SIZE: usize = RadioMessage::POSTCARD_MAX_SIZE;
const LOOP_PERIOD: Duration = Duration::from_millis(10);
const RECEIVE_TIMEOUT: Duration = Duration::from_millis(8); //Needs to be shorter than the LOOP_PERIOD

async fn send(radio: &mut Radio, state: RadioMessage) -> Result<(), radio::Error> {
    let mut scratch = [0u8; MAX_SEND_BUFFER_SIZE]; //Working space for serialization

    //bytes_to_send is a subslice of scratch which contains the serialized RadioMessage
    let bytes_to_send = postcard::to_slice(&state, &mut scratch)
        .expect("scratch is not large enough for RadioMessage");

    let mut tx_packet = Packet::new();

    tx_packet.copy_from_slice(bytes_to_send);

    radio.try_send(&mut tx_packet).await
}

async fn receive(radio: &mut Radio) -> Option<TelemetryFrameHighRate> {
    let mut rx_packet = Packet::new();

    match with_timeout(RECEIVE_TIMEOUT, radio.receive(&mut rx_packet)).await {
        // Outer match is for the timeout; inner match is for the radio receive result
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            defmt::trace!("drone_link receive: error: {:?}", e);
            return None;
        }
        Err(_) => {
            defmt::trace!("drone_link receive: timeout");
            return None;
        }
    }

    match postcard::from_bytes(&rx_packet) {
        Ok(telemetry) => Some(telemetry),
        Err(e) => {
            defmt::trace!("postcard decode error: {:?}", e);
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

    let mut command_receiver = command::subscribe();

    let mut last_mode_command = Command::ControlModeUpdate(ControlMode::Manual);

    loop {
        ticker.next().await; // Wait for the next tick before sending the next control state

        let command = command_receiver.get().await;

        if let Command::ControlModeUpdate(_) = command {
            last_mode_command = command;
        }

        // Every 25th tick re-send the last mode command so a missed mode change
        // self-heals over the lossy link.
        let is_heartbeat = sequence_count.is_multiple_of(25);

        // A pending IMU reset takes priority for this tick. It arrives via a
        // dedicated Signal (see signals::reset_imu) that latches until consumed,
        // so — unlike routing it through the streaming `command` Watch — the
        // high-rate pilot stream cannot clobber the edge before we sample it.
        // `take` consumes it, so it is forwarded exactly once.
        let command_to_send = if reset_imu::take() {
            Command::ResetImuCalibration
        } else if is_heartbeat {
            last_mode_command
        } else {
            command
        };

        let radio_message = RadioMessage {
            destination_address: RADIO_MESSAGE_DESTINATION_ADDRESS,
            sequence_count,
            command: command_to_send,
        };

        if let Err(e) = send(&mut radio, radio_message).await {
            defmt::error!("drone_link transmit: error: {:?}", e);
            continue;
        }

        // This will only run if the send succeeded, so now we wait for a response from the drone
        if let Some(telemetry) = receive(&mut radio).await {
            telemetry::set(telemetry);
        }

        sequence_count = sequence_count.wrapping_add(1);
    }
}
