use firmware_types::{
    GROUNDSTATION_COMMAND_FRAME_MAX_SIZE_BYTES, GroundstationCommand, PitchCommand, RollCommand,
    ThrottleCommand, YawCommand,
};
use postcard::accumulator::{CobsAccumulator, FeedResult};

use crate::board::UartRx;
use crate::signals::{pitch_command, roll_command, throttle_command, yaw_command};

#[embassy_executor::task]
pub async fn serial_link_rx(mut uart_rx: UartRx) -> ! {
    defmt::info!("serial_link_rx (from groundstation) task: started");

    // Set the watch signals so that the link to the drone will be in a known state and not blocking
    throttle_command::set(ThrottleCommand::ZERO);
    roll_command::set(RollCommand::ZERO);
    pitch_command::set(PitchCommand::ZERO);
    yaw_command::set(YawCommand::ZERO);

    let mut byte = [0u8; 1];
    let mut cobs: CobsAccumulator<GROUNDSTATION_COMMAND_FRAME_MAX_SIZE_BYTES> =
        CobsAccumulator::new();

    loop {
        if let Err(e) = uart_rx.read(&mut byte).await {
            defmt::warn!("serial_link_rx read error: {:?}", e);
            continue;
        }

        // one byte in → accumulator buffers until a full frame arrives
        if let FeedResult::Success { data, .. } = cobs.feed::<GroundstationCommand>(&byte) {
            throttle_command::set(data.throttle);
            roll_command::set(data.roll);
            pitch_command::set(data.pitch);
            yaw_command::set(data.yaw);
        }
    }
}
