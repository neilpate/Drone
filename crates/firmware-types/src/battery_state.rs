use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{ConsumedCurrent, Current, Voltage};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, MaxSize, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BatteryState {
    pub voltage: Voltage,
    pub current: Current,
    pub consumed_current: ConsumedCurrent,
}

impl BatteryState {
    pub fn from_bytes(bytes: &[u8; 6]) -> Self {
        // Implement the conversion from bytes to BatteryState here

        let voltage_bytes = [bytes[0], bytes[1]];
        let current_bytes = [bytes[2], bytes[3]];
        let consumed_current_bytes = [bytes[4], bytes[5]];

        let voltage = Voltage::from_bytes(&voltage_bytes);
        let current = Current::from_bytes(&current_bytes);
        let consumed_current = ConsumedCurrent::from_bytes(&consumed_current_bytes);

        Self {
            voltage,
            current,
            consumed_current,
        }
    }
}
