use crate::{AngularRate, BatteryState, ERPM, MotorID, Temperature};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ESCTelemetrySample {
    pub temperature: Temperature,
    pub battery_state: BatteryState,
    pub motor_id: MotorID,
    pub motor_rpm: AngularRate,
}

impl ESCTelemetrySample {
    fn check_crc(bytes: &[u8]) -> bool {
        // KISS/BLHeli/AM32 telemetry CRC8: poly 0x07, MSB-first, init 0, folded
        // over the first 9 bytes; byte 9 is the transmitted checksum.
        let mut crc: u8 = 0;
        for &byte in &bytes[0..9] {
            crc ^= byte;
            for _ in 0..8 {
                crc = if crc & 0x80 != 0 {
                    (crc << 1) ^ 0x07
                } else {
                    crc << 1
                };
            }
        }
        crc == bytes[9]
    }

    // `_motor_index` is the parked round-robin slot; wired once per-motor RPM lands (ADR 0027).
    pub fn from_bytes(bytes: &[u8], motor_id: MotorID) -> Option<Self> {
        if bytes.len() != 10 {
            return None;
        }

        if !Self::check_crc(bytes) {
            return None;
        }

        let temperature = Temperature::from_celsius(bytes[0] as f32);

        let battery_bytes = [bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6]];
        let battery_state = BatteryState::from_bytes(&battery_bytes);

        let erpm_bytes = [bytes[7], bytes[8]];
        let erpm = ERPM::from_bytes(&erpm_bytes);

        Some(Self {
            temperature,
            battery_state,
            motor_id,
            motor_rpm: AngularRate::from_rpm(erpm.as_motor_rpm()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_values() {
        let bytes = [0x1f, 0x06, 0x31, 0x03, 0x52, 0x00, 0xd2, 0x05, 0xdc, 0x0c];

        let telemetry = ESCTelemetrySample::from_bytes(&bytes, MotorID::Motor1).unwrap();

        assert_eq!(telemetry.temperature.as_celsius(), 31.0);
        assert_eq!(telemetry.battery_state.voltage.as_volts(), 15.85); // Example value
        assert_eq!(telemetry.battery_state.current.as_amps(), 8.5); // Example value
        assert_eq!(
            telemetry.battery_state.consumed_current.as_milliamphours(),
            210
        ); // Example value
        // 0x05DC = 1500 -> x100 = 150000 eRPM / 6 pole pairs = 25000 rpm
        assert_eq!(telemetry.motor_rpm.as_rpm(), 25000.0);
    }
}
