# ADR 0022 — Attitude estimation: complementary filter for roll and pitch

- **Status:** Proposed
- **Date:** 2026-06-27
- **Related:** [ADR 0003](0003-imu-icm42688-spi.md) (IMU: gyro + accel, no magnetometer), [ADR 0013](0013-async-communication-primitives.md) (`Watch` for shared state), [ADR 0015](0015-host-testing-no-std-crates.md) (host-testable `no_std` crates), [ADR 0016](0016-newtype-per-physical-quantity.md) (newtype per physical quantity), [ADR 0017](0017-supervisor-failsafe-state-machine.md) (pure logic in `firmware-drone-core`, single-publisher pattern), [ADR 0020](0020-telemetry-aggregator-single-publisher.md) (single-publisher fan-in), [ADR 0021](0021-coordinate-frames-and-command-semantics.md) (frames, sign conventions, angle mode)

## Context

The IMU task now produces clean six-axis samples at a steady ~1 kHz (accelerometer in g, gyro in deg/s, published on a `Watch`). Angle mode — the first flyable mode fixed by [ADR 0021](0021-coordinate-frames-and-command-semantics.md) — needs a trusted estimate of the craft's **roll and pitch angle** to self-level against. Raw sensor data is not that estimate: neither sensor alone yields a usable angle.

- The **accelerometer** measures specific force. At rest it points along gravity, so tilt can be read directly from the gravity vector — an *absolute, drift-free* reference. But it is corrupted by every vibration and any linear acceleration (motor thrust, translation), so instantaneously it is noisy and, under manoeuvres, simply wrong. Its error is high-frequency.
- The **gyro** measures angular rate. Integrating rate gives a *smooth, fast, motion-immune* angle, but integrating any residual bias makes the angle drift without bound. Its error is low-frequency.

[ADR 0021](0021-coordinate-frames-and-command-semantics.md) deliberately deferred attitude estimation to the future control decision while fixing the frames and signs the estimate must obey. [ADR 0017](0017-supervisor-failsafe-state-machine.md) left "telemetry / use of an attitude estimate" open on the same grounds. This ADR closes the *estimation* half: how raw IMU samples become a roll/pitch attitude. It does **not** decide the control law (PID structure, mixing, loop rates) — that remains the `06-control.md` slot.

The sensor suite bounds what is achievable. With a gyro + accelerometer and no usable magnetometer ([ADR 0003](0003-imu-icm42688-spi.md), and the motor-disturbed micro:bit magnetometer left unused per [ADR 0021](0021-coordinate-frames-and-command-semantics.md) §6), gravity fixes roll and pitch absolutely but says nothing about heading. Yaw is therefore not absolutely observable and stays rate-controlled, exactly as [ADR 0021](0021-coordinate-frames-and-command-semantics.md) already committed.

## Decision

### 1. Algorithm: a complementary filter

Roll and pitch are estimated with a **complementary filter** — a fixed-gain blend that low-passes the accelerometer (keep its slow, true average; reject its noise) and high-passes the gyro (keep its fast detail; reject its slow drift), the two filters summing to unity across frequency. Per axis, one recursive update:

```
angle = alpha * (angle_prev + rate * dt) + (1 - alpha) * angle_acc
```

- `angle_prev + rate * dt` — the gyro **prediction**: last estimate rotated forward by the measured rate.
- `angle_acc` — the accelerometer **correction**: the absolute tilt gravity implies.
- `alpha` near 1 — trust the gyro short-term, bleed in a little of the accelerometer's absolute truth each tick to null the drift.

`alpha` is not an arbitrary knob; it sets the crossover time constant `tau = alpha * dt / (1 - alpha)` between the two sensors. Below `tau` the gyro dominates, above it the accelerometer does.

### 2. Not a Kalman / Madgwick / Mahony filter (yet)

The complementary filter is the fixed-gain, steady-state cousin of a Kalman filter: it hard-codes the blend gain as `1 - alpha` instead of recomputing it from noise models each tick. For a gyro + accelerometer hobby quad it delivers the large majority of the benefit for a fraction of the complexity, and it is trivially host-testable. A quaternion-based Madgwick/Mahony filter, or a full Kalman, is a recognised future upgrade — warranted only if measured performance demands it, and then via a superseding ADR. This is the deliberate scope-guardrail choice: the simplest estimator that closes angle mode, not the most capable one.

### 3. Roll and pitch only; yaw stays rate-controlled

The estimator outputs **roll and pitch angle**. It does **not** produce an absolute yaw/heading: gravity gives no yaw reference and the magnetometer is unused, so heading is unobservable with this suite. This matches [ADR 0021](0021-coordinate-frames-and-command-semantics.md) §6, where yaw is a commanded *rate*, not an angle. Yaw rate continues to come straight from the gyro's Z axis for the control loop; no yaw *angle* is estimated or held.

