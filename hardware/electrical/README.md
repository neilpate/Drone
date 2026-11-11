# hardware/electrical/

Electrical and electronic design: schematics, PCB layout, fabrication outputs, wiring diagrams.

## SWD probe bench test

The [Pico SWD probe bring-up note](swd-probe-bringup.md) records the investigation
and links the preserved scope screenshot and raw waveform data.

The host-only Rust tool [swd-test](../../crates/swd-test/src/main.rs) checks outgoing
SWD electrical activity with the target board **disconnected**. It uses only the
standard library and calls the installed `probe-rs` executable.

- Connect CH1 to SWCLK, CH2 to SWDIO, and both probe grounds to probe-board GND.
- Match the scope's attenuation settings to the physical probe switches.
- Defaults: Rigol DHO804 at `192.168.2.30:5555`, CMSIS-DAP probe selector
  `2e8a:000c:E66594D0637D4534`. Optional positional arguments override these.
- Run from the repository root:
  `cargo run --manifest-path crates/swd-test/Cargo.toml`.
- The tool changes channel scales/coupling, timebase, acquisition, and trigger
  settings, captures both channels, and leaves the scope stopped.
- It checks low/high levels (1st/99th percentiles) against 3.3 V logic
  thresholds, transitions on both lines, and median SWCLK frequency against
  100 kHz (+/-20%), with at least 20 samples per period. SWDIO levels are
  measured over the clock-active window so long idle periods do not mask bits.
- It samples SWDIO on SWCLK rising edges and verifies the initial sequence:
  at least 50 high clocks, JTAG-to-SWD selection `0xE79E` (LSB first), another
  at least 50 high clocks, at least two low idle clocks, then DPIDR read request
  `0xA5` with valid start, stop, park, and even parity bits.
  Exit code 0 means all electrical and initial-sequence checks passed.
- A `probe-rs` no-target error is expected. A passing electrical test does not
  verify later transfers, target connectivity, or the absence of brief
  overshoot/ringing. The ACK-slot value is reported but is not a target
  acknowledgement with the target disconnected. Inspect edges on the scope
  separately.
- Validate the tool with `cargo test --manifest-path crates/swd-test/Cargo.toml`
  and `cargo clippy --manifest-path crates/swd-test/Cargo.toml --all-targets -- -D warnings`.

## Electrical design files

### Application-core LED smoke test

The [nRF5340 app crate](../../crates/firmware-drone-nrf5340-app/Cargo.toml)
is a workspace member (ADR 0029) but builds separately from micro:bit firmware. It selects the
`nrf5340-app-s` Embassy HAL feature and targets the secure application core
with `thumbv8m.main-none-eabihf`.
Do not build it together with the micro:bit firmware: Cargo combines dependency
features within one build, and the two Embassy chip features are incompatible.
The repository's VS Code rust-analyzer settings focus on the app and scope
build-script and Clippy checks to it. The micro:bit and groundstation require
separate editor configuration when working on them.
Run Cargo from `crates/firmware-drone-nrf5340-app` so its local target/linker
configuration is loaded: `cargo build`, then `cargo clippy --bin firmware-drone-nrf5340-app -- -D warnings`.
The app starts the external high-frequency crystal and toggles P0.28 every
100 ms using Embassy's RTC1 time driver and a blocking delay. Embassy's
default debug configuration allows access and may write UICR
to disable application and secure debug protection. The app assigns P0.31 to
the network core and releases it from force-off.
`cargo run` requires probe-rs 0.32.0 or later on PATH, selects
`nRF5340_xxAA_APPONLY`, and flashes via the Pico at 100 kHz.
probe-rs 0.30.0 has a known nRF53 recovery bug (upstream PR 3701).
The app-only target avoids accessing the network core over SWD.
Board power and a stable TC2050 contact are required.

### Network-core LED smoke test

The [radio crate](../../crates/firmware-drone-nrf5340-radio/Cargo.toml) selects
the `nrf5340-net` Embassy feature and the `thumbv8m.main-none-eabi` Rust target
(the network core has no FPU). Build it separately from the other firmware by
running `cargo build` from `crates/firmware-drone-nrf5340-radio`. Its ELF links
to network flash at `0x01000000`. Its runner selects the full
`nRF5340_xxAA` target and the Pico probe; network-core debug access and flashing
have been verified, and both LEDs blink after a cold boot.

### Running each core

From the relevant crate directory, `cargo r` builds, flashes and monitors that
core with defmt RTT:

```powershell
cd crates\firmware-drone-nrf5340-app
cargo r
```

