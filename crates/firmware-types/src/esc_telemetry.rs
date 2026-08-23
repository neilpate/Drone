use crate::{AngularRate, BatteryState, Temperature};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ESCTelemetry {
    pub temperature: Temperature,
    pub battery_state: BatteryState,
    pub motor1_rpm: AngularRate,
    pub motor2_rpm: AngularRate,
    pub motor3_rpm: AngularRate,
    pub motor4_rpm: AngularRate,
}
