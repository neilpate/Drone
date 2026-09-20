# 06 — Flight Control: Troubleshooting and Tuning Field Guide

_Date: 2026-09-20_
_Status: Living document. Append as new failure modes are understood; correct entries that turn out wrong._

This is the companion to [06-control.md](06-control.md). That doc is the *map* — how a stick deflection and a batch of IMU samples become four motor commands. This doc is the *field guide* — the recurring ways the control system misbehaves, how to tell them apart, and what actually fixes each one. It exists because the same symptoms keep coming back across airframes and sessions, and each time the temptation is to re-diagnose from scratch or to blame the control code, which has repeatedly been proven correct.

Related material:

- [06-control.md](06-control.md) — the control path as implemented.
- [progress.md](progress.md) — the dated history of how each fight was won.
- ADRs [0021](decisions/0021-coordinate-frames-and-command-semantics.md) (frames/signs), [0022](decisions/0022-attitude-estimation-complementary-filter.md) (estimator), [0023](decisions/0023-motor-numbering-layout-rotation.md) (mixer), [0024](decisions/0024-control-law-angle-mode-pd.md) (control law) — the decisions.
- [`analyze`](../crates/groundstation/src/bin/analyze.rs) — the offline telemetry analyser. Every diagnosis below is read from its report.

## Why the control system is hard

Four structural reasons the same problems recur. Keep them in mind before reaching for a gain slider.

1. **The estimator matters as much as the controller.** The controller can only be as good as its idea of "level." A wrong estimate makes a perfect controller fly wrong, and no gain change fixes it. More than one "tuning" problem has turned out to live in the estimator (see the roll-oscillation entry).
2. **Observability gap.** The only motion sensor is a 6-DoF IMU — no barometer, rangefinder, flow, or GPS. Attitude is observable (gravity is an absolute reference); altitude and horizontal position are not. The craft can hold *attitude* but not *position*. This is physics, not a bug, and it caps what the control system can ever do without new sensors.
3. **The airframe leaks into the loop.** CoM offset, motor/prop asymmetry, structural flex, and motor-vibration on the IMU all show up as control symptoms. The rigid-body assumption the whole controller rests on is only as true as the airframe.
4. **Tuning is empirical and airframe-specific.** Gains are set by the mass distribution and inertia of *this* airframe. A lighter frame, a moved battery, or a new prop can invalidate a tune. There is no "correct" gain set, only one that suits the current machine.

## The one habit: capture, then analyse

Do not debug flight behaviour from memory or eyeball. Every complaint starts the same way:

1. Fly a short run with telemetry logging on; note the `telemetry-*.tsv` path.
2. From [`crates/groundstation`](../crates/groundstation): `cargo run --release --bin analyze -- <file>`.
3. Read the report. The low-rate telemetry echoes the drone's **live `ControlSystemParameters`** ([ADR 0027](decisions/0027-split-telemetry-high-low-rate.md)), so the report shows the *actual gains flown* — never debug a run without knowing its gains.

The analyser is header-driven and reports, among other things: controller-demand-vs-model reconstruction, a rest/drift diagnosis, a rate-vs-angle phase measurement, a per-axis periodogram, and flight-segment isolation. The sections below say which line to read for each symptom.

## Symptom index

