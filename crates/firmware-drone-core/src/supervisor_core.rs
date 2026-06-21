use firmware_types::{DroneState, PilotCommand, Throttle};

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
    previous_throttle: Throttle,
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Output {
    pub state: DroneState,
    pub throttle: Throttle,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            state: DroneState::Initialising,
            ticks_without_command: 0,
            ramp_ticks: 0,
            previous_throttle: Throttle::ZERO,
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
                self.previous_throttle = cmd.throttle;
                self.ticks_without_command = 0;
                Output {
                    state: self.state,
                    throttle: cmd.throttle,
                }
            }
            Event::Tick => Output {
                state: self.state,
                throttle: Throttle::ZERO,
            },
        }
    }

    fn step_armed(&mut self, event: Event) -> Output {
        match event {
            Event::Command(cmd) => {
                self.previous_throttle = cmd.throttle;
                self.ticks_without_command = 0;
                Output {
                    state: self.state,
                    throttle: cmd.throttle,
                }
            }
            Event::Tick => {
                self.ticks_without_command = self.ticks_without_command.saturating_add(1);

                if self.ticks_without_command >= LINK_LOSS_TICKS {
                    self.state = DroneState::Degraded;
                    self.ramp_ticks = 0;
                }

                Output {
                    state: self.state,
                    throttle: self.previous_throttle,
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
                    self.previous_throttle = Throttle::ZERO;
                }

                Output {
                    state: self.state,
                    throttle: Throttle::ZERO,
                }
            }
            Event::Tick => {
                // Ramp down throttle to zero over RAMP_TICKS ticks, then stay at zero.
                let remaining = RAMP_TICKS.saturating_sub(self.ramp_ticks);
                let factor = remaining as f32 / RAMP_TICKS as f32;
                let ramped = self.previous_throttle * factor;
                self.ramp_ticks = self.ramp_ticks.saturating_add(1);
                Output {
                    state: self.state,
                    throttle: ramped,
                }
            }
        }
    }

    fn step_fault(&mut self, _event: Event) -> Output {
        Output {
            state: self.state,
            throttle: Throttle::ZERO,
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

    #[test]
    fn initialising_tick_stays_initialising_with_zero_throttle() {
        let mut s = Supervisor::new();
        let out = s.step(Event::Tick);
        assert_eq!(out.state, DroneState::Initialising);
        assert_eq!(out.throttle, Throttle::ZERO);
    }

    #[test]
    fn initialising_then_command_arms() {
        let mut s = Supervisor::new();
        let out = s.step(cmd(0.5));
        assert_eq!(out.state, DroneState::Armed);
        assert_eq!(out.throttle.as_normalised(), 0.5);
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
            let t = s.step(Event::Tick).throttle.as_normalised();
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
            assert_eq!(s.step(Event::Tick).throttle, Throttle::ZERO);
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
        assert_eq!(out.throttle, Throttle::ZERO);
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
        assert_eq!(out.throttle, Throttle::ZERO);

        let out = s.step(Event::Tick);
        assert_eq!(out.state, DroneState::Fault);
        assert_eq!(out.throttle, Throttle::ZERO);
    }
}
