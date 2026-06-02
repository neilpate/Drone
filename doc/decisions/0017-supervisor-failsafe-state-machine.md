# ADR 0017 — Supervisor task: failsafe state machine driving safe motor commands

- **Status:** Accepted
- **Date:** 2026-06-02
- **Related:** [ADR 0004](0004-concurrency-embassy-channels.md) (actor-per-task), [ADR 0007](0007-testing-and-ci-strategy.md) (`core`/`task` split), [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md) (crate naming), [ADR 0013](0013-async-communication-primitives.md) (Watch for state), [ADR 0015](0015-host-testing-no-std-crates.md) (host-test wiring)

## Context

End-to-end throttle is shipped: `comm_link` receives `PilotCommand` over IEEE 802.15.4, publishes it to a `Watch`, and `motor_controller` subscribes and drives the PWM. There is currently **no failsafe** — if the radio link drops, `motor_controller` simply stops seeing updates and the motor holds its last commanded value indefinitely. A real bench bug already exposed an adjacent failure mode (a corrupt packet decoded into a garbage `Throttle`, fixed at the wire boundary by ADR 0016). Loss-of-link is the next obvious safety hole.

A separate concern: a `supervisor` task already exists in `firmware-drone`, but its current body is a timer-driven demo (`Booting -> Idle -> Fault` on 5-second sleeps, names from the placeholder skeleton) wired to `status_led` purely so the LED has something to react to. The skeleton is right — `enum SystemState`, `Watch<SystemState>`, `subscribe()` — but the body is a placeholder.

The state machine itself is small (3 states, simple transitions) and pure (state + event -> next state + output). It is exactly the kind of logic ADR 0007 mandates living in a `*-core` crate so it can be host-tested. No such crate exists yet — `firmware-types` covers shared wire types but not behaviour. The supervisor is the first piece of real firmware logic that warrants its own `*-core` companion.

This ADR records (a) the failsafe design, (b) the architectural shape that delivers it, and (c) the creation of `firmware-drone-core` as a consequence.

## Decision

### 1. The supervisor owns the safe-command boundary

The existing `supervisor` task is repurposed from "system status indicator driver" to **the authority on what motors are allowed to do right now**. Concretely:

- `supervisor` subscribes to the raw `PilotCommand` `Watch` (currently consumed by `motor_controller`).
- `supervisor` publishes a **second, separate** `Watch<Throttle>` — the *safe* throttle.
- `motor_controller` switches its subscription: it stops reading `PilotCommand` directly and reads the safe-throttle Watch instead. It becomes a dumb actuator.

This makes the data flow:

```
comm_link  -->  pilot_command::Watch  -->  supervisor  -->  safe_throttle::Watch  -->  motor_controller
                                              |
                                              +-- also publishes SystemState (existing Watch, used by status_led)
```

Nothing downstream of `supervisor` can produce motor output that bypasses failsafe. This is the structural property that the failsafe layer exists to provide — encoded in the wiring, not in a runtime check.

### 2. Four states, named for what they mean

The current `SystemState::{Booting, Idle, Fault}` is replaced by:

```rust
pub enum SystemState {
    Initialising, // task started, never seen a PilotCommand packet
    Armed,        // packets flowing, motors follow pilot
    Degraded,     // packet flow lost, ramping throttle to zero
    Fault,        // unrecoverable error; motors held at zero
}
```

- **`Initialising`** replaces the old `Booting` — same semantics (no packet ever received). Renamed because by the time the supervisor task is running, the boot sequence proper is already complete; what's actually happening is the system is initialising its link to the pilot.
- **`Idle` is dropped** — ambiguous. Real meaning ("alive but not commanding motors") is a future state, not what the demo used the name for.
- **`Armed`** is the precise term for "receiving commands, motors live". Leaves room for a future `Disarmed` (explicit safety toggle, not in scope here).
- **`Degraded`** is the failsafe state: link lost, ramping output to zero, recoverable when packets resume *and* commanded throttle is zero.
- **`Fault`** is the terminal state: an unrecoverable error has occurred (e.g. future IMU init failure, persistent sensor fault). Motors held at zero. No automatic recovery — only a power cycle clears it. There are no fault *sources* implemented yet, so in practice the state is unreachable on day one; it is included now so the public type and `status_led`'s match arms don't churn the moment the first fault source lands.

