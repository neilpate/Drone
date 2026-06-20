# ADR 0020 — Telemetry aggregator: single-publisher fan-in for a multi-source struct

- **Status:** Accepted
- **Date:** 2026-06-20
- **Related:** [ADR 0004](0004-concurrency-embassy-channels.md) (actor-per-task), [ADR 0013](0013-async-communication-primitives.md) (Watch for state), [ADR 0017](0017-supervisor-failsafe-state-machine.md) (single-publisher boundary for motor commands)

## Context

`TelemetryState` began life as a scaffold with one field (temperature) plus a `sequence_number`, and it was constructed inside the `temperature` task. That worked only because the task invented a sequence number it had no business owning — a smell that was tolerable at one field.

As telemetry was fleshed out (adding `drone_state` and an echoed `pilot_command`), the smell sharpened into a structural problem: **no single task owns the whole `TelemetryState`.** Each task knows one slice — `temperature` knows the temperature, the supervisor knows the drone state, `remote_link` knows the last pilot command. The frame-level fields (`sequence_number`, and a future timestamp) belong to none of them.

The naive fix — let every producer read-modify-write a shared `Watch<TelemetryState>` — is exactly the multi-writer anti-pattern that [ADR 0017](0017-supervisor-failsafe-state-machine.md) rejects for motor commands. Multiple actors mutating one published struct race each other and scatter ownership. We need the opposite: a defined owner.

This ADR records the pattern for assembling a multi-source published struct from independent producers.

## Decision

### 1. A dedicated aggregator task is the sole publisher

A `telemetry_aggregator` task is the **only** caller of `telemetry::set`. No producer task touches `TelemetryState`. This is the fan-in counterpart to ADR 0017's fan-out rule: where the supervisor is the single authority that *produces* motor commands, the aggregator is the single authority that *assembles* telemetry.

### 2. Each producer publishes only its own slice

Every contributing task owns and publishes one typed signal; none of them knows `TelemetryState` exists:

```
temperature  task --> Watch<Temperature>    --\
supervisor   task --> Watch<DroneState>      ---> telemetry_aggregator --> Watch<TelemetryState> --> remote_link
remote_link  task --> Watch<PilotCommand>   --/     (sole telemetry::set,
                                                      owns sequence_number,
                                                      fixed cadence)
```

The `pilot_command` Watch already existed (published by `remote_link`, consumed by the supervisor per ADR 0017), so the throttle echo is free — the aggregator simply subscribes to it. The `temperature` task was slimmed from "build the whole struct" to "publish a `Temperature`".

### 3. The aggregator owns frame-level fields and cadence

`sequence_number` is incremented once per assembled frame by the aggregator. This dissolves the original smell: the `temperature` task no longer fabricates a sequence count it cannot meaningfully own.

### 4. Tick-driven sampling, not change-driven select

The aggregator is driven by a fixed `Ticker`. On each tick it reads the *latest* value of each source via `Watch::get()`, assembles the struct, and publishes once:

```rust
let mut ticker = Ticker::every(Duration::from_millis(100));
loop {
    ticker.next().await;
    sequence_count = sequence_count.wrapping_add(1);
    let state = TelemetryState {
        sequence_number: sequence_count,
        temperature:  temperature_receiver.get().await,
        drone_state:  status_receiver.get().await,
        pilot_command: pilot_command_receiver.get().await,
    };
    telemetry::set(state);
}
```

The alternative — `select` across every source's `.changed()` future — was rejected (see below).

## Why this shape

