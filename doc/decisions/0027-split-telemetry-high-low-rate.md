# ADR 0027 — Split telemetry into high-rate and low-rate frames

- **Status:** Proposed
- **Date:** 2026-08-22
- **Related:** [ADR 0014](0014-radio-protocol-ieee802154.md) (802.15.4 PHY limit), [ADR 0018](0018-pc-link-uart-postcard-cobs.md) (postcard link + RTT echo), [ADR 0020](0020-telemetry-aggregator-single-publisher.md) (aggregator single-publisher), [ADR 0024](0024-control-law-angle-mode-pd.md) (control parameters), [ADR 0016](0016-newtype-per-physical-quantity.md) (newtypes)

## Context

The single `Telemetry` frame has reached the radio's hard size limit, and a new data source needs to be carried.

- **The frame is full.** Measured with `postcard`'s `MaxSize`, `Telemetry` is **123 bytes** worst-case; with COBS/framing margin the frame constant is **125 bytes**. An IEEE 802.15.4 PSDU is 127 bytes including the 2-byte FCS (see [ADR 0014](0014-radio-protocol-ieee802154.md)), leaving roughly **125 usable**. The flight frame is already sitting on that ceiling.
- **ESC telemetry does not fit.** The new `DShotTelemetry` (per-ESC temperature, pack voltage/current/consumed, four motor RPMs) is **31 bytes**. Appended, the frame becomes **154 bytes — 29 over**. Even a single-motor slice (~21 bytes) overflows. It cannot be appended.
- **The frame is bloated.** It carries two full `Sensors` structs (`sensors` and `sensors_processed`), duplicating the accelerometer even though only the gyro is filtered.
- **Gains are not ground-truth.** Control parameters flow one way, GS to drone, as a `Command`. The ground station logs its *own* copy of what it believes it sent. If a `ControlSystemParameterUpdate` is dropped on the lossy link, or the drone is running its boot `::default()` gains, the log records values the drone never actually flew.
- **The data has two natural rates.** Attitude, gyro, controller demand and motor outputs change every control cycle. CPU load, control mode, chip and ESC temperatures, pack voltage/current, motor RPM and the active gains change slowly.

## Decision

### 1. The drone-to-remote reply becomes a tagged frame

```rust
pub enum TelemetryFrame {
    HighRate(HighRateTelemetry),
    LowRate(LowRateTelemetry),
}
```

One discriminant byte. Each variant is serialized on its own and stays well under the ~125-byte PHY budget.

### 2. High-rate frame — sent every round-trip (~100 Hz)

Fast-changing, flight-control-critical:

- `sequence_number`
- raw IMU: acceleration x/y/z, angular rate x/y/z
- filtered gyro: x/y/z
- `attitude` (roll, pitch)
- `pilot_command` — also the RTT echo target (see [ADR 0018](0018-pc-link-uart-postcard-cobs.md)), so it must stay high-rate
- `controller_demand`
- `motor_command`
- `drone_state` — failsafe transitions must surface promptly

The accelerometer is sent once, not duplicated. Estimated ~100 bytes, leaving ~25 bytes of margin.

### 3. Low-rate frame — sent every Nth round-trip (~10 Hz)

Slow-changing housekeeping and ground-truth:

- `sequence_number` (correlation)
- `cpu_load`
- `control_mode`
- chip `temperature`
- ESC telemetry (`DShotTelemetry`, all four motors)
- the drone's active `ControlSystemParameters`

Estimated ~93 bytes with the gains included, under the budget.

### 4. Active gains are echoed for ground-truth logging

The low-rate frame carries the drone's *currently active* `ControlSystemParameters` (per [ADR 0024](0024-control-law-angle-mode-pd.md)). The ground station logs the echoed values, not its own `self.params`, so the flight log records what the drone actually ran. This closes the dropped-update and boot-default mismatch.

### 5. The aggregator schedules the two frames

Per [ADR 0020](0020-telemetry-aggregator-single-publisher.md) the aggregator remains the single publisher. It emits `HighRate` every cycle and `LowRate` every Nth cycle (N chosen for ~10 Hz). The ESC round-robin means each motor's slice refreshes roughly every fourth low-rate frame.

### 6. The ground station carries last-known low-rate state and merges

The GS retains the most recent low-rate values and merges them into every logged row, so the TSV and analyzer still see a complete row per sample; the low-rate columns simply update at their slower cadence. The TSV header is unchanged.

### 7. Growth path: interleave additional low-rate frames

If the low-rate frame later outgrows the budget (more ESC fields, extra housekeeping), add further low-rate variants and round-robin among them rather than inflating a single frame. Recorded here so the escape hatch is explicit rather than rediscovered.

## Why this shape

- **It matches the data.** Two rates, two frames; each field rides the cadence that suits how fast it changes and how fast it is produced (ESC polling is inherently slow).
- **Flight telemetry is untouched.** The 100 Hz control-relevant stream keeps its rate and gains margin; housekeeping and ESC data move off it.
- **It de-bloats.** Dropping the duplicated accelerometer recovers bytes the old frame wasted.
- **Logs become ground-truth.** Echoing the active gains removes the GS/drone divergence that made post-flight tuning analysis unreliable.
- **RTT is preserved.** `pilot_command` stays high-rate, so the echo-match latency measurement of [ADR 0018](0018-pc-link-uart-postcard-cobs.md) is unaffected.
- **Single-publisher is preserved.** The aggregator still owns assembly and sequence numbering ([ADR 0020](0020-telemetry-aggregator-single-publisher.md)); it now also owns frame scheduling.

## Alternatives considered

- **Append ESC telemetry to the single frame** — rejected: 154 > 125, does not fit.
- **Shrink the frame with i16 fixed-point instead of f32** — frees bytes but is lossy, invasive, and ripples through the GS and analyzer parsing. Deferred; it can be layered on later independently of this split.
- **A separate ESC-only frame with no other restructuring** — solves the size problem but misses the ground-truth-gains and de-bloat wins, and still leaves CPU load, mode and temperature needlessly riding the 100 Hz frame.

## Consequences

- **Blast radius:** `firmware-types` (new frame types), `telemetry_aggregator` (build and schedule both), `remote_link` / `drone_link` (send and receive the enum), GS `main.rs` (match, merge, log echoed gains). The TSV header is unchanged.
- Low-rate fields are up to ~100 ms stale in the log; acceptable for housekeeping data.
- The aggregator gains a little scheduling logic and the GS a little match/merge state.
- Per-field cadence can be tuned later without further protocol churn.
