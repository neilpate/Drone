use crate::board;
use crate::tasks::supervisor;

#[embassy_executor::task]
pub async fn motor_controller(mut motors: board::Motors) -> ! {
    defmt::info!("motor_controller task: started");

    let mut safe_values_receiver = supervisor::subscribe_safe_values();

    motors.enable();

    loop {
        let safe_values = safe_values_receiver.changed().await;

        defmt::debug!("received safe values: {}", safe_values.throttle);

        motors.set_throttle(0, safe_values.throttle);
    }
}
