use firmware_types::{ControllerDemand, MotorCommand};

pub fn mixer(demand: ControllerDemand) -> MotorCommand {
    let throttle = demand.throttle;
    let roll = demand.roll;
    let pitch = demand.pitch;
    let yaw = demand.yaw;

    // | Motor | Corner      | Spin | Throttle | Roll (+ = right down) | Pitch (+ = nose up) | Yaw (+ = nose right) |
    // |-------|-------------|------|:--------:|:---------------------:|:-------------------:|:--------------------:|
    // | M1    | rear-right  | CCW  | +        | −                     | −                   | +                    |
    // | M2    | front-right | CW   | +        | −                     | +                   | −                    |
    // | M3    | rear-left   | CW   | +        | +                     | −                   | −                    |
    // | M4    | front-left  | CCW  | +        | +                     | +                   | +                    |

    let motor1 = throttle - roll.as_normalised() - pitch.as_normalised() + yaw.as_normalised(); // RR
    let motor2 = throttle - roll.as_normalised() + pitch.as_normalised() - yaw.as_normalised(); // FR
    let motor3 = throttle + roll.as_normalised() - pitch.as_normalised() - yaw.as_normalised(); // RL
    let motor4 = throttle + roll.as_normalised() + pitch.as_normalised() + yaw.as_normalised(); // FL

    MotorCommand {
        motor1,
        motor2,
        motor3,
        motor4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use firmware_types::{Pitch, Roll, Throttle, Yaw};

    //           front
    //      M4 (FL)     M2 (FR)
    //        CCW         CW
    //          \        /
    //           \      /
    //             hub
    //           /      \
    //          /        \
    //      M3 (RL)     M1 (RR)
    //        CW          CCW
    //           rear

    #[test]
    fn mix_just_roll() {
        let demand = ControllerDemand {
            throttle: Throttle::from_normalised(0.5),
            roll: Roll::from_normalised(0.1),
            pitch: Pitch::from_normalised(0.0),
            yaw: Yaw::from_normalised(0.0),
        };

        let motors = mixer(demand);

        //We had demanded a roll to the right, so the left motors should be faster than the right motors.
        //Just test this logic, not the exact values, as we don't want to hardcode the mixer logic in the test.

        //assert M3 > M1
        assert!(motors.motor3.as_normalised() > motors.motor1.as_normalised());

        //assert M4 > M2
        assert!(motors.motor4.as_normalised() > motors.motor2.as_normalised());

        //assert M4 > M1
        assert!(motors.motor4.as_normalised() > motors.motor1.as_normalised());

        //assert M3 > M2
        assert!(motors.motor3.as_normalised() > motors.motor2.as_normalised());
    }

    #[test]
    fn mix_just_pitch() {
        let demand = ControllerDemand {
            throttle: Throttle::from_normalised(0.5),
            roll: Roll::from_normalised(0.0),
            pitch: Pitch::from_normalised(0.1),
            yaw: Yaw::from_normalised(0.0),
        };

        let motors = mixer(demand);

        //We demanded +pitch (nose up), so the front motors should run faster than the rear motors.
        //Test the direction only, not exact values, to avoid hardcoding the mixer logic in the test.

        //assert M2 > M1 (front-right > rear-right)
        assert!(motors.motor2.as_normalised() > motors.motor1.as_normalised());

        //assert M4 > M3 (front-left > rear-left)
        assert!(motors.motor4.as_normalised() > motors.motor3.as_normalised());

        //assert M2 > M3 (front-right > rear-left)
        assert!(motors.motor2.as_normalised() > motors.motor3.as_normalised());

        //assert M4 > M1 (front-left > rear-right)
        assert!(motors.motor4.as_normalised() > motors.motor1.as_normalised());
    }

    #[test]
    fn mix_all_no_saturation() {
        let demand = ControllerDemand {
            throttle: Throttle::from_normalised(0.5),
            roll: Roll::from_normalised(0.1),
            pitch: Pitch::from_normalised(-0.2),
            yaw: Yaw::from_normalised(0.05),
        };

        let motors = mixer(demand);
        assert_eq!(motors.motor1.as_normalised(), 0.5 - 0.1 - (-0.2) + 0.05); // Rear right
        assert_eq!(motors.motor2.as_normalised(), 0.5 - 0.1 + (-0.2) - 0.05); // Front right
        assert_eq!(motors.motor3.as_normalised(), 0.5 + 0.1 - (-0.2) - 0.05); // Rear left
        assert_eq!(motors.motor4.as_normalised(), 0.5 + 0.1 + (-0.2) + 0.05); // Front left
    }

    #[test]
    fn mix_just_yaw() {
        let demand = ControllerDemand {
            throttle: Throttle::from_normalised(0.5),
            roll: Roll::from_normalised(0.0),
            pitch: Pitch::from_normalised(0.0),
            yaw: Yaw::from_normalised(0.1),
        };

        let motors = mixer(demand);

        // +yaw (nose right) is produced by the CCW motors' reaction torque, so
        // the CCW pair (M1, M4) should run faster than the CW pair (M2, M3).
        // Direction only, not exact values.

        // M1 (CCW) > M2 (CW)
        assert!(motors.motor1.as_normalised() > motors.motor2.as_normalised());
        // M1 (CCW) > M3 (CW)
        assert!(motors.motor1.as_normalised() > motors.motor3.as_normalised());
        // M4 (CCW) > M2 (CW)
        assert!(motors.motor4.as_normalised() > motors.motor2.as_normalised());
        // M4 (CCW) > M3 (CW)
        assert!(motors.motor4.as_normalised() > motors.motor3.as_normalised());
    }

    #[test]
    fn mix_just_throttle_is_even() {
        let demand = ControllerDemand {
            throttle: Throttle::from_normalised(0.5),
            roll: Roll::from_normalised(0.0),
            pitch: Pitch::from_normalised(0.0),
            yaw: Yaw::from_normalised(0.0),
        };

        let motors = mixer(demand);

        // Pure collective: every motor gets exactly the throttle, no differential.
        assert_eq!(motors.motor1.as_normalised(), 0.5);
        assert_eq!(motors.motor2.as_normalised(), 0.5);
        assert_eq!(motors.motor3.as_normalised(), 0.5);
        assert_eq!(motors.motor4.as_normalised(), 0.5);
    }

    #[test]
    fn mix_output_is_clamped() {
        // A roll demand large enough to drive the left motors past full and the
        // right motors well below idle.
        let demand = ControllerDemand {
            throttle: Throttle::from_normalised(0.9),
            roll: Roll::from_normalised(0.5),
            pitch: Pitch::from_normalised(0.0),
            yaw: Yaw::from_normalised(0.0),
        };

        let motors = mixer(demand);

        // No output ever leaves the valid 0..=1 motor range.
        assert!(
            (0.0f32..=1.0).contains(&motors.motor1.as_normalised()),
            "motor out of range: {}",
            motors.motor1.as_normalised()
        );
        assert!(
            (0.0f32..=1.0).contains(&motors.motor2.as_normalised()),
            "motor out of range: {}",
            motors.motor2.as_normalised()
        );
        assert!(
            (0.0f32..=1.0).contains(&motors.motor3.as_normalised()),
            "motor out of range: {}",
            motors.motor3.as_normalised()
        );
        assert!(
            (0.0f32..=1.0).contains(&motors.motor4.as_normalised()),
            "motor out of range: {}",
            motors.motor4.as_normalised()
        );

        // The left motors (0.9 + 0.5 = 1.4) clamp to full rather than overflowing.
        assert_eq!(motors.motor3.as_normalised(), 1.0); // M3 rear-left
        assert_eq!(motors.motor4.as_normalised(), 1.0); // M4 front-left
    }
}