| Symptom | Most likely cause | Confirm in `analyze` | Section |
|---|---|---|---|
| Veers / skirts off in one direction on takeoff | Held tilt (estimator offset) + no position hold | rest offset est vs accel; demand-vs-model +1.0 | [1](#1-veers-or-skirts-off-in-one-direction) |
| "Self-levelling doesn't work" but loop looks fine | Attitude ≠ position; drift is uncontrolled | attitude held near level, but craft translates | [1](#1-veers-or-skirts-off-in-one-direction) |
| Slow (~0.5 Hz) wallow | Too much `ki` | dominant oscillation ~0.5 Hz on the integral axis | [2](#2-oscillation-limit-cycle) |
| Fast (~3–5 Hz) ring | Too much `kp`, or estimator lag | dominant oscillation 3–5 Hz; rate-vs-angle phase | [2](#2-oscillation-limit-cycle) |
| Oscillation survives every P/D value | Estimator lag (crossover in control band) | rate leads angle by ≫ +90° | [3](#3-estimator-lag-positive-feedback) |
| Persistent lean, re-zero only helps briefly | Gyro bias leaking into the estimate | rest: accel drift ~0 but estimate drifting | [4](#4-persistent-lean-or-slow-creep) |
| Craft holds a steady tilt / needs constant trim | Real CoM or thrust imbalance | motor split; standing demand on one axis | [5](#5-com-imbalance-and-balancing) |
| Integral won't trim a known offset in a short hop | `τ = kp/ki` too large | demand trim still climbing at end of hop | [6](#6-integral-too-slow-to-trim) |
| Trim built in-hand springs the craft on release | Integrator windup against the hand | large standing demand after a cradled run | [7](#7-integrator-windup-when-held) |

---

## 1. Veers or skirts off in one direction

**Symptom.** On throttle-up the craft slides or veers off consistently one way; the pilot chops the throttle.

**First, rule out a control bug — it almost certainly is not one.** The forward control path (estimate → error → PID → demand) has been verified repeatedly: `analyze`'s demand-vs-model reconstruction returns correlation **+1.0000** with machine-precision residual. The mixer and estimator signs are bench-verified ([ADR 0023](decisions/0023-motor-numbering-layout-rotation.md)). If demand-vs-model is near +1.0, stop looking for a sign or scaling bug.

**The real mechanism — a held tilt the craft believes is level.** The controller drives the *estimate* to level. If the estimate has a static offset — say it reads 1° low on roll — then holding the estimate at zero parks the *true* attitude at ~1° of bank. A 1° tilt is `g·sin(1°) ≈ 0.17 m/s²` of sideways acceleration, and with no position feedback nothing arrests it, so the craft slides steadily one way.

**Confirm.** In the rest/drift diagnosis, compare the estimate to the accelerometer's own gravity angle (the truth reference at rest). A non-zero `est − accel` offset with near-zero drift is a static calibration/mounting offset.

**Fix.** Re-zero the IMU on a genuinely level surface (spirit-level it). This has taken the roll offset from ~1° to ~0.1° and removed the skirt. Note that a gyro-bias leak (below) will slowly re-acquire a tilt over tens of seconds, so re-zeroing buys a clean window rather than a permanent cure.

**The part re-zeroing cannot fix.** Self-levelling holds **attitude, not position**. Even a perfectly level craft drifts, because horizontal position and velocity are unmeasured. "It won't hold still on throttle alone" is not a tuning failure — it is the observability gap. The only real fixes are to *pilot* the position out by hand, or to add horizontal sensing (optical flow + a downward rangefinder). No gain change makes an attitude controller hold position.

## 2. Oscillation (limit cycle)

Read the dominant-oscillation periodogram in `analyze`. The frequency tells you which knob:

- **Slow, ~0.5 Hz wallow, on the axis that has the integral** — classic **too much `ki`**. The integral winds past the null, overshoots, winds back. Back `ki` off to the lowest value that still trims. (Historically the 0.5 Hz wallow was the `ki` ceiling; in the 2026-09-20 session `ki` up to 0.12 showed *no* wallow, so the ceiling is airframe-dependent — do not assume a fixed limit.)
- **Fast, ~3–5 Hz ring** — the attitude loop itself is marginally stable. Two causes, distinguished by the rate-vs-angle phase (section 3):
  - **Too much `kp`** — the loop crosses over with too little phase margin and rings at the crossover frequency. Lower `kp`; the ring frequency drops as you do (a confirming signature). The lower-inertia axis (usually roll) crosses first, so decouple roll and pitch gains.
  - **Estimator lag** — see section 3. If the phase is badly wrong, no `kp`/`kd` value will kill the ring; you are fighting the wrong layer.
- **Buzz in the tens of Hz** — `kd` amplifying gyro noise. Lower `kd` or the D-term filter cutoff. Note the telemetry Nyquist (~40–45 Hz) makes a peak near the ceiling unreliable; log gyro at the loop rate to resolve a true high-frequency buzz.

`kd` is easy to leave doing nothing: at the bring-up gains, D was ~70× smaller than P, so the loop was effectively pure-proportional — which *will* limit-cycle no matter how P is set. If lowering P only *moves* the oscillation rather than killing it, the missing ingredient is damping, not less P.

## 3. Estimator lag (positive feedback)

**Symptom.** A roll or pitch oscillation that survives *every* P/D value tried.

**Mechanism.** Angle is the integral of rate, so a gyro must **lead** its own attitude estimate by **+90°**. If the complementary filter's gyro/accel crossover sits inside the control band, motion-coupled accelerometer noise distorts the estimate exactly where it matters, adding phase lag. Once the excess phase pushes the rate-to-angle lead well past +90° (toward +180°), proportional feedback becomes *positive* feedback at the oscillation frequency, and the loop rings regardless of gain.

**Confirm.** `analyze` reports the rate-vs-angle phase per axis. Expect ~+90°. The roll-oscillation episode measured ~+150° — about +60° of excess phase — which was the smoking gun.

**Fix.** Move the crossover well below the control band by raising the filter coefficient α ([ADR 0022](decisions/0022-attitude-estimation-complementary-filter.md)). Raising α exposes any latent gyro bias as a steady attitude offset (`θ = α·b·dt/(1−α)`), so zero the gyro bias at startup as well — which then frees α to go lower for motion rejection with no bias penalty. This is the canonical "wrong tool, wrong layer" fix: the symptom looked like a gain problem and was an estimator problem.

**Carry-forward.** Mild estimator lag (phase > +90° but not catastrophic) is still visible in the current filter and is one of the reasons an off-CoM IMU hurts. Moving the IMU to the CoM (the next airframe) reduces the motion coupling at the source; online gyro-bias estimation is the durable software upgrade.

## 4. Persistent lean or slow creep

**Symptom.** A steady attitude offset, or a slow drift that re-zeroing cures only temporarily.

**Confirm.** Rest/drift diagnosis: if the accelerometer angle is stable (drift ~0) but the *estimate* drifts, a residual **gyro bias** is leaking through the filter. If both are stable but offset, it is a static calibration/mounting offset (section 1).

**Fix.** Re-zero clears it for a while; the bias climbs again per run (observed rising from ~0.06 to ~2.3 dps across a session), so the real fix is **online bias estimation** rather than a one-shot startup calibration. A separate, dynamic version of this — a lean that appears only when armed — is **motor-vibration rectification** on the IMU, which needs RPM-tracking notch filtering (and therefore per-motor RPM from DShot eRPM) to fix properly.

## 5. CoM imbalance and balancing

**Symptom.** The craft holds level but needs a constant trim to do it; one axis carries a standing demand; motors are visibly unequal.

**Mechanism.** A CoM offset is a steady torque. To hold level the mixer must run the motors under the heavy corner harder. The integral supplies this trim automatically (that is what it is *for*), but a large trim spends control headroom — motors under the heavy corner sit closer to saturation, leaving less authority for that axis.

**Reading the motor split.** In the `analyze` active/flight motor means, **a motor works hardest under the heaviest corner**. Use the balance lines directly:

- `front − rear` split and the standing `demand_pitch` both gauge fore-aft CoM.
- `left − right` split and standing `demand_roll` gauge lateral CoM.
- Drive both toward zero by moving mass **toward the softest (lightest) corner**.

**Method that works.** With a shapeable nose ballast (Blu-Tack), shave preferentially off the *hardest corner* — that attacks both the fore-aft and the lateral imbalance at once. The response is roughly linear in mass moved, so the change from one adjustment extrapolates the next. Target: the four motor means converge to within a few thousandths and the standing roll/pitch demands fall to near zero. Example progression from one session: front−rear split 0.075 → 0.047 → 0.025, `demand_pitch` 0.038 → 0.024 → 0.012.

**Separating CoM from a weak motor.** If a specific corner is consistently low on liftoff, it is either CoM toward that corner or a weak motor/prop on it. The **liftoff transition is the cleaner test**: the integral resets at zero throttle and takes a second or two to wind, so the way the craft tips *as it leaves the ground* shows the raw imbalance before the integral masks it. To decide which: balance the bare frame on a fingertip at the geometric centre, props off. Tips → CoM (move mass). Sits level → motor/prop (inspect that corner's prop for damage/orientation and its motor for bearing drag). A single-axis rig pivoted **on the CoM** is the safe way to see true closed-loop response — a pendulum-stable rig (pivot below CoM) flatters the controller and hides instability; a neutral one shows the truth.

## 6. Integral too slow to trim

**Symptom.** A known offset (e.g. a CoM lean) is not trimmed out within a short hop, even though the integral is "working."

**Mechanism.** With centred sticks the integral clears a steady offset first-order, with time constant:

$$\tau = \frac{k_p}{k_i}$$

The integral is stored in contribution units (`I += ki · error/max_tilt · dt`, output adds `I` directly), which makes the accumulation loop-rate-independent and live-`ki` retuning bumpless. At the bring-up `ki = 0.02`, `τ ≈ 5.5–7.5 s` — 95% correction takes ~3τ ≈ 17–22 s, far longer than a short hop. And the integral **resets to zero at zero throttle** (anti-windup, [ADR 0024](decisions/0024-control-law-angle-mode-pd.md)), so every chop throws away what it built.

**Fix.** Raise `ki` so `τ ≈ 1–2 s` (e.g. `ki ≈ 0.07` against `kp` 0.11–0.15). Then the trim builds within a few seconds of hover. `ki` is a *pre-flight* setting — dial it on the ground, no in-flight interaction needed. Raise it incrementally and watch the periodogram for the slow wallow of section 2. Note the trim *value* the integral converges to is set by the airframe's physics, not by `ki` — a higher `ki` reaches the same trim faster, it does not change it.

**Design note.** The `throttle == 0` reset is deliberate (it stops windup on the bench and between arms). If you want a trim to persist across brief throttle chops for a series of hops, the refinement is to reset **only when disarmed** rather than on every zero-throttle sample — a small control-law change with a safety angle worth recording in [ADR 0024](decisions/0024-control-law-angle-mode-pd.md) if adopted.

## 7. Integrator windup when held

**Symptom.** After tuning or trimming with the craft cradled in hand, it lurches one way the instant it is released.

**Mechanism.** Holding the craft — even loosely, even at non-zero throttle — constrains its attitude. The control loop cannot null the error by rotating the craft (your hand prevents it), so the integrator keeps accumulating against whatever tilt your hand imposes. It winds up a large trim for that held attitude. On release, the over-wound trim has nothing opposing it and springs the craft that way. If you cradle it slightly nose-down, it winds a nose-up trim and pitches up on release.

**Consequence.** You cannot get a valid free-flight trim while holding the craft. The integral must wind against the *real* free-flight dynamics. This is why real flight controllers gate or freeze the integrator until the craft is detected airborne — the `throttle == 0` reset is a crude version, defeated by holding at non-zero throttle.

**Fix.** Trim and tune the integral in free flight (accept a couple of seconds of drift while it winds), or on a CoM-pivoted rig that frees the axis without constraining it. Do not judge a trim from a cradled run.

## Quantitative relationships worth keeping

- **Integral trim time constant:** `τ = kp / ki`. Set `ki` for `τ ≈ 1–2 s`.
- **Estimator health:** a gyro axis must lead its own attitude estimate by **+90°**. Excess phase toward +180° is positive feedback.
- **Complementary-filter offset from bias:** `θ = α·b·dt/(1−α)` — the steady lean a residual gyro bias `b` produces, growing with α. Zero the gyro bias so α can go low.
- **Tilt-to-drift:** a held tilt `θ` gives `g·sin(θ)` lateral acceleration — ~0.17 m/s² per degree. This is why a 1° estimate offset is not cosmetic.
- **Balance:** the hardest-working motor sits under the heaviest corner; drive the front−rear and left−right splits (and the standing demands) to zero.

## Reference points (airframe-specific — do not treat as universal)

- **Known-good old/heavier micro:bit frame (2026-09-20):** `kp_roll = 0.11`, `kp_pitch = 0.15`, `kd = 0.16` both axes, `ki = 0.07` both axes. The compiled-in `ControlSystemParameters::default()` is *hotter* than this on roll — a fresh boot without a live push or a flash-saved tune ([ADR 0025](decisions/0025-persist-control-parameters-flash.md)) runs hot.
- **Any new frame needs a fresh tune.** A lighter frame has lower inertia, so the same `kp` drives it harder — expect to pull `kp` down and scale `ki` with it to keep `τ ≈ 2 s`. Re-zero the IMU on a level surface before the first hop.
- **Roll vs pitch asymmetry.** A frame long in one axis has unequal inertia (`I_yy ≠ I_xx`); the lighter axis wants gentler gains. Decouple roll and pitch rather than sharing a gain.

## Open carry-forwards

- **Estimator lag** — the complementary filter still shows mild excess phase; IMU-near-CoM and online bias estimation are the fixes ([ADR 0022](decisions/0022-attitude-estimation-complementary-filter.md)).
- **Gyro bias climbing per run** — points at online bias estimation over one-shot startup calibration.
- **Motor-vibration rectification** — an armed-only lean; needs RPM-tracking notch filtering, gated on DShot eRPM.
- **Position drift** — unfixable without horizontal sensing; the ceiling on any "hover" ambition until a rangefinder/flow sensor is added.
