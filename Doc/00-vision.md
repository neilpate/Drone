# 00 — Vision

_Date: 2026-05-21_

## What we're building

A **working quadcopter built from scratch**, on real hardware, running firmware we wrote ourselves.

## Why

The drone is the **vehicle for learning**, not the end product. The goal is to understand the *entire* stack end-to-end, from electrons to autonomous behaviour, by building each layer ourselves rather than configuring an existing one.

Concretely, by the time we're done we want to deeply understand:

- **Sensors** — how an IMU works, what raw gyro/accel data actually looks like, why it needs filtering.
- **Sensor fusion** — how noisy 3-axis gyro + accel data becomes a stable orientation estimate (complementary filter → Mahony → maybe EKF).
- **Control theory in practice** — PID loops, why tuning is hard, why cascaded loops (rate → angle) exist.
- **Motor mixing** — how three desired torques + a thrust command become four motor outputs.
- **Real-time embedded systems** — interrupt-driven I/O, control-loop timing, why jitter kills stability.
- **Radio link** — RC protocols, packet framing, failsafes.
- **Power & electronics** — LiPo chemistry, ESCs, current draw, regulation.
- **Safety engineering** — what failsafes matter and why.

## Why roll our own (not PX4 / ArduPilot)

Using an existing stack would get a drone flying much faster — but the inner loops, sensor drivers, and fusion code would remain a black box. Since *understanding* is the deliverable, we accept the cost of rebuilding what already exists in exchange for genuinely owning every line of code in the flight loop.

This is the same trade-off as writing your own kernel vs. using Linux: nobody does it because it's economical, they do it to learn how kernels actually work.

## Quality bar

This is a hobby project, but it doubles as a public showcase of the author's (and assisting AI's) software-engineering practice. "It's just a hobby project" is not a valid excuse to skip a test, leave a TODO, or commit a half-baked decision. Concretely:

- Engineering choices target current best practice for embedded Rust (Embassy, `probe-rs`, `defmt`, `embedded-test`, `rerun`, ADR-driven design).
- The code, the docs, the commit history, and the ADRs are all written as if a stranger will read them — because they will.
- Phase boundaries are also showcase moments: a tagged commit, a short write-up, a photo or video where it makes sense.
- Shortcuts are taken consciously and recorded, never silently.

## Non-goals (explicit)

To keep scope from exploding, we are explicitly **not** trying to:

- Build something competitive with commercial / hobby-grade flight controllers.
- Achieve full autonomy (waypoint missions, SLAM, computer vision) — at least not in the first arc.
- Produce a polished product.
- Reinvent radio-link physical layer (we'll use an off-the-shelf RX module).
- Reinvent the ESC protocol (we'll speak existing standards like DShot).

We're writing the **firmware** — the part that turns sensor data into motor commands. We're using off-the-shelf parts for everything below that line (motors, ESCs, RX, frame, battery).

## Definition of success

The project is split into hardware-build phases, smallest-first. Each phase has a drone-side build state, a drone-side firmware milestone, and a parallel ground-station milestone. Don't move on until the previous phase is solid.

| Phase | Drone build state | Drone firmware milestone | Ground-station milestone |
|---|---|---|---|
| **1 — Initial prototyping** | micro:bit on bench, ICM-42688 wired up, **one motor + ESC on a clamp**, bench PSU (no battery), hard-tethered to desk | blinky → IMU sample over SPI → **Mahony fusion** → single-motor closed-loop response to tilt, simple PWM to one ESC | second micro:bit + PC, USB serial bridge, PS controller read on PC, plot raw IMU + fused attitude on PC, send single throttle command |
| **2 — Advanced prototyping** | micro:bit on bench, **4 motors + ESCs**, 3D-printed rigid mount (not flight-frame), bench PSU, hard-tethered | full motor mixer, **DShot** instead of PWM, four-motor response to tilt (still bench, no thrust), basic failsafe (cut throttle on link loss) | controller commands all 4 channels (throttle / roll / pitch / yaw), telemetry shows all 4 motor outputs |
| **3 — Initial flights** | **flight hardware selected and committed**: frame class, motors, ESCs, propellers, LiPo battery, RX. Lighter 3D-printed flight frame, **battery on board**, **safety tether** (catch line, not power) | first hover attempts, PID tuning, more aggressive failsafe (e.g. land mode), in-air diagnostics | telemetry over RF only (no USB to drone), arming / disarming UX |
| **4 — New hardware bring-up** | **custom PCBA with nRF5340 module**, designed around the Phase 3 battery / frame / motor / ESC choices (not new selections). Lightest 3D-printed frame, hard-tethered, **bench PSU for first power-on** (battery comes back once the regulator and current draw are validated) | port Embassy + actor code from micro:bit, validate every subsystem on new hardware (no flight) | unchanged from Phase 3 in shape; now talking to new MCU. Air protocol unchanged. |
| **5 — First flight on new hardware** | nRF5340 PCBA, flight frame, battery, safety tether | re-tune PIDs for new dynamics, replicate Phase 3 capabilities on new hardware | unchanged |

### Beyond Phase 5

Explicitly out of scope for the first arc, but recorded so the design doesn't accidentally rule them out:

- **Analog FPV camera + 5.8 GHz VTX.** Orthogonal payload: a tiny analog camera and 5.8 GHz video transmitter sit on the airframe and use their own 5.8 GHz radio link to a standalone receiver. The flight controller does not see them. Drives a small weight / mount budget on the Phase 4 PCBA and frame, but no firmware impact.
- **Altitude hold / position hold / autonomy.** Would require either a barometer (altitude) or GPS + optical flow (position), plus a much bigger fusion stack (EKF). Not on the path; not ruled out.

## Approach principles

- **Build the test rig before the drone.** A tethered / propeller-less bench setup is what makes iteration safe and fast.
- **Instrument everything.** Stream data out over serial / USB / WiFi from the start. Tuning blind is impossible.
- **One variable at a time.** Standard engineering discipline — even more important when something can take your fingers off.
- **Phased commits.** Don't try to write the EKF before the complementary filter works.

## Open questions

Resolved (see `Doc/decisions/`):

- MCU and dev board — micro:bit v2 (nRF52833) for Phases 1–3, custom nRF5340 PCBA for Phases 4–5. ADR 0002.
- Firmware language — Rust, `no_std`. ADR 0002.
- IMU part number — ICM-42688-P on SPI. ADR 0003.
- Concurrency / HAL — Embassy + channel-based actor pattern, no BSP. ADR 0004.
- PC-side software language — Rust, with a shared `proto` crate for the wire protocol. ADR 0005.

Still open (each will get its own ADR when resolved):

- Frame class (size / weight) — drives motor / prop / battery selection. Needed by Phase 3.
- Radio link — second micro:bit + ESB covers Phases 1–2; longer-term choice still open.
- Wire framing / encoding between drone and PC (postcard, COBS-framed bincode, JSON for bring-up, …).
- PC-side GUI / plotting framework — strong candidate is [`rerun.io`](https://rerun.io) (Rust-native time-series + 3D visualisation, designed for robotics telemetry). `egui` + `egui_plot` is the fallback. Not locked in.
- Failsafe behaviour — must be settled before Phase 3 free flight.
- Custom PCBA design (Phase 4) — nRF5340 module choice, carrier-board layout, power tree.
