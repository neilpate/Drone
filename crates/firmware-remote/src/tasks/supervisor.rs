use embassy_time::Timer;
use firmware_types::RemoteState;

use crate::signals::status;

#[embassy_executor::task]
pub async fn supervisor() -> ! {
    defmt::info!("supervisor task: started");
    status::set(RemoteState::Booting);

    Timer::after_millis(5000).await;
    defmt::info!("supervisor task: -> Idle");
    status::set(RemoteState::Idle);

    Timer::after_millis(5000).await;
    defmt::info!("supervisor task: -> Fault");
    status::set(RemoteState::Fault);

    loop {
        core::future::pending::<()>().await;
    }
}
