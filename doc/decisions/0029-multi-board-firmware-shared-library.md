# ADR 0029 — Multi-board firmware: shared library plus thin per-board binaries

- **Status:** Accepted
- **Date:** 2026-09-20
- **Related:** [ADR 0010](0010-board-support-package.md) (the BSP seam this revises — the seam moves out of the binary into a shared library), [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md) (crate naming this extends and partly supersedes), [ADR 0015](0015-host-testing-no-std-crates.md) (the pure host-tested boundary `firmware-drone-core` keeps), [ADR 0026](0026-phase4-custom-pcba-nrf5340.md) (nRF5340 platform, dual-core), [ADR 0014](0014-radio-protocol-ieee802154.md) (802.15.4, which lands on the network core), [ADR 0004](0004-concurrency-embassy-channels.md) (Embassy task model this works within)

## Context

The firmware runs on two physically different boards over the project's life ([00-vision.md](../00-vision.md)):

- **Phases 1–3:** BBC micro:bit v2 — nRF52833, single Cortex-M4F, `thumbv7em-none-eabihf`, one binary, radio done inline as a task.
- **Phases 4–5:** custom carrier PCBA — nRF5340 module ([ADR 0026](0026-phase4-custom-pcba-nrf5340.md)), dual Cortex-M33 (`thumbv8m.main-none-eabihf`), 802.15.4 on the network core ([ADR 0014](0014-radio-protocol-ieee802154.md)) and application logic on the application core.

[ADR 0010](0010-board-support-package.md) put the board seam **inside** the `firmware-drone` binary: a feature-selected `board` module (`microbit_v2.rs`, a hypothetical `drone_rev_a.rs`) exposing name-stable wrapper types (`board::Motors`, `board::Imu`, …), so tasks take opaque board types and never see pins. That design was correct and remains the foundation here — but it silently assumed both boards would be **the same target triple** and **a single binary**. The nRF5340 breaks both assumptions:

1. **Different target triple.** A single crate has one `.cargo/config.toml` target and one active `embassy-nrf` chip feature (`nrf52833` vs `nrf5340-app-s`). Two triples cannot co-compile into one binary crate.
2. **Dual-core, two binaries.** The nRF5340 needs a separate application-core binary and network-core binary talking over IPC. The micro:bit is one binary.

Underneath both sits a hard Embassy rule: **`#[embassy_executor::task]` functions cannot be generic.** So "make the tasks generic over the board" is not directly available.

The alternative we are explicitly rejecting is copy-pasting `firmware-drone` into a second board crate: the two copies would diverge on every bug fix and every new task. The goal is one home for the logic, thin per-board glue, and no divergence.

**Scope: drone firmware only.** The remote (`firmware-remote`) stays on the micro:bit for the foreseeable future — there is no plan to move it to the nRF5340 — so it faces no multi-board pressure and is deliberately left untouched here. It keeps its current single-board structure, its own `firmware-remote-core`, and its own radio code. `RadioLink` in this ADR is therefore a *drone-internal* trait, not a shared drone/remote interface. If the remote ever gains a second board, this ADR is the template to mirror (as `firmware-remote-shared`), not a constraint it must satisfy today.

## Decision

### 1. Extract a `firmware-drone-shared` library; keep `firmware-drone-core` pure

A new library crate **`firmware-drone-shared`** holds everything board-agnostic that today lives inside the `firmware-drone` binary: the task bodies, the global signal statics, and the peripheral traits (§3).

`firmware-drone-core` stays a **separate, pure crate** — no async, no HAL, no Embassy, host-tested with `cargo test` per [ADR 0015](0015-host-testing-no-std-crates.md). `firmware-drone-shared` **layers on top of it**: `core` is the flight-logic library (supervisor, control law, mixer, sensor fusion, DShot encoding); `shared` is the Embassy task/wiring library that drives that logic. Merging them was considered and rejected — it would destroy the host-test boundary, because `shared` depends on the async runtime and cannot be `cargo test`ed the same way.

