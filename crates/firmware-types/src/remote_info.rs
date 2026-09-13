use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
pub struct RemoteInfo {
    pub firmware_version: u32,
}
