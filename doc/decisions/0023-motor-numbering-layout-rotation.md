# ADR 0023 — Motor numbering, layout, and rotation directions

- **Status:** Proposed
- **Date:** 2026-07-18
- **Related:** [ADR 0019](0019-airframe-class-3in-4s-printed.md) (X-quad airframe class), [ADR 0021](0021-coordinate-frames-and-command-semantics.md) (frames + attitude sign conventions the mixer derives from), [ADR 0010](0010-board-support-package.md) (BSP: the `Motor` enum + pin map), [ADR 0017](0017-supervisor-failsafe-state-machine.md) (supervisor is the sole publisher of motor commands), [ADR 0022](0022-attitude-estimation-complementary-filter.md) (attitude estimate feeding the control loop that feeds the mixer)

## Context

The airframe is a symmetric quad-X ([ADR 0019](0019-airframe-class-3in-4s-printed.md)) with four motor outputs on the nRF52833's PWM0 peripheral, exposed through the BSP as the `Motor` enum ([ADR 0010](0010-board-support-package.md)). Until now those outputs were just PWM channels: `set_throttle` could address each one, but no channel had an assigned **physical corner** or **rotation direction**.

Closing the control loop needs both. The motor **mixer** — the stage that turns a throttle plus roll/pitch/yaw corrections into four per-motor thrust commands — is a fixed table of signs over the four corners. That table is only correct if every output has a known corner and a known spin direction. Getting a single sign wrong is the classic "quad flips the instant it arms," so the mapping has to be pinned down and written before the mixer is coded, exactly the way [ADR 0021](0021-coordinate-frames-and-command-semantics.md) pinned the frames before the command types.

Two things are not free choices and one is:

