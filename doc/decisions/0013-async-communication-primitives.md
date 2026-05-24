# ADR 0013 — Async inter-task communication: when to use Channel, Watch, Signal, PubSubChannel

- **Status:** Accepted
- **Date:** 2026-05-24
- **Related:** [ADR 0004](0004-concurrency-embassy-channels.md), [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md)

## Context

[ADR 0004](0004-concurrency-embassy-channels.md) established the concurrency
model: each subsystem is an `async` task with a typed `Channel` inbox.
That pattern fits **commands** — discrete work items where each one must be
processed in order, exactly once.

It does not cover every communication shape we'll need. The first concrete
case it doesn't fit is `SystemStatus`: a single piece of system-wide state
(`Booting` / `Idle` / `Busy` / `Fault` ...) that several subsystems will want
to observe — the LED renderer now, and later a buzzer, a telemetry packet
field, a refuse-to-arm gate. Many observers, latest value wins, intermediate
values can be silently dropped. A queue is the wrong shape, and Embassy
has purpose-built primitives for the cases ADR 0004 didn't cover.

`embassy-sync` ships four primitives in this space, and they cover the
problem space neatly along two orthogonal axes:

- **State vs events.** State = "what is the current value?" — if you missed
  the second-to-last update, you don't care. Events = "what happened?" —
  every value is meaningful and missing one is a fault.
- **One observer vs many observers.**

|                          | One observer | Many observers   |
|--------------------------|--------------|------------------|
| **Latest-value state**   | `Signal`     | `Watch`          |
| **Every value matters**  | `Channel`    | `PubSubChannel`  |

This ADR establishes the project-wide convention for which primitive belongs
in which cell, and the per-primitive conventions for using them.

## Decision

### The 2×2 rule

Pick a primitive by answering two questions:

1. **Is this state or an event?** If a slow consumer should skip to the
   latest, it's state. If a slow consumer must catch up through every value,
   it's an event.
2. **One consumer or potentially many?** "Potentially many" counts — promote
   to a multi-observer primitive the moment a second consumer is plausible,
   not when it arrives.

The cell determines the primitive. No other primitive should be reached for
to "be safe" — each cell's primitive is the right weight for that cell.

### `Channel` — single-consumer command queue (per ADR 0004)

The actor-pattern inbox. One task owns a subsystem and drains its `Channel`
in FIFO order; producers fan in. ADR 0004 covers this — no change. Use for:

- Motor commands.
- Outbound comms frames.
- Anything where a queue with exactly-once delivery in order is the
  natural fit.

### `Watch` — multi-observer shared state

The publish/subscribe primitive for current-state. Use for:

- `SystemStatus` (the first concrete user).
- Future: arming state, link quality, battery voltage, current setpoint —
  anything where "the current value" is the answer and more than one
  subsystem wants to react.

#### Conventions

- Type lives in `firmware-drone-core` (per [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md));
  the `Watch` static and producer helpers live alongside it.
- Mutex parameter: `CriticalSectionRawMutex` unless there is a measured
  reason to use a cheaper one. Allows setting from any context including
  ISRs without footguns.
- Capacity (`N`): start generous — `MAX_SUBSCRIBERS = 8` for project-wide
  state, with a comment explaining the trade-off (~8 bytes RAM per slot;
  bump if `receiver()` returns `None` at boot).
- Consumers call `STATIC.receiver().unwrap()` once at task start. If `N`
  is too small the `unwrap` panics on first boot — desired failure mode,
  caught immediately, fixed by bumping the constant.
- Producers expose a thin `set()` helper over `STATIC.sender().send(...)`.
  The `Watch` static itself is `pub` so consumers can call `.receiver()`
  directly — no wrapper that hides what consumers actually need.

### `Signal` — single-observer state

The degenerate single-consumer form of `Watch`. Use for:

- ISR → owning driver task handoff (e.g. IMU `DRDY` line: ISR signals,
  driver task awaits, reads sample over SPI). Often hidden inside
  `embassy-nrf`'s async APIs; rarely written directly in our code.
- One-shot initialization gates ("wait for IMU self-test to finish") when
  exactly one task gates on it. If gating is system-wide, model it as a
  `SystemStatus` transition instead.

Promote to `Watch` the moment a second observer is plausible.

### `PubSubChannel` — multi-consumer event stream

Buffered broadcast where every event matters and multiple consumers each
need to see all of them. Slow subscribers receive `WaitResult::Lagged(n)`
instead of silently dropping events — missed events are *visible*, which is
what distinguishes it from `Watch`.

Not currently in use. Plausible future uses:

- IMU sample bus consumed by the control loop, on-board logger, and
  telemetry encoder simultaneously.
- Log/event bus with multiple sinks (RTT, telemetry, on-board flash).

When the first concrete user appears, that issue gets to set the
conventions for `CAP` / `SUBS` / `PUBS` sizing and the back-pressure
choice (`publish().await` vs `publish_immediate()`).

## Consequences

- Subsystems that need to react to shared state subscribe to it; producers
  don't need to know who's listening. Adding a consumer is a local change.
- Subscriber and publisher counts are bounded at compile time. Adding one
  past the cap fails loudly at first boot — intentional.
- Latest-value semantics for `Watch`/`Signal` mean a fast producer can
  silently overwrite values a consumer hasn't observed yet. Correct for
  state, wrong for events; the 2×2 rule prevents the mismatch.
- ADR 0004 is unchanged: `Channel` remains the right tool for commands.
  This ADR adds three primitives to the toolbox alongside it.
- A task can hold any combination — typically a command `Channel` (its
  inbox) plus one or more `Watch` receivers (state it observes) plus
  whatever `PubSubChannel` subscribers it needs for event streams. The
  primitives are orthogonal, not competing.

## Anti-temptations

- Don't reach for `PubSubChannel` "to be safe" on state. It's heavier, and
  a fast producer either blocks on or laps a slow consumer. State is
  `Watch`.
- Don't use `Signal` for "two-way notification between tasks" if either
  side could grow a second listener. Use `Watch` from day one if
  multi-observer is plausible.
- Don't use `Channel` for state. A slow consumer will see stale values
  long after they ceased to matter, or the channel fills and blocks the
  producer.
- Don't roll your own pub/sub on top of `Mutex<RefCell<...>>` + manual
  notification. `Watch` and `PubSubChannel` already bundle the wake
  mechanism with the storage — bypassing them defeats the async model.
