use embassy_time::{Duration, with_timeout};
use firmware_types::{ESCTelemetrySample, MotorID};

use crate::board;
use crate::signals::esc_telemetry_sample;

const READ_TIMEOUT_MS: u64 = 250; // frames arrive at the rate-limited request cadence

#[embassy_executor::task]
pub async fn esc_telemetry(mut esc_telemetry_rx: board::ESCTelemetry) -> ! {
    defmt::info!("esc_telemetry task: started");

    let mut motor_id = MotorID::None; // single-motor bring-up

    // Large enough to hold a whole request burst (several back-to-back frames).
    // read_until_idle returns the burst at the trailing gap; we then CRC-scan it.
    let mut buffer = [0u8; 64];

    // Set once just so the telemetry loop does not block
    esc_telemetry_sample::set(ESCTelemetrySample::default());

    loop {
        motor_id.increment_with_wraparound();
        esc_telemetry_sample::request(motor_id);

        match with_timeout(
            Duration::from_millis(READ_TIMEOUT_MS),
            esc_telemetry_rx.read(&mut buffer),
        )
        .await
        {
            Ok(Ok(n)) if n >= 10 => {
                // Find the first CRC-valid 10-byte frame within the captured burst.
                let mut i = 0;
                let mut found = false;
                while i + 10 <= n {
                    if let Some(telemetry) =
                        ESCTelemetrySample::from_bytes(&buffer[i..i + 10], motor_id)
                    {
                        found = true;

                        esc_telemetry_sample::set(telemetry);

                        break;
                    }
                    i += 1;
                }
                if !found {
                    defmt::trace!("esc_telemetry: no valid frame in {} bytes", n);
                }
            }
            Ok(Ok(_)) => {}
            Ok(Err(_e)) => {}
            Err(_) => {}
        }
    }
}