```
firmware-types        (wire types)          ─┐
firmware-drone-core   (pure flight logic)   ─┤→ firmware-drone-shared (tasks, signals, traits)
                                             ┘        │
                                                      ├→ firmware-drone-microbit        (bin, nRF52833)
                                                      ├→ firmware-drone-nrf5340-app     (bin, app core)
                                                      └→ firmware-drone-nrf5340-net      (bin, net core) ← firmware-types only
```

### 2. Two task classes: fully-shared, and shared-body-plus-shim

Tasks split by whether they touch hardware.

- **Group A — no hardware** (supervisor, attitude estimator, control system, telemetry aggregator, sensors aggregator). These take only signals and core logic. They move into `firmware-drone-shared` **including their `#[embassy_executor::task]` attribute unchanged** — they are already board-agnostic concrete functions.

- **Group B — touches hardware** (imu, motor controller, remote link, status LED, temperature, ESC telemetry, config manager). The task **body** becomes a generic `async fn` in `firmware-drone-shared`, bounded by a peripheral trait (§3). Each board binary supplies a three-line `#[embassy_executor::task]` **shim** that passes its concrete board type:

  ```rust
  // firmware-drone-shared — board-agnostic body
  pub async fn motor_controller_body(mut motors: impl MotorDriver) -> ! {
      let mut rx = signals::motor_command::subscribe();
      loop { motors.set_all_motors(rx.get().await); }
  }

  // firmware-drone-microbit — the only per-board glue
  #[embassy_executor::task]
  async fn motor_controller(motors: board::Motors) -> ! {
      firmware_drone_shared::motor_controller_body(motors).await
  }
  ```

  This generic-body-plus-per-board-shim pattern is the standard Embassy answer to non-generic tasks, and it is the direct generalisation of [ADR 0010](0010-board-support-package.md): the name-stable `board::Motors` trick still holds *within* each binary, and the trait bound is what lets one body serve every binary's concrete type.

- **Board-unique tasks are allowed in principle.** The pattern does not force every task onto every board: if a board ever carries hardware with no counterpart on the other, its task lives only in that binary and is spawned only there. None are currently anticipated — the two boards are expected to be task-symmetric — but the rule is *shared by default, board-unique only where the hardware exists on one board alone*, not *identical on every board*. `spawn_common` (§8) covers the shared Group-A tasks; any board-unique task is spawned alongside it in that board's `main`.

### 3. Peripheral traits live inside `firmware-drone-shared`

The seam between Group-B bodies and hardware is a small set of traits, defined in `firmware-drone-shared` next to the bodies that consume them (not in a separate `-hal` crate — one fewer crate, nothing else consumes them):

`MotorDriver`, `ImuReader`, `RadioLink`, `StatusIndicator`, `TemperatureSource`, `EscTelemetrySource`, `ConfigStore`.

Each board's `board` module implements these for its wrapper types. They are mostly a rename of methods that already exist as inherent methods on the micro:bit wrappers.

### 4. Dependency rule: `firmware-drone-shared` must not depend on `embassy-nrf`

This is the load-bearing constraint that makes two target triples possible. `firmware-drone-shared` depends only on **chip-agnostic** crates: `embassy-executor`, `embassy-time`, `embassy-sync`, plus `firmware-types`, `firmware-drone-core`, `defmt`. The chip feature (`nrf52833` / `nrf5340-app-s`) lives in `embassy-nrf`, which is mutually exclusive per build and therefore stays **per-binary only**.

Consequently the following stay in each `main.rs`, never in the library: `embassy_nrf::init` and clock config, the `embassy-time` time driver (supplied by `embassy-nrf`), the `InterruptExecutor` and its `#[interrupt]` handler (SWI/EGU names differ per chip), the global `defmt-rtt` logger, the `panic-probe` handler, `memory.x`, `build.rs`, and `.cargo/config.toml`. `firmware-drone-shared` takes only the `defmt` macros. The library says *what runs*; the binary says *on what, wired how, clocked by what*.

