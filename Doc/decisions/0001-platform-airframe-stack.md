# ADR 0001 — Platform, airframe, and firmware approach

- **Status:** Accepted
- **Date:** 2026-05-21

## Context

The Drone project is starting from zero. Before any hardware is bought or code written, four fork-in-the-road decisions shape everything downstream:

1. Real hardware vs. simulator (Gazebo / jMAVSim / AirSim / Webots).
2. Airframe type (quadcopter, fixed-wing, hexacopter, VTOL, etc.).
3. Primary goal (learning, autonomous nav, FPV / racing, mission-specific).
4. Build on top of an existing flight stack (PX4, ArduPilot) vs. write our own firmware from scratch.

The user has strong systems-programming experience (C, low-level I/O, raw TCP) and is explicitly trying to learn, not to ship a product. The drone is the artefact; understanding is the deliverable.

## Decision

1. **Real hardware only.** No simulator-first phase. Simulators hide the messy parts (sensor noise, vibration, RF, power, EMI) that are precisely what we want to learn.
2. **Quadcopter.** Simplest kinematics, symmetric, hover-capable, by far the best-documented community.
3. **Goal: learning.** Success is measured by depth of understanding, not by features delivered.
4. **Roll our own firmware.** No PX4, no ArduPilot, no Betaflight. We write the IMU drivers, sensor fusion, control loops, motor mixing, and radio decoding ourselves.

Off-the-shelf parts are used for everything *below* the firmware line: motors, ESCs (we'll speak an existing protocol like DShot, not invent one), receiver module, frame, battery.

## Consequences

### What this commits us to

- **A much longer path to first flight.** We accept this.
- **Doing the hard parts the hard way**: sensor fusion (complementary → Mahony → potentially EKF), real-time timing on bare metal, cascaded PID tuning by hand.
- **A bench / tether rig as a first-class deliverable.** Free-flying an untested control loop with spinning props is unacceptable. Safe iteration depends on having instrumented bench setups before each tier of flight.
- **Heavy investment in instrumentation** — streaming telemetry, host-side plotting — from day one. You cannot tune control loops blind.
- **A phased plan** (see [00-vision.md](../00-vision.md) "Definition of success") so the scope doesn't collapse under its own weight.

### What this rules out (for now)

- Fast progress to autonomous missions, GPS waypoints, computer vision. These are explicit stretch goals only.
- Using community tooling that assumes PX4 / ArduPilot (Mission Planner, QGroundControl in their default flows, MAVLink-based ecosystems). We'll likely roll a minimal custom ground-station / telemetry tool instead.
- A drop-in fix if something fundamental turns out to be wrong — every layer is ours to debug.

### What stays open

This ADR pins down the *kind* of project. It does **not** decide:

- MCU family and dev board.
- Firmware language (C / C++ / Rust).
- Specific IMU and other sensors.
- Frame class, motors, ESCs, battery.
- Radio link choice.
- Host-side tooling for telemetry / plotting.

Each of those gets its own ADR.
