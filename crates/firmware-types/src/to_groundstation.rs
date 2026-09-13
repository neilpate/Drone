use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::remote_info::RemoteInfo;
use crate::telemetry_frame::TelemetryFrame;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
pub enum ToGroundStation {
    Telemetry(TelemetryFrame),
    RemoteInfo(RemoteInfo),
}

// The maximum size of a `ToGroundStation` message, in bytes, when serialized with `postcard` and COBS-framed.
pub const MAX_SIZE_BYTES: usize =
    ToGroundStation::POSTCARD_MAX_SIZE + ToGroundStation::POSTCARD_MAX_SIZE / 254 + 2;
