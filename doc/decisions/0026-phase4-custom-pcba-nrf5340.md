# ADR 0026 — Phase 4 custom PCBA: nRF5340 module on a hand-designed carrier

- **Status:** Proposed
- **Date:** 2026-08-19
- **Related:** [ADR 0001](0001-platform-airframe-stack.md) (the micro:bit → custom-PCBA migration path this begins), [ADR 0002](0002-mcu-and-language.md) (locked micro:bit v2 for Phases 1–3 and named nRF5340 for Phases 4–5 — this realises that), [ADR 0003](0003-imu-icm42688-spi.md) (IMU carried over), [ADR 0010](0010-board-support-package.md) (the BSP layer a new board slots into), [ADR 0014](0014-radio-protocol-ieee802154.md) (802.15.4, now on the nRF5340 network core), [ADR 0018](0018-pc-link-uart-postcard-cobs.md) (the PC link, simplified by native USB), [ADR 0019](0019-airframe-class-3in-4s-printed.md) (4S airframe class this must power from), [ADR 0006](0006-mechanical-cad-fusion360.md) (mechanical CAD; this adds the electrical-EDA counterpart)

## Context

The micro:bit v2 has carried Phases 1–3: bring-up, four-motor bench work, and the current tethered tuning. It has done its job, but the physical realisation is now the bottleneck — an external IMU breakout, ribbon cables, a second micro:bit, and loose wiring make the airframe clumsy and are actively impeding bench testing. [ADR 0001](0001-platform-airframe-stack.md) and [ADR 0002](0002-mcu-and-language.md) always planned the move to a custom nRF5340 PCBA for Phases 4–5; the clunky packaging is the trigger to commit.

This ADR commits to designing that board and fixes the platform-level choices — module vs bare silicon, programming/debug interface, sensor and ESC interfaces, EDA tool — so the schematic can start ADR-first. It does not finalise the schematic, exact part numbers, power topology, or board layout; those are settled at design time and captured as they land.

The micro:bit is **not** retired by this decision. It remains the flying/tuning platform through the new board's design and fabrication (a long-lead effort); the tuning done on it is the prerequisite that de-risks the new board's bring-up. Design proceeds in parallel with continued Phase 2/3 work, not instead of it.

## Decision

### 1. A pre-certified nRF5340 module on a hand-designed carrier, not bare silicon

The board is built around a **pre-certified nRF5340 module** soldered to a carrier we design in-house, not a bare nRF5340 QFN. The module supplies the parts that are hardest to get right on a first RF board without lab gear: the antenna and its matching network, the 32 MHz crystal, the DC/DC inductors, and the regulatory certification. We design only the carrier — power, sensor buses, ESC interface, connectors, layout — which is where the useful learning is and where first-board risk is manageable.

The 32.768 kHz low-frequency crystal is **not** supplied by the module and is deliberately not fitted on the carrier — see §10.

Bare silicon is rejected for the first board: RF matching and antenna tuning are unforgiving without a VNA and chamber, and getting them wrong fails silently. This stays inside the nRF family per the scope guardrail ([AGENTS.md](../../AGENTS.md)).

### 2. Module selection criteria (specific part deferred)

The chosen module must be:

- **Programmable / open** — runs our own Rust/Embassy image, flashed over SWD (all the third-party modules are just SoC + RF and meet this).
- **Antenna included** (PCB/chip antenna) or a u.FL/IPEX option.
- **Footprint / stock** — a well-documented footprint and reliable availability.

Hand-solderability is **no longer a selection criterion**, because assembly is turnkey (§9): a reflow house handles bottom-pad LGA modules and the QFN IMU just as well as castellated edges. Candidate families (final part chosen at design time): **Raytac MDBT53**, **Fanstel** nRF5340 modules, **u-blox NORA-B1**, **Ezurio (Laird) BL5340**, **Insight SiP** — all in contention now that LGA is not a barrier.

