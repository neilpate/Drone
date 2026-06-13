use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};

use firmware_types::TelemetryState;

const MAX_SUBSCRIBERS: usize = 8;

static TELEMETRY: Watch<CriticalSectionRawMutex, TelemetryState, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver = embassy_sync::watch::Receiver<
    'static,
    CriticalSectionRawMutex,
    TelemetryState,
    MAX_SUBSCRIBERS,
>;

pub fn subscribe_telemetry() -> Receiver {
    TELEMETRY.receiver().unwrap()
}

pub fn set_state(telemetry: TelemetryState) {
    TELEMETRY.sender().send(telemetry);
}
