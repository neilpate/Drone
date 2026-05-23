# ADR 0010 — Board Support Package (BSP) layer

- **Status:** Accepted
- **Date:** 2026-05-23
- **Related:** [ADR 0002](0002-mcu-and-language.md), [ADR 0004](0004-concurrency-embassy-channels.md), [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md)

## Context

The drone firmware will run on at least two physically different boards over the project's lifetime ([AGENTS.md](../../AGENTS.md), [00-vision.md](../00-vision.md)):

- **Phases 1–3:** BBC micro:bit v2 (nRF52833). 5×5 LED matrix, no IMU, no motors — sensors and actuators are breadboarded onto the edge connector.
- **Phases 4–5:** custom carrier PCBA, nRF5340. Real status LED, IMU populated on the PCB, motor PWM fanned out to ESCs.

The two boards share an MCU family (nRF52 / nRF53) and a HAL (`embassy-nrf`), but **every physical pin assignment will differ**. The IMU's SPI bus, the motor PWM channels, the status LED — all change.

Without a deliberate seam, every task in the firmware ends up hard-coded against micro:bit pin literals (`p.P0_21`, `p.P0_28`, etc.). When the custom board arrives, porting becomes "audit every pin reference across every task and every driver".

The first sub-task hit by this is the heartbeat LED added during Phase 1 bring-up (`crates/firmware-drone/src/main.rs`). Its current form names `p.P0_21` / `p.P0_28` directly inside the spawn site. Fixing only the spawn site is not enough — the question is structural and applies to every future driver.

## Decision

### 1. A single `board` module inside `firmware-drone`

The seam lives **inside the `firmware-drone` binary crate**, not as a separate crate.

```
crates/firmware-drone/src/
├── main.rs
├── tasks/                  # (later — one file per task)
└── board/
    ├── mod.rs              # re-exports the selected board
    ├── microbit_v2.rs      # nRF52833, micro:bit v2
    └── drone_rev_a.rs      # nRF5340, custom carrier (Phase 4+)
```

`mod.rs` selects one implementation via Cargo features and re-exports it as `crate::board`:

```rust
#[cfg(feature = "board-microbit-v2")]
mod microbit_v2;
#[cfg(feature = "board-microbit-v2")]
pub use microbit_v2::*;

#[cfg(feature = "board-drone-rev-a")]
mod drone_rev_a;
#[cfg(feature = "board-drone-rev-a")]
pub use drone_rev_a::*;
```

Exactly one board feature is active per build. Mutual exclusion is enforced by a `compile_error!` guard.

### 2. The BSP contract

Every board module must expose **the same public surface**:

```rust
pub const NAME: &str;

pub struct Board {
    pub heartbeat_row: peripherals::P0_21,   // type varies per board
    pub heartbeat_col: peripherals::P0_28,
    // pub imu_spi:    peripherals::SPI3,    // added as Phase 1 grows
    // pub imu_cs:     peripherals::P0_xx,
    // pub motor0_pwm: peripherals::PWM0,
    // ...
}

impl Board {
    pub fn new(p: embassy_nrf::Peripherals) -> Self { ... }
}
```

- **Field names are logical** ("heartbeat_row", "imu_cs", "motor0_pwm"). They identify the *role*, never the pin number.
- **Field types are specific** — the concrete `embassy_nrf::peripherals::P0_xx` / `SPIx` / `PWMx` types declared directly on the field, with a doc comment naming the physical pin. The board module owns the physical mapping.
- `Board::new` consumes the raw `Peripherals` struct from `embassy_nrf::init()` and hands back the logically-named `Board`. The constructor is the *only* place in the file that references PAC field names.
- Adding a new logical role means adding a field to *every* board's `Board` struct. The compiler refuses to build a board that's missing a field its tasks expect.

### 3. The `board_pins!` macro is the canonical implementation

The shape above is what every board module must *expose*. In practice, board files never write the `struct Board` and `impl Board { new }` by hand — they use a small declarative macro defined once in `board/mod.rs`:

```rust
super::board_pins! {
    /// Heartbeat LED — row 1 of the 5×5 matrix (P0.21).
    heartbeat_row: P0_21,
    /// Heartbeat LED — col 1 of the 5×5 matrix (P0.28).
    heartbeat_col: P0_28,
}
```

This expands to exactly the `Board` struct and `Board::new` constructor described above. Each role↔pin pair appears once. Doc comments on each entry attach to the generated field and show up in `cargo doc`.

Reasoning:

- Plain Rust forces each pin name to be written twice (in the field type and in the constructor body) because `Peripherals` field access is by literal name. The macro is the standard escape — `microbit`, `nrf52840-dk`, `atsamd-hal` and several Embassy board crates use variations of the same pattern.
- The macro is local to `board/mod.rs`, ~30 lines, and only handles `peripherals::$pin` mappings. When a board needs a peripheral that doesn't fit this shape (a configured SPI bus, a pin with a power-enable sequence), the escape hatch is to **declare the simple pins via the macro and hand-write the rest**, or to replace the auto-generated `new` with a custom one.

