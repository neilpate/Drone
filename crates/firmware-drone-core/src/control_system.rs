use firmware_types::{
    Attitude, ControllerDemand, ImuData, PilotCommand, PitchCommand, RollCommand, YawCommand,
};

pub fn update(
    pilot_command: PilotCommand,
    attitude: Attitude,
    imu_data: ImuData,
) -> ControllerDemand {
    const MAX_TILT_DEGREES: f32 = 30.0;

    const MAX_TILT_RATE_DEGREES_PER_SECOND: f32 = 45.0;

    const MAX_YAW_RATE_DEGREES_PER_SECOND: f32 = 45.0;

    const KP_ROLL: f32 = 0.9;
    const KD_ROLL: f32 = 0.1;

    const KP_PITCH: f32 = 0.9;
    const KD_PITCH: f32 = 0.1;

    const KP_YAW: f32 = 0.5;

    let roll_setpoint = pilot_command.roll.as_normalised() * MAX_TILT_DEGREES;
    let pitch_setpoint = pilot_command.pitch.as_normalised() * MAX_TILT_DEGREES;
    let yaw_rate_setpoint = pilot_command.yaw.as_normalised() * MAX_YAW_RATE_DEGREES_PER_SECOND;

    let roll_out = KP_ROLL * (roll_setpoint - attitude.roll.as_degrees()) / MAX_TILT_DEGREES
        - KD_ROLL * imu_data.angular_rate_x.as_degrees_per_second()
            / MAX_TILT_RATE_DEGREES_PER_SECOND;

    let pitch_out = KP_PITCH * (pitch_setpoint - attitude.pitch.as_degrees()) / MAX_TILT_DEGREES
        - KD_PITCH * imu_data.angular_rate_y.as_degrees_per_second()
            / MAX_TILT_RATE_DEGREES_PER_SECOND;

    let yaw_out = KP_YAW * (yaw_rate_setpoint - imu_data.angular_rate_z.as_degrees_per_second())
        / MAX_YAW_RATE_DEGREES_PER_SECOND;

    ControllerDemand {
        throttle: pilot_command.throttle,
        roll: RollCommand::from_normalised(roll_out),
        pitch: PitchCommand::from_normalised(pitch_out),
        yaw: YawCommand::from_normalised(yaw_out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use firmware_types::{AngularRate, ThrottleCommand};

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

    #[test]
    fn throttle_passes_through_unchanged() {
        let (mut pilot, att, imu) = neutral();
        pilot.throttle = ThrottleCommand::from_normalised(0.6);
        let out = update(pilot, att, imu);
        assert_eq!(out.throttle, ThrottleCommand::from_normalised(0.6));
    }

    #[test]
    fn level_and_centred_demands_nothing() {
        let (pilot, att, imu) = neutral();
        let out = update(pilot, att, imu);
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
        let out = update(pilot, att, imu);
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
        let out = update(pilot, att, imu);
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
        let out = update(pilot, att, imu);
        assert!(out.pitch.as_normalised() < 0.0);
    }

    #[test]
    fn positive_pitch_stick_commands_positive_pitch() {
        let (mut pilot, att, imu) = neutral();
        pilot.pitch = PitchCommand::from_normalised(0.5);
        let out = update(pilot, att, imu);
        assert!(out.pitch.as_normalised() > 0.0);
    }

    #[test]
    fn positive_pitch_rate_is_damped() {
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(0.0, 100.0, 0.0),
        );
        assert!(out.pitch.as_normalised() < 0.0);
    }

    #[test]
    fn positive_yaw_stick_commands_positive_yaw() {
        // Yaw is rate mode: +yaw stick asks for a nose-right rate. (ADR 0021/0024)
        let (mut pilot, att, imu) = neutral();
        pilot.yaw = YawCommand::from_normalised(0.5);
        let out = update(pilot, att, imu);
        assert!(out.yaw.as_normalised() > 0.0);
    }

    #[test]
    fn positive_yaw_rate_is_damped() {
        // Centred yaw stick but yawing nose-right (+gyro_z): oppose it.
        let out = update(
            PilotCommand::ZERO,
            Attitude::default(),
            gyro(0.0, 0.0, 100.0),
        );
        assert!(out.yaw.as_normalised() < 0.0);
    }

    #[test]
    fn yaw_ignores_attitude() {
        // Yaw is rate-only (no angle term): roll/pitch attitude must not change
        // the yaw demand. (ADR 0024)
        let out_level = update(PilotCommand::ZERO, Attitude::default(), ImuData::default());
        let out_tilted = update(
            PilotCommand::ZERO,
            Attitude::from_degrees(20.0, -15.0),
            ImuData::default(),
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
        let out = update(pilot, att, imu);
        assert_eq!(out.pitch.as_normalised(), 0.0);
        assert_eq!(out.yaw.as_normalised(), 0.0);
    }

    #[test]
    fn large_roll_error_saturates_to_full_authority() {
        // A huge attitude error must clamp to the normalised authority limit.
        let att = Attitude::from_degrees(0.0, -90.0);
        let out = update(PilotCommand::ZERO, att, ImuData::default());
        assert_eq!(out.roll.as_normalised(), 1.0);
    }
}
