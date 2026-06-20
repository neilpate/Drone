use embassy_time::{Duration, Ticker};
use firmware_types::TelemetryState;

use crate::signals::{pilot_command, status, telemetry, temperature};

#[embassy_executor::task]
pub async fn telemetry_aggregator() -> ! {
    defmt::info!("telemetry aggregator task: started");

    let mut sequence_count: u32 = 0;

    let mut status_receiver = status::subscribe();
    let mut temperature_receiver = temperature::subscribe();
    let mut pilot_command_receiver = pilot_command::subscribe();

    let mut ticker = Ticker::every(Duration::from_millis(10));

    loop {
        ticker.next().await;

        sequence_count = sequence_count.wrapping_add(1);

        let drone_state = status_receiver.get().await;
        let temperature = temperature_receiver.get().await;
        let pilot_command = pilot_command_receiver.get().await;
        let state = TelemetryState {
            drone_state,
            sequence_number: sequence_count,
            temperature,
            pilot_command,
        };

        telemetry::set(state);
    }
}