### 4. Frame, axes, and signs follow ADR 0021

The accelerometer measures **specific force**, so at rest its reading points *opposite* gravity: in the FRD body frame (+Z down) a level craft reads `a_z = -1 g`, with `a_x` going positive as the nose rises and `a_y` positive as the craft rolls left. The roll and pitch that gravity implies, with the right-hand sign rules of [ADR 0021](0021-coordinate-frames-and-command-semantics.md) §3 (+roll = right-side-down, +pitch = nose-up):

```
roll_acc  = atan2( -a_y, -a_z )
pitch_acc = atan2(  a_x, sqrt(a_y^2 + a_z^2) )
```

These are the **FRD** forms, deliberately not the more commonly quoted `atan2(a_y, a_z)` / `atan2(-a_x, ...)`. Those assume a *Z-up* accelerometer reading `+1 g` at rest; an FRD (+Z-down) specific-force reading is the negated vector, so using the Z-up forms directly would put roll 180° out and invert pitch — a wrong-way self-level. Equivalently: negate the whole accel vector first (a down-positive gravity vector, `+1 g` at rest) and the textbook forms apply. Checked against bench readings: level -> (0, 0); nose-up -> +pitch; roll-right -> +roll.

Roll pairs with the gyro's X (forward) axis, pitch with Y (right), yaw rate with Z (down). The IMU is mounted **axis-aligned to the body FRD frame**, confirmed by a static tilt test (level `a_z = -1 g`; nose-up gives `+a_x`; roll-right gives `-a_y`), so the filter reads the chip axes directly as FRD with no remap. Re-run that tilt test if the sensor is ever remounted. Angles are held internally in **radians** (the natural output of `atan2`), converted to degrees only at the human-facing / telemetry boundary.

### 5. Sample timing: nominal fixed `dt`

The filter uses a **nominal `dt` equal to the IMU sample period** (currently 1 ms from the polled IMU task). This is justified, not lazy: the nRF timer that paces the poll is crystal-backed (~40 ppm), far more accurate than the filter needs, and the small absolute rate error is a negligible constant scale on integrated angle. The poll running asynchronously to the sensor's own ODR can occasionally duplicate or skip a sample, but a duplicated sample contributes a near-zero rotation and does not accumulate. Moving to INT1 data-ready sampling, or to a measured per-tick `dt`, is a future refinement (it would also make a skipped sample *detectable*), to be adopted only if data shows it matters — consistent with [ADR 0003](0003-imu-icm42688-spi.md)'s stated INT1 intent without making it a prerequisite here.

### 6. Gyro bias removed at startup

Before filtering, a **gyro zero-bias** is measured once at startup by averaging each gyro axis while the craft is held still, then subtracted from every subsequent sample. The complementary filter handles residual drift, but removing the constant offset up front makes it markedly better behaved. This implies a brief "hold still" window during initialisation and is a prerequisite the supervisor's `Initialising` state ([ADR 0017](0017-supervisor-failsafe-state-machine.md)) can cover.

### 7. Where the logic lives, and who publishes it

- The filter is **pure logic in `firmware-drone-core`** — a small stateful type (holding the current roll/pitch estimate and the gyro bias) with an `update(accel, gyro, dt) -> Attitude` method, unit-tested on the host like the supervisor core ([ADR 0015](0015-host-testing-no-std-crates.md), [ADR 0017](0017-supervisor-failsafe-state-machine.md)). No Embassy, no hardware, no `defmt` in the core.
- A thin **estimator stage** drives that pure type at the IMU sample rate (subscribing to the IMU `Watch`) and is the **single publisher** of an attitude `Watch`, which the control loop and telemetry subscribe to. This is the single-publisher pattern of [ADR 0017](0017-supervisor-failsafe-state-machine.md) / [ADR 0020](0020-telemetry-aggregator-single-publisher.md): exactly one task owns the attitude estimate; everyone else reads it. Whether this stage is a standalone task or folded into the IMU task is an implementation detail; the ownership rule is not.

### 8. Output type: an `Attitude` of angle newtypes

The estimate is a small shared struct (working name `Attitude`) in `firmware-types`, carrying roll and pitch as **distinct angle newtypes** rather than bare `f32`s, per [ADR 0016](0016-newtype-per-physical-quantity.md), so a roll/pitch swap is a type error. The angle newtype stores radians with `as_radians()` / `as_degrees()` accessors, mirroring the existing `Acceleration` / `AngularRate` newtypes. As with the §9 deflection types of [ADR 0021](0021-coordinate-frames-and-command-semantics.md), two structurally identical angle newtypes are within the macro-generation allowance of [ADR 0016](0016-newtype-per-physical-quantity.md) once a third appears.

### 9. Tunable parameters named but not valued

