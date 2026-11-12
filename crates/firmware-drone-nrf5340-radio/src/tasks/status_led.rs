use embassy_nrf::gpio::Output;
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn status_led(mut led: Output<'static>) -> ! {
    loop {
        led.toggle();
        Timer::after(Duration::from_millis(200)).await;
    }
}
