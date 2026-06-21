# ADR 0021 — Coordinate frames, attitude sign conventions, and pilot-command semantics

- **Status:** Proposed
- **Date:** 2026-06-21
- **Related:** [ADR 0003](0003-imu-icm42688-spi.md) (IMU on SPI), [ADR 0013](0013-async-communication-primitives.md) (`Watch<PilotCommand>`), [ADR 0016](0016-newtype-per-physical-quantity.md) (newtype per physical quantity), [ADR 0017](0017-supervisor-failsafe-state-machine.md) (supervisor consumes `PilotCommand`), [ADR 0018](0018-pc-link-uart-postcard-cobs.md) (the link that carries the command)

## Context

`PilotCommand` currently carries one control: `throttle`. With the gamepad path working end to end (gamepad → `PilotCommand` → radio → drone → telemetry → plot), the obvious next step is to add the remaining three controls — roll, pitch, yaw — so the craft can be steered, not just throttled.

The trap is that the moment a tilt field is added, a pile of conventions is committed to *implicitly*: which body axis is roll versus pitch, which sign is "nose-up", whether a stick value means a desired *angle* or a desired *rate*, and whether the remote or the drone decides that. Bake those in ad hoc inside the first implementation and they have to be unpicked later when the IMU driver and the control loop must all agree on the same frame and signs — exactly the silent, hard-to-debug class of error called out in the project glossary (body-vs-world frame, NED-vs-ENU, quaternion sign order).

No existing ADR pins these down. The glossary in `AGENTS.md` gestures at NED and body frame but nothing authoritative fixes the frames, the rotation signs, or the meaning of a stick deflection. This ADR does that, so the conventions exist *before* the code that assumes them.

Hardware bounds the scope. The external IMU chosen for attitude sensing ([ADR 0003](0003-imu-icm42688-spi.md)) is an ICM-42688-P: a 6-DoF gyro + accelerometer, no magnetometer. That is enough to know which way is down (accelerometer) and how fast the craft is rotating (gyro), so it supports **rate** and **angle** control. The micro:bit v2 *does* carry a magnetometer on its internal I2C bus (an ST LSM303AGR, a combined accel + magnetometer), so an absolute heading reference is not strictly absent from the airframe. But it is best treated as if it were: a magnetometer bolted to the frame next to motor and ESC currents is heavily disturbed, which is why many quads fly "mag-less" regardless, and the part is specific to the micro:bit — it disappears at the Phase 4 migration to a custom nRF5340 board ([ADR 0001](0001-platform-airframe-stack.md), [ADR 0002](0002-mcu-and-language.md)), whose only motion sensor is the external ICM-42688 unless a magnetometer is deliberately added. Using it for heading hold is therefore a deliberate, board-bound later choice, not a foundation to build on. Altitude or position hold need sensing the craft does not have at all (a barometer / rangefinder / position estimate). The command semantics chosen here stay inside what can be closed reliably with the gyro + accelerometer alone — the sensor suite that survives the board migration.

This ADR fixes **conventions and command semantics only**. The control law itself — the PID/cascade structure, loop rates, attitude estimation — is a separate, later decision (the `06-control.md` slot).

## Decision

### 1. World frame: NED

The world (ground-fixed) frame is **NED** — North, East, Down — a right-handed, local-tangent frame:

