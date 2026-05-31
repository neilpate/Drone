use crate::board;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn motor_controller(mut motors: board::Motors) -> ! {
    defmt::info!("motor_controller task: started");

    motors.enable();

    loop {
        motors.set_throttle(0, 100);
        Timer::after_millis(1000).await;

        motors.set_throttle(0, 0);
        Timer::after_millis(3000).await;

        // motors.set_duty(0, 800);
        // Timer::after_millis(2000).await;
    }
}
