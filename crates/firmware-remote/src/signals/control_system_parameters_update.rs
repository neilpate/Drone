use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

use firmware_types::ControlSystemParameters;

static CONTROL_SYSTEM_PARAMETERS_UPDATE: Signal<CriticalSectionRawMutex, ControlSystemParameters> =
    Signal::new();

/// Latch a control-system-parameters update from the ground station.
///
/// A `Signal` (single-observer event, per ADR 0013) rather than the shared
/// `command` Watch. A gains change is a one-shot "apply these" event, not
/// streaming state: if it rode the resent `command` Watch, the remote would
/// keep re-injecting the last-sent gains into the drone every tick — clobbering
/// the values the drone loads from flash on a reboot (ADR 0025). The signal
/// latches until `drone_link` consumes it, so it is forwarded exactly once.
pub fn signal(params: ControlSystemParameters) {
    CONTROL_SYSTEM_PARAMETERS_UPDATE.signal(params);
}

/// Consume a pending control-system-parameters-update request, if any. Non-blocking.
pub fn take() -> Option<ControlSystemParameters> {
    CONTROL_SYSTEM_PARAMETERS_UPDATE.try_take()
}
