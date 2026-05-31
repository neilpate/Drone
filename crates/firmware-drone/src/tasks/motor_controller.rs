use crate::board;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn motor_controller(mut motor: board::Motor) -> ! {
    defmt::info!("motor_controller task: started");

    loop {
        motor.forward();
        Timer::after_millis(1000).await;

        motor.coast();
        Timer::after_millis(1000).await;

        motor.reverse();
        Timer::after_millis(1000).await;

        motor.coast();
        Timer::after_millis(1000).await;
    }
}