`status_led`'s match arms are updated to cover the new variants; its blink-pattern logic is unchanged in spirit.

### 3. Tick-driven event loop

The supervisor task is driven by **two event sources, selected concurrently**:

```rust
loop {
    let event = match select(pilot_command_rx.changed(), ticker.next()).await {
        Either::First(cmd) => Event::Command(cmd),
        Either::Second(_)  => Event::Tick,
    };
    let (next_state, output) = supervisor_core::step(state, event);
    if next_state != state { status_tx.send(next_state); }
    safe_throttle_tx.send(output);
    state = next_state;
}
```

Two reasons:

- A purely event-driven supervisor cannot ramp down smoothly — between packet arrivals nothing wakes it.
- The state machine itself stays trivial: it counts ticks since the last command. It does not call `Instant::now()`, it does not know what a `Duration` is. Time is whatever the task says it is.

`Ticker::every(TICK_PERIOD)` provides the heartbeat. Tick period is a constant in `firmware-drone-core`.

### 4. State machine lives in `firmware-drone-core` (new crate)

This ADR creates **`crates/firmware-drone-core/`** — the first non-`firmware-types` `*-core` crate. It is the host-testable companion to `firmware-drone` per ADR 0007's hard rule.

Initial contents:

```
firmware-drone-core/
  Cargo.toml          # no_std on target, std for tests via cfg_attr (ADR 0015)
  src/
    lib.rs
    supervisor.rs     # SystemState enum, Event enum, step() function, constants
```

`SystemState` moves to `firmware-drone-core` (re-exported from `firmware-drone::tasks::supervisor` for compatibility with `status_led`). The Embassy task and `Watch` statics stay in `firmware-drone` — those are target-only.

The `step` function is a pure `fn(state, event) -> (state, Throttle)`. Tests live inline as `#[cfg(test)] mod tests` per ADR 0015. Coverage targets:

- `Initialising + Command -> Armed` (first packet arms).
- `Armed + Tick * N -> Degraded` after `LINK_LOSS_TICKS` ticks without a command.
- `Degraded` ramps output linearly from `last_known_throttle` to `Throttle::ZERO` over `RAMP_TICKS`.
- `Degraded + Command(throttle == 0) -> Armed` (recovery requires zero throttle).
- `Degraded + Command(throttle > 0) -> stays Degraded` (refuse mid-air re-engage).

### 5. Constants and their initial values

All in `firmware-drone-core::supervisor`:

- `TICK_PERIOD: Duration = Duration::from_millis(10)` — matches the remote's transmit cadence.
- `LINK_LOSS_TICKS: u16 = 10` — 100 ms of silence triggers `Degraded`. Generous enough to absorb a few dropped packets, tight enough to feel responsive.
- `RAMP_TICKS: u16 = 50` — 500 ms linear ramp from last commanded throttle to zero. Matches the "learning-grade default" noted in the prior session handoff.

These are **starting values**, expected to be tuned. They are not in an ADR because they're parameters, not decisions; the decision is "they live as named constants in core, not magic numbers in the task".

## Why this shape

