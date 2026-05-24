# ADR 0013 — Pub/sub of shared state via `embassy_sync::Watch`

- **Status:** Accepted
- **Date:** 2026-05-24
- **Related:** [ADR 0004](0004-concurrency-embassy-channels.md), [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md)

## Context

[ADR 0004](0004-concurrency-embassy-channels.md) established the concurrency
model: each subsystem is an `async` task with a typed `Channel` inbox.
That pattern fits **commands** — discrete work items, FIFO, exactly-once
delivery to one worker (motor commands, comms frames, etc.).

It does not fit **shared state observed by multiple tasks**. Concrete first
example: `SystemStatus` (`Booting` / `Idle` / `Busy` / `Fault` ...). The LED
task wants to render it. A future buzzer task will want to sonify it. A
future telemetry packet will want to include it. A future refuse-to-arm gate
will want to read it. Many observers, latest value wins, old values silently
dropped — a queue is the wrong shape, and a single-consumer `Signal` is
the wrong shape too.

`embassy-sync` ships four primitives in this space:

| Primitive | Observers | Each observer sees | Use for |
|---|---|---|---|
| `Signal` | 1 effective | Latest value | Single observer of state |
| `Watch` | N (compile-time bound) | Latest value | Many observers of state |
| `Channel` | N, exactly one wins per send | Each value once, FIFO | Command queue, work items |
| `PubSubChannel` | N (compile-time bound) | Every event, with backlog | Event broadcast, telemetry stream |

`Watch` is purpose-built for the "shared current state" case.

## Decision

### 1. `Watch` is the publish/subscribe primitive for shared current-state

Any value that conceptually answers "what is the current X?" and is observed
by more than one task is published through an `embassy_sync::Watch`.
Examples that will appear over time: `SystemStatus`, link quality,
battery state, arming state, current setpoint.

### 2. Single-observer state still uses `Signal`

If there is genuinely only one consumer and there will never be a second,
prefer `Signal` — same mental model, slightly cheaper. Promote to `Watch`
the moment a second observer appears.

### 3. Command queues continue to use `Channel` (ADR 0004 unchanged)

Commands are events with at-most-once delivery and a queue: `Channel`.
This ADR adds a primitive; it does not displace ADR 0004.

### 4. Event broadcast (every-event-counts) uses `PubSubChannel`

For telemetry streams or event logs where each event must be visible to
each subscriber even if one falls behind, `PubSubChannel` is the tool.
Not currently in use; documented here for completeness.

### 5. Shared-state types live in `firmware-drone-core`

Per [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md), `firmware-drone-core`
is the no-HAL, hardware-free home for shared logic and types. The
`SystemStatus` enum, the `Watch` static, and the producer helpers live
there. Board-specific or task-specific code (e.g. the
`SystemStatus → blink pattern` mapping inside `status_led`) stays in the
consumer.

### 6. Capacity is generous, runtime failure is loud

`Watch<_, _, N>` is bounded at compile time. Pick `N` with comfortable
headroom — start at `8` for project-wide state like `SystemStatus`
(~8 bytes RAM per slot, cheap). Each consumer calls
`STATUS.receiver().unwrap()` at task start; if `N` is too small, the
`unwrap` panics on first boot. This is the desired failure mode: caught
immediately, fixed by bumping a constant and reflashing.

A constant with an explanatory comment documents the trade-off at the
declaration site:

```rust
/// Maximum simultaneous receivers. Bump if `STATUS.receiver()` ever
/// returns `None` at boot. Cost: ~8 bytes of RAM per slot.
const MAX_SUBSCRIBERS: usize = 8;
```

### 7. Producers expose a thin `set()` helper; consumers take a receiver directly

Producer side:

```rust
pub fn set(s: SystemStatus) {
    STATUS.sender().send(s);
}
```

Consumer side acquires its own `Receiver` once at task start and races
`rx.changed()` against ongoing work via `embassy_futures::select`.

The `Watch` static itself is `pub` so consumers can call `.receiver()`
directly — no wrapper layer that hides what consumers genuinely need.

## Consequences