### 3. Programming and debug: SWD header, no onboard USB or DAPLink

The micro:bit carries a second MCU (an nRF52820 running DAPLink) that owns USB and provides CMSIS-DAP debug, drag-and-drop flashing, and a USB-CDC serial bridge. We do **not** replicate that interface chip. Instead:

- **Programming/debug:** expose the nRF5340's **SWD** on a header and use an **external probe** (J-Link, a CMSIS-DAP probe such as the Raspberry Pi Debug Probe, an nRF5340-DK's on-board debugger, or a spare micro:bit as a probe). `probe-rs` drives all of these, unchanged from today.
- **PC / telemetry link:** no USB connector or USB circuitry is included on this board. Telemetry uses the radio link; any wired PC link is deferred.

This drops an entire MCU and its layout from the board. The postcard + COBS framing and shared types remain available to whichever transport is selected for the PC link.

### 4. IMU carried over: ICM-42688-P on SPI

The IMU decision ([ADR 0003](0003-imu-icm42688-spi.md)) stands — ICM-42688-P on SPI, now placed directly on the board instead of a breakout. Known part, known driver, known bring-up.

### 5. ESC interface: DShot

The board commits to **DShot** for the four motor signals (superseding the micro:bit-era PWM), routed to a 4-in-1 ESC connector. This has a firmware consequence (timer + DMA setup on the nRF5340) tracked separately; at the board level it fixes four signal lines and the connector.

### 6. Power from 4S

The board regulates from the 4S pack ([ADR 0019](0019-airframe-class-3in-4s-printed.md), ~16.8 V max) down to the 3.3 V rail. Battery voltage and current telemetry come from the ESC telemetry data; no battery-voltage divider is fitted on the flight controller. Regulator topology (single buck vs staged, or leaning on an ESC BEC) and protection are settled at schematic time (open question).

### 7. EDA tool: KiCad

Electrical design is done in **KiCad**, the counterpart to [ADR 0006](0006-mechanical-cad-fusion360.md)'s Fusion 360 for mechanical. Open-source, scriptable, no license gate, and the idiomatic hobby/OSS-hardware choice. Source (`.kicad_pro`/`.kicad_sch`/`.kicad_pcb`) and fabrication outputs live under `hardware/electrical/`.

### 8. Firmware continuity

The nRF5340 is **dual-core**: the 802.15.4 radio ([ADR 0014](0014-radio-protocol-ieee802154.md)) runs on the network core, application logic on the application core — a shift from the single-core nRF52833. The BSP ([ADR 0010](0010-board-support-package.md)) gains a new Cargo-feature-selected board for the PCBA; because tasks already take erased BSP wrapper types and never see pins, most task code is expected to port behind that feature, with the core-split and DShot output being the genuinely new firmware work.

### 9. Assembly: turnkey PCBA, not hand-assembled

The board is **fabricated and assembled by the vendor** (turnkey PCBA), not hand-soldered. The two hardest joints — the QFN/LGA ICM-42688 with its hidden thermal pad, and the module — are exactly where hand-assembly most often fails and turns bring-up into an "is it the board or my soldering?" hunt; a reflow house does both reliably, and supplies the stencil and paste we would otherwise need.

In practice this is **partial turnkey**: the vendor sources common passives/ICs from its own library, and the RF module (and possibly the IMU) are sourced externally (Digikey/Mouser) and either consigned or ordered on our behalf, since RF modules are often absent from prototype-house parts libraries. Vendor is deferred (§ open questions), but the field is prototype-friendly turnkey houses — the low-cost China route (JLCPCB, PCBWay) or the EU route with easier external-part sourcing and shorter lead to the UK (Aisler, Eurocircuits).

This is a deliberate trade of hand-assembly learning for lower bring-up risk, consistent with the ADR's learning-first-but-achievable framing: the learning budget goes to schematic, power, layout and bring-up rather than to reworking solder joints.

### 10. Clocks: HFXO on-module, no 32.768 kHz crystal fitted