- **Single-publisher mirrors ADR 0017.** One owner per published struct is already the project's rule for motor commands. Applying it to telemetry keeps the codebase consistent: a reader who understands the supervisor understands the aggregator.
- **Tick-driven fits telemetry's semantics.** Telemetry is inherently a periodic snapshot — you want the latest value of each field *at send time*, not a frame emitted on every intermediate change. Sampling delivers exactly that.
- **Tick-driven scales without changing the loop shape.** Adding a field is one more `.get()` line. A change-driven `select` grows an arm per source and ends up at `select_array` or nesting once there are more than a handful of sources (the IMU, battery, etc. are coming).
- **Tick-driven decouples cadence from producer rates.** Temperature updates at 2 Hz, the IMU will update at hundreds of Hz; the telemetry frame should not fire on every IMU tick. A fixed 100 ms (10 Hz) cadence is independent of how fast any source churns.
- **`Watch`, not `Signal`, for the sources.** Sampling needs a non-consuming "give me the latest" — that is `Watch::get()`. `Signal::wait()` consumes, which suits a change-driven consumer but not a sampler. Per ADR 0013's state-vs-events split, retained-latest state is `Watch` territory.
- **100 ms cadence chosen for the echo.** At the original 500 ms the echoed `pilot_command` quantised to 2 Hz and lagged visibly when the slider moved quickly. 100 ms makes the round-trip echo feel live while remaining far below any meaningful CPU cost. Temperature being resampled faster than it changes is harmless — it republishes the same value.

## Consequences

### What this commits us to

- `telemetry_aggregator` is the **only** publisher of `TelemetryState`. New telemetry sources publish their own slice to their own `Watch` and add one `.get()` line to the aggregator; they never construct `TelemetryState`.
- Producer tasks stay single-responsibility: each publishes one typed signal.
- The telemetry cadence is one named constant in one place.

### What this rules out

- Producers constructing or mutating `TelemetryState` directly.
- A shared `Watch<TelemetryState>` written by multiple tasks (the read-modify-write multi-writer anti-pattern).

### What stays open

- **First-frame blocking.** `get().await` waits for each source's first value, so no frame is assembled until every source — `pilot_command` in particular — has published at least once. In practice this is fine: `remote_link` only sends telemetry in response to a received command, so a command has always arrived by then. If telemetry is ever wanted before the first command (bench use with no remote), seed the source Watches with defaults.
- **Per-field timestamps / staleness.** A field can be up to one tick stale, and there is no per-field "last updated" marker. Irrelevant for current telemetry; revisit if a consumer needs to detect a stalled source.
- **Single-publisher is convention, not compile-time-enforced.** `telemetry::set` is a free function over a global `static Watch`, so the "only the aggregator publishes" rule is upheld by discipline and review, not by the type system. A capability-based alternative — construct the `Watch` once in `main` and move the sole `Sender` into the aggregator, exporting no global `set` — would make it enforced (and would make tasks unit-testable in isolation, per ADR 0007). This was considered and **deliberately deferred**: at the current scale (five single-publisher signals, one author, all wired in one `main`) the enforcement is not worth converting the whole signals layer from ambient globals to explicit wiring, and a stray publisher is caught cheaply by review. The same looseness applies uniformly to every signal (`motor_command`, `status`, `temperature`, `pilot_command`), so any change should be made layer-wide, not special-cased to telemetry. Tripwires to revisit: a second person starts touching the firmware, the task count climbs into the teens, or a wrong-publisher bug actually occurs.

### What this closes

- ADR 0017 left open "telemetry of supervisor state to the remote". `drone_state` is now a `TelemetryState` field assembled by the aggregator and flows to the groundstation over the link from ADR 0018. That open item is resolved.

## References

- ADR 0004 — actor-per-task pattern; the aggregator is an instance of it.
- ADR 0013 — `Watch` for retained state; the primitive for every source channel and the telemetry output.
- ADR 0017 — single-publisher boundary for motor commands; this ADR is its fan-in dual, and closes its telemetry open item.
- Implementation: [`crates/firmware-drone/src/tasks/telemetry_aggregator.rs`](../../crates/firmware-drone/src/tasks/telemetry_aggregator.rs), the per-source signals under [`crates/firmware-drone/src/signals/`](../../crates/firmware-drone/src/signals/), and [`crates/firmware-types/src/telemetry_state.rs`](../../crates/firmware-types/src/telemetry_state.rs).
