// use crate::mixer::mixer;
use firmware_types::{
    ControllerDemand, DroneState, MotorCommand, PilotCommand, Pitch, Roll, Throttle, Yaw,
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
    state: DroneState,
    ticks_without_command: u16,
    ramp_ticks: u16,
    previous_demand: PilotCommand,
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
        }
    }

    pub fn step(&mut self, event: Event) -> Output {
        match self.state {
            DroneState::Initialising => self.step_initialising(event),
            DroneState::Armed => self.step_armed(event),
            DroneState::Degraded => self.step_degraded(event),
            DroneState::Fault => self.step_fault(event),
        }
    }

    fn step_initialising(&mut self, event: Event) -> Output {
        match event {
            Event::Command(cmd) => {
                self.state = DroneState::Armed;

                self.previous_demand = cmd;

                self.ticks_without_command = 0;

                let controller_demand = ControllerDemand {
                    throttle: cmd.throttle,
                    roll: cmd.roll,
                    pitch: cmd.pitch,
                    yaw: cmd.yaw,
                };

                let mix = mixer(controller_demand);

                Output {
                    state: self.state,
                    motor_command: mix,
                }
            }
            Event::Tick => {
                let controller_demand = ControllerDemand::ZERO;

                let mix = mixer(controller_demand);

                Output {
                    state: self.state,
                    motor_command: mix,
                }
            }
        }
    }

    fn step_armed(&mut self, event: Event) -> Output {
        match event {
            Event::Command(cmd) => {
                self.previous_demand = cmd;
                self.ticks_without_command = 0;

                let controller_demand = ControllerDemand {
                    throttle: cmd.throttle,
                    roll: cmd.roll,
                    pitch: cmd.pitch,
                    yaw: cmd.yaw,
                };

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
                        roll: Roll::ZERO,
                        pitch: Pitch::ZERO,
                        yaw: Yaw::ZERO,
                    };

                    let mixed = mixer(controller_demand);
                    Output {
                        state: self.state,
                        motor_command: mixed,
                    }
                } else {
                    let controller_demand = ControllerDemand {
                        throttle: self.previous_demand.throttle,
                        roll: self.previous_demand.roll,
                        pitch: self.previous_demand.pitch,
                        yaw: self.previous_demand.yaw,
                    };

                    let mixed = mixer(controller_demand);

                    Output {
                        state: self.state,
                        motor_command: mixed,
                    }
                }
            }
        }
    }

    fn step_degraded(&mut self, event: Event) -> Output {
        match event {
            Event::Command(cmd) => {
                if cmd.throttle == Throttle::ZERO {
                    self.state = DroneState::Armed;
                    self.ticks_without_command = 0;
                    self.previous_demand = PilotCommand::ZERO;
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
                    roll: Roll::ZERO,
                    pitch: Pitch::ZERO,
                    yaw: Yaw::ZERO,
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
        let controller_demand = ControllerDemand::ZERO;

        let mixed = mixer(controller_demand);

        Output {
            state: self.state,
            motor_command: mixed,
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
    use firmware_types::{PilotCommand, Pitch, Roll, Throttle, Yaw};

    /// Build a `Command` event at a given normalised throttle.
    fn cmd(throttle: f32) -> Event {
        Event::Command(PilotCommand {
            sequence_count: 0,
            throttle: Throttle::from_normalised(throttle),
            roll: Roll::ZERO,
            pitch: Pitch::ZERO,
            yaw: Yaw::ZERO,
        })
    }

    /// These tests only ever issue zero-deflection commands, so every
    /// `MotorCommand` the supervisor produces is pure collective: all four
    /// motors equal. Assert that coherence across all four components and
    /// return the shared normalised value.
    fn collective(mc: MotorCommand) -> f32 {
        let v = mc.motor1.as_normalised();
        assert_eq!(mc.motor2.as_normalised(), v, "motor2 differs from motor1");
        assert_eq!(mc.motor3.as_normalised(), v, "motor3 differs from motor1");
        assert_eq!(mc.motor4.as_normalised(), v, "motor4 differs from motor1");
        v
    }

    #[test]
    fn initialising_tick_stays_initialising_with_zero_throttle() {
        let mut s = Supervisor::new();
        let out = s.step(Event::Tick);
        assert_eq!(out.state, DroneState::Initialising);
        assert_eq!(collective(out.motor_command), 0.0);
    }

    #[test]
    fn initialising_then_command_arms() {
        let mut s = Supervisor::new();
        let out = s.step(cmd(0.5));
        assert_eq!(out.state, DroneState::Armed);
        assert_eq!(collective(out.motor_command), 0.5);
    }

    #[test]
    fn armed_command_resets_link_loss_counter() {
        let mut s = Supervisor::new();
        s.step(cmd(0.5));
        // accumulate ticks just short of the threshold
        for _ in 0..LINK_LOSS_TICKS - 1 {
            s.step(Event::Tick);
        }
        // a fresh command must reset the counter
        s.step(cmd(0.7));
        // we should now be able to tick almost the full threshold again
        for _ in 0..LINK_LOSS_TICKS - 1 {
            assert_eq!(s.step(Event::Tick).state, DroneState::Armed);
        }
    }

    #[test]
    fn armed_degrades_after_link_loss_ticks() {
        let mut s = Supervisor::new();
        s.step(cmd(0.5));
        // up to LINK_LOSS_TICKS - 1 silent ticks: still Armed
        for _ in 0..LINK_LOSS_TICKS - 1 {
            assert_eq!(s.step(Event::Tick).state, DroneState::Armed);
        }
        // the next tick crosses the threshold
        assert_eq!(s.step(Event::Tick).state, DroneState::Degraded);
    }

    #[test]
    fn degraded_ramps_monotonically_to_zero_then_holds() {
        let mut s = Supervisor::new();
        s.step(cmd(1.0));
        // drive the state into Degraded
        for _ in 0..LINK_LOSS_TICKS {
            s.step(Event::Tick);
        }
        assert_eq!(s.state, DroneState::Degraded);

        // Ramp produces RAMP_TICKS + 1 samples: factor goes 50/50, 49/50, ..., 0/50.
        // Output must be non-increasing and the final sample exactly zero.
        let mut last = 1.0_f32;
        for i in 0..=RAMP_TICKS {
            let t = collective(s.step(Event::Tick).motor_command);
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

        // further ticks stay clamped at zero
        for _ in 0..5 {
            assert_eq!(collective(s.step(Event::Tick).motor_command), 0.0);
        }
    }

    #[test]
    fn degraded_refuses_re_engage_with_nonzero_throttle() {
        let mut s = Supervisor::new();
        s.step(cmd(0.5));
        for _ in 0..LINK_LOSS_TICKS {
            s.step(Event::Tick);
        }
        assert_eq!(s.state, DroneState::Degraded);

        // a fresh non-zero command must NOT re-arm
        let out = s.step(cmd(0.8));
        assert_eq!(out.state, DroneState::Degraded);
        assert_eq!(collective(out.motor_command), 0.0);
    }

    #[test]
    fn degraded_recovers_cleanly_with_zero_command() {
        let mut s = Supervisor::new();
        s.step(cmd(0.5));
        // fully degrade and ramp out
        for _ in 0..LINK_LOSS_TICKS + RAMP_TICKS {
            s.step(Event::Tick);
        }
        assert_eq!(s.state, DroneState::Degraded);

        // a zero-throttle command re-arms
        let out = s.step(cmd(0.0));
        assert_eq!(out.state, DroneState::Armed);

        // crucial: the link-loss counter must have reset, otherwise
        // the very next tick flips us straight back to Degraded
        assert_eq!(
            s.step(Event::Tick).state,
            DroneState::Armed,
            "counter must reset on recovery"
        );
    }

    #[test]
    fn fault_absorbs_all_events() {
        let mut s = Supervisor::new();
        // there is currently no public path into Fault; reach in for the test
        s.state = DroneState::Fault;

        let out = s.step(cmd(1.0));
        assert_eq!(out.state, DroneState::Fault);
        assert_eq!(collective(out.motor_command), 0.0);

        let out = s.step(Event::Tick);
        assert_eq!(out.state, DroneState::Fault);
        assert_eq!(collective(out.motor_command), 0.0);
    }

    #[test]
    fn armed_forwards_attitude_command_through_mixer() {
        let mut s = Supervisor::new();
        s.step(cmd(0.0)); // Initialising -> Armed

        // A full command with attitude demand, while Armed, must be forwarded to
        // the mixer verbatim (throttle + all three axes), not flattened to
        // collective. Assert the output is exactly what the mixer produces for
        // the same demand.
        let throttle = Throttle::from_normalised(0.5);
        let roll = Roll::from_normalised(0.2);
        let pitch = Pitch::from_normalised(-0.1);
        let yaw = Yaw::from_normalised(0.05);

        let expected = mixer(ControllerDemand {
            throttle,
            roll,
            pitch,
            yaw,
        });

        let out = s.step(Event::Command(PilotCommand {
            sequence_count: 1,
            throttle,
            roll,
            pitch,
            yaw,
        }));

        assert_eq!(out.state, DroneState::Armed);
        assert_eq!(out.motor_command, expected);
        // Sanity: this really is a differential command, not accidentally collective.
        assert_ne!(
            out.motor_command.motor1.as_normalised(),
            out.motor_command.motor3.as_normalised()
        );
    }

    #[test]
    fn arming_command_forwards_attitude() {
        // The very first command (Initialising -> Armed) must also carry its
        // attitude through, not just throttle.
        let mut s = Supervisor::new();

        let throttle = Throttle::from_normalised(0.4);
        let roll = Roll::from_normalised(0.3);

        let expected = mixer(ControllerDemand {
            throttle,
            roll,
            pitch: Pitch::ZERO,
            yaw: Yaw::ZERO,
        });

        let out = s.step(Event::Command(PilotCommand {
            sequence_count: 0,
            throttle,
            roll,
            pitch: Pitch::ZERO,
            yaw: Yaw::ZERO,
        }));

        assert_eq!(out.state, DroneState::Armed);
        assert_eq!(out.motor_command, expected);
    }

    #[test]
    fn armed_tick_holds_full_previous_demand() {
        // A missed command (Tick while Armed, before link loss) is a zero-order
        // hold of the ENTIRE last demand - throttle AND attitude - so the output
        // matches mixing the previous command verbatim.
        let mut s = Supervisor::new();

        let throttle = Throttle::from_normalised(0.5);
        let roll = Roll::from_normalised(0.3);

        s.step(Event::Command(PilotCommand {
            sequence_count: 0,
            throttle,
            roll,
            pitch: Pitch::ZERO,
            yaw: Yaw::ZERO,
        }));

        let expected = mixer(ControllerDemand {
            throttle,
            roll,
            pitch: Pitch::ZERO,
            yaw: Yaw::ZERO,
        });

        let out = s.step(Event::Tick);
        assert_eq!(out.state, DroneState::Armed);
        assert_eq!(out.motor_command, expected);
        // Sanity: attitude is genuinely held (differential), not neutralised.
        assert_ne!(
            out.motor_command.motor1.as_normalised(),
            out.motor_command.motor3.as_normalised()
        );
    }

    #[test]
    fn degraded_neutralises_attitude_on_the_transition_tick() {
        // Held attitude must not leak into the failsafe: the very tick that trips
        // Degraded must already be wings-level - attitude zeroed the instant loss
        // is declared, not one tick later - while the throttle is still held.
        let mut s = Supervisor::new();

        s.step(Event::Command(PilotCommand {
            sequence_count: 0,
            throttle: Throttle::from_normalised(0.5),
            roll: Roll::from_normalised(0.3),
            pitch: Pitch::ZERO,
            yaw: Yaw::ZERO,
        }));

        // Silent ticks up to (but not including) the threshold: still Armed.
        for _ in 0..LINK_LOSS_TICKS - 1 {
            let out = s.step(Event::Tick);
            assert_eq!(out.state, DroneState::Armed);
        }

        // The threshold-crossing tick transitions to Degraded AND neutralises
        // attitude in the same output.
        let out = s.step(Event::Tick);
        assert_eq!(out.state, DroneState::Degraded);
        // Attitude gone (collective), throttle still held on this first frame.
        assert_eq!(collective(out.motor_command), 0.5);
    }
}