A related guardrail: keep Group-A task bodies thin, delegating to `firmware-drone-core` for anything non-trivial, so the non-host-testable surface that lands in `shared` stays as small as possible.

### 5. Per-board binary crates, and rename `firmware-drone` → `firmware-drone-microbit`

Three binary crates, named symmetrically:

- **`firmware-drone-microbit`** — nRF52833, single core. This is the current `firmware-drone` crate, renamed.
- **`firmware-drone-nrf5340-app`** — nRF5340 application core: all the tasks and the app-side of the radio IPC.
- **`firmware-drone-nrf5340-net`** — nRF5340 network core: the 802.15.4 bridge (§6).

The rename happens now, as part of the extraction, so the crate set is symmetric from the start rather than one asymmetric legacy name. This extends [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md) and supersedes its provisional `firmware-pcba` placeholder for the Phase 4 firmware.

Because board selection is now by **crate identity** (one board per binary), [ADR 0010](0010-board-support-package.md)'s `board-*` Cargo features are no longer needed and are dropped: each binary compiles its single `board` module unconditionally. Features would only return if two *same-triple* boards ever shared one binary — not the case here (the two chips are different triples).

### 6. The nRF5340 network core is a dumb raw-frame byte pipe

The network-core binary owns the 802.15.4 PHY and does nothing but shuttle **raw PHY frames** across the inter-core IPC (the nRF5340 IPC peripheral over a shared-RAM region — see §10 for the caveats). All postcard/COBS decoding and all `firmware-types` knowledge stay on the application core, inside `remote_link_body`.

This keeps the net-core binary tiny and near-dependency-free (no `firmware-types`, no core logic), and it means `RadioLink` is one symmetric interface — `receive()` / `send(frame)` — implemented on the micro:bit by the on-chip radio directly, and on the nRF5340 app core by the IPC endpoint. `remote_link_body` is identical over both. Decoding on the net core (typed IPC messages) was rejected: it would duplicate the wire protocol across two cores and drag `firmware-types` onto the net core.

### 7. `StatusIndicator` is intent-shaped, not `on()`/`off()`

The trait is `set_state(SystemStatus)`, not low-level `on()`/`off()`. The micro:bit renders state on its 5×5 charlieplex matrix; the PCBA renders it on whatever indicator it has. The interface expresses intent so each board owns its own rendering. This is the one wrapper whose *interface* (not just implementation) is deliberately reshaped during the move.

### 8. Shared `spawn_common(&spawner)` helper

`firmware-drone-shared` exposes a `spawn_common(&spawner)` helper that spawns the Group-A (no-hardware) tasks. Both the micro:bit and nRF5340-app binaries call it, so that spawn list cannot diverge. Board-specific spawns (the Group-B shims, which need board resources) stay in each `main`.

### 9. Migrate in two phases

- **Phase 1 (now):** create `firmware-drone-shared`; move the Group-A tasks and all signal statics into it; add `spawn_common`; rename `firmware-drone` → `firmware-drone-microbit`. Single consumer, pure refactor, no behaviour change. Shrinks the binary immediately and de-risks the rest.
- **Phase 2 (during nRF5340 bring-up):** introduce the seven traits and convert the Group-B bodies to generic `async fn`s, driven by the real second board. The traits are then shaped by two concrete implementations, not one imagined one.

### 10. The nRF5340 two-binary split is not two independent programs

The split is **forced by silicon, not chosen**: in `embassy-nrf` the `radio` module exists only under `nrf5340-net`, not under `nrf5340-app-s`. `RADIO` is physically a network-core peripheral the app core cannot touch, so ADR 0014's 802.15.4 *must* run on the net core and the app core *must* reach it across an inter-core transport. "Dumb pipe" (§6) is only the choice to run as little protocol as possible on the net side; the transport itself is unavoidable once you want the radio.

