# AGENTS.md — Assistant Context File

This file is the **shared memory** for any AI coding assistant (GitHub Copilot, Claude, Cursor, etc.) working on this repo. It lives in the repo so it survives PC changes and assistant switches.

Most modern assistants automatically read `AGENTS.md` (and/or `.github/copilot-instructions.md`, `CLAUDE.md`, `.cursorrules`) at the repo root. Keep this file as the canonical source; the others, if needed, should just point here.

---

## How to use this file

- **Read this first** at the start of any session before doing work.
- **Update it** whenever a decision is made, a convention is set, or a non-obvious fact is learned.
- Keep entries short. Link out to `Doc/` for long-form content.
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
- Use **[Conventional Commits](https://www.conventionalcommits.org/)**: `type(scope): subject` — e.g. `docs(adr): add 0003 IMU selection`, `feat(imu): read WHO_AM_I over SPI`, `fix(spi): correct CPOL for ICM-42688`.
  - Common types: `feat`, `fix`, `docs`, `refactor`, `chore`, `test`, `build`, `ci`, `perf`.
  - Scope is optional but encouraged (`adr`, `imu`, `spi`, `hal`, `docs`, etc.).
  - Subject: imperative mood, lowercase, ≤72 chars, no trailing period.
  - Body (optional) explains the *why* when not obvious. Separate from subject by blank line.
  - Breaking change: `feat(api)!: ...` or `BREAKING CHANGE:` footer.
- Don't auto-commit on the user's behalf unless explicitly asked. Suggest a commit (with a Conventional-Commits-formatted message) when a sensible boundary is reached.
- Never `--force` push, never amend a pushed commit, never `reset --hard` without asking.

---

## Project: Drone

**Status:** Vision agreed (2026-05-21). Hardware/language baseline chosen (2026-05-21). IMU chosen (2026-05-21). Phase plan redefined (2026-05-23, hardware-build oriented; see [Doc/00-vision.md](Doc/00-vision.md)). Ready to start Phase 1 bring-up once parts arrive.

### Scope guardrail (read every time)

This is a **hobby learning project**, not a commercial-grade flight controller. When weighing options, bias toward:

- What teaches the most per unit of effort, not what flies best.
- Staying in the **nRF52 / nRF53 family** for hardware continuity. The agreed migration path is **micro:bit v2 (Phases 1–3) → custom nRF5340 PCBA, designed in-house (Phases 4–5)**. Do not propose STM32H7 / ELRS / "real FC" upgrades unless the user explicitly asks.
- One-off build, cost-insensitive within reason — but "commercial-grade" is not a target. "Good enough to learn from and to make a quad that flies in a garden" is.
- The drone is the **vehicle for learning** ([00-vision.md](Doc/00-vision.md)). Understanding the stack is the deliverable; flight performance is a side effect.
- **Video downlink (post-Phase 5)** is intended to be **analog FPV** — orthogonal payload, no FC firmware impact. Do not propose digital HD systems (DJI O3, HDZero) or DIY WiFi streaming as upgrades unless asked.

**Locked-in choices** (see [ADR 0001](Doc/decisions/0001-platform-airframe-stack.md), [ADR 0002](Doc/decisions/0002-mcu-and-language.md), [ADR 0003](Doc/decisions/0003-imu-icm42688-spi.md), [ADR 0004](Doc/decisions/0004-concurrency-embassy-channels.md)):
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

**Documentation lives in:** [Doc/](Doc/README.md)
**Architecture Decision Records:** `Doc/decisions/` (one file per decision, format `NNNN-title.md`).

---

## Conventions

- Markdown for all docs.
- Use ADRs for any non-trivial design choice.
- Keep the doc index ([Doc/README.md](Doc/README.md)) in sync when adding new doc files.

---

## Decisions log (quick index)

- [0001](Doc/decisions/0001-platform-airframe-stack.md) — Real-hardware quadcopter, roll our own firmware, learning-first scope. (2026-05-21)
- [0002](Doc/decisions/0002-mcu-and-language.md) — BBC micro:bit v2 (nRF52833) + Rust for Phases 1–3. (2026-05-21, amended by 0004)
- [0003](Doc/decisions/0003-imu-icm42688-spi.md) — External IMU: ICM-42688-P on SPI. (2026-05-21)
- [0004](Doc/decisions/0004-concurrency-embassy-channels.md) — Concurrency model: Embassy + channel-based actor pattern, no BSP. (2026-05-22)

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
