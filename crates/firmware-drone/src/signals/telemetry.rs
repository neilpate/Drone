use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::TelemetryFrameHighRate;

const MAX_SUBSCRIBERS: usize = 8;

static TELEMETRY: Watch<CriticalSectionRawMutex, TelemetryFrameHighRate, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver = embassy_sync::watch::Receiver<
    'static,
    CriticalSectionRawMutex,
    TelemetryFrameHighRate,
    MAX_SUBSCRIBERS,
>;

pub fn subscribe() -> Receiver {
    TELEMETRY.receiver().unwrap()
}

pub fn set(telemetry: TelemetryFrameHighRate) {
    TELEMETRY.sender().send(telemetry);
}
