use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::CpuLoad;

const MAX_SUBSCRIBERS: usize = 8;

static CPU_LOAD: Watch<CriticalSectionRawMutex, CpuLoad, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, CpuLoad, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    CPU_LOAD.receiver().unwrap()
}

pub fn set(cpu_load: CpuLoad) {
    CPU_LOAD.sender().send(cpu_load);
}
