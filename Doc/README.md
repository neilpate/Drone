# Drone Project — Documentation

This directory holds the design notes, decisions, and references for the Drone project.

## Structure

Domain docs are numbered `0N-topic.md`. Process / tooling docs are unnumbered and live alongside them. Anything marked _(not yet written)_ is a planned slot, not a stub.

- `README.md` — this file; index and conventions.
- `00-vision.md` — high-level goals: what we're building, why, and the phase plan.
- `01-requirements.md` — functional and non-functional requirements. _(not yet written)_
- `02-architecture.md` — system architecture, components, data flow.
- `03-hardware.md` — hardware choices, sensors, actuators, wiring. _(not yet written)_
- `04-software-stack.md` — languages, frameworks, libraries, build setup. _(not yet written)_
- `05-communication.md` — RF link, USB, message formats, framing. _(not yet written)_
- `06-control.md` — flight control, PID tuning, sensor fusion notes. _(not yet written)_
- `07-safety.md` — failsafes, arming, battery monitoring, bench discipline. **Prerequisite for Phase 3 free flight.**
- `ci-and-testing.md` — CI tiers, local-first feedback, hook setup, runner. _(not yet written; will land with the first Cargo workspace)_
- `dev-environment.md` — toolchain setup, `probe-rs`, board labelling, two-board workflow. Windows-specific notes included.
- `decisions/` — Architecture Decision Records (ADRs), one file per decision.
- `research/` — external references, datasheets, links, notes. See [research/README.md](research/README.md) for what belongs there.

Mechanical CAD files (`.f3d` + `.step` + print-ready `.stl` / `.3mf`) will live in a top-level `Mech/` folder when the first part lands ([ADR 0006](decisions/0006-mechanical-cad-fusion360.md)).

## Conventions

- Markdown for all docs.
- Use ADRs in `decisions/` for any non-trivial choice (format: `NNNN-title.md`).
- Keep entries dated; append rather than overwrite when capturing evolving thinking.
- Link to source files in the repo using relative paths.
- Writing style is engineering documentation, not blog prose. See `AGENTS.md` Writing style for the rules.

## Status

Vision and architecture agreed. Seven ADRs in place ([decisions/](decisions/README.md)). No code yet — next step is Phase 1 bring-up once hardware (ICM-42688 breakout, motor + ESC + bench PSU) arrives. See [`00-vision.md`](00-vision.md) for the phase plan.
