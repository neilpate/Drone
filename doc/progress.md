# Progress log

A reverse-chronological log of notable milestones. The [README](../README.md) reflects only the current state; this file keeps the dated history so the front page stays uncluttered.

## 2026-06-21 — Pilot roll/pitch/yaw, gamepad sticks, four-axis plot

The pilot command grew from throttle-only to full attitude. `firmware-types` gained `Roll`, `Pitch` and `Yaw` — normalised −1..+1 deflection newtypes that scrub NaN and clamp at the wire boundary like `Throttle`, kept as three parallel hand-written types rather than a macro (per [ADR 0016](decisions/0016-newtype-per-physical-quantity.md)). They ride in both `PilotCommand` and `GroundstationCommand`; the remote decodes all four axes from the ground-station frame and fans them into per-axis signals, and the drone echoes them back in telemetry. On the PC, the `groundstation` gained sliders for the three new axes and a PlayStation gamepad mapping: Mode 2 sticks (left X → yaw, right X → roll, right Y → pitch) with throttle staying on the right trigger so a self-centring stick can't sit at half-throttle. The live plot now shows throttle, roll, pitch and yaw; the internal-temperature trace was dropped as no longer useful. Frames (body FRD, world NED), sign conventions and the raw-deflection command model were settled first in [ADR 0021](decisions/0021-coordinate-frames-and-command-semantics.md). Closes [#13](https://github.com/neilpate/Drone/issues/13).

![Ground station live plot: throttle, roll, pitch and yaw traces driven by a PlayStation gamepad, alongside the matching on-screen sliders.](images/groundstation%202.png)

## 2026-06-20 — Host test suite brought under a guard