This is documented separately from the contract because the contract is what tasks and `main` depend on; the macro is an implementation detail of how board files satisfy that contract. A future board with quirks may bypass the macro without violating the contract.

### 4. Tasks take board-agnostic types

Task signatures must not name a specific pin or peripheral type. They take either:

- **Type-erased handles** (`AnyPin`, `AnyChannel`, etc.) when they need to work across boards with different concrete types.
- **Generic parameters bounded by HAL traits** (`<SPI: SpiBus, CS: OutputPin>`) when the type-erasure cost matters.

For the heartbeat task specifically: `async fn heartbeat(row: AnyPin, col: AnyPin) -> !`.

The `.degrade()` call that bridges specific → erased happens **at the spawn site in `main`**, not inside the board module. The board module keeps full type information in `Board`; main pays the type-erasure cost only for the tasks that need it.

### 5. `main.rs` contains zero physical pin literals

After this ADR lands, `crates/firmware-drone/src/main.rs` references **only** `board::*` and the `embassy_nrf::init` call. Grepping `main.rs` for `P0_`, `P1_`, `SPI[0-9]`, `PWM[0-9]`, `TIMER[0-9]`, or any other peripheral literal must return zero hits. This is the structural invariant the BSP exists to enforce.

If a task needs a peripheral the board doesn't yet expose, the fix is to add a field to every `Board` struct — not to reach into `p.*` from `main`.

### 6. Cargo feature naming and default

```toml
[features]
default = ["board-microbit-v2"]
board-microbit-v2 = []
board-drone-rev-a = []   # added when the custom PCBA lands
```

- Feature names are `board-<short-name>` with a hyphen — matches Cargo convention.
- The default is whichever board is "current development" — micro:bit v2 today; will migrate to `board-drone-rev-a` once the PCBA is the primary target.
- CI eventually builds every board feature in a matrix. Until then, "it compiles for the default board" is the bar.

## Why this shape

- **The seam belongs inside the binary, not a separate crate.** A BSP crate would be over-engineering for two boards that share a HAL family and a firmware. The `firmware-drone` binary is already board-specific in the trivial sense (it picks a chip, a memory layout, a linker script); the `board` module just makes the dependency explicit and swappable. If the custom-PCBA work spawns a second firmware (e.g. an instrumentation board), promoting `board` to a `firmware-drone-bsp` crate is a mechanical move at that point.
- **Specific types in `Board`, erased types at the spawn site.** This preserves the option to use concrete types for hot-path drivers (where the compiler can inline register accesses) while keeping the cross-board task signatures portable. Erasing inside the BSP would throw away type information unnecessarily.
- **`Board` as a plain struct, not a trait.** With two boards in scope and exactly one active per build, a trait abstraction with associated types would pay generic-instantiation cost everywhere for zero practical benefit. The Cargo-feature switch already gives us static dispatch and a single concrete type per build.
- **Cargo feature selection rather than runtime detection.** The chip differs between boards (nRF52833 vs nRF5340); the linker script, `embassy-nrf` feature flags, and probe-rs chip target all differ. There is no scenario where one binary needs to support both. Compile-time selection is the honest model.

## Consequences

### What this commits us to

- Adding a new logical peripheral role means **editing every `board/*.rs` file**, not just the one for the current target board. The compiler enforces this — a missing field on the new board fails to build under that board's feature.
- `firmware-drone-core` (per [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md)) remains unaware of `board`. The BSP is a `firmware-drone` (binary) concern; pure logic in `-core` is parameterised by trait bounds, not by board.
- Hot-path drivers that want concrete peripheral types instead of `AnyPin` / `dyn` must declare their portability strategy explicitly (per-board task variants under `#[cfg(feature = ...)]`, or generics-with-trait-bounds). The default for new tasks is "take erased types".
- `[features]` and `default` on `firmware-drone` are part of the public build contract. Removing or renaming a board feature is a breaking change to the build matrix and gets called out in the commit message.

### What this rules out

- Any `p.P0_xx` / `p.SPI3` / etc. literal in `main.rs` outside the `board` module.
- A "central" BSP crate shared by multiple firmwares. If `firmware-ground` ever needs a BSP, it gets its own `board` module inside `firmware-ground`, on the same contract. The duplication is acceptable; the coupling would not be.
- Runtime board detection (reading a strapping pin, checking a board ID byte, etc.). Different chips, different linker scripts — runtime detection is structurally impossible.
- Multiple board features active at once. `compile_error!` guards both "none selected" and "more than one selected".

### What stays open

- **Exact field set on `Board`.** Heartbeat row/col is the only field today. IMU, motors, radio link, etc. are added as Phase 1 progresses — driven by the first task that needs each.
- **Whether `Board` eventually grows board-specific initialisation methods** (e.g. `Board::init_imu(self) -> (ImuDriver, Board)` for a board that needs a power-enable pin toggle first). Likely yes for the custom PCBA; deferred until that case appears.
- **CI matrix for board features.** Recorded once CI itself exists ([ADR 0007](0007-testing-and-ci-strategy.md) deferred CI implementation).
- **`firmware-ground` BSP.** Not in scope here; if it needs one, it follows the same contract.
