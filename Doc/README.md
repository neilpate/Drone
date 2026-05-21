# Drone Project — Documentation

This directory holds the design notes, decisions, and references for the Drone project.

## Structure

- `README.md` — this file; index and conventions
- `00-vision.md` — high-level goals: what we're building and why
- `01-requirements.md` — functional and non-functional requirements
- `02-architecture.md` — system architecture, components, data flow
- `03-hardware.md` — hardware choices, sensors, actuators, wiring
- `04-software-stack.md` — languages, frameworks, libraries, OS
- `05-communication.md` — protocols (radio, WiFi, telemetry), message formats
- `06-control.md` — flight control, PID tuning, sensor fusion notes
- `07-safety.md` — failsafes, geofencing, battery monitoring
- `decisions/` — Architecture Decision Records (ADRs), one file per decision
- `research/` — external references, datasheets, links, notes

## Conventions

- Markdown for all docs.
- Use ADRs in `decisions/` for any non-trivial choice (format: `NNNN-title.md`).
- Keep entries dated; append rather than overwrite when capturing evolving thinking.
- Link to source files in the repo using relative paths.

## Status

Project just started — nothing built yet. Next step: fill in `00-vision.md` to agree on what kind of drone this is (quadcopter? fixed-wing? simulated? real hardware?) and what the primary purpose is.