Mechanically the transport is a familiar pattern: **a lock-free SPSC ring in shared RAM per direction, with an inter-core interrupt as the doorbell** (the systems-programming equivalent of a virtio vring + eventfd, or a NIC descriptor ring + MSI). The two Cortex-M33s have no data cache, so shared RAM is coherent between them — ordering needs a memory barrier (`DMB`), not cache flush/invalidate. Details and a bring-up plan are captured in [research/nrf5340-intercore-ipc.md](../research/nrf5340-intercore-ipc.md).

The binaries share a hardware-imposed contract that §5–§6 gloss over, recorded here so Phase 2 does not mistake them for independent programs:

- **The app core brings up the net core.** On reset only the app core runs. It must make the shared-RAM region accessible to the net core (an SPU permission; whether `embassy_nrf::init` covers it or it needs a `pac` poke is the first thing to establish), initialise the ring headers, then call `reset::release_network_core()`. `RADIO` is *not* granted — it is inherently net-core-owned. Net-core startup is an app-core responsibility, not a separate boot.
- **The shared-RAM region is a cross-binary linker contract.** Both `memory.x` files must agree, byte-for-byte, on the region's address and size. This is the one place the two binaries are coupled at link time, with no compiler to check it; a mismatch corrupts the pipe silently.
- **The primitives exist; the transport does not.** `embassy-nrf` supplies the doorbell (`ipc::Ipc` + `Event`/`EventTrigger`, signal-only, no payload), the net-core release (`reset::release_network_core()`), and the net-core `radio` (the same `ieee802154::Radio` API the micro:bit uses today). It does **not** supply the shared-RAM ring, the SPU carve-out, or any dual-core example (the `examples/nrf5340` set is blinky/uart/rng/gpiote only). So the risk is *integration*, not *feasibility* — assembling known parts with no reference to copy.
- **De-risk with a spike before writing any `RadioLink` code.** Two minimal binaries: app core inits the region and releases the net core; ping-pong a `u32` counter through one ring with a doorbell each way; confirm the round-trip and eyeball the latency on hardware. If that works, the whole `RadioLink`-over-IPC design rests on known-good plumbing; if the SPU/memory setup fights, it is learned on 30 lines, not through the radio path. Fallback if the bare ring proves painful: a thin ICMsg-style framed transport (still our own Rust), not the heavyweight RPMsg/OpenAMP stack.
- **Secure world only.** The app core runs secure-only (`nrf5340-app-s`); the TrustZone secure/non-secure split is not used. One fewer axis of complexity for a hobby build.
- **Latency is a budget, not a deadline.** The reply path now crosses app → net over IPC before reaching the air, adding to the ~100 Hz round-trip and the RTT echo ([ADR 0027](0027-split-telemetry-high-low-rate.md)). Raw 802.15.4 PHY has no hardware-ACK deadline ([ADR 0014](0014-radio-protocol-ieee802154.md)), so this is a budget to measure, not a hard timing constraint — but it must be measured, not assumed.

### 11. Widen the core/task split opportunistically during the move

The extraction is the natural moment to push more pure logic down into `firmware-drone-core`, growing the host-tested surface ([ADR 0015](0015-host-testing-no-std-crates.md)) and thinning the async tasks. It also directly serves this ADR's goal: anything in `core` is automatically shared by every board and needs no trait. A scan of the current tasks and the micro:bit board wrapper found the candidates below. These are **candidates, not commitments** — extraction stays opportunistic and need not all land in Phase 1.

