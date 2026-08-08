use firmware_types::{
    Attitude, ControlSystemParameters, ControllerDemand, ImuData, PilotCommand, PitchCommand,
    RollCommand, ThrottleCommand, YawCommand,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Controller {
    roll_integral: f32,
    pitch_integral: f32,
    yaw_integral: f32,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            roll_integral: 0.0,
            pitch_integral: 0.0,
            yaw_integral: 0.0,
        }
    }

    const MAX_INTEGRAL: f32 = 0.5;

    pub fn update(
        &mut self,
        parameters: ControlSystemParameters,
        pilot_command: PilotCommand,
        attitude: Attitude,
        imu_data: ImuData,
        dt: f32,
    ) -> ControllerDemand {
        let roll_setpoint = pilot_command.roll.as_normalised() * parameters.max_tilt_degrees;
        let pitch_setpoint = pilot_command.pitch.as_normalised() * parameters.max_tilt_degrees;
        let yaw_rate_setpoint =
            pilot_command.yaw.as_normalised() * parameters.max_yaw_rate_degrees_per_second;

        let roll_error_norm =
            (roll_setpoint - attitude.roll.as_degrees()) / parameters.max_tilt_degrees;

        let pitch_error_norm =
            (pitch_setpoint - attitude.pitch.as_degrees()) / parameters.max_tilt_degrees;

        let roll_p_term = parameters.kp_roll * roll_error_norm;
        let roll_d_term = -parameters.kd_roll * imu_data.angular_rate_x.as_degrees_per_second()
            / parameters.max_tilt_rate_degrees_per_second;

        // Calculate the roll output by summing the P, D, and I terms
        // Note we use the previous i term
        let roll_out = roll_d_term + roll_p_term + self.roll_integral;

        let pitch_p_term = parameters.kp_pitch * pitch_error_norm;
        let pitch_d_term = -parameters.kd_pitch * imu_data.angular_rate_y.as_degrees_per_second()
            / parameters.max_tilt_rate_degrees_per_second;

        let pitch_out = pitch_p_term + pitch_d_term + self.pitch_integral;

        let yaw_out = parameters.kp_yaw
            * (yaw_rate_setpoint - imu_data.angular_rate_z.as_degrees_per_second())
            / parameters.max_yaw_rate_degrees_per_second;

        // Now calculate the new i terms for the next iteration, but only if the throttle is not zero.
        // If the throttle is zero, we reset the i term to zero to avoid windup.
        if pilot_command.throttle == ThrottleCommand::ZERO {
            self.pitch_integral = 0.0;
            self.roll_integral = 0.0;
            self.yaw_integral = 0.0;
        } else {
            let roll_winding_up_positive =
                roll_error_norm >= 0.0 && self.roll_integral < Self::MAX_INTEGRAL;
            let roll_winding_up_negative =
                roll_error_norm <= 0.0 && self.roll_integral > -Self::MAX_INTEGRAL;

            let pitch_winding_up_positive =
                pitch_error_norm >= 0.0 && self.pitch_integral < Self::MAX_INTEGRAL;
            let pitch_winding_up_negative =
                pitch_error_norm <= 0.0 && self.pitch_integral > -Self::MAX_INTEGRAL;

            // Only integrate the roll if we have not saturated the integral term and the error is in the same direction as the integral term.
            if roll_winding_up_positive || roll_winding_up_negative {
                self.roll_integral += roll_error_norm * parameters.ki_roll * dt;
                self.roll_integral = self
                    .roll_integral
                    .clamp(-Self::MAX_INTEGRAL, Self::MAX_INTEGRAL);
            }

            // Only integrate the pitch if we have not saturated the integral term and the error is in the same direction as the integral term.
            if pitch_winding_up_positive || pitch_winding_up_negative {
                self.pitch_integral += pitch_error_norm * parameters.ki_pitch * dt;
                self.pitch_integral = self
                    .pitch_integral
                    .clamp(-Self::MAX_INTEGRAL, Self::MAX_INTEGRAL);
            }
        }

        ControllerDemand {
            throttle: pilot_command.throttle,
            roll: RollCommand::from_normalised(roll_out),
            pitch: PitchCommand::from_normalised(pitch_out),
            yaw: YawCommand::from_normalised(yaw_out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use firmware_types::{AngularRate, ThrottleCommand};

    const SAMPLE_PERIOD_S: f32 = 0.001;

    /// Neutral inputs: level attitude, zero angular rate, centred sticks, zero
    /// throttle. The controller should demand nothing from this.
    fn neutral() -> (PilotCommand, Attitude, ImuData) {
        (PilotCommand::ZERO, Attitude::default(), ImuData::default())
    }

    /// An `ImuData` carrying only the given body angular rates (deg/s).
    fn gyro(x: f32, y: f32, z: f32) -> ImuData {
        ImuData {
            angular_rate_x: AngularRate::from_degrees_per_second(x),
            angular_rate_y: AngularRate::from_degrees_per_second(y),
            angular_rate_z: AngularRate::from_degrees_per_second(z),
            ..ImuData::default()
        }
    }

    /// Parameters with all gains non-zero and all distinct, used by every test
    /// so that assertions are independent of production defaults and any
    /// accidental field-swap would produce a visibly wrong result.
    fn test_params() -> ControlSystemParameters {
        ControlSystemParameters {
            kp_roll: 1.2,
            kd_roll: 0.3,
            ki_roll: 0.0,
            kp_pitch: 0.8,
            kd_pitch: 0.15,
            ki_pitch: 0.0,
            kp_yaw: 0.6,
            kd_yaw: 0.0,
            ki_yaw: 0.0,
            max_tilt_degrees: 25.0,
            max_tilt_rate_degrees_per_second: 400.0,
            max_yaw_rate_degrees_per_second: 180.0,
        }
    }

    /// Test shim: run the stateful controller once from a fresh state. The
    /// pre-integral tests exercise single-shot P/D behaviour, so constructing a
    /// fresh `Controller` per call is equivalent to the old stateless function
    /// and keeps these tests focused on one `update` at a time.
    fn update(
        pilot_command: PilotCommand,
        attitude: Attitude,
        imu_data: ImuData,
        parameters: ControlSystemParameters,
    ) -> ControllerDemand {
        Controller::new().update(
            parameters,
            pilot_command,
            attitude,
            imu_data,
            SAMPLE_PERIOD_S,
        )
    }

    #[test]
    fn throttle_passes_through_unchanged() {
        let (mut pilot, att, imu) = neutral();
        pilot.throttle = ThrottleCommand::from_normalised(0.6);
        let out = update(pilot, att, imu, test_params());
        assert_eq!(out.throttle, ThrottleCommand::from_normalised(0.6));
    }

    #[test]
    fn level_and_centred_demands_nothing() {
        let (pilot, att, imu) = neutral();
        let out = update(pilot, att, imu, test_params());
        assert_eq!(out.roll.as_normalised(), 0.0);
        assert_eq!(out.pitch.as_normalised(), 0.0);
        assert_eq!(out.yaw.as_normalised(), 0.0);
    }

    #[test]
    fn roll_left_tilt_commands_positive_roll_to_correct() {
        // Rolled left (negative roll), sticks centred: the controller should
        // command a positive (right-side-down) roll to level out. (ADR 0021)
        let (pilot, _, imu) = neutral();
        let att = Attitude::from_degrees(0.0, -10.0);
        let out = update(pilot, att, imu, test_params());
        assert!(
            out.roll.as_normalised() > 0.0,
            "expected positive roll correction, got {}",
            out.roll.as_normalised()
        );
    }

    #[test]
    fn positive_roll_stick_commands_positive_roll() {
        // +roll stick asks for right-side-down: positive demand. (ADR 0021)
        let (mut pilot, att, imu) = neutral();
        pilot.roll = RollCommand::from_normalised(0.5);
        let out = update(pilot, att, imu, test_params());
        assert!(out.roll.as_normalised() > 0.0);
    }

    #[test]
    fn positive_roll_rate_is_damped() {
        // Level and centred but rolling right-down (+gyro_x): the D term must
        // oppose the motion with a negative roll demand.
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(100.0, 0.0, 0.0),
            test_params(),
        );
        assert!(
            out.roll.as_normalised() < 0.0,
            "D term should oppose positive roll rate"
        );
    }

    #[test]
    fn nose_up_pitch_commands_negative_pitch_to_correct() {
        // Nose-up (+pitch), sticks centred: command negative pitch to level. (ADR 0021)
        let (pilot, _, imu) = neutral();
        let att = Attitude::from_degrees(10.0, 0.0);
        let out = update(pilot, att, imu, test_params());
        assert!(out.pitch.as_normalised() < 0.0);
    }

    #[test]
    fn positive_pitch_stick_commands_positive_pitch() {
        let (mut pilot, att, imu) = neutral();
        pilot.pitch = PitchCommand::from_normalised(0.5);
        let out = update(pilot, att, imu, test_params());
        assert!(out.pitch.as_normalised() > 0.0);
    }

    #[test]
    fn positive_pitch_rate_is_damped() {
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(0.0, 100.0, 0.0),
            test_params(),
        );
        assert!(out.pitch.as_normalised() < 0.0);
    }

    #[test]
    fn positive_yaw_stick_commands_positive_yaw() {
        // Yaw is rate mode: +yaw stick asks for a nose-right rate. (ADR 0021/0024)
        let (mut pilot, att, imu) = neutral();
        pilot.yaw = YawCommand::from_normalised(0.5);
        let out = update(pilot, att, imu, test_params());
        assert!(out.yaw.as_normalised() > 0.0);
    }

    #[test]
    fn positive_yaw_rate_is_damped() {
        // Centred yaw stick but yawing nose-right (+gyro_z): oppose it.
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(0.0, 0.0, 100.0),
            test_params(),
        );
        assert!(out.yaw.as_normalised() < 0.0);
    }

    #[test]
    fn yaw_ignores_attitude() {
        // Yaw is rate-only (no angle term): roll/pitch attitude must not change
        // the yaw demand. (ADR 0024)
        let out_level = update(
            PilotCommand::ZERO,
            Attitude::default(),
            ImuData::default(),
            test_params(),
        );
        let out_tilted = update(
            PilotCommand::ZERO,
            Attitude::from_degrees(20.0, -15.0),
            ImuData::default(),
            test_params(),
        );
        assert_eq!(
            out_level.yaw.as_normalised(),
            out_tilted.yaw.as_normalised()
        );
    }

    #[test]
    fn roll_stick_does_not_affect_pitch_or_yaw() {
        // Per-axis independence: a roll demand must not bleed into the other axes.
        let (mut pilot, att, imu) = neutral();
        pilot.roll = RollCommand::from_normalised(0.7);
        let out = update(pilot, att, imu, test_params());
        assert_eq!(out.pitch.as_normalised(), 0.0);
        assert_eq!(out.yaw.as_normalised(), 0.0);
    }

    #[test]
    fn large_roll_error_saturates_to_full_authority() {
        // A huge attitude error must clamp to the normalised authority limit.
        let att = Attitude::from_degrees(0.0, -90.0);
        let out = update(PilotCommand::ZERO, att, ImuData::default(), test_params());
        assert_eq!(out.roll.as_normalised(), 1.0);
    }

    // -------------------------------------------------------------------------
    // Precise numerical tests: verify the actual computed value, not just sign.
    // Expected values are derived from the formulas in `update()` by hand using
    // the constants from `test_params()`.  Any wrong formula, wrong gain field,
    // or wrong normalisation divisor will produce a different number and fail.
    // -------------------------------------------------------------------------

    /// Check two f32 values are within 1e-5 of each other.
    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    // --- Roll axis ---

    #[test]
    fn roll_p_term_value_is_correct() {
        // att.roll = -10°, stick zero, gyro zero.
        // roll_out = kp_roll * (0 - (-10)) / max_tilt
        //          = 1.2 * 10 / 25 = 0.48
        let att = Attitude::from_degrees(0.0, -10.0);
        let out = update(PilotCommand::ZERO, att, ImuData::default(), test_params());
        assert!(
            approx(out.roll.as_normalised(), 0.48),
            "got {}",
            out.roll.as_normalised()
        );
        assert_eq!(out.pitch.as_normalised(), 0.0);
        assert_eq!(out.yaw.as_normalised(), 0.0);
    }

    #[test]
    fn roll_d_term_value_is_correct() {
        // att level, stick zero, gyro_x = 50 dps.
        // roll_out = -kd_roll * 50 / max_tilt_rate
        //          = -0.3 * 50 / 400 = -0.0375
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(50.0, 0.0, 0.0),
            test_params(),
        );
        assert!(
            approx(out.roll.as_normalised(), -0.0375),
            "got {}",
            out.roll.as_normalised()
        );
        assert_eq!(out.pitch.as_normalised(), 0.0);
        assert_eq!(out.yaw.as_normalised(), 0.0);
    }

    #[test]
    fn roll_p_and_d_sum_correctly() {
        // att.roll = -10°, gyro_x = 50 dps, stick zero.
        // roll_out = 1.2*10/25 - 0.3*50/400 = 0.48 - 0.0375 = 0.4425
        let att = Attitude::from_degrees(0.0, -10.0);
        let out = update(PilotCommand::ZERO, att, gyro(50.0, 0.0, 0.0), test_params());
        assert!(
            approx(out.roll.as_normalised(), 0.4425),
            "got {}",
            out.roll.as_normalised()
        );
    }

    #[test]
    fn roll_stick_scales_correctly() {
        // pilot.roll = 0.6, att level, gyro zero.
        // roll_setpoint = 0.6 * 25 = 15°
        // roll_out = kp_roll * 15 / 25 = 1.2 * 0.6 = 0.72
        let (mut pilot, att, imu) = neutral();
        pilot.roll = RollCommand::from_normalised(0.6);
        let out = update(pilot, att, imu, test_params());
        assert!(
            approx(out.roll.as_normalised(), 0.72),
            "got {}",
            out.roll.as_normalised()
        );
    }

    #[test]
    fn roll_full_output_combines_stick_attitude_and_rate() {
        // pilot.roll=0.4, att.roll=-5°, gyro_x=20 dps.
        // roll_setpoint = 0.4*25 = 10°; error = 10-(-5) = 15°
        // roll_out = 1.2*15/25 - 0.3*20/400 = 0.72 - 0.015 = 0.705
        let (mut pilot, _, _) = neutral();
        pilot.roll = RollCommand::from_normalised(0.4);
        let att = Attitude::from_degrees(0.0, -5.0);
        let out = update(pilot, att, gyro(20.0, 0.0, 0.0), test_params());
        assert!(
            approx(out.roll.as_normalised(), 0.705),
            "got {}",
            out.roll.as_normalised()
        );
    }

    // --- Pitch axis ---

    #[test]
    fn pitch_p_term_value_is_correct() {
        // att.pitch = 10° (nose up), stick zero, gyro zero.
        // pitch_out = kp_pitch * (0-10) / max_tilt
        //           = 0.8 * (-10) / 25 = -0.32
        let att = Attitude::from_degrees(10.0, 0.0);
        let out = update(PilotCommand::ZERO, att, ImuData::default(), test_params());
        assert!(
            approx(out.pitch.as_normalised(), -0.32),
            "got {}",
            out.pitch.as_normalised()
        );
        assert_eq!(out.roll.as_normalised(), 0.0);
        assert_eq!(out.yaw.as_normalised(), 0.0);
    }

    #[test]
    fn pitch_d_term_value_is_correct() {
        // att level, stick zero, gyro_y = 60 dps.
        // pitch_out = -kd_pitch * 60 / max_tilt_rate
        //           = -0.15 * 60 / 400 = -0.0225
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(0.0, 60.0, 0.0),
            test_params(),
        );
        assert!(
            approx(out.pitch.as_normalised(), -0.0225),
            "got {}",
            out.pitch.as_normalised()
        );
        assert_eq!(out.roll.as_normalised(), 0.0);
        assert_eq!(out.yaw.as_normalised(), 0.0);
    }

    #[test]
    fn pitch_p_and_d_sum_correctly() {
        // att.pitch = 10°, gyro_y = 60 dps, stick zero.
        // pitch_out = 0.8*(-10)/25 - 0.15*60/400 = -0.32 - 0.0225 = -0.3425
        let att = Attitude::from_degrees(10.0, 0.0);
        let out = update(PilotCommand::ZERO, att, gyro(0.0, 60.0, 0.0), test_params());
        assert!(
            approx(out.pitch.as_normalised(), -0.3425),
            "got {}",
            out.pitch.as_normalised()
        );
    }

    #[test]
    fn pitch_stick_scales_correctly() {
        // pilot.pitch = 0.6, att level, gyro zero.
        // pitch_setpoint = 0.6*25 = 15°
        // pitch_out = 0.8*15/25 = 0.48
        let (mut pilot, att, imu) = neutral();
        pilot.pitch = PitchCommand::from_normalised(0.6);
        let out = update(pilot, att, imu, test_params());
        assert!(
            approx(out.pitch.as_normalised(), 0.48),
            "got {}",
            out.pitch.as_normalised()
        );
    }

    #[test]
    fn pitch_full_output_combines_stick_attitude_and_rate() {
        // pilot.pitch=-0.3, att.pitch=8°, gyro_y=-40 dps.
        // pitch_setpoint = -0.3*25 = -7.5°; error = -7.5-8 = -15.5°
        // pitch_out = 0.8*(-15.5)/25 - 0.15*(-40)/400
        //           = -0.496 + 0.015 = -0.481
        let (mut pilot, _, _) = neutral();
        pilot.pitch = PitchCommand::from_normalised(-0.3);
        let att = Attitude::from_degrees(8.0, 0.0);
        let out = update(pilot, att, gyro(0.0, -40.0, 0.0), test_params());
        assert!(
            approx(out.pitch.as_normalised(), -0.481),
            "got {}",
            out.pitch.as_normalised()
        );
    }

    // --- Yaw axis ---

    #[test]
    fn yaw_p_term_value_is_correct() {
        // Stick zero, gyro_z = -30 dps (yawing left).
        // yaw_rate_setpoint = 0; error = 0-(-30) = +30
        // yaw_out = kp_yaw * 30 / max_yaw_rate = 0.6*30/180 = 0.1
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(0.0, 0.0, -30.0),
            test_params(),
        );
        assert!(
            approx(out.yaw.as_normalised(), 0.1),
            "got {}",
            out.yaw.as_normalised()
        );
        assert_eq!(out.roll.as_normalised(), 0.0);
        assert_eq!(out.pitch.as_normalised(), 0.0);
    }

    #[test]
    fn yaw_stick_scales_correctly() {
        // pilot.yaw = 0.5, gyro_z = 0.
        // yaw_setpoint = 0.5*180 = 90 dps
        // yaw_out = 0.6*(90-0)/180 = 0.3
        let (mut pilot, att, imu) = neutral();
        pilot.yaw = YawCommand::from_normalised(0.5);
        let out = update(pilot, att, imu, test_params());
        assert!(
            approx(out.yaw.as_normalised(), 0.3),
            "got {}",
            out.yaw.as_normalised()
        );
    }

    #[test]
    fn yaw_stick_and_rate_error_combine_correctly() {
        // pilot.yaw=0.4, gyro_z=18 dps.
        // yaw_setpoint = 0.4*180 = 72 dps; error = 72-18 = 54 dps
        // yaw_out = 0.6*54/180 = 0.18
        let (mut pilot, att, _) = neutral();
        pilot.yaw = YawCommand::from_normalised(0.4);
        let out = update(pilot, att, gyro(0.0, 0.0, 18.0), test_params());
        assert!(
            approx(out.yaw.as_normalised(), 0.18),
            "got {}",
            out.yaw.as_normalised()
        );
    }

    // --- Saturation ---

    #[test]
    fn negative_roll_error_saturates_to_minus_one() {
        // Huge positive roll (right-side-down): output must clamp to -1.0.
        // roll_out = 1.2*(0-90)/25 = -4.32 → clamped to -1.0
        let att = Attitude::from_degrees(0.0, 90.0);
        let out = update(PilotCommand::ZERO, att, ImuData::default(), test_params());
        assert_eq!(out.roll.as_normalised(), -1.0);
    }

    #[test]
    fn positive_pitch_error_saturates_to_plus_one() {
        // Nose-down 90°: pitch_out = 0.8*(0-(-90))/25 = 2.88 → clamped to 1.0
        let att = Attitude::from_degrees(-90.0, 0.0);
        let out = update(PilotCommand::ZERO, att, ImuData::default(), test_params());
        assert_eq!(out.pitch.as_normalised(), 1.0);
    }

    #[test]
    fn negative_pitch_error_saturates_to_minus_one() {
        // Nose-up 90°: pitch_out = 0.8*(0-90)/25 = -2.88 → clamped to -1.0
        let att = Attitude::from_degrees(90.0, 0.0);
        let out = update(PilotCommand::ZERO, att, ImuData::default(), test_params());
        assert_eq!(out.pitch.as_normalised(), -1.0);
    }

    #[test]
    fn yaw_saturates_positive() {
        // Huge leftward yaw rate: setpoint=0, gyro_z=-900 dps
        // yaw_out = 0.6*(0-(-900))/180 = 3.0 → clamped to 1.0
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(0.0, 0.0, -900.0),
            test_params(),
        );
        assert_eq!(out.yaw.as_normalised(), 1.0);
    }

    #[test]
    fn yaw_saturates_negative() {
        // Huge rightward yaw rate: setpoint=0, gyro_z=900 dps
        // yaw_out = 0.6*(0-900)/180 = -3.0 → clamped to -1.0
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(0.0, 0.0, 900.0),
            test_params(),
        );
        assert_eq!(out.yaw.as_normalised(), -1.0);
    }

    // --- Independence and passthrough ---

    #[test]
    fn throttle_does_not_affect_attitude_demands() {
        // Full throttle with level attitude and zero rate: attitude demands stay zero.
        let (mut pilot, att, imu) = neutral();
        pilot.throttle = ThrottleCommand::from_normalised(1.0);
        let out = update(pilot, att, imu, test_params());
        assert_eq!(out.roll.as_normalised(), 0.0);
        assert_eq!(out.pitch.as_normalised(), 0.0);
        assert_eq!(out.yaw.as_normalised(), 0.0);
    }

    #[test]
    fn kd_yaw_field_has_no_effect() {
        // kd_yaw is in the struct but not wired into the formula yet.
        // Changing it must not change the output.
        let p = test_params();
        let p_high_kd_yaw = ControlSystemParameters { kd_yaw: 99.9, ..p };
        let imu = gyro(0.0, 0.0, 30.0);
        let out_base = update(PilotCommand::ZERO, Attitude::default(), imu, p);
        let out_kdyaw = update(PilotCommand::ZERO, Attitude::default(), imu, p_high_kd_yaw);
        assert_eq!(out_base.yaw.as_normalised(), out_kdyaw.yaw.as_normalised());
    }

    #[test]
    fn full_negative_stick_saturates_each_axis() {
        // Roll, pitch, yaw sticks all at -1.0 with large attitude errors and
        // rates in the same direction: all three outputs must clamp to -1.0.
        //
        // Roll:  setpoint=-25°, att.roll=+90° → error=-115°; gyro_x=+900 → D negative
        //        roll_out = 1.2*(-115)/25 - 0.3*900/400 = -5.52 - 0.675 → -1.0
        // Pitch: setpoint=-25°, att.pitch=+90° → error=-115°; gyro_y=+900 → D negative
        //        pitch_out = 0.8*(-115)/25 - 0.15*900/400 = -3.68 - 0.3375 → -1.0
        // Yaw:   setpoint=-180 dps, gyro_z=+900 → error=-1080
        //        yaw_out = 0.6*(-1080)/180 = -3.6 → -1.0
        let mut pilot = PilotCommand::ZERO;
        pilot.roll = RollCommand::from_normalised(-1.0);
        pilot.pitch = PitchCommand::from_normalised(-1.0);
        pilot.yaw = YawCommand::from_normalised(-1.0);
        let att = Attitude::from_degrees(90.0, 90.0);
        let imu = gyro(900.0, 900.0, 900.0);
        let out = update(pilot, att, imu, test_params());
        assert_eq!(out.roll.as_normalised(), -1.0);
        assert_eq!(out.pitch.as_normalised(), -1.0);
        assert_eq!(out.yaw.as_normalised(), -1.0);
    }

    #[test]
    fn gains_of_zero_produce_zero_attitude_demand() {
        // If all gains are zero the controller must output zero demands regardless
        // of attitude or rate (but still pass throttle through).
        let zero_gains = ControlSystemParameters {
            kp_roll: 0.0,
            kd_roll: 0.0,
            ki_roll: 0.0,
            kp_pitch: 0.0,
            kd_pitch: 0.0,
            ki_pitch: 0.0,
            kp_yaw: 0.0,
            kd_yaw: 0.0,
            ki_yaw: 0.0,
            max_tilt_degrees: 25.0,
            max_tilt_rate_degrees_per_second: 400.0,
            max_yaw_rate_degrees_per_second: 180.0,
        };
        let att = Attitude::from_degrees(45.0, -30.0);
        let imu = gyro(100.0, 200.0, 300.0);
        let out = update(PilotCommand::ZERO, att, imu, zero_gains);
        assert_eq!(out.roll.as_normalised(), 0.0);
        assert_eq!(out.pitch.as_normalised(), 0.0);
        assert_eq!(out.yaw.as_normalised(), 0.0);
    }

    // -------------------------------------------------------------------------
    // Integral (I) term.
    //
    // The I term is *stateful* — it accumulates across ticks — so unlike the
    // P/D tests above these cannot use the fresh-per-call `update` shim. They
    // drive a single `Controller` across several `update` calls and observe the
    // integral through the output. Recall the controller uses the integral as
    // it stood at the *start* of the tick, then integrates for the next tick,
    // so the effect of a given tick's error shows up on the following call.
    // -------------------------------------------------------------------------

    /// Parameters that isolate the roll/pitch integral: P and D zeroed so the
    /// only nonzero roll/pitch output is the I term. Yaw P is left on so the
    /// "yaw never integrates" test still has a P term to watch.
    fn i_only_params(ki_roll: f32, ki_pitch: f32) -> ControlSystemParameters {
        ControlSystemParameters {
            kp_roll: 0.0,
            kd_roll: 0.0,
            ki_roll,
            kp_pitch: 0.0,
            kd_pitch: 0.0,
            ki_pitch,
            kp_yaw: 0.6,
            kd_yaw: 0.0,
            ki_yaw: 0.0,
            max_tilt_degrees: 25.0,
            max_tilt_rate_degrees_per_second: 400.0,
            max_yaw_rate_degrees_per_second: 180.0,
        }
    }

    /// A pilot command with sticks centred but throttle up, so the integrator
    /// runs — it is reset on every tick where throttle is zero.
    fn armed_pilot() -> PilotCommand {
        let mut pilot = PilotCommand::ZERO;
        pilot.throttle = ThrottleCommand::from_normalised(0.5);
        pilot
    }

    #[test]
    fn roll_integral_accumulates_linearly_with_constant_error() {
        // att.roll = -10° with max_tilt 25° -> error_norm = +0.4.
        // increment/tick = error_norm * ki_roll * dt = 0.4 * 0.5 * 0.001 = 2e-4.
        // Output uses the *previous* integral, so it lags one tick behind.
        let mut c = Controller::new();
        let p = i_only_params(0.5, 0.0);
        let att = Attitude::from_degrees(0.0, -10.0);
        let pilot = armed_pilot();
        let imu = ImuData::default();

        let o1 = c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        assert!(
            approx(o1.roll.as_normalised(), 0.0),
            "tick1 {}",
            o1.roll.as_normalised()
        );
        let o2 = c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        assert!(
            approx(o2.roll.as_normalised(), 2e-4),
            "tick2 {}",
            o2.roll.as_normalised()
        );
        let o3 = c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        assert!(
            approx(o3.roll.as_normalised(), 4e-4),
            "tick3 {}",
            o3.roll.as_normalised()
        );
    }

    #[test]
    fn roll_integral_follows_error_sign() {
        // att.roll = +10° -> error_norm = -0.4 -> integral winds negative.
        let mut c = Controller::new();
        let p = i_only_params(0.5, 0.0);
        let att = Attitude::from_degrees(0.0, 10.0);
        let pilot = armed_pilot();
        let imu = ImuData::default();

        c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        let o2 = c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        assert!(
            approx(o2.roll.as_normalised(), -2e-4),
            "got {}",
            o2.roll.as_normalised()
        );
    }

    #[test]
    fn roll_integral_stacks_on_top_of_p_term() {
        // kp_roll = 1.2, ki_roll = 0.5, gyro zero (no D). att.roll = -10°.
        // P = 1.2 * 0.4 = 0.48 (constant); I adds 2e-4/tick on top.
        let mut c = Controller::new();
        let mut p = test_params();
        p.ki_roll = 0.5;
        let att = Attitude::from_degrees(0.0, -10.0);
        let pilot = armed_pilot();
        let imu = ImuData::default();

        let o1 = c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        assert!(
            approx(o1.roll.as_normalised(), 0.48),
            "tick1 {}",
            o1.roll.as_normalised()
        );
        let o2 = c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        assert!(
            approx(o2.roll.as_normalised(), 0.48 + 2e-4),
            "tick2 {}",
            o2.roll.as_normalised()
        );
    }

    #[test]
    fn roll_integral_saturates_at_positive_max() {
        // Large ki drives the integral hard; it must clamp at +MAX_INTEGRAL and
        // never exceed it, no matter how long the error persists.
        let mut c = Controller::new();
        let p = i_only_params(100.0, 0.0);
        let att = Attitude::from_degrees(0.0, -10.0); // positive error
        let pilot = armed_pilot();
        let imu = ImuData::default();

        let mut out = 0.0;
        for _ in 0..1000 {
            out = c
                .update(p, pilot, att, imu, SAMPLE_PERIOD_S)
                .roll
                .as_normalised();
        }
        assert!(approx(out, Controller::MAX_INTEGRAL), "got {out}");
    }

    #[test]
    fn roll_integral_saturates_at_negative_max() {
        let mut c = Controller::new();
        let p = i_only_params(100.0, 0.0);
        let att = Attitude::from_degrees(0.0, 10.0); // negative error
        let pilot = armed_pilot();
        let imu = ImuData::default();

        let mut out = 0.0;
        for _ in 0..1000 {
            out = c
                .update(p, pilot, att, imu, SAMPLE_PERIOD_S)
                .roll
                .as_normalised();
        }
        assert!(approx(out, -Controller::MAX_INTEGRAL), "got {out}");
    }

    #[test]
    fn saturated_integral_unwinds_when_error_reverses() {
        // Wind hard to +MAX with a positive error, then flip the error sign.
        // Anti-windup must let it unwind immediately (the guard only blocks
        // growth *deeper* into saturation, never recovery).
        let mut c = Controller::new();
        let p = i_only_params(100.0, 0.0);
        let pilot = armed_pilot();
        let imu = ImuData::default();

        let pegged = Attitude::from_degrees(0.0, -10.0); // positive error
        for _ in 0..1000 {
            c.update(p, pilot, pegged, imu, SAMPLE_PERIOD_S);
        }
        let saturated = c
            .update(p, pilot, pegged, imu, SAMPLE_PERIOD_S)
            .roll
            .as_normalised();
        assert!(
            approx(saturated, Controller::MAX_INTEGRAL),
            "not saturated: {saturated}"
        );

        // Reverse the error. First reversed call still outputs the old (MAX)
        // integral; the *next* call reveals the unwind.
        let reversed = Attitude::from_degrees(0.0, 10.0); // negative error
        c.update(p, pilot, reversed, imu, SAMPLE_PERIOD_S);
        let unwound = c
            .update(p, pilot, reversed, imu, SAMPLE_PERIOD_S)
            .roll
            .as_normalised();
        assert!(
            unwound < saturated,
            "expected unwind, got {unwound} (was {saturated})"
        );
    }

    #[test]
    fn zero_throttle_resets_integral() {
        // Wind the integral up, then a zero-throttle tick must clear it so the
        // next armed tick starts from zero (no leftover authority after disarm).
        let mut c = Controller::new();
        let p = i_only_params(100.0, 0.0);
        let att = Attitude::from_degrees(0.0, -10.0);
        let pilot_up = armed_pilot();
        let imu = ImuData::default();

        for _ in 0..1000 {
            c.update(p, pilot_up, att, imu, SAMPLE_PERIOD_S);
        }
        let wound = c
            .update(p, pilot_up, att, imu, SAMPLE_PERIOD_S)
            .roll
            .as_normalised();
        assert!(wound > 0.4, "expected wound-up integral, got {wound}");

        // Zero-throttle tick clears the internal integral.
        c.update(p, PilotCommand::ZERO, att, imu, SAMPLE_PERIOD_S);

        // Next armed tick at zero error must produce zero output (integral gone).
        let after = c
            .update(p, pilot_up, Attitude::default(), imu, SAMPLE_PERIOD_S)
            .roll
            .as_normalised();
        assert!(approx(after, 0.0), "integral not reset, got {after}");
    }

    #[test]
    fn changing_ki_does_not_bump_output() {
        // Bumpless tuning: the integral is stored in output/contribution units,
        // so changing ki reweights only *future* increments, never the already
        // accumulated value. Two futures from the same wound-up state — one
        // keeping ki, one changing it 10x — must produce the same next output.
        let mut c = Controller::new();
        let att = Attitude::from_degrees(0.0, -10.0);
        let pilot = armed_pilot();
        let imu = ImuData::default();

        let p_a = i_only_params(0.5, 0.0);
        for _ in 0..500 {
            c.update(p_a, pilot, att, imu, SAMPLE_PERIOD_S);
        }

        let mut c_same = c; // Controller is Copy
        let mut c_changed = c;
        let out_same = c_same
            .update(p_a, pilot, att, imu, SAMPLE_PERIOD_S)
            .roll
            .as_normalised();
        let p_b = i_only_params(5.0, 0.0); // ki changed 10x
        let out_changed = c_changed
            .update(p_b, pilot, att, imu, SAMPLE_PERIOD_S)
            .roll
            .as_normalised();
        assert!(
            approx(out_same, out_changed),
            "ki change bumped output: {out_same} vs {out_changed}"
        );
    }

    #[test]
    fn pitch_integral_accumulates_independently_of_roll() {
        // ki_pitch on, ki_roll off. att.pitch = -10° -> error_norm = +0.4.
        // Pitch integral winds; roll integral must stay untouched at zero.
        let mut c = Controller::new();
        let p = i_only_params(0.0, 0.5);
        let att = Attitude::from_degrees(-10.0, 0.0);
        let pilot = armed_pilot();
        let imu = ImuData::default();

        c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        let o3 = c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        assert!(
            approx(o3.pitch.as_normalised(), 4e-4),
            "pitch {}",
            o3.pitch.as_normalised()
        );
        assert!(
            approx(o3.roll.as_normalised(), 0.0),
            "roll leaked {}",
            o3.roll.as_normalised()
        );
    }

    #[test]
    fn yaw_never_integrates() {
        // Yaw has no I term. A constant yaw-rate error over many ticks must
        // produce a constant (pure-P) output, never a growing one.
        let mut c = Controller::new();
        let p = test_params(); // kp_yaw = 0.6, ki_yaw = 0
        let pilot = armed_pilot();
        let att = Attitude::default();
        let imu = gyro(0.0, 0.0, 30.0); // 30 dps yaw rate

        // yaw_out = kp_yaw * (0 - 30) / 180 = -0.1, constant.
        let first = c
            .update(p, pilot, att, imu, SAMPLE_PERIOD_S)
            .yaw
            .as_normalised();
        assert!(approx(first, -0.1), "first {first}");
        for _ in 0..1000 {
            c.update(p, pilot, att, imu, SAMPLE_PERIOD_S);
        }
        let last = c
            .update(p, pilot, att, imu, SAMPLE_PERIOD_S)
            .yaw
            .as_normalised();
        assert!(approx(first, last), "yaw drifted: {first} -> {last}");
    }
}
