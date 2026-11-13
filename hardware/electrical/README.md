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

### Application-core firmware

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
The app starts the external high-frequency crystal and creates the BSP and
flight tasks. Before spawning tasks, it checks the radio-core IPC loopback.
Embassy's default debug configuration allows access and may write UICR
to disable application and secure debug protection. P0.03 is configured as
GPIO rather than NFC for motor M1. The app BSP hands P0.31 (the radio LED) to
the network core in `Board::new`, consuming its pin singleton so no app-core
driver can claim it.
`cargo run` requires probe-rs 0.32.0 or later on PATH, selects
`nRF5340_xxAA_APPONLY`, and flashes via the Pico at 100 kHz.
probe-rs 0.30.0 has a known nRF53 recovery bug (upstream PR 3701).
The app-only target avoids accessing the network core over SWD.
Board power and a stable TC2050 contact are required.

For the bare-metal motor-pin bench test, disconnect the ESC and power the board
separately. Run `cargo run --example motor-gpio-test` from the app crate. It
disables NFCT and drives M1 (P0.03) low then high, about 2 s each, with the
input buffer connected. The status LED shows the value read back from the pin,
so it should alternate in step with the drive; M1 should measure about 0 V and
3.3 V. If UICR still selects NFC pins, the LED stays on and the pins are not
configured. The test restores the UICR debug-protection settings at startup
without modifying UICR. Flash the normal app to restore DShot operation.

The v2 IMU mounting requires a 180-degree rotation about Y into the FRD body
frame: `(body_x, body_y, body_z) = (-sensor_x, sensor_y, -sensor_z)`, for both
accelerometer and gyro. Bench tilt and rotation checks confirmed this mapping
on 2026-11-12. The app BSP applies it during byte conversion, before calibration
or estimation. Level and stationary should read approximately `(0, 0, -1)` g.

The app status LED on P0.28 is active-high: the PCB connects its anode to the
GPIO and its cathode through a resistor to ground. It starts off and displays
the supervisor-state blink pattern. Repeated publications of the same state
do not restart the pattern; only a different state interrupts it.

### Network-core radio bridge

The [radio crate](../../crates/firmware-drone-nrf5340-radio/Cargo.toml) selects
the `nrf5340-net` Embassy feature and the `thumbv8m.main-none-eabi` Rust target
(the network core has no FPU). Build it separately from the other firmware by
running `cargo build` from `crates/firmware-drone-nrf5340-radio`. Its ELF links
to network flash at `0x01000000`. Its runner selects the full
`nRF5340_xxAA` target and the Pico probe; network-core debug access and flashing
were verified with the previous LED smoke-test images. The current image is
a raw 802.15.4 payload bridge; it depends on `nrf5340-ipc` but has no postcard
or `firmware-types` dependency. Its `board` module (`src/board.rs`) owns the
clock setup, radio interrupt binding and pin map, and exposes the radio, the
status LED on P0.31 and an `AppCoreInterface` wrapping the IPC endpoint. The
entry point creates the board and independently spawns `tasks::status_led` and
`tasks::remote_link`. The LED is a heartbeat, not an IPC-readiness indicator.
The remote-link task bridges raw payloads; startup probes from the app are
answered inside `nrf5340-ipc` and never reach it. Each task lives in its own
file under the radio crate's `src/tasks/`.

### Inter-core radio transport

Implemented 2026-11-12; **working on hardware**. After a power-cycle both LEDs
blink, the app logs `radio-core IPC loopback passed`, the network core logs
`app-core IPC probe answered`, and command/telemetry traffic flows with the
micro:bit remote on channel 20. Network was flashed first, then app, using
probe-rs 0.32.0 without chip-wide recovery. Both cores build and pass Clippy
separately; the queue has host tests:
`cargo test -p nrf5340-ipc` and
`cargo clippy -p nrf5340-ipc --all-targets -- -D warnings`.

- The concrete app BSP `RadioCoreInterface` lives alongside the other wrappers
  in `src/board/nrf5340.rs` and wraps `nrf5340-ipc`; the radio BSP's
  `AppCoreInterface` is its network-core counterpart. The micro:bit
  implementation is unchanged. Postcard and command dispatch stay on the app
  core; Embassy's radio `Packet` stays on the network core.
- Both linkers reserve `0x20070000..0x20080000`. The first 1076 bytes contain
  a version/magic word and two four-slot SPSC queues. Each slot contains a
  length and up to 125 payload bytes, packed into atomic `u32` words. The PHY
  header, CRC and LQI are not sent through IPC.
- The app holds the network core off, configures SPU RAM regions 56--63 as
  non-secure read/write, initializes both queues, configures IPC and releases
  the network core. The network domain is non-secure; no access to the rest of
  the app's RAM is granted. Both binaries check the linker region and the
  shared layout size.
- IPC channel/event 0 is reserved on both cores. A doorbell follows each
  enqueue/dequeue. Acquire/release publication and a `DMB` before notification
  order memory accesses. The interrupt wakes the local async waiter; the
  queue, not the event bit, determines whether data is available.
- At startup the app sends `drone-ipc-v1` through the real frame queues and
  waits up to two seconds for the echo. On success the app logs
  `radio-core IPC loopback passed`; a mismatch or timeout stops app startup.
  The network
  core has no startup phase or timeout: `nrf5340-ipc`'s network-side
  `Endpoint::receive` echoes the probe whenever one arrives (logging
  `app-core IPC probe answered`) and returns only other frames. Both halves of
  the handshake live in `nrf5340-ipc`; the probe bytes are private to it. Both
  log lines were observed on hardware on 2026-11-12.
- `receive()` returns the actual frame length and is cancellation-safe.
  `send()` means **queued**, not successfully transmitted over the air.
  A full app-to-net queue waits at most 50 ms; oversize payloads fail
  explicitly. A full net-to-app queue drops the incoming frame with a warning
  rather than blocking outgoing radio traffic. On-air send errors are logged
  on the network core; IPC does not add retries or delivery guarantees.
- Chip-temperature sampling was removed from the app. It logs the absence
  and seeds temperature with NaN, keeping the unchanged sensors aggregator
  and telemetry schema live without inventing a measurement.

Before hardware testing, disconnect motors or remove propellers. Use matching
app and network images, then cold-boot both cores; do not combine this app
with the old network LED-only image. An independent network-core reset or
reflash reattaches to the existing queues without panicking, and frames sent
by the app while it was down may be transmitted late. The app does not
re-probe, so restart the app (or power-cycle) to re-run the startup check.
A startup failure leaves the BSP's continuous motor
STOP frames running but does not verify physical motor behavior.

Remaining hardware acceptance: measure round-trip latency through the extra
IPC hop and test link-loss behavior props-off. Working command/telemetry
traffic is not flight-readiness evidence.

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
two-core target. Both runners were tested with the previous smoke-test images:
each printed its own boot message, and both images verified unchanged after
running the other core. Both LEDs blinked after a subsequent cold boot.
The current IPC/radio images were verified on hardware on 2026-11-12. Stop the
RTT monitor with Ctrl+C.

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
