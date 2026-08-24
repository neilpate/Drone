use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{ControlMode, ControlSystemParameters, CpuLoad, ESCTelemetry, Temperature};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TelemetryFrameLowRate {
    pub sequence_number: u32,
    pub cpu_load: CpuLoad,
    pub control_mode: ControlMode,
    /// MCU die temperature. ESC temperatures are carried in `esc_telemetry`.
    pub temperature: Temperature,
    pub esc_telemetry: ESCTelemetry,
    /// The drone's currently active gains, echoed so the ground station logs what actually flew.
    pub control_parameters: ControlSystemParameters,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AngularRate, BatteryState};

    #[test]
    fn postcard_round_trip() {
        let original = TelemetryFrameLowRate {
            sequence_number: 999,
            cpu_load: CpuLoad::from_percentage(50.0),
            control_mode: ControlMode::Manual,
            temperature: Temperature::from_celsius(25.0),
            esc_telemetry: ESCTelemetry {
                temperature: Temperature::from_celsius(30.0),
                battery_state: BatteryState::default(),
                motor1_rpm: AngularRate::from_degrees_per_second(0.0),
                motor2_rpm: AngularRate::from_degrees_per_second(0.0),
                motor3_rpm: AngularRate::from_degrees_per_second(0.0),
                motor4_rpm: AngularRate::from_degrees_per_second(0.0),
            },
            control_parameters: ControlSystemParameters::default(),
        };
        let mut buf = [0u8; TelemetryFrameLowRate::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: TelemetryFrameLowRate = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
