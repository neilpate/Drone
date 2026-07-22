# ADR 0024 — Control law: single-loop PD, angle mode for roll/pitch, rate mode for yaw

- **Status:** Proposed
- **Date:** 2026-07-22
- **Related:** [ADR 0021](0021-coordinate-frames-and-command-semantics.md) (frames, sign conventions, angle-mode + raw-deflection command model), [ADR 0022](0022-attitude-estimation-complementary-filter.md) (the roll/pitch estimate this consumes; yaw-as-rate), [ADR 0023](0023-motor-numbering-layout-rotation.md) (the mixer this feeds), [ADR 0017](0017-supervisor-failsafe-state-machine.md) (the supervisor this slots into; sole publisher of motor commands), [ADR 0016](0016-newtype-per-physical-quantity.md) (newtypes), [ADR 0015](0015-host-testing-no-std-crates.md) / [ADR 0007](0007-testing-and-ci-strategy.md) (host-testable core), [ADR 0013](0013-async-communication-primitives.md) (`Watch` for shared state)

## Context

Every piece around the control loop now exists: a roll/pitch attitude estimate ([ADR 0022](0022-attitude-estimation-complementary-filter.md)), a motor mixer ([ADR 0023](0023-motor-numbering-layout-rotation.md)), and a failsafe supervisor that is the sole publisher of motor commands ([ADR 0017](0017-supervisor-failsafe-state-machine.md)). What is missing is the law in the middle. Today the supervisor's `Armed` path forwards the raw pilot deflections straight into the mixer — open-loop: the sticks command motor differential directly, with no feedback from the craft's actual attitude, so it cannot self-level and will not hold an angle.

This ADR fills the `06-control.md` slot that [ADR 0022](0022-attitude-estimation-complementary-filter.md) and [ADR 0023](0023-motor-numbering-layout-rotation.md) both defer to: the control law that turns pilot command plus attitude estimate into the mixer's roll/pitch/yaw demand, closing the loop for **angle mode** (self-levelling) flight — the first flyable mode fixed by [ADR 0021](0021-coordinate-frames-and-command-semantics.md).