Feature work paused to close the testing gap before it grew. The host-testable crates are now all reachable from a bare `cargo test`: `firmware-types` joined `firmware-drone-core` and `firmware-remote-core` in the workspace `default-members`, which immediately surfaced (and fixed) a stale `TelemetryState` test referencing a removed variant — exactly the value of the change. `Temperature` gained the round-trip and wire-deserialize tests it was missing, mirroring the `Throttle` pattern. A tracked [`pre-push` hook](../.githooks/pre-push) now runs the whole host suite (workspace default-members + the out-of-workspace `groundstation`) before any push, so a green `main` no longer depends on remembering to run the tests. The runner is [cargo-nextest](https://nexte.st/), which collapses the per-binary output into one aggregated `N tests run: N passed` summary. The how-to lives in [doc/ci-and-testing.md](ci-and-testing.md). Still open: the `groundstation` pure helpers (state mapping, COBS framing, gamepad clamp) want extracting out of `main.rs` so they can be tested too.

## 2026-06-20 — Full round trip, live plot, 100 Hz, gamepad input

The loop is closed and visible. Telemetry now flows all the way back to the PC — the remote's `serial_link_tx` task forwards each `TelemetryState` out the UART using postcard + COBS framing ([ADR 0018](decisions/0018-pc-link-uart-postcard-cobs.md), now in real use, not just throttle), and the `groundstation` decodes it and plots throttle, temperature and drone-state as live time series with [`egui_plot`](https://github.com/emilk/egui_plot) (per-signal show/hide, clear, latest-frame readout). A dedicated `telemetry_aggregator` task on the drone is the sole publisher of `TelemetryState`, tick-sampling its sources and stamping the sequence number ([ADR 0020](decisions/0020-telemetry-aggregator-single-publisher.md)). Cadence pushed to **100 Hz** end-to-end — aggregator, radio round-trip and UART all run on a 10 ms tick (UART ~18 % utilised), and the plotted throttle is butter-smooth despite the value crossing UART → IEEE 802.15.4 → drone → aggregator → radio → remote → UART on every frame. Finally, the throttle source can now be a **Bluetooth gamepad**: the groundstation reads it via [`gilrs`](https://gitlab.com/gilrs-project/gilrs) and maps the right trigger (analog 0.0–1.0) onto the throttle the slider used to own.

![Ground station live plot: a smooth throttle trace alongside temperature and drone-state time series.](images/groundstation%201.png)

The smooth blue throttle trace above is doing more work than it looks: every sample on that line left the PC as a gamepad/slider value, crossed USB-CDC → UART → IEEE 802.15.4 to the drone, was sampled by the aggregator, then flew back drone → remote → UART → PC before being plotted — all at 100 Hz.

## 2026-06-14 — Telemetry pipeline drone → remote

First payload flowing the other way. A new `telemetry` task on the drone reads the nRF52833 internal TEMP sensor at 2 Hz, converts the `I30F2` fixed-point reading to `f32` °C, and broadcasts a `TelemetryState` (sequence number + temperature) on a `Watch`. `remote_link` subscribes and ships the latest sample back to the remote in the radio reply to each `PilotCommand`, so the previous hard-coded placeholder telemetry is gone. Half of the round-trip chain is real now; forwarding the value out the remote's UART to the groundstation, then plotting it, is next. Alongside the feature: per-role state enums (`DroneState`, `RemoteState`) promoted into `firmware-types`, and the per-firmware signals layout settled into one-file-per-signal under `crates/firmware-<role>/src/signals/<name>.rs` so signal modules never define wire types inline.

## 2026-06-09 — PC ground-station controlling motor speed

First end-to-end pilot input. A new host-only `groundstation` crate (binary `gs`) opens an [egui](https://github.com/emilk/egui) window with a single throttle slider; dragging it sends the value as ASCII over USB-CDC at 115 200 8N1 to the remote micro:bit. On the remote, `serial_link` parses the line and publishes a `Throttle` to a `Watch`; `drone_link` subscribes and replaces its self-incrementing test counter with the live slider value. Plain ASCII for the first cut — the postcard + COBS framing described in [ADR 0018](decisions/0018-pc-link-uart-postcard-cobs.md) lands when telemetry needs it. egui chosen as a deliberately low-friction GUI: immediate-mode, built-in `Slider`, no Elm-architecture or web-toolchain to learn; we did not go deep on it on purpose.

## 2026-06-02 — Supervisor failsafe wired in

The `supervisor` task now sits between `remote_link` and `motor_controller` and is the sole publisher of motor commands ([ADR 0017](decisions/0017-supervisor-failsafe-state-machine.md)). Loss-of-link is detected when no `PilotCommand` arrives for 100 ms (10 ticks of a 10 ms ticker), at which point the supervisor publishes a zero-throttle `MotorCommand` and flags `SystemState::Degraded`. Verified on the bench: powering off the remote stops the motor within ~100 ms. New host-testable crate `firmware-drone-core` exists for the pure state-machine logic; the actual `Initialising`/`Armed`/`Degraded`/`Fault` machine and ramp-down behaviour are next.

## 2026-06-01 — End-to-end throttle control on hardware

The remote sweeps a `Throttle` value, ships it inside a `PilotCommand` over the IEEE 802.15.4 link ([ADR 0014](decisions/0014-radio-protocol-ieee802154.md)), the drone's `remote_link` task republishes via `Watch<PilotCommand>` ([ADR 0013](decisions/0013-async-communication-primitives.md)), and the `motor_controller` task drives a brushed motor through the L9110 H-bridge via the nRF PWM peripheral. First shared wire types live in `firmware-types`, which is also the first crate to carry host unit tests — postcard round-trip plus a custom `Deserialize` that enforces the `Throttle` 0..=1 invariant at the wire boundary (the bench surfaced a real garbage-packet hard fault and the test pins the fix in place). Test-wiring pattern recorded in [ADR 0015](decisions/0015-host-testing-no-std-crates.md).

## Earlier in Phase 1 — bring-up

Cargo workspace bootstrapped, Embassy-based firmware booting and logging over RTT (`defmt`), board-support-package layer in place ([ADR 0010](decisions/0010-board-support-package.md)), source-level debugging working from VS Code via the probe-rs DAP adapter.