- Subsystems that need to react to `SystemStatus` (or any future shared state)
  do so by subscribing, not by being wired up by `main`. New consumers add
  themselves; producers don't need to know who's listening.
- Subscriber count is bounded at compile time. Adding a consumer past the
  cap fails loudly at first boot (intentional — see decision 6).
- Latest-value semantics: a fast producer can silently overwrite values
  the consumer hasn't observed yet. Correct for state, wrong for events —
  use `PubSubChannel` if every event matters.
- The pattern composes with ADR 0004: a task can have both a command
  `Channel` (its inbox) and one or more `Watch` receivers (state it
  observes). They're orthogonal primitives, not competing ones.

## Alternatives considered

- **`Signal` for everything.** Fails the multi-consumer requirement —
  `Signal` semantically supports only one waiter. Reserved for the
  genuinely-single-consumer case.
- **`Channel` for shared state.** Wrong semantics: queue, not latest-value.
  A slow consumer would see stale values long after they ceased to matter,
  or the channel would fill and block the producer. Either is wrong for
  "what is the current status".
- **`PubSubChannel` for shared state.** Works, but heavier than needed:
  it maintains a backlog so slow subscribers don't miss events. We don't
  want that for state — a slow LED task should see the latest, not catch
  up through stale intermediates.
- **Hand-rolled atomics + custom wakers.** Reinventing `Watch` poorly.
  Embassy's primitive is well-tested and the right shape; no reason to
  build our own.
- **Global `Mutex<RefCell<SystemStatus>>` with manual notification.**
  Works but defeats the async model: consumers either poll (wasteful)
  or we need a separate wake mechanism. `Watch` bundles both.

## Reference: choosing among the four `embassy-sync` primitives

The four primitives collapse into a 2×2 along two orthogonal axes:

- **State vs events.** State = "what is the current value?" — if you missed
  the second-to-last update, you don't care. Events = "what happened?" —
  every value is meaningful and missing one is a fault.
- **One observer vs many observers.**

|                          | One observer | Many observers   |
|--------------------------|--------------|------------------|
| **Latest-value state**   | `Signal`     | `Watch`          |
| **Every value matters**  | `Channel`    | `PubSubChannel`  |

Decision rule: ask "is this state or an event?" first, then "one consumer
or many?". Each cell maps to exactly one primitive.

### Concrete example per cell

**`Signal` — single-observer state.** Often hidden behind driver APIs
rather than written directly. The canonical case is ISR → one owning task:
e.g. an IMU `DRDY` GPIO interrupt fires, the ISR calls `signal(())`,
the IMU driver task awaits `wait()` and reads the new sample over SPI.
Only the driver cares about `DRDY`; latest-value semantics are fine
(missing a "ready" pulse doesn't matter — the next SPI read gets whatever
the sensor currently has). Embassy uses this pattern internally for most
async peripherals; in this project we expect few direct uses.

**`Watch` — multi-observer state.** `SystemStatus` (this ADR). Future
likely uses: arming state, link quality, battery voltage, current setpoint.
Anything where "the current value" is the answer and more than one
subsystem wants to react.

**`Channel` — single-consumer event queue.** The command pattern from
[ADR 0004](0004-concurrency-embassy-channels.md). One task owns a subsystem
(motors, comms, etc.) and drains its inbox in order. Producers fan in;
each command must be processed exactly once, in FIFO order.

**`PubSubChannel` — multi-consumer event stream.** Streams where every
event matters and multiple consumers each need to see all of them.
Plausible future use: an IMU sample bus consumed by the control loop,
the on-board logger, and a telemetry encoder simultaneously. Slow
subscribers receive `WaitResult::Lagged(n)` instead of silently dropping
data — missed events are *visible*, which is the property that
distinguishes it from `Watch`.

### Anti-temptations

- Don't reach for `PubSubChannel` "to be safe" on state — it's heavier
  and a fast producer either blocks on or laps a slow consumer. State is
  `Watch`.
- Don't use `Signal` for "two-way notification between tasks" if either
  side could grow a second listener. Promote to `Watch` from day one if
  multi-observer is plausible.
- Don't use `Channel` for state. A slow consumer will see stale values
  long after they've ceased to matter, or the channel fills and blocks
  the producer.