It does **not** decide the gain values (those are bench/flight-tuned, like the filter's `alpha`), nor the airframe, nor the estimator — only the shape of the controller.

## Decision

### 1. Angle mode for roll/pitch, rate mode for yaw

The pilot stick is interpreted per axis, and the split is forced by the sensor suite ([ADR 0022](0022-attitude-estimation-complementary-filter.md)):

- **Roll and pitch are angle-controlled.** Gravity gives an absolute roll/pitch estimate, so the stick commands a **desired angle**: centre the sticks and the craft returns to level. This is the self-levelling behaviour of angle mode.
- **Yaw is rate-controlled.** There is no observable yaw angle (no magnetometer in use), so the stick commands a **desired yaw rate**, controlled against the gyro's Z axis — exactly as [ADR 0021](0021-coordinate-frames-and-command-semantics.md) §6 and [ADR 0022](0022-attitude-estimation-complementary-filter.md) §3 already committed.

The controller is therefore not uniform: two angle axes and one rate axis.

### 2. A single-loop PD per axis, not a cascade

Each axis is a **single-loop proportional-derivative** controller, not the cascaded angle→rate structure of Betaflight/PX4.

- **P** acts on the axis error (desired minus measured).
- **D** damps rotation using the measured body rate from the gyro (see §5).

The cascade (an outer P on angle producing a rate setpoint for an inner rate PID) is the recognised, more capable structure and the natural future upgrade — the control-loop analogue of the Kalman/Madgwick upgrade named for the estimator. It is deliberately deferred here under the project scope guardrail: a single-loop PD is the simplest thing that self-levels, it is one expression per axis, it is trivially host-testable like the mixer and the filter, and it evolves into the cascade later without discarding anything (the angle-error P becomes the outer loop).

### 3. Setpoint scaling: sticks to angles and rates

The raw normalised deflections (−1..+1, per [ADR 0021](0021-coordinate-frames-and-command-semantics.md)) are scaled into physical setpoints by fixed maxima:

```
roll_setpoint   = roll_stick  * MAX_TILT_ANGLE     # degrees
pitch_setpoint  = pitch_stick * MAX_TILT_ANGLE     # degrees
yaw_rate_setpoint = yaw_stick * MAX_YAW_RATE       # degrees / second
```

`MAX_TILT_ANGLE` (the full-stick bank/pitch angle, e.g. ~30°) and `MAX_YAW_RATE` (full-stick yaw rate, e.g. ~180°/s) are tuning constants (§9), not chosen here. Throttle is not a controlled axis — it passes through untouched (§7).

### 4. The per-axis control laws

With angles from the estimate ([ADR 0022](0022-attitude-estimation-complementary-filter.md)) and rates from the gyro, all in degrees / degrees-per-second and in the FRD sign conventions of [ADR 0021](0021-coordinate-frames-and-command-semantics.md):

```
# roll and pitch: angle mode
roll_out  = Kp_roll  * (roll_setpoint  - est_roll)  - Kd_roll  * gyro_x
pitch_out = Kp_pitch * (pitch_setpoint - est_pitch) - Kd_pitch * gyro_y

# yaw: rate mode
yaw_out   = Kp_yaw   * (yaw_rate_setpoint - gyro_z)
```

`gyro_x`/`gyro_y`/`gyro_z` are the roll/pitch/yaw body rates. Each output is a **normalised authority** term destined for the mixer's roll/pitch/yaw inputs ([ADR 0023](0023-motor-numbering-layout-rotation.md)); the gains carry the unit conversion (degrees of error → normalised authority) and are set at tuning time.

A sign sanity-check against [ADR 0021](0021-coordinate-frames-and-command-semantics.md) (+roll = right-side-down): stick right → positive `roll_setpoint`; level, the error is positive → `roll_out` positive → the mixer raises the left motors → the craft banks right, and if it overshoots, the `−Kd·gyro_x` term opposes the rotation. The same reasoning holds for pitch; yaw drives the gyro Z rate toward the commanded rate.

### 5. Derivative on the gyro, not on the error

The **D** term acts on the **measured body rate** (the gyro), not on the time-derivative of the error. Two reasons, both standard:

- **No derivative kick.** Differentiating the error would spike the output every time the pilot steps the setpoint (a stick flick); differentiating the measurement does not.
- **The gyro is already a clean rate.** It measures angular rate directly, so no noisy numerical differentiation is needed — the rate signal is a first-class sensor output, not a derived quantity.

For roll and pitch the damped rate is the same-axis gyro (`gyro_x`, `gyro_y`); this is what turns the P-only self-leveller from a bouncy oscillator into a settled one.

### 6. Integral deferred, with the anti-windup plan named

The first cut is **PD, no integral**. A P-on-angle / D-on-rate controller self-levels; the steady-state offset an I term removes (trim, small CG or thrust imbalance) is a refinement, not a prerequisite for a first hover, and leaving it out keeps the initial controller stateless and easy to reason about.

When the **I** term is added it needs, and this ADR names as required at that point: **integral clamping** (bound the accumulated term), **conditional integration** (stop accumulating while the output is saturated, to prevent windup), and a **reset on disarm / mode entry** (do not carry stale integral across an arm). Adding I turns the controller from a pure function into a small stateful type, mirroring the estimator's shape.

### 7. Output authority and throttle passthrough

- **Throttle passes through.** The controller produces only roll/pitch/yaw authority; the collective throttle flows from the pilot command straight into the `ControllerDemand`'s throttle field. The mixer ([ADR 0023](0023-motor-numbering-layout-rotation.md)) combines throttle with the three corrections.
- **Per-axis clamping.** Each controller output is clamped to a maximum authority so no single axis can consume all the throttle headroom and starve the others; the mixer additionally clamps the final per-motor values to 0..1. The authority limit is a tuning constant (§9).

The controller's output is exactly a `ControllerDemand` (throttle + roll/pitch/yaw), the same type the mixer already consumes — so nothing downstream of the mixer changes.

### 8. Where the logic lives, and how it meets the supervisor

- The control law is **pure logic in `firmware-drone-core`** — setpoint scaling, the per-axis PD, and output clamping as a host-tested unit ([ADR 0015](0015-host-testing-no-std-crates.md), [ADR 0007](0007-testing-and-ci-strategy.md)), like the mixer and the filter. For the PD cut it is a pure function of (pilot command, attitude, gyro rates) → `ControllerDemand`; it becomes a small stateful type when the integral lands (§6).
- A thin **controller stage** drives it from the attitude `Watch` ([ADR 0022](0022-attitude-estimation-complementary-filter.md)), the IMU `Watch` (for the gyro rates), and the pilot command, and is the **single publisher** of a `ControllerDemand` `Watch` — the same single-publisher pattern as the estimator and aggregator ([ADR 0017](0017-supervisor-failsafe-state-machine.md), [ADR 0020](0020-telemetry-aggregator-single-publisher.md)).
- The **supervisor** ([ADR 0017](0017-supervisor-failsafe-state-machine.md)) remains the sole publisher of motor commands and the sole authority on the failsafe states. In `Armed` it takes the controller's `ControllerDemand` and mixes it (replacing today's raw-pilot passthrough); in `Degraded` / `Fault` it **ignores** the controller entirely and emits its own safe demand (attitude neutralised, throttle ramped), exactly as it does now. Loss-of-link detection stays with the supervisor. This keeps the failsafe override trivially able to disregard a misbehaving control loop.

