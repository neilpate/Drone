# AGENTS.md — Assistant Context File

This file is the **shared memory** for any AI coding assistant (GitHub Copilot, Claude, Cursor, etc.) working on this repo. It lives in the repo so it survives PC changes and assistant switches.

Most modern assistants automatically read `AGENTS.md` (and/or `.github/copilot-instructions.md`, `CLAUDE.md`, `.cursorrules`) at the repo root. Keep this file as the canonical source; the others, if needed, should just point here.

---

## How to use this file

- **Read this first** at the start of any session before doing work.
- **Update it** whenever a decision is made, a convention is set, or a non-obvious fact is learned.
- Keep entries short. Link out to `doc/` for long-form content.
- Date significant additions: `(YYYY-MM-DD)`.

---

## About the user

- New to web tech (HTML/CSS/JS/TS, HTTP, WebSocket).
- Experienced with **systems programming**: raw TCP, C-level / low-level work, general programming.
- Prefers understanding the **"why"** before the "how". Explain concepts, then implement.
- When introducing web/high-level concepts, compare to systems-programming equivalents.
- Likes **structure** — docs, ADRs, clear layout before code.

## Working style

- Be brief. No filler.
- Don't over-engineer. Only do what's asked or clearly necessary.
- Don't create files unless needed.
- Don't add comments / docstrings / type hints to code you didn't change.
- Implement rather than just suggest, unless ambiguity requires a question.
- **Hold the showcase quality bar.** This is a hobby project but also a public showcase. "It's just a hobby project" is not a valid excuse to skip a test, leave a TODO, or take an undocumented shortcut. Engineering choices target current best practice for embedded Rust. See [doc/00-vision.md](doc/00-vision.md) “Quality bar”.
- **`main` stays `rustfmt`-clean and `clippy`-clean.** Every commit satisfies `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` against the active board feature. Suppressions (`#[allow(...)]`, `#[rustfmt::skip]`) require a justifying comment on the line above. See [ADR 0012](doc/decisions/0012-lint-and-format-policy.md).
- **Prefer the idiomatic choice.** Where practical, do what someone fluent in the ecosystem would expect — Cargo conventions, Embassy patterns, standard crate / module / file naming, standard layouts. A stranger landing in the repo should not be surprised by *how* anything is done. Surprises are reserved for problem-specific decisions (recorded in an ADR), not for picking a non-standard layout, naming scheme, or pattern when a standard one fits.

## Writing style (docs, ADRs, commit messages, replies)

- Keep it professional. This is engineering documentation, not a blog post.
- No casual section headings: avoid `TL;DR`, `tl;dr`, `In a nutshell`, `The gist`, etc. Use `Summary`, `Overview`, or just lead with the point.
- No emojis unless the user explicitly asks.
- No marketing tone ("blazing fast", "powerful", "seamless").
- No exclamation marks in docs/ADRs.
- Em-dashes, plain prose, and short sentences are fine and preferred over chatty hedging.
- When summarising trade-offs, state them plainly; don't soften with "it's worth noting that\u2026" filler.

## Code-writing boundary (important)

- **The user writes the code.** This is a learning project — writing it themselves is the point.
- Assistant's job: docs, ADRs, explaining concepts, reviewing code, diagnosing bugs, pointing to references.
- Do **not** scaffold projects, write `Cargo.toml`s, or produce implementation code unless the user explicitly asks for it.
- Code snippets in *explanations* are fine when illustrating a concept. Writing the actual firmware is not.

## Git / commit discipline

