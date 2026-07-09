# Drone Project — Documentation

This directory holds the design notes, decisions, and references for the Drone project.

## Structure

Domain docs are numbered `0N-topic.md`. Process / tooling docs are unnumbered and live alongside them. Anything marked _(not yet written)_ is a planned slot, not a stub.

- `README.md` — this file; index and conventions.
- `00-vision.md` — high-level goals: what we're building, why, and the phase plan.
- `progress.md` — reverse-chronological log of notable milestones (the dated history the README used to carry).
- `01-requirements.md` — functional and non-functional requirements. _(not yet written)_
- `02-architecture.md` — system architecture, components, data flow.
- `03-hardware.md` — hardware choices, sensors, actuators, wiring. _(not yet written)_
- `04-software-stack.md` — languages, frameworks, libraries, build setup. _(not yet written)_
- `05-communication.md` — RF link, USB, message formats, framing. _(not yet written)_
- `06-control.md` — flight control, PID tuning, sensor fusion notes. _(not yet written)_
- `07-safety.md` — failsafes, arming, battery monitoring, bench discipline. **Prerequisite for Phase 3 free flight.**
- `ci-and-testing.md` — what is tested where, how to run the suite, and the pre-push hook (enable once per clone with `git config core.hooksPath .githooks`).
- `dev-environment.md` — toolchain setup, `probe-rs`, board labelling, two-board workflow. Windows-specific notes included.
- `decisions/` — Architecture Decision Records (ADRs), one file per decision.
- `learning/` — short notes on things I did not know before starting this project (Rust, Embassy, ARM, embedded conventions). See [learning/README.md](learning/README.md).
- `research/` — external references, datasheets, links, notes. See [research/README.md](research/README.md) for what belongs there.

Mechanical CAD files (`.f3d` source + print-ready `.stl` / `.3mf`) live under [`hardware/mechanical/`](../hardware/mechanical/) ([ADR 0006](decisions/0006-mechanical-cad-fusion360.md)). Electrical / PCBA design files live under [`hardware/electrical/`](../hardware/electrical/) (Phase 4 onwards). Rust crates live under [`crates/`](../crates/).

## Conventions

- Markdown for all docs.
- Use ADRs in `decisions/` for any non-trivial choice (format: `NNNN-title.md`).
- Keep entries dated; append rather than overwrite when capturing evolving thinking.
- Link to source files in the repo using relative paths.
- Writing style is engineering documentation, not blog prose. See `AGENTS.md` Writing style for the rules.

## Status

Vision and architecture agreed. Twenty-one ADRs in place ([decisions/](decisions/README.md)). Phase 1 bring-up in progress: firmware boots, LED heartbeat runs, BSP layer in place. Active backlog lives in [GitHub Issues](https://github.com/neilpate/Drone/issues) with a kanban view at [project #5](https://github.com/users/neilpate/projects/5) — see [ADR 0011](decisions/0011-task-tracking-issues-and-batches.md). Next firmware work: FICR.DEVICEID boot banner, board-ID refuse-to-arm, then ICM-42688 SPI bring-up once the breakout arrives.
