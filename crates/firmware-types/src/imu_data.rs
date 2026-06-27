use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{Acceleration, AngularRate};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ImuData {
    pub acceleration_x: Acceleration,
    pub acceleration_y: Acceleration,
    pub acceleration_z: Acceleration,
    pub angular_rate_x: AngularRate,
    pub angular_rate_y: AngularRate,
    pub angular_rate_z: AngularRate,
}