- Commit **often**, like a senior dev — not after every keystroke, but at every logical, reviewable unit of work (a decision recorded, a feature working, a refactor done).
- Don't sit on a large pile of unrelated changes.
- Use **[scoped commits](https://scopedcommits.com/)** (the Linux / Git / Go style): `scope: subject` — e.g. `adr: add 0003 IMU selection`, `imu: read WHO_AM_I over SPI`, `spi: correct CPOL for ICM-42688`. No `type` prefix (no `feat:`/`fix:`/`chore:`).
  - The **scope is the area of the codebase touched** — the subsystem, crate, or component — not a change type. It is the most important part of the subject and is **required**, not optional. Natural scopes here: `imu`, `spi`, `supervisor`, `telemetry`, `groundstation`, `firmware-types`, `hal`, `board`, `adr`, `doc`, `readme`, `ci`.
  - The *type* of change (fix, feature, refactor) should be obvious from the subject's wording, so we don't encode it. If you can't tell which scope a commit belongs to, it's probably doing too much — split it.
  - Subject: imperative mood, lowercase after the scope, ≤72 chars, no trailing period.
  - Body explains the *why* (motivation, and why this solution) when not obvious — favour writing one. Separate from subject by a blank line.
  - For a change spanning two clear areas, pick the dominant scope or use a combined prefix (e.g. `groundstation,readme:`); if it genuinely spans many, that's a sign to split the commit.
- Don't auto-commit on the user's behalf unless explicitly asked. Suggest a commit (with a scoped-commit message) when a sensible boundary is reached.
- Never `--force` push, never amend a pushed commit, never `reset --hard` without asking.

## End-of-session ritual

- When the user signals end of session ("done for today", "good stopping point", etc.), update the **Status** section of [README.md](README.md) to reflect the latest progress: current phase, what's working on hardware, what's next. Then commit + push along with any other end-of-session work.
- Whenever the README **Status** gains a new milestone, also append a dated entry for it to [doc/progress.md](doc/progress.md) (reverse-chronological, newest first). The README holds only the current snapshot; `progress.md` is the append-only dated history. Any screenshot the README retires should be preserved in its `progress.md` entry.
- This keeps the front door of the repo honest for any stranger landing in cold — and for future-you on another PC.

---

## Project: Drone

**Status:** Vision agreed (2026-05-21). hardware/language baseline chosen (2026-05-21). IMU chosen (2026-05-21). Phase plan redefined (2026-05-23, hardware-build oriented; see [doc/00-vision.md](doc/00-vision.md)). Ready to start Phase 1 bring-up once parts arrive.

### Scope guardrail (read every time)

This is a **hobby learning project**, not a commercial-grade flight controller. When weighing options, bias toward:

- What teaches the most per unit of effort, not what flies best.
- Staying in the **nRF52 / nRF53 family** for hardware continuity. The agreed migration path is **micro:bit v2 (Phases 1–3) → custom nRF5340 PCBA, designed in-house (Phases 4–5)**. Do not propose STM32H7 / ELRS / "real FC" upgrades unless the user explicitly asks.
- One-off build, cost-insensitive within reason — but "commercial-grade" is not a target. "Good enough to learn from and to make a quad that flies in a garden" is.
- The drone is the **vehicle for learning** ([00-vision.md](doc/00-vision.md)). Understanding the stack is the deliverable; flight performance is a side effect.
- **Video downlink (post-Phase 5)** is intended to be **analog FPV** — orthogonal payload, no FC firmware impact. Do not propose digital HD systems (DJI O3, HDZero) or DIY WiFi streaming as upgrades unless asked.

**Locked-in choices** (see [ADR 0001](doc/decisions/0001-platform-airframe-stack.md), [ADR 0002](doc/decisions/0002-mcu-and-language.md), [ADR 0003](doc/decisions/0003-imu-icm42688-spi.md), [ADR 0004](doc/decisions/0004-concurrency-embassy-channels.md)):
- **Real hardware** (no simulator).
- **Quadcopter** airframe.
- **Roll our own firmware** from scratch — no PX4 / ArduPilot.
- **Primary goal: learning** — understand the whole stack end-to-end.
- **MCU:** BBC micro:bit v2 (nRF52833, Cortex-M4F) for Phases 1–3. Two boards owned — second one becomes ground station / remote.
- **Language:** Rust (`no_std`, `probe-rs`, `defmt`).
- **HAL / concurrency:** `embassy-nrf` (async HAL, no BSP) + `embassy-executor` + `embassy-sync`. Channel-based actor pattern — each subsystem is an `async` task with a typed `Channel` inbox.
- **External IMU:** ICM-42688-P on SPI (6-DoF, no mag for now). INT1-driven sampling.

**Next open questions:**
- Frame size / motor / ESC / battery class — not urgent until Phase 3. Likely 3″–5″ prop class to leave headroom for the Phase-6+ analog FPV payload.
- Radio link — deferred; second micro:bit covers early needs.
- Host-side telemetry tooling (Phase 1 will need it).
- **Custom PCBA design (Phase 4)** — nRF5340 module on a carrier board, KiCad, hand-rolled. Future ADR when committed.

**Backlog and task tracking:**
- Tasks live as **GitHub issues** in this repo: <https://github.com/neilpate/Drone/issues>.
- A kanban-style Projects v2 board (<https://github.com/users/neilpate/projects/5>) is a *view* over selected issues — not a separate backlog. Add an issue to the board explicitly (`gh project item-add 5 --owner neilpate --url <issue-url>`) when it's worth tracking visually.
- Labels are the cross-cutting taxonomy: `phase-1..5`, `area:firmware`, `area:hardware`, `area:doc`, `area:tooling`, etc. Filter / slice by label, not by custom Project fields.
- See closed issue [#2](https://github.com/neilpate/Drone/issues/2) for the decision rationale.

**Documentation lives in:** [doc/](doc/README.md)
**Hardware files live in:** [hardware/](hardware/README.md) (mechanical CAD under `hardware/mechanical/`, electrical / PCBA under `hardware/electrical/`).
**Rust crates live in:** [crates/](crates/README.md) (empty until Phase 1 design lands).
**Architecture Decision Records:** `doc/decisions/` (one file per decision, format `NNNN-title.md`).

---

## Conventions

- Markdown for all docs.
- Use ADRs for any non-trivial design choice.
- Keep the doc index ([doc/README.md](doc/README.md)) in sync when adding new doc files.
- **Shorthand `#N`** (e.g. `#1`, `#42`) refers to a **GitHub issue or PR** in `neilpate/Drone` unless context makes that obviously wrong. ADRs are always referenced as `ADR NNNN` or `ADR 0011`, never `#11`.

---

## Tooling gotchas

- **Windows PowerShell mangles non-ASCII in pipelines.** Em-dashes (`—`), en-dashes, curly quotes, and anything outside ASCII get corrupted into mojibake (e.g. `├ö├ç├Â`) when piped through commands like `gh issue view ... | Out-File ... | gh issue edit ...`. The console code page (CP437 / CP1252) decodes UTF-8 bytes mid-pipeline.
  - **Rule:** when round-tripping any text that might contain non-ASCII (issue/PR bodies, commit messages, doc snippets), do **not** pipe through PowerShell. Write the file directly (UTF-8, no BOM) and pass it with `gh ... --body-file <path>` or `git commit -F <path>`.
  - Pure ASCII (`-`, `--`, plain quotes) survives any pipeline.
  - Same applies to `git`, `curl`, any tool whose output may be re-fed to another tool on Windows pwsh.

---

## Decisions log (quick index)

- [0001](doc/decisions/0001-platform-airframe-stack.md) — Real-hardware quadcopter, roll our own firmware, learning-first scope. (2026-05-21)
- [0002](doc/decisions/0002-mcu-and-language.md) — BBC micro:bit v2 (nRF52833) + Rust for Phases 1–3. (2026-05-21, amended by 0004)
- [0003](doc/decisions/0003-imu-icm42688-spi.md) — External IMU: ICM-42688-P on SPI. (2026-05-21)
- [0004](doc/decisions/0004-concurrency-embassy-channels.md) — Concurrency model: Embassy + channel-based actor pattern, no BSP. (2026-05-22)
- [0005](doc/decisions/0005-pc-software-language-rust.md) — PC-side software written in Rust; shared `proto` crate for the wire protocol. (2026-05-23)
- [0006](doc/decisions/0006-mechanical-cad-fusion360.md) — Mechanical CAD: Fusion 360; commit `.f3d` source + `.stl` mesh. (2026-05-23, amended 2026-07-09 — STEP export dropped)
- [0007](doc/decisions/0007-testing-and-ci-strategy.md) — Testing and CI: unit-test everything possible, local-first feedback, `core`/`task` split, HIL deferred. (2026-05-23)
- [0008](doc/decisions/0008-repository-folder-layout.md) — Repository folder layout: `crates/`, `doc/`, `hardware/{mechanical,electrical}/`, all lowercase. (2026-05-23)
- [0009](doc/decisions/0009-workspace-bootstrap-and-crate-naming.md) — Workspace bootstrap from day one; `firmware-<role>` naming; `core`/`task` split realised as sibling crates. (2026-05-23)
- [0010](doc/decisions/0010-board-support-package.md) — Board Support Package (BSP) layer: `board` module inside `firmware-drone`, Cargo-feature-selected, tasks take erased types. (2026-05-23)
- [0011](doc/decisions/0011-task-tracking-issues-and-batches.md) — Task tracking: GitHub Issues as canonical backlog, Projects board as view, labels as taxonomy, batched filing (no upfront enumeration, no time-boxing). (2026-05-24)
- [0012](doc/decisions/0012-lint-and-format-policy.md) — Lint and format policy: `main` stays `rustfmt`-clean and `clippy`-clean; suppressions require a justifying comment. (2026-05-24)
- [0013](doc/decisions/0013-async-communication-primitives.md) — Async inter-task communication: `Channel` for commands (per ADR 0004), `Watch` for multi-observer state, `Signal` for single-observer state, `PubSubChannel` for multi-consumer event streams. 2×2 rule: state vs events × one vs many. Shared-state types live in `firmware-drone-core`. (2026-05-24)
- [0014](doc/decisions/0014-radio-protocol-ieee802154.md) — Radio link protocol: IEEE 802.15.4 (raw PHY/MAC) via embassy-nrf's `ieee802154::Radio`, channel 20, no higher-layer stack. HFCLK must be external xtal for any 2.4 GHz use. (2026-05-27)
- [0015](doc/decisions/0015-host-testing-no-std-crates.md) — Host-testable `no_std` crates: `#![cfg_attr(not(test), no_std)]`, inline `#[cfg(test)] mod tests`, dev-deps pinned directly when target/host features diverge, `cargo test` (no `--workspace`) honours `default-members`. (2026-06-01)
- [0016](doc/decisions/0016-newtype-per-physical-quantity.md) — Newtype per physical quantity for shared types: distinct newtypes (not type aliases, not a shared `PercentageValue` base) so the type system catches argument-order bugs; macro acceptable once ≥3 structurally-identical newtypes exist. (2026-06-01)
- [0017](doc/decisions/0017-supervisor-failsafe-state-machine.md) — Supervisor task as failsafe state machine: 4-state enum (`Initialising`/`Armed`/`Degraded`/`Fault`), tick-driven, pure logic in new `firmware-drone-core` crate, supervisor is the sole publisher of motor commands (republishes a `Watch<Throttle>` that `motor_controller` subscribes to). (2026-06-02)
- [0018](doc/decisions/0018-pc-link-uart-postcard-cobs.md) — PC ground-station link: USB-CDC virtual COM port → UART on nRF52833 (TXD `P0_06`, RXD `P1_08` on micro:bit v2), 115 200 8N1, postcard + COBS framing via `postcard::accumulator`, shared `firmware-types` types, new `crates/groundstation/` (binary `gs`). Initial direction PC → remote only; telemetry deferred. (2026-06-05, Proposed) (2026-06-02)
- [0019](doc/decisions/0019-airframe-class-3in-4s-printed.md) — Airframe and propulsion class: 3" ducted cinewhoop, 4S LiPo (450–650 mAh, XT30), 1507-class motors @ ~3500–3800 KV, DShot 4-in-1 ESC ~25–35 A, fully 3D-printed modular frame in PETG. Class-level decision; specific part numbers and frame geometry deferred to ordering / CAD time. (2026-06-14, Proposed)
- [0020](doc/decisions/0020-telemetry-aggregator-single-publisher.md) — Telemetry aggregator: a dedicated task is the sole publisher of `TelemetryState`, assembling it by tick-driven sampling (`Watch::get()` at 10 ms / 100 Hz, matched to the radio round-trip) of per-source `Watch`es; each producer publishes only its own slice and the aggregator owns frame-level fields (`sequence_number`). Fan-in dual of ADR 0017's single-publisher motor-command rule; closes 0017's telemetry-of-supervisor-state open item. (2026-06-20)
- [0021](doc/decisions/0021-coordinate-frames-and-command-semantics.md) — Coordinate frames, attitude sign conventions, and pilot-command semantics: world NED + body FRD (both right-handed, coincide level+north); right-hand sign rules (+roll right-down, +pitch nose-up, +yaw nose-right), 3-2-1 Euler order; angle (self-levelling) mode first; remote sends raw normalised stick deflections (−1..+1, throttle 0..1) and the drone interprets per mode (mode-independent wire); `PilotCommand` gains roll/pitch/yaw deflection newtypes (per ADR 0016). Control law, arm/mode channel, and tuning values deferred. (Proposed) (2026-06-21)
- [0022](doc/decisions/0022-attitude-estimation-complementary-filter.md) — Attitude estimation: complementary filter for roll and pitch. Fixed-gain blend (low-pass accel, high-pass gyro); roll/pitch only — yaw stays rate-controlled (no mag, no heading observability). Frames/signs per ADR 0021; accel angles via `atan2`, radians internally. Nominal fixed `dt` (1 ms, crystal-paced) justified over measured `dt`/INT1 for now. Gyro bias zeroed at startup. Pure stateful filter in `firmware-drone-core` (host-tested), driven by a single-publisher estimator stage owning an attitude `Watch` of angle newtypes (per ADR 0016/0017/0020). Not Kalman/Madgwick yet; control law still the `06-control.md` slot. (Proposed) (2026-06-27)
- [0023](doc/decisions/0023-motor-numbering-layout-rotation.md) — Motor numbering, layout, and rotation directions: symmetric quad-X, Betaflight numbering (M1 rear-right, M2 front-right, M3 rear-left, M4 front-left) matching the `Motor1..4` enum and PWM0 ch0..3 / P0.10,P0.09,P0.12,P0.02. Props-out rotation (M1&M4 CCW, M2&M3 CW; adjacent-opposite/diagonal-same). Derives the mixer sign table from ADR 0021 signs (+roll=right-down, +pitch=nose-up, +yaw=nose-right) — to be bench/flight-verified. Direction set/corrected in AM32, not by re-soldering. Layout already in firmware; mixer not yet implemented (motor_controller mirrors throttle to all four). (Proposed) (2026-07-18)

---

## Glossary / gotchas

- **IMU** — Inertial Measurement Unit: accelerometer + gyroscope (often + magnetometer). Our primary sense of orientation.
- **ESC** — Electronic Speed Controller. Takes a signal (PWM / DShot) and drives a brushless motor. One per motor.
- **PID** — Proportional / Integral / Derivative controller. The classic inner-loop control algorithm for attitude.
- **Attitude** — orientation (roll / pitch / yaw). "Attitude control" = keeping the craft level / pointed where commanded.
- **Body frame vs world frame** — body: axes fixed to the drone (forward/right/down). World: fixed to the ground. Sensor data is body-frame; navigation is world-frame; conversions matter and are an endless source of bugs.
- **NED vs ENU** — two conventions for the world frame. NED = North-East-Down (aerospace). ENU = East-North-Up (ROS / robotics). Pick one and stick to it; document it.
- **Quaternion** — 4-number representation of orientation, avoids gimbal lock. Sign / order conventions (w,x,y,z vs x,y,z,w) vary by library — always check.
- **DShot** — digital ESC protocol; replaces old analog PWM. Cleaner, more accurate, supports telemetry.
- **ELRS (ExpressLRS)** — modern open-source long-range radio link protocol. Current community standard.
- **MAVLink** — telemetry / command protocol used by PX4 / ArduPilot. We probably *won't* use it (we're rolling our own), but worth knowing it exists.
- **Bench test / tether** — running the drone with props off, or tied down, before free flight. Non-negotiable safety step.