- **Roll-our-own enum, not `statig`/`smlang`.** Three flat states with data-carrying variants are exactly what Rust enums + exhaustive `match` express idiomatically. State-machine crates earn their keep on hierarchical state machines with 10+ states; here they would be ceremony around four `match` arms. Standard embedded-Rust judgement call (consistent with AGENTS.md "prefer the idiomatic choice").
- **Tick-counter, not `Instant`/`Duration` plumbing.** The state machine is reactive: it sees a stream of events, doesn't ask the world what time it is. This keeps `firmware-drone-core` free of any `embassy-time` dependency and makes host tests trivially deterministic — feed the events, assert the output, no clock mocking needed. If we ever need variable-rate ticking, switching to `Event::Elapsed(Duration)` is a local change.
- **Two Watches, not one.** Reusing the existing `pilot_command` Watch and inserting a "safe" flag on it would let `motor_controller` accidentally subscribe to the unsafe one. Having a separate `safe_throttle::Watch` makes the boundary structural — there is no path from `comm_link` to motors that does not go through the supervisor.
- **`firmware-drone-core` as a sibling crate.** Per ADR 0009, `*-core` is realised as a sibling crate, not a module inside the bin crate. This is the first time we need it for behaviour (`firmware-types` doesn't count — it's data). Setting up the crate now establishes the pattern for future behaviour modules (`flight_controller`, attitude estimator).
- **Refuse-recovery-until-zero is in the state machine.** It's a four-line `match` arm and the obvious place. Any other location (e.g. in `motor_controller`) duplicates safety logic across actors — exactly what the supervisor exists to centralise.

## Consequences

### What this commits us to

- A new crate `firmware-drone-core`, listed in `[workspace.default-members]` (host-testable per ADR 0015).
- The supervisor task is the **only** publisher of motor commands. New motor-output sources (e.g. a future `flight_controller`) must publish *into* the supervisor's view of "what the pilot wants", not directly to motors.
- `motor_controller` is dumb — it transports a `Throttle` to PWM, nothing else.
- `SystemState` is a public type from `firmware-drone-core`, not an internal detail of the task. Any future task that wants to react to system state subscribes to the existing `Watch`.
- Failsafe parameters (`LINK_LOSS_TICKS`, `RAMP_TICKS`, `TICK_PERIOD`) live as named constants, tunable in one place.
- Host tests for the failsafe state machine ship with the crate. The test file is a permanent part of the regression surface.

### What this rules out

- Failsafe logic in `motor_controller`, `comm_link`, or any other task. Loss-of-link handling lives in exactly one place.
- A supervisor that makes *no* output and merely flags a status — the supervisor must actively republish a safe command, otherwise the structural-boundary property in §1 doesn't hold.
- `Instant::now()` calls inside `firmware-drone-core`. Time is an event from the task, not a global the core can reach for.
- Pulling in a state-machine crate (`statig`, `smlang`, etc.) until the state space justifies it — i.e. a hierarchical state machine, not a flat 3-state one.

### What stays open

- **Arming UX.** Currently the first received packet arms automatically. A real arming switch (`PilotCommand.armed: bool` or similar) is deferred — adds a state but no new architectural decisions.
- **Telemetry of supervisor state to the remote.** `SystemState` is internally observable on the drone (via `status_led`), but the remote does not see it. Will fold in when `TelemetryState` is fleshed out.
- **`flight_controller` task.** Currently `motor_controller` consumes the supervisor's safe `Throttle` directly; once attitude control lands, a `flight_controller` will sit between supervisor and per-motor mix. That's a future ADR — this one only defines the boundary the flight controller will plug into.
- **Fault state reintroduction.** Already present as a variant; needs a real fault *source* (IMU init failure, battery low, etc.) before the transition into it is wired up.

## References

- ADR 0004 — actor-per-task pattern that the supervisor is an instance of.
- ADR 0007 — `core`/`task` split, which this ADR exercises for the first time on real behaviour.
- ADR 0013 — `Watch` as the primitive for both the status broadcast and the safe-throttle channel.
- ADR 0015 — the `cfg_attr(not(test), no_std)` pattern that `firmware-drone-core` will use.
- Implementation: [`crates/firmware-drone-core/`](../../crates/firmware-drone-core/) (created by this ADR), [`crates/firmware-drone/src/tasks/supervisor.rs`](../../crates/firmware-drone/src/tasks/supervisor.rs) (rewritten body).