| Candidate | From | To (`core`) | ~lines | Why pure / payoff |
|---|---|---|---|---|
| ICM-42688 raw bytes → `ImuData` (`convert_bytes`) | `board/microbit_v2.rs` | new parser, LSB scales as params | ~23 | Big-endian unpack + scale, no I/O. **Highest payoff**: both boards use the ICM-42688 ([ADR 0003](0003-imu-icm42688-spi.md)), so the parser is shared verbatim instead of duplicated per board. |
| Status-LED pattern for `DroneState` (`pattern_for_state`) | `tasks/status_led.rs` | pattern table | ~23 | Pure `enum → pattern`; the `StatusIndicator` impls just render the result. |
| IMU calibration averaging (`calibrate_imu`) | `tasks/imu.rs` | mean-over-samples fn | ~42 | Stateless mean over a sample slice; host-testable with a fixed vector. |
| CPU-load formula (elapsed → load) | `tasks/load_profiler.rs` | arithmetic fn | ~8 | Integer/`f32` arithmetic, no I/O. |

Already clean (delegate to `core` today, no action): supervisor, attitude estimator, control system, temperature, DShot framing.

Left in the tasks deliberately (small and tightly coupled to signals or the `.await` loop — extraction would add parameter bloat for little gain): radio command dispatch (`remote_link`), ESC frame scan (`esc_telemetry`), telemetry-frame assembly (`telemetry_aggregator`), sensor struct packing (`sensors_aggregator`).

The test for "belongs in `core`": deterministic compute with no `.await`, no HAL type, no signal access — raw→physical conversion, parsing, table lookups, formulas. The IMU parser is the one worth prioritising, because its payoff is multi-board reuse, not just testability.

## Consequences

**What this commits us to**

- The board seam is now a **library boundary**, not an in-binary module boundary. [ADR 0010](0010-board-support-package.md) §1 ("a single `board` module inside `firmware-drone`") is revised: each binary still has its own `board` module, but the shared logic that consumes it lives one crate up. The name-stability and no-pins-in-tasks rules from 0010 are unchanged and still enforced.
- Every new drone task is written once, in `firmware-drone-shared` — as a full task (Group A) or a generic body plus per-board shims (Group B). Adding a task to only one board is now the exception that needs justifying, not the default.
- `firmware-drone-shared` stays disciplined about its dependency list: adding `embassy-nrf` (directly or transitively) is a defect that breaks the multi-target build.
- Any new Group-B peripheral needs a trait in `firmware-drone-shared` and an `impl` in each board's `board` module.
- The move is also used to widen the core/task split (§11): identified pure logic migrates to `firmware-drone-core` as opportunity allows — the ICM-42688 byte parser especially, since both boards share the IMU — but this is not a Phase-1 gate.

**What it rules out / trade-offs**

- More crates and one indirection (body ↔ shim) versus a single binary. Accepted: the alternative is divergence, which is worse.
- `firmware-drone-shared` is not host-testable the way `firmware-drone-core` is (it pulls in the async runtime). Pure logic that wants host tests belongs in `core`, not `shared` — the split from [ADR 0015](0015-host-testing-no-std-crates.md) is the guide for which crate a new piece goes in.
- The remote (`firmware-remote`) is out of scope and stays micro:bit-only. It keeps its current single-board structure and its own radio code; this ADR is a template it can copy later, not a change it must absorb now.
- The nRF5340 inter-core transport (§10) is an *integration* risk, not a feasibility one: the primitives exist in `embassy-nrf` but must be assembled into a shared-RAM ring with no example to copy. Phase 2 de-risks it with a standalone spike ([research/nrf5340-intercore-ipc.md](../research/nrf5340-intercore-ipc.md)) before the `RadioLink`-over-IPC design is committed; it remains the biggest unknown in this plan.
- rust-analyzer runs one target per window ([repo convention](../../AGENTS.md)), and `cargo build --workspace` cannot target two triples at once: the nRF5340 binaries (`thumbv8m.main`) are a different triple from the micro:bit default (`thumbv7em`), so — like `groundstation` — they must be workspace-excluded and built/verified from their own directories. `firmware-drone-shared` itself compiles under whichever consumer's target pulls it in; both are Cortex-M, so it is effectively target-agnostic.

**Sequencing**

Phase 1 is safe to start immediately and is independent of hardware. Phase 2 is gated on the nRF5340 board bring-up ([ADR 0028](0028-fabricate-v2-board-pcbway.md)).
