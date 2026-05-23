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

Day-to-day, the author has both micro:bits plugged into the dev machine via USB (one for flashing the drone, one for the ground-station role). `probe-rs` distinguishes them by **DAPLink serial number**, which is on the silver shield on the back of each board.

- Record each board's DAPLink serial in the `board_id` table alongside `FICR.DEVICEID` and `ROLE-NN`.
- `cargo run` / `cargo flash` invocations specify the probe selector explicitly: `--probe <vid>:<pid>:<serial>` or a shorter alias once `xtask` exists.
- **Never** rely on "the first probe found" — that's how you flash the wrong board.

---

## Future additions

This doc will grow as bring-up exposes real friction. Anticipated additions:

- Exact `rust-toolchain.toml` and `Cargo.toml` workspace layout.
- `xtask` commands (build, flash, test, lint — per [ADR 0007](decisions/0007-testing-and-ci-strategy.md)).
- Pre-push hook installation steps.
- Pin map for the custom nRF5340 PCBA (Phase 4).
- Recovery procedure for a bricked board (mass-erase via `probe-rs erase`).
