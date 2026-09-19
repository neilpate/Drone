use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

static SAVE_CONFIG: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Latch a save-config request from the ground station.
///
/// A `Signal` (single-observer event, per ADR 0013) rather than the shared
/// `command` Watch: the reset is a one-shot event, but the ground station
/// streams `PilotCommand` state through `command` at ~100 Hz. Routing the
/// event through the same last-value register let a pilot command clobber it
/// before the 10 ms `drone_link` relay ever sampled it. The signal latches
/// until the relay consumes it, so the edge cannot be lost.
pub fn signal() {
    SAVE_CONFIG.signal(());
}

/// Consume a pending save-config request, if any. Non-blocking.
pub fn take() -> bool {
    SAVE_CONFIG.try_take().is_some()
}
