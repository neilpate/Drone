# Dev environment

_Status: Living document. Will grow substantially once Phase 1 bring-up begins. Today it captures the conventions and host-OS notes that need to exist before the first `cargo` command runs._

This is the practical-side companion to the architecture and ADR docs: which OS, which toolchain, how the two micro:bits are labelled and told apart, what gotchas hit on Windows specifically. Aimed at the future "stranger landing cold in the repo" (see [00-vision.md](00-vision.md) Definition of done) as much as at the author.

---

## Host OS

The author develops on **Windows 11**. The toolchain is intended to work equally well on macOS and Linux — nothing in the build pipeline is Windows-specific — but the notes below call out Windows quirks where they exist.

If a future contributor (or future self) is on Linux / macOS, most of this doc applies unchanged; the **Windows-specific** subsection is the only part to skip.

---

## Toolchain (target state)

To be installed at the start of Phase 1. Versions to be locked via `rust-toolchain.toml` once the Cargo workspace exists.

- **Rust** via [`rustup`](https://rustup.rs/) — stable channel, plus the `thumbv7em-none-eabihf` target (nRF52833 is Cortex-M4F).
- **`probe-rs`** — for flashing and `defmt` log streaming over the micro:bit's onboard DAPLink. Install via `cargo install probe-rs-tools` once the workspace lands.
- **`flip-link`** — stack-overflow protection linker for `no_std` binaries. `cargo install flip-link`.
- **`cargo-binutils`** — `cargo size`, `cargo nm` for binary inspection. Optional but useful. `cargo install cargo-binutils` + `rustup component add llvm-tools-preview`.
- **VS Code** with the `rust-analyzer` and `probe-rs.probe-rs-debugger` extensions.

The first commit that adds the Cargo workspace will also add a `rust-toolchain.toml` pinning these versions and a `.vscode/extensions.json` recommending the extensions above.

### Verified on real hardware

The above toolchain — Rust 1.95 stable, `thumbv7em-none-eabihf`, `probe-rs`, `flip-link`, `defmt`, `defmt-rtt`, `panic-probe`, the Embassy stack — has been validated end-to-end on a BBC micro:bit v2 (2026-05-23): build, flash via the on-board CMSIS-DAP probe, `defmt` log stream over RTT, and source-level debugging from VS Code via the `probe-rs.probe-rs-debugger` extension. First on-target output was the boot banner; first peripheral exercised was the 5x5 LED matrix (P0.21 row drive + P0.28 column sink, 1 Hz heartbeat).

### Source-level debugging in VS Code

The repo ships `.vscode/launch.json`, `.vscode/tasks.json`, and `.vscode/extensions.json` to make F5 a working "build, flash, attach debugger, stream `defmt`" cycle. Requirements:

- `probe-rs.probe-rs-debugger` extension installed (VS Code prompts on first open via [`.vscode/extensions.json`](../.vscode/extensions.json)).
- The same `probe-rs` CLI on `PATH` that `cargo run` uses (versions of CLI and extension should track each other — major drift breaks the DAP wire format).

What F5 does: runs the `build firmware-drone (debug)` task, then the extension flashes the resulting ELF, halts the core at the reset vector (in `cortex-m-rt`'s startup, *before* `main`), and attaches. `defmt` frames from RTT are decoded into the Debug Console with timestamps. Breakpoints are hardware breakpoints set in the Cortex-M4's FPB unit — about six are available, after which new ones silently fail to bind.

The launch config defaults to `haltAfterReset: false` on the debug profile so the firmware runs straight to your breakpoints; the release profile keeps `haltAfterReset: true` for cases where a panic at `static` init or in startup needs to be caught.

Stepping through `async` code: Embassy lowers each `async fn` into a state machine, so single-stepping over `.await` is misleading. Set breakpoints after the await, not on it.

### Windows-specific notes

- **DAPLink USB driver.** The micro:bit v2 enumerates as a CMSIS-DAP v2 device on Windows 10/11 without any driver install — WinUSB is bound automatically. If `probe-rs list` does not see the board, check Device Manager: under "Universal Serial Bus devices" there should be a "DAPLink CMSIS-DAP" entry. If it appears under "Other devices" with a yellow exclamation, install [Zadig](https://zadig.akeo.ie/) and bind WinUSB manually.
- **Line endings.** Git on Windows defaults to `core.autocrlf=true`, which rewrites `LF` to `CRLF` on checkout. This is mostly harmless but can confuse `defmt` log parsing if log lines are captured to a file and read on a different OS. Recommendation: set `core.autocrlf=input` for this repo (`git config core.autocrlf input`) so files are stored and checked out as `LF`.
- **Path length.** Cargo's `target/` directory nests deep. On older Windows installs with the 260-char `MAX_PATH` limit still active, builds can fail with cryptic I/O errors. Enable long paths: `git config --system core.longpaths true` and the Windows registry / group-policy "Enable Win32 long paths" setting.
- **Antivirus on `target/`.** Defender real-time scanning of `target/` slows incremental builds dramatically. Add `target/` (and the repo root, if you trust your own code) to Defender's exclusions: Settings → Privacy & security → Windows Security → Virus & threat protection → Exclusions.
- **Terminal.** Prefer PowerShell 7 (`pwsh`) over `cmd.exe`. The repo's conventions and any helper scripts assume PowerShell semicolons over `&&`.

---

## Board labelling convention

Multiple physically identical micro:bit v2 boards are in play (two today, possibly more later, plus the custom nRF5340 PCBA at Phase 4). Telling them apart **physically and in logs** matters more than it sounds — a flashed-the-wrong-board mistake at Phase 3 means a `flight` build running on the ground station.

### Physical labels

Every board carries a **printed sticker on the battery-pack side**, visible while the USB cable is plugged in, with:

```
DRONE-01    or    GROUND-01    or    DRONE-PCBA-01
```

Format: `ROLE-NN`, where:

- `ROLE` ∈ { `DRONE`, `GROUND`, `DRONE-PCBA` } — what the board is *currently* used as. If a board is repurposed, peel and relabel.
- `NN` — zero-padded sequence number within that role. New boards get the next number. Numbers are never reused, even after a board is retired.

Stickers go on with a permanent marker over masking tape if a printer isn't to hand. The point is unambiguity, not aesthetics.

### Logical identification (in firmware / logs)

The nRF52833 has a 64-bit factory device ID (`FICR.DEVICEID[0..1]`). The firmware reads this at boot and:

- Includes it in the boot `defmt` banner: `[boot] role=DRONE-01 deviceid=0xDEADBEEF12345678 build=tethered`.
- Sends it in the first telemetry packet so the ground station can confirm it's talking to the expected board.

A small `board_id` lookup table maps `FICR.DEVICEID` → `ROLE-NN`. Lives in firmware (a `const` array) until the board count grows past trivial, at which point it moves to a config file. Adding a new board = one line in the table + a sticker + a commit.

### Ground-station UI

The ground-station UI prominently displays:

- The connected board's `ROLE-NN` and `deviceid`.
- The build flavour (`bench` / `tethered` / `flight`, per [07-safety.md B.8](07-safety.md)).
- The power-limit state (low / full, per [07-safety.md B.9](07-safety.md)).

A mismatch between expected and actual role is a **hard refuse-to-arm** condition.

---

## Two-board workflow

Day-to-day, the author has both micro:bits plugged into the dev machine via USB (one for the drone role, one for the ground-station / remote role). `probe-rs` distinguishes them by **DAPLink serial number**, which is also on the silver shield on the back of each board.

Probe selection is baked into each firmware crate's `.cargo/config.toml`, so `cargo run` is unambiguous regardless of how many probes are connected:

```toml
# crates/firmware-drone/.cargo/config.toml
[target.thumbv7em-none-eabihf]
runner = "probe-rs run --chip nRF52833_xxAA --probe 0d28:0204:<drone-board-serial>"
```

The remote crate has the same shape with its own serial. The selector format is `<vid>:<pid>:<serial>` — `probe-rs list` prints all three.

Practical notes:

- **Workflow.** `cd crates/firmware-drone && cargo run` always flashes the drone board; `cd crates/firmware-remote && cargo run` always flashes the remote. Use two VS Code integrated terminals (right-click → Rename) so the two `defmt` log streams are obviously distinct in the panel.
- **Serials are committed.** They identify specific physical boards. If you swap which board plays which role — or move to a different machine with different boards — update the two `.cargo/config.toml` files. One-line edit each.
- **Why not env vars.** Cargo's `runner` field is passed literally to the spawned process (no shell, no `${VAR}` expansion). Env-var indirection would need a cross-platform shell wrapper. Per-crate runner with the serial baked in is simpler and survives Windows / Unix equally.
- **Never** rely on "the first probe found" — that's how you flash the wrong board.

Once `xtask` exists ([ADR 0007](decisions/0007-testing-and-ci-strategy.md)), a `cargo xtask flash --role drone` alias may replace the bare `cargo run`, but the per-crate `.cargo/config.toml` will still be the source of truth for which probe is which.

## Build flavour per phase

Maps the phases ([00-vision.md](00-vision.md)) to the Cargo feature flags defined in [07-safety.md B.8](07-safety.md):

| Phase | Drone build | Ground-station build |
|---|---|---|
| 1 | `--features bench` | n/a (Cargo feature) |
| 2 | `--features bench` (bench-restrained, no props); `--features bench` *inside the test enclosure* for full-power spin-up | n/a |
| 3 — tethered | `--features tethered` | n/a |
| 3 — free flight | `--features flight` | n/a |
| 4 | `--features bench` | n/a |
| 5 — tethered | `--features tethered` | n/a |
| 5 — free flight | `--features flight` | n/a |

Ground-station code is a separate binary on the PC and on the second micro:bit — it doesn't carry the bench/tethered/flight axis. Its build flavour question is just debug vs release.

The build script will refuse to compile if zero or more than one of `{bench, tethered, flight}` is enabled. Picking the wrong one is a safety incident waiting to happen ([07-safety.md B.8](07-safety.md)).

---

## Future additions

This doc will grow as bring-up exposes real friction. Anticipated additions:

- Exact `rust-toolchain.toml` and `Cargo.toml` workspace layout.
- `xtask` commands (build, flash, test, lint — per [ADR 0007](decisions/0007-testing-and-ci-strategy.md)).
- Pre-push hook installation steps.
- Pin map for the custom nRF5340 PCBA (Phase 4).
- Recovery procedure for a bricked board (mass-erase via `probe-rs erase`).