- +X = North, +Y = East, +Z = **Down** (toward the earth's centre, the direction gravity pulls).
- Altitude is the negation of the third axis: `altitude = -Z`.
- It is a *local* frame, defined at the flying site and tangent to the surface there. Over the distances this craft flies, it is a fixed flat 3-D grid.

NED is the aerospace standard; flight-dynamics references, textbooks, and existing libraries assume it, so adopting it avoids translating every equation we read.

### 2. Body frame: FRD

The body (airframe-fixed) frame is **FRD** — Forward, Right, Down:

- +X = Forward (out the nose), +Y = Right, +Z = Down (out the belly).
- Same right-handed convention as NED; FRD and NED **coincide exactly** when the craft is level and pointing north.
- The IMU is bolted to the airframe, so its measurements are body-frame quantities. The rotation that maps NED onto FRD *is* the craft's attitude.

### 3. Attitude angles and sign conventions

Attitude is expressed as the three Euler angles, applied in the aerospace-standard **3-2-1 (Z-Y-X, yaw-then-pitch-then-roll)** order, taking NED to FRD. Positive senses follow the right-hand rule about each body axis:

| Angle | Symbol | About axis | Positive sense |
|-------|--------|------------|----------------|
| Roll  | φ (phi)   | X (forward) | right side **down** |
| Pitch | θ (theta) | Y (right)   | nose **up** |
| Yaw   | ψ (psi)   | Z (down)    | nose **right** (heading increases N→E→S→W) |

Because +Z points down, positive yaw is clockwise viewed from above (a *right* turn) — the opposite apparent sense to a Z-up (ENU / graphics) frame. This is intentional and is the aerospace norm.

### 4. Axis colours for diagrams and physical references

All axis diagrams, CAD annotations, and physical bench references use the near-universal **RGB = XYZ** mapping: **X = red, Y = green, Z = blue**, with **Z pointing down** to match FRD/NED. This keeps every spatial reference in the project (docs, a printed desk triad, future visualisations) mutually consistent.

### 5. Control mode for the first cut: angle mode

The first flyable mode is **angle mode** (self-levelling / stabilise): stick deflection commands a desired *tilt angle*, and centred sticks command zero tilt, so releasing the sticks returns the craft to level.

"Friendly to fly" is treated as a hard requirement, not a nicety: the project's whole point is that the builder can learn to fly the result. Angle mode is the single biggest lever for that — the flight controller runs the fast inner loop (resisting rotation and self-levelling far quicker than human reflexes), leaving the human to supply only the slow position/velocity judgement by eye. It is achievable with the gyro + accelerometer alone.

**Rate mode** (acro: stick = rotation rate, centred sticks hold the current attitude) is a recognised future mode, not the default.

### 6. Stick semantics

In angle mode the four controls mean:

| Control | Meaning | Resting value |
|---------|---------|---------------|
| Throttle | direct total thrust, `0..=1` | 0 (idle) |
| Roll  | desired roll angle  (via deflection × max-tilt) | 0 (level) |
| Pitch | desired pitch angle (via deflection × max-tilt) | 0 (level) |
| Yaw   | desired yaw **rate** (via deflection × max-rate) | 0 (no turn) |

Yaw is a *rate* even in angle mode. The on-board magnetometer is left unused for now (heavily disturbed by motor currents), so there is no trusted absolute heading to hold an angle against; and pilots expect to be able to point the nose anywhere and have it stay. Both point to rate control of yaw. This matches every hobby flight controller.

### 7. The remote sends raw deflections; the drone interprets

`PilotCommand` carries **raw normalised stick deflections**, not interpreted setpoints:

- Roll / pitch / yaw: bipolar, normalised `-1.0..=1.0` (centre = 0).
- Throttle: unipolar, `0.0..=1.0` (the existing `Throttle`).

The **drone** owns the interpretation — mapping a deflection to an angle or a rate using its tunable limits, according to the active flight mode. The remote stays "dumb": it reports stick positions, exactly as a hobby RC transmitter reports gimbal positions and lets the flight controller interpret them.

This keeps the wire format **mode-independent**: adding a future flight mode (rate, altitude-hold) is a drone-side change, not a change to `PilotCommand` or the radio protocol. It also matches the "drone is the brain" architecture in `02-architecture.md`.

### 8. `PilotCommand` shape

`PilotCommand` gains roll, pitch, and yaw deflection fields alongside `throttle` and the existing `sequence_count`. Per [ADR 0016](0016-newtype-per-physical-quantity.md), each deflection is a distinct newtype rather than a bare `f32`, so the type system catches an argument-order swap (passing roll where pitch is expected). The three stick-deflection newtypes are structurally identical (a clamped `-1.0..=1.0`), which is the case ADR 0016 names as acceptable to generate with a macro once at least three such newtypes exist.

Arm/disarm and flight-mode selection are **not** added here (see "What stays open").

### 9. Tunable parameters named but not valued

The following are control-feel parameters, fixed at bench-tuning time, not in this ADR:

- **Maximum tilt angle** — the angle at full roll/pitch deflection. A gentle cap (order 20–25°) is the classic trainer setting: a full stick jab leans modestly rather than flipping or bolting.
- **Maximum yaw rate** — the rate at full yaw deflection.
- **Expo and dead-zone** — softening near stick centre and ignoring the gamepad's spring-return slop, for controllable fine hovering.

## Why this shape

- **Aerospace conventions, everywhere.** NED/FRD, the right-hand sign rules, and 3-2-1 order are what the references, datasheets, and existing code assume. Matching them means no per-source translation and no novel convention for a stranger to learn. Right-handedness throughout removes the most common class of sign bug.
- **Same convention for world and body, distinct vocabulary.** Identical handedness lets the *numbers* translate cleanly; the different names (North vs Forward) force the author to state *which frame* a vector lives in — the discipline that prevents body-into-world-equation mistakes.
- **Angle mode first is dictated by both safety and sensors.** It is the flyable, forgiving mode, and it is exactly what a gyro + accelerometer can deliver. Rate and the automatic outer-loop modes (altitude, position) need either more skill or more sensing, so they are deferred without being designed out.
- **Raw deflections decouple the wire from the control law.** Interpreting on the drone means the remote never needs to know the control mode, the max-tilt, or the loop structure. New modes and retuning are drone-side, behind a stable command type — the protocol-stack layering argued in the cascaded-control discussion.
- **Yaw as a rate** follows from leaving the (motor-disturbed) magnetometer unused and from pilot expectation; holding yaw to an angle would mean trusting a heading reference we are deliberately not relying on yet.

## Consequences

### What this commits us to

- Every attitude / vector quantity in the firmware is expressed in NED (world) or FRD (body), right-handed, with the signs in §3. The IMU's raw sensor axes are rotated into FRD at the driver boundary (a documented per-mounting sign/axis remap), not propagated raw.
- `PilotCommand` carries mode-independent, normalised stick deflections (roll/pitch/yaw bipolar newtypes + the existing `Throttle`); the radio and link formats ([ADR 0018](0018-pc-link-uart-postcard-cobs.md)) carry the same.
- The first flyable mode is angle (self-levelling); the supervisor and control loop are built around that.
- All spatial diagrams and references use RGB = XYZ with Z down.

### What this rules out

- Z-up / ENU / left-handed conventions anywhere in the firmware. If a third-party library insists on ENU, the conversion happens at an explicit, documented boundary, not by quietly relabelling axes.
- The remote computing interpreted setpoints (angles or rates). It sends deflections only.
- Rate (acro) as the default first mode.

### What stays open

- **Arm/disarm and flight-mode channel.** Deliberately out of scope here. Both affect `PilotCommand`'s shape and the safety model and warrant their own decision; angle mode is assumed as the sole mode until then. Arming in particular is a safety prerequisite before any motor spins on a built airframe.
- **The control law.** PID/cascade structure, loop rates, motor mixing, and attitude estimation (and whether attitude is held internally as a quaternion, with Euler φ/θ/ψ only as the human-facing convention) are the `06-control.md` decision, not this one. This ADR fixes the frame and command contract the controller will be written against.
- **The tuning values** in §9.
- **Magnetometer / absolute heading hold.** The micro:bit's on-board LSM303AGR magnetometer exists but is left unused for now (motor/ESC disturbance), and it is micro:bit-specific — it goes away at the Phase 4 custom-board migration, where heading sensing would have to be designed in deliberately. Enabling it for heading hold is a future, board-bound decision, not a missing-hardware blocker. **Altitude hold, position hold** do need sensing the craft lacks (baro / rangefinder / position estimate) and stay out of scope until that hardware exists.

## References

- ADR 0003 — IMU (ICM-42688-P, gyro + accel, no magnetometer); bounds the achievable modes.
- ADR 0013 — `Watch<PilotCommand>`; the primitive the command flows through on the drone.
- ADR 0016 — newtype per physical quantity; governs the new roll/pitch/yaw deflection types.
- ADR 0017 — the supervisor consumes `PilotCommand`; it gains the new fields' handling.
- ADR 0018 — the ground-station link that carries `PilotCommand` (and its echo in telemetry).
- `doc/02-architecture.md` — "drone is the brain" split that motivates §7.
- `doc/06-control.md` — the future control-law ADR/notes this decision sets the stage for.
- External: aerospace NED/FRD frame and 3-2-1 Euler convention; hobby flight-controller stick modes (angle vs rate).
