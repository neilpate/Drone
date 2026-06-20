use embassy_time::{Duration, Timer};

use crate::board;
use crate::signals::temperature;

#[embassy_executor::task]
pub async fn temperature(mut temperature_sensor: board::TemperatureSensor) -> ! {
    defmt::info!("temperature task: started");

    loop {
        // I30F2 fixed-point: raw hardware units are 0.25 °C.
        let raw_temperature = temperature_sensor.read().await;

        // Compile-time divide by 4 — the `2` lives in the type parameter.
        let temperature_celsius = raw_temperature.to_num::<f32>();

        let temperature = firmware_types::Temperature::from_celsius(temperature_celsius);

        temperature::set(temperature);

        Timer::after(Duration::from_millis(500)).await;
    }
}
