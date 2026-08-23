use core::sync::atomic::{AtomicU8, Ordering};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::{ESCTelemetrySample, MotorID};

const MAX_SUBSCRIBERS: usize = 8;

static ESC_TELEMETRY_SAMPLE: Watch<CriticalSectionRawMutex, ESCTelemetrySample, MAX_SUBSCRIBERS> =
    Watch::new();

pub type Receiver = embassy_sync::watch::Receiver<
    'static,
    CriticalSectionRawMutex,
    ESCTelemetrySample,
    MAX_SUBSCRIBERS,
>;

pub fn subscribe() -> Receiver {
    ESC_TELEMETRY_SAMPLE.receiver().unwrap()
}

pub fn set(esc_telemetry: ESCTelemetrySample) {
    ESC_TELEMETRY_SAMPLE.sender().send(esc_telemetry);
}

/// Which motor the ESC-telemetry task wants the DShot driver to flag next.
static TELEM_REQUEST: AtomicU8 = AtomicU8::new(MotorID::None as u8);

/// Queue a one-shot telemetry request for `motor` (0..=3).
pub fn request(motor: MotorID) {
    TELEM_REQUEST.store(motor as u8, Ordering::Relaxed);
}

/// Take the pending request, clearing it (one-shot). `None` if nothing pending.
pub fn take_request() -> Option<MotorID> {
    match TELEM_REQUEST.swap(MotorID::None as u8, Ordering::Relaxed) {
        x if x == MotorID::None as u8 => None,
        motor => Some(unsafe { core::mem::transmute(motor) }),
    }
}
