use firmware_types::{COMMAND_FRAME_MAX_SIZE_BYTES, Command, PilotCommand};
use postcard::accumulator::{CobsAccumulator, FeedResult};

use crate::board::UartRx;
use crate::signals::{command, reset_imu, save_config};

#[embassy_executor::task]
pub async fn serial_link_rx(mut uart_rx: UartRx) -> ! {
    defmt::info!("serial_link_rx (from groundstation) task: started");

    // Set the watch signals so that the link to the drone will be in a known state and not blocking
    command::set(Command::PilotCommand(PilotCommand::ZERO));

    let mut byte = [0u8; 1];
    let mut cobs: CobsAccumulator<COMMAND_FRAME_MAX_SIZE_BYTES> = CobsAccumulator::new();

    loop {
        if let Err(e) = uart_rx.read(&mut byte).await {
            defmt::warn!("serial_link_rx read error: {:?}", e);
            continue;
        }

        // one byte in → accumulator buffers until a full frame arrives
        if let FeedResult::Success { data, .. } = cobs.feed::<Command>(&byte) {
            // One-off commands (IMU reset, save-config) are events, not streaming
            // state: route each to its own dedicated Signal. If they went through
            // the `command` Watch like streaming state, `drone_link` would resend
            // them every tick and the drone would re-action them ~100x/s. Only
            // streaming state (pilot sticks, mode) belongs on the `command` Watch.
            match data {
                Command::ResetImuCalibration => reset_imu::signal(),
                Command::SaveConfig => save_config::signal(),
                other => command::set(other),
            }
        }
    }
}