Or, starting from the repository root:

```powershell
cd crates\firmware-drone-nrf5340-radio
cargo r
```

The app uses the application-only probe-rs target; the radio uses the full
two-core target. Both runners were tested: each printed its own boot message,
and both images verified unchanged after running the other core. Both LEDs
blinked after a subsequent cold boot. Stop the RTT monitor with Ctrl+C.

The radio runner requires a working app image to start and release the network
core. If probe-rs reports core 1 locked, check that the board is powered, the
app is running, and the TC2050 has good contact. Do not automatically grant an
erase: that message alone does not establish why access is unavailable.

### Flashing without an RTT console

The following Windows PowerShell commands build and flash one image at a time
using the Pico probe. They verify the written image and do not request a
chip-wide erase or permission to recover locked cores. Run from the repository
root. Only run `probe-rs download` if `cargo build` succeeds:

```powershell
cd crates\firmware-drone-nrf5340-app
cargo build
probe-rs download --chip nRF5340_xxAA_APPONLY --probe 2e8a:000c:E66594D0637D4534 --protocol swd --speed 100 --verify ..\..\target\thumbv8m.main-none-eabihf\debug\firmware-drone-nrf5340-app
```

For the network core, start from the repository root again:

```powershell
cd crates\firmware-drone-nrf5340-radio
cargo build
probe-rs download --chip nRF5340_xxAA --probe 2e8a:000c:E66594D0637D4534 --protocol swd --speed 100 --verify ..\..\target\thumbv8m.main-none-eabi\debug\firmware-drone-nrf5340-radio
```

After flashing either image, release the probe and power-cycle the board. The
app must run and release the network core before full-target access can work.
App flashing followed by verification of the unchanged network image, and
network flashing followed by verification of the unchanged app image, were
tested on the board. The TC2050 contact is intermittent: if SWD fails during
flashing, re-establish contact and verify or reflash the affected image.

The full target can report a core as locked when its debug access port is
unavailable. Recovery with `--allow-erase-all` can erase either core's image;
restore both images if recovery reports both cores locked. Do not use a
chip-wide erase to work around an access failure without first diagnosing it.
`probe-rs download` is distinct from `cargo run`: it does not start an RTT
console.

**Primary content (Phase 4 onwards):** custom nRF5340 carrier-board PCBA — the successor to the BBC micro:bit v2 for Phases 4–5 ([ADR 0002](../../doc/decisions/0002-mcu-and-language.md)).

**Tool:** KiCad (assumed, not yet ADR'd — decision lands with the Phase 4 PCBA design ADR).

**Expected committed file types:**

- `*.kicad_pro`, `*.kicad_sch`, `*.kicad_pcb`, `*.kicad_sym`, `*.kicad_mod` — KiCad project, schematic, board, symbol library, footprint library.
- `*.step` — 3D export of the assembled board, for mechanical fit checks against `hardware/mechanical/`.
- Fabrication outputs (Gerbers, drill files, BOM, pick-and-place) under a `fab/` subfolder per board revision, generated on tag.

**Naming:** by board, e.g. `flight-controller-rev-a/`, `flight-controller-rev-b/`. New revision = new folder, never overwrite.

## v2 Module Symbol

The custom `MDBT53_P1M` symbol in [v2/drone_fc_v2_symbols.kicad_sym](v2/drone_fc_v2_symbols.kicad_sym) includes all 65 module pads, checked against the [Raytac Rev. E datasheet](../../doc/hardware/%5BnRF5340%5D%20MDBT53-1M%20%26%20MDBT53-P1M_Ver.E%20spec.pdf), section 2.5 (pages 15-18). (2026-09-05)

- U3A retains the existing wired pin positions; U3B exposes the remaining GPIO; U3C exposes the regulator connections. These are units of one physical module, not separate components.
- GPIO pins use manufacturer names and bidirectional electrical types. Application functions belong on schematic net labels, not in the library pin names. Refer to the datasheet for fixed alternate functions such as analog inputs, NFC, crystal, QSPI, and high-speed SPI.
- Every unused pad has an explicit schematic no-connect. U3C follows normal-voltage LDO mode (section 8.1.1): VDD and VDDH are tied to 3.3 V, and DCCH, DCCD, DECD, DCC, and DECR are unconnected. Firmware must not enable DC/DC without the required external circuit.
- The library and schematic copies are synchronised. All pre-existing electrical connections were preserved when completing the symbol. The PCB and manufacturing outputs were not updated; the earlier schematic LED polarity correction remains separate from the manufactured board.
