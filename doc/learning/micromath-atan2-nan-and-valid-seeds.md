# micromath `atan2` returns NaN at the origin (std does not), and seed defaults must be physically valid

## The observation

On hardware, the estimated **roll** came back as `NaN` in telemetry while **pitch** was fine. The host unit tests for the estimator were all green. Reflashing didn't help; the roll stayed NaN forever.

## The cause

The attitude estimator seeds its state from the **first IMU sample**, computing the tilt straight from the accelerometer with `atan2`:

```
roll_acc  = (-a_y).atan2(-a_z)
pitch_acc =  a_x.atan2((a_y*a_y + a_z*a_z).sqrt())
```

The IMU task published an **all-zero** accelerometer reading as its startup default (`a = (0, 0, 0)`). That is a physically impossible sample — an accelerometer at rest always reads ~1 g — and it makes `atan2` **degenerate**: the gravity vector has zero length, so there is no defined tilt.

On the **target**, `atan2` is **micromath's** approximation (std is unavailable in `no_std`). micromath returns **NaN** at the origin, where **std returns 0**. So the bug is invisible on the host and only appears on hardware.

### Why roll and not pitch

The two calls feed `atan2` different signs of zero:

- **Roll** negates: `atan2(-0.0, -0.0)` → micromath **NaN**.
- **Pitch** does not: `atan2(+0.0, +0.0)` (the `sqrt` yields `+0.0`) → micromath **0**.

micromath handles the sign-of-zero cases differently, so only roll came out NaN.

### Why it sticks forever

Once the state is seeded NaN, the complementary blend keeps it NaN on every tick, because NaN is absorbing:

```
roll = alpha * (prev_roll + rate*dt) + (1 - alpha) * roll_acc
     = alpha * (NaN + ...)            + ...
     = NaN
```

Real, finite data arriving later cannot recover it — `NaN + finite = NaN`.

## Why the host tests missed it

1. **std `atan2(0, 0) = 0`, not NaN.** The failure mode simply does not exist on the host.
2. The estimator tests seed from *valid* gravity vectors (`accel_for(roll, pitch)`), never a zero vector.

The tolerance-based host tests validate the filter's **logic and signs** — they cannot validate micromath's **numeric edge cases**, which differ from std. That divergence is the price of using a fast approximate `atan2`/`sqrt` on the target.

## The fixes (two layers)

1. **Root cause — valid seed default.** The IMU startup default must be a *physically valid* orientation: level = 1 g down the `-Z` (FRD) axis, i.e. `a = (0, 0, -1)`, not all-zeros. Any value fed to `atan2` for seeding must be a real gravity vector. (`crates/firmware-drone/src/tasks/imu.rs`.)

2. **Defence in depth — scrub non-finite at the type boundary.** `RollAngle` / `PitchAngle` / `YawAngle::from_degrees` now scrub `NaN` to `0.0`, so a degenerate estimate can never propagate into the control path — the same NaN-scrub the command newtypes already do (ADR 0016). Today attitude only feeds telemetry, so a NaN is merely a display glitch; once the PID closes the loop (attitude → demand → mixer), a NaN attitude would become NaN motor commands on an armed craft. The scrub closes that hazard before it matters.

## General rules

- **Seed / init defaults must be physically valid**, not "all zeros." A zero-length vector, a zero quaternion, etc. are degenerate inputs to the trig that consumes them.
- **Scrub non-finite (`NaN`/`inf`) at the type boundaries that feed control.** Cheap, and it turns "undefined behaviour on an armed drone" into "a safe fallback."
- **micromath (and other fast libm approximations) can differ from std at edge cases** (origin, poles, sign-of-zero). Host tests that use std math validate logic, not those edges — be suspicious of degenerate inputs that only the target will evaluate with the approximation.
