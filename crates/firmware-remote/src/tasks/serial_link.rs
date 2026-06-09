use crate::board::Uart;
// use embassy_time::{Duration, Ticker, with_timeout};

// use firmware_types::{PilotCommand, TelemetryState, Throttle};

#[embassy_executor::task]
pub async fn serial_link(mut uart: Uart) -> ! {
    defmt::info!("serial_link task: started");

    loop {
        uart.write(b"Hello").await.unwrap();
    }
}