### 9. Tunable parameters named but not valued

Fixed at bench/flight-tuning time, not in this ADR:

- **`Kp` / `Kd` per axis** — the loop gains (six numbers: P and D for roll, pitch, yaw; yaw's D may stay zero in the first cut).
- **`MAX_TILT_ANGLE`, `MAX_YAW_RATE`** — the full-stick setpoint maxima.
- **Per-axis output authority limit** — the clamp of §7.
- **`Ki` and its anti-windup bounds** — only once the integral of §6 is added.

## Why this shape

- **It matches the sensors and the mode.** Angle mode on the two observable axes and rate mode on the unobservable one is the only honest reading of a gyro-plus-accelerometer suite, and it is exactly what [ADR 0021](0021-coordinate-frames-and-command-semantics.md)/[ADR 0022](0022-attitude-estimation-complementary-filter.md) already fixed.
- **Simplest law that flies.** Per the scope guardrail, a single-loop PD is chosen over a cascade because it self-levels, it is one line per axis, and it is host-testable — the cascade is deferred, not designed out.
- **Derivative on measurement** is the standard defence against derivative kick, and the gyro hands us the rate for free.
- **Pure core, single publisher, supervisor gate.** The law is pure and unit-tested; a stage owns the `ControllerDemand`; the supervisor keeps its role as the one place that turns intent into motor commands and can always override the loop. No new architectural pattern — the same one used three times already.

## Consequences

### What this commits us to

- A `ControllerDemand` produced by pure `firmware-drone-core` logic and published on a single-owner `Watch`; the supervisor consumes it in `Armed` and mixes it.
- The supervisor's `Armed` path stops forwarding raw pilot deflections and instead mixes the controller's output; the `Degraded` / `Fault` paths are unchanged and continue to override.
- Roll/pitch tracking a **commanded angle** and self-levelling on stick centre; yaw tracking a **commanded rate**.
- A bench bring-up step: gains tuned props-off then in a tethered hover, since a wrong sign or a grossly wrong gain is the flip-on-arm failure mode — the same caution as the mixer sign table ([ADR 0023](0023-motor-numbering-layout-rotation.md) §4).

### What this rules out (for now)

- **A cascaded angle→rate controller** — not the first cut; a future superseding ADR if measured performance demands it.
- **An integral term** in the first cut (§6) — added on measured need, with the named anti-windup machinery.
- **Acro / rate mode as a pilot-selectable mode**, horizon/other blended modes, and any heading (yaw-angle) hold — all out of scope until there is a mode channel and, for heading, a usable magnetometer.

### What stays open

- **The gain values and setpoint maxima** (§9) — tuning, not design.
- **The arm / mode channel** — how the pilot arms and (later) selects modes; still deferred from [ADR 0021](0021-coordinate-frames-and-command-semantics.md).
- **Loop rate and jitter budget for the controller stage** — expected to run at the estimate/IMU cadence; pinned at implementation.
- **The exact liveness wiring** between the controller stage and the supervisor's loss-of-link timer — an implementation detail under [ADR 0017](0017-supervisor-failsafe-state-machine.md).

## References

- ADR 0021 — frames, sign conventions, angle mode, raw-deflection commands; the conventions this law obeys.
- ADR 0022 — the roll/pitch estimate this consumes; yaw stays a rate.
- ADR 0023 — the mixer this feeds; `ControllerDemand` in, per-motor commands out.
- ADR 0017 — the supervisor: sole publisher of motor commands, failsafe override.
- ADR 0016 / 0015 / 0007 / 0013 — newtypes, host-testable core, testing strategy, `Watch`.
- `doc/06-control.md` — the longer-form control write-up this ADR anchors, to be filled as the loop is built and tuned.
- External: PID control, derivative-on-measurement, integral anti-windup; the cascaded angle/rate controller as the recognised upgrade path.
