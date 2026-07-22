# When the control loop looks unstable, suspect the bench rig

**Observation.** With the closed-loop (self-levelling) controller running, the drone behaved erratically the moment the motors had power: the roll/pitch **demand** telemetry was full of noise and the motors made the audible "restart" tones of an ESC rebooting. The obvious suspect was the freshly written PID. It was not. In **manual** (open-loop) mode the motors responded sensibly and the motor mapping was provably correct, which said the plant and the mixer were fine — the instability was coming from the **test rig**, through two independent problems. Both are worth writing down because both are invisible in host tests and obvious only once seen. (2026-07-22)

The general lesson first, because it is the reusable part: **when a feedback loop looks unstable, confirm the inputs and the power before touching the gains.** A controller can only be as good as the sensor data it reacts to and the actuators it drives. Open-loop mode (raw stick passthrough, no feedback) is the key diagnostic — it lets you exercise the plant with the loop *broken*, so you can tell "the loop is wrong" from "the loop is fed garbage".

## Problem 1 — a hard-mounted IMU turns motor vibration into gyro noise

The IMU was bolted rigidly to the frame. Spinning motors and props vibrate that frame, and a rigid mount transmits the vibration straight into the gyroscope and accelerometer, which read it as real motion.

Why it destabilises the loop specifically: the derivative term multiplies the *rate of change* of the gyro signal (`D = KD · gyro`). Vibration is high-frequency, so the D term amplifies it far more than the slow, real attitude motion — turning sensor hash into large, jittery motor commands. The complementary filter's accelerometer term gets corrupted too (vibration is specific force), though the heavy gyro weighting (`alpha = 0.98`) hides most of that. So: rigid mount → noisy gyro → amplified by D → noisy demand → twitchy motors.

**Confirmation:** simply lifting the IMU off the frame by hand cut the demand noise dramatically. That is the smoking gun for vibration coupling — nothing in the firmware changed, only the mechanical path.

**Fix:** soft-mount the IMU (foam, gummy standoffs, vibration-isolating tape) — firmly retained so it can't flop (low-frequency motion the estimator *would* track) but not metal-to-frame rigid. A soft mount is a **mechanical low-pass filter**: the compliant material attenuates the high-frequency shake before it reaches the sensor, exactly like an electrical RC filter attenuates high frequencies before they reach an ADC. A **gyro low-pass filter** in firmware (either the ICM-42688's on-chip anti-alias/UI filters, or a software first-order LPF) is the complementary fix for the residual, and de-fangs the D-term amplification directly — at the cost of some phase lag to trade off. Balancing the props attacks the source.

Every real flight controller is soft-mounted for this reason; no gyro survives being hard-coupled to a spinning-motor frame.

## Problem 2 — a bench PSU cannot power motors (regen and brownout)

The second tell was the bench power supply's relays clicking constantly and the ESC repeatedly rebooting. Motors are a brutal load for a lab supply, for four reasons:

1. **Peak current** — inrush and throttle steps demand large, spiky currents a bench supply often cannot source.
2. **Foldback current-limiting** — at the limit it collapses its *voltage* rather than holding it, browning out the ESC (you hear the ESC's startup tones as it resets).
3. **Slow transient response** — the regulation loop sags the rail on a fast load step before it catches up.
4. **It cannot *sink* current** — this is the one that clicks the relays. A decelerating motor makes the ESC **regenerate** current back into the supply. A battery absorbs it; a PSU cannot, so the rail voltage spikes up, trips over-voltage protection, and the output relay cycles. Every throttle-down can trigger it.

So on each throttle change the rig either browns out (sag) or trips OVP (regen spike), the ESC resets, and the whole system is disturbed — indistinguishable from a "screwey" control loop until you notice the *supply* is the thing misbehaving.

**Fix:** power the motors from the **LiPo** (the flight source anyway) — low source impedance, huge transient current on tap, and it *absorbs* regen so there is no OVP trip. **Stopgap** if the battery is not to hand: a big low-ESR electrolytic capacitor across the ESC power input (a local energy reservoir that sources transients and soaks regen spikes; rate it well above the rail voltage), raise the PSU current limit so it does not foldback, keep throttle changes gentle (slow decel = less regen), and optionally a power resistor bled across the rail to dissipate regen. Even then a bench PSU is only good for gentle, low-throttle observations.

## Takeaways

- **Isolate the plant from the loop.** An open-loop / manual mode is worth building early purely as a diagnostic: it proved the mixer and motor mapping were correct and pointed the finger at the rig.
- **Plot the raw sensor inputs, not just the outputs.** The added gyro/accel/demand telemetry is what made "the inputs are noisy" visible; without it the instability is just a black box.
- **Change one thing at a time.** Vibration and power were two independent gremlins stacked on top of each other (and a hard-coded receive buffer was a third, unrelated one found the same evening). Chasing them together, or blaming the newest code, would have wasted hours.
- **The newest code is not automatically the culprit.** The PID was written last, so it drew suspicion first — but it was correct; the bench rig around it was not.