- **Numbering differs by ecosystem.** Betaflight and ArduPilot/PX4 number the four motors differently. Rolling our own firmware, we must pick one convention and record it, or every cross-reference to community material becomes a decoding exercise.
- **Rotation directions are constrained.** On any quad, **adjacent motors spin opposite and diagonal motors spin the same**. This is what cancels net reaction torque (so the craft doesn't spin on its own) and what lets differential motor torque produce yaw. Within that constraint there are two mirror schemes, **props-out** and **props-in**.
- **Which corner is which channel is a wiring choice** we get to make when soldering the ESC signal outputs and mounting the motors.

## Decision

### 1. Layout and numbering: quad-X, Betaflight-style

The four motors sit at the corners of a symmetric X, numbered in the **Betaflight** convention. Viewed from **above**, front between the two forward arms:

```text
              front
         M4 (FL)     M2 (FR)
           CCW         CW
             \        /
              \      /
                hub
              /      \
             /        \
         M3 (RL)     M1 (RR)
           CW          CCW
              rear
```

- **M1 = rear-right, M2 = front-right, M3 = rear-left, M4 = front-left.**
- The `Motor` enum uses the same 1-based names (`Motor1`..`Motor4`), so the code and this diagram read identically.

The Betaflight numbering is chosen over ArduPilot/PX4 purely because it is the dominant convention in the small-quad / FPV world this build draws its parts and community knowledge from.

### 2. Channel and pin assignment

Each motor maps to a fixed PWM channel and micro:bit v2 edge-connector pin (the authoritative copy lives in the [`board`](../../crates/firmware-drone/src/board/microbit_v2.rs) pin map):

| Motor | Corner | PWM channel | nRF pin |
|-------|--------|-------------|---------|
| M1 | rear-right  | PWM0 ch0 | P0.10 |
| M2 | front-right | PWM0 ch1 | P0.09 |
| M3 | rear-left   | PWM0 ch2 | P0.12 |
| M4 | front-left  | PWM0 ch3 | P0.02 |

The chosen ESC (Sequre Blueson A1, [ADR 0019](0019-airframe-class-3in-4s-printed.md)) numbers its own four motor-pad groups in this same Betaflight-style layout — M1 and M4 on opposite (diagonal) corners, adjacent numbers on adjacent corners (i.e. "4 opposite 1"). This is confirmed against the board (vendor wiring diagram kept at [`doc/hardware/blueson-a1-en-long.jpg`](../hardware/blueson-a1-en-long.jpg); the vendor docs give the pad *numbering* but not a phase pinout). So the ESC's labelled M1–M4 outputs map one-to-one onto the channels above, and no re-numbering is needed at the ESC-to-motor boundary. The individual **phase** wiring within each motor is a separate matter — see §5.

### 3. Rotation directions: props-out

Rotation follows the **props-out** scheme (top blades sweep outward at the front — the usual default, cleaner air over a forward-facing camera):

- **M1 and M4 (one diagonal): CCW** (viewed from above).
- **M2 and M3 (other diagonal): CW.**

This satisfies the adjacent-opposite / diagonal-same constraint. Each motor carries the prop of matching handedness (a set is 2 CW + 2 CCW).

### 4. Mixer sign table (derived, to be verified)

With corners and rotations fixed, the mixer signs follow directly from the attitude sign conventions in [ADR 0021](0021-coordinate-frames-and-command-semantics.md) (+roll = right side down, +pitch = nose up, +yaw = nose right, in body FRD). Each motor's command is:

```
output(Mi) = throttle
           + roll_cmd  * roll_sign(Mi)
           + pitch_cmd * pitch_sign(Mi)
           + yaw_cmd   * yaw_sign(Mi)
```

with signs:

| Motor | Corner | Spin | Throttle | Roll (+ = right down) | Pitch (+ = nose up) | Yaw (+ = nose right) |
|-------|--------|------|:--------:|:---------------------:|:-------------------:|:--------------------:|
| M1 | rear-right  | CCW | + | − | − | + |
| M2 | front-right | CW  | + | − | + | − |
| M3 | rear-left   | CW  | + | + | − | − |
| M4 | front-left  | CCW | + | + | + | + |

Reasoning (each derived from ADR 0021, not copied from a stock table — our positive-direction conventions are our own, so the numeric signs need not match Betaflight's internal mix):

- **Roll** (+ = right down): raise the **left** motors, lower the **right** — a torque about +X that drops the right side. Left = M3, M4 (+); right = M1, M2 (−).
- **Pitch** (+ = nose up): raise the **front** motors, lower the **rear**. Front = M2, M4 (+); rear = M1, M3 (−).
- **Yaw** (+ = nose right): a prop spinning **CCW** (from above) reacts a nose-right (about +Z-down) torque onto the airframe, so raise the CCW motors and lower the CW. CCW = M1, M4 (+); CW = M2, M3 (−).

These signs are a **derivation, not yet a validated fact**. They must be confirmed before free flight — first on the bench (props off, tethered), then in a controlled first hover — because a mis-derivation here is the flip-on-arm failure mode. The derivation is recorded so the verification has something concrete to check against.

### 5. Direction is set/corrected in the ESC, not by re-soldering

Motor phase wires are soldered in arbitrary order (phase order only sets direction, which is recoverable). Actual rotation is confirmed on a **props-off spin test** and any wrong-way motor is brought into line with §3 — either by swapping any two of its three phase wires, or by flipping its direction in the **AM32 configurator** (with DShot, reversible in software live).

**As built:** the phase order was arbitrary at solder time and several motors came up spinning the wrong way; they were corrected on the bench with a two-wire phase swap until all four matched the §3 scheme. So the per-motor phase-to-pad mapping is not a fixed, documented pinout — it is whatever the spin test settled on, and the source of truth for direction is the confirmed rotation, not the wire colours.

## Consequences

### What this commits us to

- The mixer, when written, reads this exact corner/rotation mapping; its sign table is §4. The `Motor` enum doc comment in the BSP mirrors this ADR and must move in lockstep with it.
- Any later change to a corner assignment or a rotation direction is a change to the mixer signs too — the two cannot drift apart without the craft becoming unstable. Such a change supersedes this ADR rather than editing it.
- The props-off spin test is a required bring-up step, and its outcome (each motor's confirmed direction) is the validation of §3.

### What this rules out / leaves open

- **ArduPilot/PX4 numbering** — explicitly not used; community material in that convention must be re-mapped.
- **Props-in** — not used unless a future camera/prop-wash reason justifies it, which would be a new ADR.
- **Mixer scaling and saturation handling** (how roll/pitch/yaw authority trades against throttle headroom, motor-command clamping) — a control-law concern, deferred to the `06-control.md` slot alongside the PID structure ([ADR 0022](0022-attitude-estimation-complementary-filter.md) is the estimator half).
- **ESC protocol** — standard servo PWM for now, DShot later; orthogonal to this mapping.

### Current state

The layout, numbering, channel/pin assignment (§1–§2) and the props-out rotation scheme (§3) are already reflected in the firmware (`Motor1`..`Motor4`, the pin map, and the `Motor` enum doc comment). The mixer (§4) is not yet implemented — `motor_controller` currently drives all four channels with the same throttle as a bench placeholder, with no attitude mixing.
