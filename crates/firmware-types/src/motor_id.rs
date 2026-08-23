use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MotorID {
    #[default]
    None,
    Motor1,
    Motor2,
    Motor3,
    Motor4,
}

impl MotorID {
    pub fn increment_with_wraparound(&mut self) {
        *self = match self {
            MotorID::None => MotorID::Motor1,
            MotorID::Motor1 => MotorID::Motor2,
            MotorID::Motor2 => MotorID::Motor3,
            MotorID::Motor3 => MotorID::Motor4,
            MotorID::Motor4 => MotorID::Motor1,
        }
    }
}