Fixed at bench-tuning time, not in this ADR:

- **`alpha`** (equivalently the crossover `tau`) — the gyro/accelerometer blend. A starting point around `0.98`–`0.995` at 1 kHz is typical; the value is tuned against logged flight data, not chosen here.
- **Gyro-bias averaging window** — how long the startup "hold still" sampling runs.
- **Accelerometer-trust gating** (a future refinement, not in the first cut) — reducing or skipping the accelerometer correction when `|a|` strays far from 1 g, to reject the correction during hard manoeuvres. Added only if the un-gated filter misbehaves.

## Why this shape

- **It matches the sensors and the mode.** A complementary filter is exactly what a gyro + accelerometer can feed, and roll/pitch self-levelling is exactly what angle mode ([ADR 0021](0021-coordinate-frames-and-command-semantics.md)) needs. Nothing here assumes sensing the craft lacks.
- **Simplest estimator that closes the loop.** Per the project scope guardrail, the fixed-gain blend is chosen over Kalman/Madgwick because it is enough, it is one multiply-add per axis, and it is trivially testable. The more capable estimators are deferred, not designed out.
- **Pure core, single publisher.** Putting the math in `firmware-drone-core` and giving one stage ownership of the attitude `Watch` reuses the supervisor/aggregator architecture wholesale ([ADR 0017](0017-supervisor-failsafe-state-machine.md), [ADR 0020](0020-telemetry-aggregator-single-publisher.md)): host-tested logic, no shared mutable state, a stranger finds the same pattern they have already seen.
- **Conventions inherited, not reinvented.** Frames, signs, and yaw-as-rate come straight from [ADR 0021](0021-coordinate-frames-and-command-semantics.md); this ADR adds no new spatial convention, only the formulas that consume the existing one.
- **Honest about timing.** A nominal `dt` is defensible on the clock numbers, and the path to a measured `dt` / INT1 is named so the simplification is a recorded choice, not an accident.

## Consequences

### What this commits us to

- An attitude estimate exists as roll/pitch angle newtypes in `firmware-types`, produced by pure `firmware-drone-core` logic and published on a single-owner `Watch`.
- The estimator consumes FRD-aligned IMU data (the sensor is mounted axis-aligned to the FRD frame, §4) and obeys the [ADR 0021](0021-coordinate-frames-and-command-semantics.md) signs; the control loop is written against `Attitude`, not raw IMU samples.
- Startup includes a gyro-bias calibration step (a "hold still" window), which the supervisor's `Initialising` state accommodates.
- Telemetry can carry the estimated roll/pitch (closing the [ADR 0017](0017-supervisor-failsafe-state-machine.md) open item), via the aggregator of [ADR 0020](0020-telemetry-aggregator-single-publisher.md).

### What this rules out

- Estimating or holding an absolute **yaw/heading** with the current suite. Yaw stays a commanded rate; heading hold is a future, magnetometer-bound decision.
- Treating raw accelerometer tilt or raw integrated gyro as "the angle" anywhere in the control path — both go through the filter.
- A per-tick measured `dt` or INT1-driven sampling **in the first cut** (named as a future refinement, not adopted now).
- Accelerometer-trust gating in the first cut (same — a refinement behind measured need).

### What stays open

- **The control law itself.** PID/cascade structure, loop rates, motor mixing — the `06-control.md` decision. This ADR delivers the estimate that law consumes; it does not design the law.
- **Tuning values** (`alpha`/`tau`, bias window) per §9.
- **Upgrade to a quaternion estimator** (Madgwick/Mahony) or a Kalman filter, should logged performance demand it — a future superseding ADR.
- **INT1 data-ready sampling and measured `dt`** — refinements to §5, adopted on evidence.
- **Magnetometer / heading hold** — unchanged from [ADR 0021](0021-coordinate-frames-and-command-semantics.md): a future, board-bound choice, not a missing-hardware blocker.

## References

- ADR 0003 — IMU (gyro + accelerometer, no magnetometer); bounds the estimate to roll/pitch.
- ADR 0013 — `Watch` for shared state; the attitude estimate is published on one.
- ADR 0015 — host-testable `no_std` crates; the filter core is unit-tested this way.
- ADR 0016 — newtype per physical quantity; governs the new angle newtypes.
- ADR 0017 — pure logic in `firmware-drone-core`, single-publisher rule; reused here.
- ADR 0020 — single-publisher fan-in; the estimator owns the attitude `Watch`.
- ADR 0021 — frames, sign conventions, angle mode, yaw-as-rate; the conventions this estimate obeys.
- `doc/06-control.md` — the future control-law decision this estimate feeds.
- External: the complementary filter as a fixed-gain steady-state approximation of a Kalman filter; Madgwick/Mahony quaternion filters as the recognised upgrade path.
