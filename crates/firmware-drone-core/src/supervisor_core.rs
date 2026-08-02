// use crate::mixer::mixer;
use firmware_types::{
    ControllerDemand, DroneState, MotorCommand, PilotCommand, PitchCommand, RollCommand,
    ThrottleCommand, YawCommand,
};

use crate::mixer::mixer;

#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Event {
    Command(PilotCommand),
    Tick,
}

pub const LINK_LOSS_TICKS: u16 = 10;
pub const RAMP_TICKS: u16 = 50;

#[derive(PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Supervisor {
    pub state: DroneState,
    ticks_without_command: u16,
    ramp_ticks: u16,
    previous_demand: PilotCommand,
    arm_gesture_detected: bool,
    disarm_gesture_detected: bool,
    gesture_detection_ticks: u16,
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Output {
    pub state: DroneState,
    pub motor_command: MotorCommand,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            state: DroneState::Initialising,
            ticks_without_command: 0,
            ramp_ticks: 0,
            previous_demand: PilotCommand::ZERO,
            arm_gesture_detected: false,
            disarm_gesture_detected: false,
            gesture_detection_ticks: 0,
        }
    }

    pub fn step(&mut self, event: Event, controller_demand: ControllerDemand) -> Output {
        match self.state {
            DroneState::Initialising => self.step_initialising(event),
            DroneState::Disarmed => self.step_disarmed(event),
            DroneState::Armed => self.step_armed(event, controller_demand),
            DroneState::Degraded => self.step_degraded(event, controller_demand),
            DroneState::Fault => self.step_fault(event),
        }
    }

    fn step_initialising(&mut self, _event: Event) -> Output {
        // For now just transition straight to disarmed
        // In the future we want to wait for all the sub systems to be ready before we allow arming

        self.state = DroneState::Disarmed;

        Output {
            state: self.state,
            motor_command: MotorCommand::ZERO,
        }

        // match event {
        //     Event::Command(_) => {

        //         self.state = DroneState::Disarmed;

        //         Output {
        //             state: self.state,
        //             motor_command: MotorCommand::ZERO,
        //         }
        //     }
        //     Event::Tick => Output {
        //         state: DroneState::Disarmed,
        //         motor_command: MotorCommand::ZERO,
        //     },
        // }
    }

    fn step_disarmed(&mut self, event: Event) -> Output {
        match event {
            Event::Command(cmd) => {
                //To transition to armed the throttle must be held at zero while the yaw is held at full right for a few seconds. This is to prevent accidental arming of the drone.
                //For now we will just check for a single command with the throttle at zero and the yaw at full right to transition to armed. In the future we will want to implement a timer to ensure that the gesture is held for a few seconds before transitioning to armed.

                if (cmd.throttle == ThrottleCommand::ZERO)
                    && cmd.yaw == YawCommand::ZERO
                    && self.arm_gesture_detected
                {
                    self.state = DroneState::Armed;
                    self.arm_gesture_detected = false; // Reset the arm gesture detection flag after arming
                    self.ticks_without_command = 0;
                    self.previous_demand = PilotCommand::ZERO;
                    self.gesture_detection_ticks = 0;
                }

                if cmd.throttle == ThrottleCommand::ZERO && cmd.yaw.as_normalised() >= 0.95 {
                    // Gesture is in process of being detected, increment the counter and check if it has been held for long enough to be considered a valid gesture

                    self.gesture_detection_ticks = self.gesture_detection_ticks.saturating_add(1);

                    if self.gesture_detection_ticks >= 50 {
                        self.arm_gesture_detected = true;
                    }
                } else {
                    // Reset the gesture detection if the throttle is not zero or the yaw is not full right

                    self.gesture_detection_ticks = 0;
                }

                Output {
                    state: self.state,
                    motor_command: MotorCommand::ZERO,
                }
            }
            Event::Tick => Output {
                state: self.state,
                motor_command: MotorCommand::ZERO,
            },
        }
    }

    fn step_armed(&mut self, event: Event, controller_demand: ControllerDemand) -> Output {
        match event {
            Event::Command(cmd) => {
                if (cmd.throttle == ThrottleCommand::ZERO)
                    && cmd.yaw == YawCommand::ZERO
                    && self.disarm_gesture_detected
                {
                    self.state = DroneState::Disarmed;
                    self.disarm_gesture_detected = false; // Reset the disarm gesture detection flag after disarming
                    self.ticks_without_command = 0;
                    self.previous_demand = PilotCommand::ZERO;
                }

                if cmd.throttle == ThrottleCommand::ZERO && cmd.yaw.as_normalised() <= -0.95 {
                    self.disarm_gesture_detected = true;
                }

                self.previous_demand = cmd;
                self.ticks_without_command = 0;

                let mixed = mixer(controller_demand);

                Output {
                    state: self.state,
                    motor_command: mixed,
                }
            }
            Event::Tick => {
                self.ticks_without_command = self.ticks_without_command.saturating_add(1);

                if self.ticks_without_command >= LINK_LOSS_TICKS {
                    self.state = DroneState::Degraded;
                    self.ramp_ticks = 0;

                    let controller_demand = ControllerDemand {
                        throttle: self.previous_demand.throttle,
                        roll: RollCommand::ZERO,
                        pitch: PitchCommand::ZERO,
                        yaw: YawCommand::ZERO,
                    };

                    let mixed = mixer(controller_demand);
                    Output {
                        state: self.state,
                        motor_command: mixed,
                    }
                } else {
                    let mixed = mixer(controller_demand);

                    Output {
                        state: self.state,
                        motor_command: mixed,
                    }
                }
            }
        }
    }

    fn step_degraded(&mut self, event: Event, _controller_demand: ControllerDemand) -> Output {
        match event {
            Event::Command(cmd) => {
                //We received some kind of command, so the link is back. But for safety we will transition back to disarmed.

                if cmd.throttle == ThrottleCommand::ZERO {
                    self.state = DroneState::Disarmed;
                    self.ticks_without_command = 0;
                    self.previous_demand = PilotCommand::ZERO;
                    self.arm_gesture_detected = false;
                    self.disarm_gesture_detected = false;
                }

                let controller_demand = ControllerDemand::ZERO;

                let mixed = mixer(controller_demand);

                Output {
                    state: self.state,
                    motor_command: mixed,
                }
            }
            Event::Tick => {
                // Ramp down throttle to zero over RAMP_TICKS ticks, then stay at zero.
                let remaining = RAMP_TICKS.saturating_sub(self.ramp_ticks);
                let factor = remaining as f32 / RAMP_TICKS as f32;
                let ramped_throttle = self.previous_demand.throttle * factor;
                self.ramp_ticks = self.ramp_ticks.saturating_add(1);

                // In Degraded state, attitude is neutralised to zero and the throttle is ramped down to zero. This ensures that the drone does not continue to fly uncontrollably after losing link with the pilot.
                let controller_demand = ControllerDemand {
                    throttle: ramped_throttle,
                    roll: RollCommand::ZERO,
                    pitch: PitchCommand::ZERO,
                    yaw: YawCommand::ZERO,
                };

                let mixed = mixer(controller_demand);

                Output {
                    state: self.state,
                    motor_command: mixed,
                }
            }
        }
    }

    fn step_fault(&mut self, _event: Event) -> Output {
        Output {
            state: self.state,
            motor_command: MotorCommand::ZERO,
        }
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use firmware_types::{
        ControlMode, PilotCommand, PitchCommand, RollCommand, ThrottleCommand, YawCommand,
    };

    /// Build a `Command` event at a given normalised throttle (yaw and attitude
    /// all zero).
    fn cmd(throttle: f32) -> Event {
        Event::Command(PilotCommand {
            sequence_count: 0,
            throttle: ThrottleCommand::from_normalised(throttle),
            roll: RollCommand::ZERO,
            pitch: PitchCommand::ZERO,
            yaw: YawCommand::ZERO,
            control_mode: ControlMode::Stabilized,
        })
    }

    /// Build a `Command` event with the given throttle and yaw deflections.
    fn cmd_yaw(throttle: f32, yaw: f32) -> Event {
        Event::Command(PilotCommand {
            sequence_count: 0,
            throttle: ThrottleCommand::from_normalised(throttle),
            roll: RollCommand::ZERO,
            pitch: PitchCommand::ZERO,
            yaw: YawCommand::from_normalised(yaw),
            control_mode: ControlMode::Stabilized,
        })
    }

    /// These tests only ever issue zero-deflection commands, so every
    /// `MotorCommand` the supervisor produces is pure collective: all four
    /// motors equal. Assert coherence across all four components and return
    /// the shared normalised value.
    fn collective(mc: MotorCommand) -> f32 {
        let v = mc.motor1.as_normalised();
        assert_eq!(mc.motor2.as_normalised(), v, "motor2 differs from motor1");
        assert_eq!(mc.motor3.as_normalised(), v, "motor3 differs from motor1");
        assert_eq!(mc.motor4.as_normalised(), v, "motor4 differs from motor1");
        v
    }

    /// Drive a fresh `Supervisor` through the full arm sequence, leaving it in
    /// `DroneState::Armed` with the link-loss counter at zero.
    ///
    /// Sequence:
    ///   1. One `Tick` — Initialising → Disarmed.
    ///   2. 50 commands: throttle=0, yaw=1.0 — gesture window fills.
    ///   3. One command: throttle=0, yaw=0 — arm fires.
    fn arm_supervisor(s: &mut Supervisor) {
        s.step(Event::Tick, ControllerDemand::ZERO);
        for _ in 0..50 {
            s.step(cmd_yaw(0.0, 1.0), ControllerDemand::ZERO);
        }
        s.step(cmd(0.0), ControllerDemand::ZERO);
        assert_eq!(
            s.state,
            DroneState::Armed,
            "arm_supervisor: did not reach Armed"
        );
    }

    #[test]
    fn initialising_transitions_to_disarmed_on_first_event() {
        let mut s = Supervisor::new();
        let out = s.step(Event::Tick, ControllerDemand::ZERO);
        assert_eq!(out.state, DroneState::Disarmed);
        assert_eq!(collective(out.motor_command), 0.0);
    }

    #[test]
    fn first_command_also_transitions_to_disarmed() {
        let mut s = Supervisor::new();
        let out = s.step(cmd(0.0), ControllerDemand::ZERO);
        assert_eq!(out.state, DroneState::Disarmed);
        assert_eq!(collective(out.motor_command), 0.0);
    }

    #[test]
    fn arm_transition_outputs_zero_motors() {
        // The arm transition must not produce a motor spike regardless of
        // controller demand at that instant.
        let mut s = Supervisor::new();
        s.step(Event::Tick, ControllerDemand::ZERO);
        for _ in 0..50 {
            s.step(cmd_yaw(0.0, 1.0), ControllerDemand::ZERO);
        }
        let demand = ControllerDemand {
            throttle: ThrottleCommand::from_normalised(0.5),
            roll: RollCommand::from_normalised(0.3),
            pitch: PitchCommand::ZERO,
            yaw: YawCommand::ZERO,
        };
        let out = s.step(cmd(0.0), demand);
        assert_eq!(out.state, DroneState::Armed);
        assert_eq!(collective(out.motor_command), 0.0);
    }

    #[test]
    fn armed_command_resets_link_loss_counter() {
        let mut s = Supervisor::new();
        arm_supervisor(&mut s);
        s.step(cmd(0.5), ControllerDemand::ZERO);
        // accumulate ticks just short of the threshold
        for _ in 0..LINK_LOSS_TICKS - 1 {
            s.step(Event::Tick, ControllerDemand::ZERO);
        }
        // a fresh command must reset the counter
        s.step(cmd(0.7), ControllerDemand::ZERO);
        // we should now be able to tick almost the full threshold again
        for _ in 0..LINK_LOSS_TICKS - 1 {
            assert_eq!(
                s.step(Event::Tick, ControllerDemand::ZERO).state,
                DroneState::Armed
            );
        }
    }

    #[test]
    fn armed_degrades_after_link_loss_ticks() {
        let mut s = Supervisor::new();
        arm_supervisor(&mut s);
        s.step(cmd(0.5), ControllerDemand::ZERO);
        // up to LINK_LOSS_TICKS - 1 silent ticks: still Armed
        for _ in 0..LINK_LOSS_TICKS - 1 {
            assert_eq!(
                s.step(Event::Tick, ControllerDemand::ZERO).state,
                DroneState::Armed
            );
        }
        // the next tick crosses the threshold
        assert_eq!(
            s.step(Event::Tick, ControllerDemand::ZERO).state,
            DroneState::Degraded
        );
    }

    #[test]
    fn degraded_ramps_monotonically_to_zero_then_holds() {
        let mut s = Supervisor::new();
        arm_supervisor(&mut s);
        s.step(cmd(1.0), ControllerDemand::ZERO);
        // drive the state into Degraded
        for _ in 0..LINK_LOSS_TICKS {
            s.step(Event::Tick, ControllerDemand::ZERO);
        }
        assert_eq!(s.state, DroneState::Degraded);

        // Ramp produces RAMP_TICKS + 1 samples: factor goes 50/50, 49/50, ..., 0/50.
        // Output must be non-increasing and the final sample exactly zero.
        let mut last = 1.0_f32;
        for i in 0..=RAMP_TICKS {
            let t = collective(s.step(Event::Tick, ControllerDemand::ZERO).motor_command);
            assert!(
                t <= last,
                "ramp not monotonic at tick {}: {} > {}",
                i,
                t,
                last
            );
            last = t;
        }
        assert_eq!(
            last, 0.0,
            "ramp did not reach zero after RAMP_TICKS + 1 ticks"
        );

        // Further ticks stay clamped at zero; state remains Degraded until
        // an explicit zero-throttle command recovers it.
        for _ in 0..5 {
            let out = s.step(Event::Tick, ControllerDemand::ZERO);
            assert_eq!(out.state, DroneState::Degraded);
            assert_eq!(collective(out.motor_command), 0.0);
        }
    }

    #[test]
    fn degraded_refuses_re_engage_with_nonzero_throttle() {
        let mut s = Supervisor::new();
        arm_supervisor(&mut s);
        s.step(cmd(0.5), ControllerDemand::ZERO);
        for _ in 0..LINK_LOSS_TICKS {
            s.step(Event::Tick, ControllerDemand::ZERO);
        }
        assert_eq!(s.state, DroneState::Degraded);

        // a fresh non-zero command must NOT re-arm
        let out = s.step(cmd(0.8), ControllerDemand::ZERO);
        assert_eq!(out.state, DroneState::Degraded);
        assert_eq!(collective(out.motor_command), 0.0);
    }

    #[test]
    fn degraded_recovers_to_disarmed_with_zero_command() {
        let mut s = Supervisor::new();
        arm_supervisor(&mut s);
        s.step(cmd(0.5), ControllerDemand::ZERO);
        // fully degrade and ramp out
        for _ in 0..LINK_LOSS_TICKS + RAMP_TICKS {
            s.step(Event::Tick, ControllerDemand::ZERO);
        }
        assert_eq!(s.state, DroneState::Degraded);

        // a zero-throttle command recovers to Disarmed (not Armed directly)
        let out = s.step(cmd(0.0), ControllerDemand::ZERO);
        assert_eq!(out.state, DroneState::Disarmed);
        assert_eq!(collective(out.motor_command), 0.0);

        // a subsequent tick stays in Disarmed (counter was reset on recovery)
        assert_eq!(
            s.step(Event::Tick, ControllerDemand::ZERO).state,
            DroneState::Disarmed,
            "counter must reset on recovery"
        );
    }

    #[test]
    fn fault_absorbs_all_events() {
        let mut s = Supervisor::new();
        // there is currently no public path into Fault; reach in for the test
        s.state = DroneState::Fault;

        let out = s.step(cmd(1.0), ControllerDemand::ZERO);
        assert_eq!(out.state, DroneState::Fault);
        assert_eq!(collective(out.motor_command), 0.0);

        let out = s.step(Event::Tick, ControllerDemand::ZERO);
        assert_eq!(out.state, DroneState::Fault);
        assert_eq!(collective(out.motor_command), 0.0);
    }

    #[test]
    fn armed_mixes_controller_demand_not_pilot_command() {
        let mut s = Supervisor::new();
        arm_supervisor(&mut s);

        // While Armed, a Command mixes the CONTROLLER demand (the stabilised
        // output), not the raw pilot sticks. Assert the output is exactly what
        // the mixer produces for the controller_demand argument, and that the
        // pilot command's own attitude does not leak through.
        let demand = ControllerDemand {
            throttle: ThrottleCommand::from_normalised(0.5),
            roll: RollCommand::from_normalised(0.2),
            pitch: PitchCommand::from_normalised(-0.1),
            yaw: YawCommand::from_normalised(0.05),
        };
        let expected = mixer(demand);

        // The pilot command carries deliberately different attitude to prove it
        // is ignored in favour of the controller demand while Armed.
        let out = s.step(
            Event::Command(PilotCommand {
                sequence_count: 1,
                throttle: ThrottleCommand::from_normalised(0.5),
                roll: RollCommand::from_normalised(-0.9),
                pitch: PitchCommand::from_normalised(0.9),
                yaw: YawCommand::from_normalised(-0.9),
                control_mode: ControlMode::Stabilized,
            }),
            demand,
        );

        assert_eq!(out.state, DroneState::Armed);
        assert_eq!(out.motor_command, expected);
        // Sanity: this really is a differential command, not accidentally collective.
        assert_ne!(
            out.motor_command.motor1.as_normalised(),
            out.motor_command.motor3.as_normalised()
        );
    }

    #[test]
    fn armed_first_command_mixes_controller_demand() {
        // The first command while Armed mixes the controller demand (stabilised
        // output), not the raw pilot sticks.
        let mut s = Supervisor::new();
        arm_supervisor(&mut s);

        let demand = ControllerDemand {
            throttle: ThrottleCommand::from_normalised(0.4),
            roll: RollCommand::from_normalised(0.3),
            pitch: PitchCommand::ZERO,
            yaw: YawCommand::ZERO,
        };
        let expected = mixer(demand);

        // The pilot command carries deliberately different attitude to prove it
        // is ignored in favour of the controller demand.
        let out = s.step(
            Event::Command(PilotCommand {
                sequence_count: 0,
                throttle: ThrottleCommand::from_normalised(0.4),
                roll: RollCommand::from_normalised(-0.9),
                pitch: PitchCommand::from_normalised(0.9),
                yaw: YawCommand::ZERO,
                control_mode: ControlMode::Stabilized,
            }),
            demand,
        );

        assert_eq!(out.state, DroneState::Armed);
        assert_eq!(out.motor_command, expected);
    }

    #[test]
    fn armed_tick_mixes_controller_demand() {
        // A missed command (Tick while Armed, before link loss) still mixes the
        // latest controller demand: the stabilised output keeps reaching the
        // motors even when no fresh pilot frame arrives.
        let mut s = Supervisor::new();
        arm_supervisor(&mut s);

        let demand = ControllerDemand {
            throttle: ThrottleCommand::from_normalised(0.5),
            roll: RollCommand::from_normalised(0.3),
            pitch: PitchCommand::ZERO,
            yaw: YawCommand::ZERO,
        };
        let expected = mixer(demand);

        let out = s.step(Event::Tick, demand);
        assert_eq!(out.state, DroneState::Armed);
        assert_eq!(out.motor_command, expected);
        // Sanity: the demand is a genuine differential, not neutralised.
        assert_ne!(
            out.motor_command.motor1.as_normalised(),
            out.motor_command.motor3.as_normalised()
        );
    }

    #[test]
    fn degraded_neutralises_attitude_on_the_transition_tick() {
        // Held attitude must not leak into the failsafe: the very tick that trips
        // Degraded must already be wings-level — attitude zeroed the instant loss
        // is declared, not one tick later — while the throttle is still held.
        let mut s = Supervisor::new();
        arm_supervisor(&mut s);

        s.step(
            Event::Command(PilotCommand {
                sequence_count: 0,
                throttle: ThrottleCommand::from_normalised(0.5),
                roll: RollCommand::from_normalised(0.3),
                pitch: PitchCommand::ZERO,
                yaw: YawCommand::ZERO,
                control_mode: ControlMode::Stabilized,
            }),
            ControllerDemand::ZERO,
        );

        // Silent ticks up to (but not including) the threshold: still Armed.
        for _ in 0..LINK_LOSS_TICKS - 1 {
            let out = s.step(Event::Tick, ControllerDemand::ZERO);
            assert_eq!(out.state, DroneState::Armed);
        }

        // The threshold-crossing tick transitions to Degraded AND neutralises
        // attitude in the same output.
        let out = s.step(Event::Tick, ControllerDemand::ZERO);
        assert_eq!(out.state, DroneState::Degraded);
        // Attitude gone (collective), throttle still held on this first frame.
        assert_eq!(collective(out.motor_command), 0.5);
    }
}
