use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

static RESET_IMU: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Latch an IMU-recalibration request from the ground station.
///
/// A `Signal` (single-observer event, per ADR 0013) rather than the shared
/// `command` Watch: the reset is a one-shot event, but the ground station
/// streams `PilotCommand` state through `command` at ~100 Hz. Routing the
/// event through the same last-value register let a pilot command clobber it
/// before the 10 ms `drone_link` relay ever sampled it. The signal latches
/// until the relay consumes it, so the edge cannot be lost.
pub fn signal() {
    RESET_IMU.signal(());
}

/// Consume a pending IMU-recalibration request, if any. Non-blocking.
pub fn take() -> bool {
    RESET_IMU.try_take().is_some()
}
