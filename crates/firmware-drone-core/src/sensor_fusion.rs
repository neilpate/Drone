use firmware_types::{Attitude, ImuData, PitchAngle, RollAngle};
// no_std target only: brings `atan2`/`sqrt` onto f32 via micromath. In host
// (`test`) builds std's inherent f32 methods are used instead, so importing the
// trait there would be an unused import (and the two use slightly different
// approximations — filter tests must tolerance accordingly).
#[cfg(not(test))]
use micromath::F32Ext;

// use postcard::experimental::max_size::MaxSize;
// use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct AttitudeEstimator {
    initialised: bool,
    prev_attitude: Attitude,
}

impl AttitudeEstimator {
    pub fn new() -> Self {
        Self {
            initialised: false,
            prev_attitude: Attitude::default(),
        }
    }

    pub fn update(&mut self, imu_data: ImuData, dt: f32) -> Attitude {
        if !self.initialised {
            // First update: seed the attitude from the accelerometer.
            let roll = (-imu_data.acceleration_y.as_g())
                .atan2(-imu_data.acceleration_z.as_g())
                .to_degrees();
            let pitch = imu_data
                .acceleration_x
                .as_g()
                .atan2(
                    (imu_data.acceleration_y.as_g() * imu_data.acceleration_y.as_g()
                        + imu_data.acceleration_z.as_g() * imu_data.acceleration_z.as_g())
                    .sqrt(),
                )
                .to_degrees();

            let attitude = Attitude {
                roll: RollAngle::from_degrees(roll),
                pitch: PitchAngle::from_degrees(pitch),
            };
            self.prev_attitude = attitude;
            self.initialised = true;
            return attitude;
        }

        let alpha = 0.98; // Complementary filter coefficient

        let roll_pred = self.prev_attitude.roll.as_degrees()
            + imu_data.angular_rate_x.as_degrees_per_second() * dt;
        let pitch_pred = self.prev_attitude.pitch.as_degrees()
            + imu_data.angular_rate_y.as_degrees_per_second() * dt;

        let roll_acc = (-imu_data.acceleration_y.as_g())
            .atan2(-imu_data.acceleration_z.as_g())
            .to_degrees();

        let pitch_acc = imu_data
            .acceleration_x
            .as_g()
            .atan2(
                (imu_data.acceleration_y.as_g() * imu_data.acceleration_y.as_g()
                    + imu_data.acceleration_z.as_g() * imu_data.acceleration_z.as_g())
                .sqrt(),
            )
            .to_degrees();

        let roll = alpha * roll_pred + (1.0 - alpha) * roll_acc;
        let pitch = alpha * pitch_pred + (1.0 - alpha) * pitch_acc;

        let attitude = Attitude {
            roll: RollAngle::from_degrees(roll),
            pitch: PitchAngle::from_degrees(pitch),
        };
        self.prev_attitude = attitude;
        self.initialised = true;
        attitude
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use firmware_types::{Acceleration, AngularRate};

    // Inputs are written in attitude units (see `accel_for`), so each test reads
    // as degrees in and degrees out. Assertions use tolerances, not exact
    // equality, because f32 arithmetic rounds at every step (sin/cos, atan2,
    // sqrt, blend). (The target uses micromath while these host tests use std
    // math, so don't read these exact values as the target's — but that is not
    // why the tolerance is here; the rounding is.)

    /// No rotation (all gyro axes zero), in deg/s.
    const STILL: (f32, f32, f32) = (0.0, 0.0, 0.0);

    /// The specific force (in g) a *still* craft at the given roll/pitch reads —
    /// the inverse of the filter's accelerometer correction. Yaw is deliberately
    /// absent: gravity is vertical, so yaw does not change its projection onto the
    /// body axes (the same reason the estimator cannot observe yaw from accel).
    /// Lets the tests speak in attitude units instead of raw g.
    fn accel_for(roll_deg: f32, pitch_deg: f32) -> (f32, f32, f32) {
        let roll = roll_deg.to_radians();
        let pitch = pitch_deg.to_radians();
        (
            pitch.sin(),
            -roll.sin() * pitch.cos(),
            -roll.cos() * pitch.cos(),
        )
    }

    /// Build an `ImuData` from an accel triple (g) and a gyro triple (deg/s).
    fn imu(accel: (f32, f32, f32), gyro: (f32, f32, f32)) -> ImuData {
        ImuData {
            acceleration_x: Acceleration::from_g(accel.0),
            acceleration_y: Acceleration::from_g(accel.1),
            acceleration_z: Acceleration::from_g(accel.2),
            angular_rate_x: AngularRate::from_degrees_per_second(gyro.0),
            angular_rate_y: AngularRate::from_degrees_per_second(gyro.1),
            angular_rate_z: AngularRate::from_degrees_per_second(gyro.2),
        }
    }

    /// An already-running estimator (past the seeding step) holding the given
    /// roll/pitch, in degrees. Use this to exercise the steady-state blend; use
    /// `AttitudeEstimator::new()` to exercise first-sample seeding.
    fn running(roll_deg: f32, pitch_deg: f32) -> AttitudeEstimator {
        AttitudeEstimator {
            initialised: true,
            prev_attitude: Attitude {
                roll: RollAngle::from_degrees(roll_deg),
                pitch: PitchAngle::from_degrees(pitch_deg),
            },
        }
    }

    #[test]
    fn level_and_still_stays_level() {
        let out = running(0.0, 0.0).update(imu(accel_for(0.0, 0.0), STILL), 0.01);
        assert!(out.roll.as_degrees().abs() < 1e-3);
        assert!(out.pitch.as_degrees().abs() < 1e-3);
    }

    #[test]
    fn accel_correction_pulls_estimate_toward_level() {
        // Estimate says 10 deg roll, accel says level, gyro still. One step bleeds
        // (1 - alpha) = 2% of the error out: 0.98*10 + 0.02*0 = 9.8.
        let out = running(10.0, 0.0).update(imu(accel_for(0.0, 0.0), STILL), 0.01);
        assert!((out.roll.as_degrees() - 9.8).abs() < 0.05);
    }

    #[test]
    fn positive_roll_rate_raises_roll_only() {
        // +100 deg/s about X for 0.1 s = +10 deg gyro prediction; accel level so
        // the correction is zero (0.98*10 = 9.8). Pitch must not move (isolation).
        let out = running(0.0, 0.0).update(imu(accel_for(0.0, 0.0), (100.0, 0.0, 0.0)), 0.1);
        assert!((out.roll.as_degrees() - 9.8).abs() < 0.05);
        assert!(out.pitch.as_degrees().abs() < 1e-3);
    }

    #[test]
    fn positive_pitch_rate_raises_pitch_only() {
        let out = running(0.0, 0.0).update(imu(accel_for(0.0, 0.0), (0.0, 100.0, 0.0)), 0.1);
        assert!((out.pitch.as_degrees() - 9.8).abs() < 0.05);
        assert!(out.roll.as_degrees().abs() < 1e-3);
    }

    #[test]
    fn converges_to_accel_roll_right_side_down() {
        // A steady +30 deg right roll with no gyro: the blend must converge there
        // and leave pitch level.
        let sample = imu(accel_for(30.0, 0.0), STILL);
        let mut e = running(0.0, 0.0);
        let mut out = Attitude::default();
        for _ in 0..1000 {
            out = e.update(sample, 0.01);
        }
        assert!((out.roll.as_degrees() - 30.0).abs() < 0.5);
        assert!(out.pitch.as_degrees().abs() < 0.5);
    }

    #[test]
    fn converges_to_accel_pitch_nose_up() {
        let sample = imu(accel_for(0.0, 20.0), STILL);
        let mut e = running(0.0, 0.0);
        let mut out = Attitude::default();
        for _ in 0..1000 {
            out = e.update(sample, 0.01);
        }
        assert!((out.pitch.as_degrees() - 20.0).abs() < 0.5);
        assert!(out.roll.as_degrees().abs() < 0.5);
    }

    #[test]
    fn seeds_from_accel_on_first_update() {
        // A brand-new estimator snaps its first update straight to the accel tilt
        // with no slew, ignoring the gyro on that step (seeding uses accel only).
        // +30 deg roll plus a large spurious gyro rate that must be ignored.
        let out =
            AttitudeEstimator::new().update(imu(accel_for(30.0, 0.0), (500.0, 0.0, 0.0)), 0.01);
        assert!((out.roll.as_degrees() - 30.0).abs() < 0.1);
        assert!(out.pitch.as_degrees().abs() < 0.1);
    }
}