The two oscillators are sourced differently, and the module covers only one of them:

- **HFCLK (32 MHz).** The HFXO crystal is inside the module. It is mandatory for any 2.4 GHz operation ([ADR 0014](0014-radio-protocol-ieee802154.md)) and needs nothing on the carrier.
- **LFCLK (32.768 kHz).** The module does **not** contain an LFXO. Raytac ships a recommended-crystal spec in the MDBT53 design guide, and their own MDBT53-DB-40 reference board fits one externally. We do **not** fit it. LFCLK runs from the internal RC oscillator.

The internal RC is sufficient because nothing in this design needs crystal-grade low-frequency timing. The 802.15.4 radio derives its timing from HFCLK, not LFCLK; LFCLK only drives the RTC behind the Embassy time driver. The ±500 ppm figure usually quoted for the LFXO is a **Bluetooth Low Energy** requirement, and we do not run BLE.

This is confirmed by the existing firmware rather than assumed: the micro:bit boards set `hfclk_source = ExternalXtal` but never set `lfclk_source`, so they have always run LFCLK from the internal RC — through every flight and tuning session to date.

**`P0.00` and `P0.01` are nonetheless reserved** and left unassigned. They are the nRF5340's XL1/XL2 pins, so using them as GPIO would make fitting an LFXO a board respin rather than a rework. Reserving two pins on a module with 48 costs nothing and keeps the option open.

The trade accepted: the internal RC needs periodic calibration against HFCLK to hold roughly ±250 ppm and drifts further over temperature. That is immaterial for a 1 ms control loop but would need revisiting if BLE, long low-power sleep intervals, or long-horizon absolute timing ever appear.

## Consequences

- **Commits to a multi-stage hardware effort** — schematic → layout → fab → assembly → bring-up — with real lead times. Sequenced in parallel with continued micro:bit tuning, not blocking it.
- **A cleaner, integrated airframe** — IMU, MCU, radio, ESC interface and power on one board; the ribbon-cable clutter that triggered this goes away.
- **Extends [ADR 0002](0002-mcu-and-language.md)** from micro:bit (Phases 1–3) to the nRF5340 (Phases 4–5) as always planned; does not supersede it.
- **Simpler than the micro:bit** — the interface chip, USB connector and USB routing are omitted — at the cost of needing an external SWD probe on the bench (which `probe-rs` already uses).
- **New firmware surface:** dual-core bring-up, radio telemetry and DShot output. The BSP abstraction contains the blast radius.
- **First electrical design in the repo** — establishes KiCad and the `hardware/electrical/` layout for all future board work.
- **One fewer part and two fewer pads** from omitting the LFXO, at the cost of a less accurate LFCLK and a constraint to keep `P0.00`/`P0.01` free. Adding the crystal later is a schematic and layout change, not a rework of anything already fitted.
- **Learning-first and achievable** — the module removes the RF/cert risk that would otherwise sink a first board, leaving the tractable, educational parts.
- **Turnkey assembly de-risks bring-up** — the QFN IMU and the module are machine-placed and reflowed, removing hand-solder joints as a failure mode; the cost is external sourcing of the module (partial turnkey) and giving up hand-assembly as a learning exercise (a deliberate trade). It also **widens module choice** — LGA parts (u-blox NORA-B1) are viable again, since hand-solderability no longer gates selection.

## Open questions

- Exact module part number (stock, footprint, antenna type) — §2.
- PCBA vendor and route (low-cost China vs EU sourcing/lead time), and the consign-vs-source logistics for the module — §9.
- Power topology: single buck vs staged, protection, whether to rely on an ESC BEC — §6.
- Mechanical mounting standard (20×20 vs 30.5×30.5) for the 3" class, and stack vs all-in-one layout.
- Whether to include a barometer / other sensors on this revision or keep it minimal.
- Board bring-up order and a power-on checklist (its own doc when layout is done).
