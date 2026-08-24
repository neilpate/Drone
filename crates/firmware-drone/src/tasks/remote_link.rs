use embassy_nrf::radio;
use embassy_nrf::radio::ieee802154::Packet;
use embassy_time::{Duration, with_timeout};
use firmware_types::{
    Command, PilotCommand, RadioMessage, TELEMETRY_FRAME_MAX_SIZE_BYTES, TelemetryFrame,
};

use crate::board::Radio;
use crate::radio_link;
use crate::signals::{
    control_mode_update, control_system_parameter_update, imu_calibrate, pilot_command, telemetry,
};

const RECEIVE_TIMEOUT: Duration = Duration::from_millis(50); //5× the 10ms remote period — generous for early bring-up

async fn receive(radio: &mut Radio) -> Option<RadioMessage> {
    let mut rx_packet = Packet::new();

    match with_timeout(RECEIVE_TIMEOUT, radio.receive(&mut rx_packet)).await {
        // Outer match is for the timeout; inner match is for the radio receive result
        Ok(Ok(())) => {} //Received a packet successfully within the timeout
        Ok(Err(e)) => {
            defmt::trace!("remote_link receive: error: {:?}", e);
            return None;
        }
        Err(_) => {
            defmt::trace!("remote_link receive: timeout");
            return None;
        }
    }

    match postcard::from_bytes(&rx_packet) {
        Ok(radio_message) => Some(radio_message),
        Err(e) => {
            defmt::trace!("postcard decode error: {:?}", e);
            None
        }
    }
}

async fn send(radio: &mut Radio, telemetry: TelemetryFrame) -> Result<(), radio::Error> {
    let mut scratch = [0u8; TELEMETRY_FRAME_MAX_SIZE_BYTES]; //Working space for serialization

    //bytes_to_send is a subslice of scratch which contains the serialized TelemetryState
    let bytes_to_send = postcard::to_slice(&telemetry, &mut scratch)
        .expect("scratch is large enough for TelemetryState");

    let mut tx_packet = Packet::new();

    tx_packet.copy_from_slice(bytes_to_send);

    radio.try_send(&mut tx_packet).await
}

#[embassy_executor::task]
pub async fn remote_link(mut radio: Radio) -> ! {
    defmt::info!("remote_link task: started");

    pilot_command::set(PilotCommand::default()); //Set the watch signal so that the link to the drone will be in a known state and not blocking
    control_mode_update::set(firmware_types::ControlMode::Manual); //Set the watch signal so that the link to the drone will be in a known state and not blocking
    control_system_parameter_update::set(firmware_types::ControlSystemParameters::default()); //Set the watch signal so that the link to the drone will be in a known state and not blocking

    let mut telemetry_receiver = telemetry::subscribe();

    radio.set_channel(radio_link::CHANNEL);

    loop {
        let Some(radio_message) = receive(&mut radio).await else {
            continue;
        };

        if radio_message.destination_address != firmware_types::RADIO_MESSAGE_DESTINATION_ADDRESS {
            defmt::trace!(
                "remote_link: received message with unexpected destination address: 0x{:08X}",
                radio_message.destination_address
            );
            continue;
        }

        // The message received over the radio link could be one of several types of commands, so we match on the enum to determine what to do with it.
        // Most of the time it will be a PilotCommand
        match radio_message.command {
            Command::PilotCommand(pilot_command) => {
                // defmt::debug!("remote link: pilot_command={:?}", pilot_command);
                pilot_command::set(pilot_command);
            }
            Command::ControlModeUpdate(control_mode) => {
                // defmt::debug!("remote link: mode={:?}", control_mode);
                control_mode_update::set(control_mode);
            }
            Command::ControlSystemParameterUpdate(control_system_parameters) => {
                // defmt::debug!("remote link: parameters={:?}", control_system_parameters);

                control_system_parameter_update::set(control_system_parameters);
            }

            Command::ResetImuCalibration => {
                imu_calibrate::set(true);
            }
        }

        let telemetry_state = telemetry_receiver.get().await; //Get the latest telemetry state from the telemetry task

        if let Err(e) = send(&mut radio, telemetry_state).await {
            defmt::warn!("failed to send telemetry: {:?}", e);
        }
    }
}
