use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};

#[derive(Clone, Copy, Eq, PartialEq, Debug, defmt::Format)]
pub enum SystemState {
    Booting,
    Idle,
    Fault,
}

const MAX_SUBSCRIBERS: usize = 8;

pub static STATUS: Watch<CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS> = Watch::new();

pub type StatusReceiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS>;

pub fn set(s: SystemState) {
    STATUS.sender().send(s);
}
