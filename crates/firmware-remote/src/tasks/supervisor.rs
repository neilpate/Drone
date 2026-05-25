use embassy_time::Timer;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};

#[derive(Clone, Copy, Eq, PartialEq, Debug, defmt::Format)]
pub enum SystemState {
    Booting,
    Idle,
    Fault,
}

const MAX_SUBSCRIBERS: usize = 8;

static STATUS: Watch<CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS> = Watch::new();

pub type StatusReceiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS>;

pub fn subscribe() -> StatusReceiver {
    STATUS.receiver().unwrap()
}

fn set(s: SystemState) {
    STATUS.sender().send(s);
}

#[embassy_executor::task]
pub async fn supervise() -> ! {
    defmt::info!("supervisor task: started");
    set(SystemState::Booting);

    Timer::after_millis(5000).await;
    defmt::info!("supervisor task: -> Idle");
    set(SystemState::Idle);

    Timer::after_millis(5000).await;
    defmt::info!("supervisor task: -> Fault");
    set(SystemState::Fault);

    loop {
        core::future::pending::<()>().await;
    }
}
