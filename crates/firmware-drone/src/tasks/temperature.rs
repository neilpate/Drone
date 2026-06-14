use embassy_time::{Duration, Timer};
use firmware_types::TelemetryState;

use crate::board;
use crate::signals::telemetry;

#[embassy_executor::task]
pub async fn temperature(mut temperature_sensor: board::TemperatureSensor) -> ! {
    defmt::info!("telemetry task: started");

    let mut sequence_count: u32 = 0;

    loop {
        sequence_count = sequence_count.wrapping_add(1);

        // I30F2 fixed-point: raw hardware units are 0.25 °C.
        let raw_temperature = temperature_sensor.read().await;

        // Compile-time divide by 4 — the `2` lives in the type parameter.
        let temperature_celsius = raw_temperature.to_num::<f32>();

        let temperature = firmware_types::Temperature::from_celsius(temperature_celsius);

        let telemetry = TelemetryState {
            sequence_number: sequence_count,
            temperature,
        };

        telemetry::set(telemetry);

        Timer::after(Duration::from_millis(500)).await;
    }
}
