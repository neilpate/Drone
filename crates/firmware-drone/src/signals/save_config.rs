use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

static SAVE_CONFIG: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub fn signal() {
    SAVE_CONFIG.signal(());
}

// Blocking wait for the save config signal
pub async fn wait_on_signal() {
    SAVE_CONFIG.wait().await;
}
