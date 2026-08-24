use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::TelemetryFrame;

const MAX_SUBSCRIBERS: usize = 8;

static TELEMETRY: Watch<CriticalSectionRawMutex, TelemetryFrame, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, TelemetryFrame, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    TELEMETRY.receiver().unwrap()
}

pub fn set(telemetry: TelemetryFrame) {
    TELEMETRY.sender().send(telemetry);
}
