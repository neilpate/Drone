# ADR 0028 — Fabricate the v2 flight-controller board (PCBWay turnkey)

- **Status:** Proposed
- **Date:** 2026-09-03
- **Related:** [ADR 0026](0026-phase4-custom-pcba-nrf5340.md) (the platform-level decision this executes — module vs silicon, SWD, DShot, turnkey assembly, no LFXO; **this ADR supersedes 0026's Aisler vendor selection**), [ADR 0003](0003-imu-icm42688-spi.md) (IMU on the board), [ADR 0014](0014-radio-protocol-ieee802154.md) (radio the module carries), [ADR 0019](0019-airframe-class-3in-4s-printed.md) (4S the board powers from), [ADR 0002](0002-mcu-and-language.md) (micro:bit → nRF5340 migration this advances)

## Context

[ADR 0026](0026-phase4-custom-pcba-nrf5340.md) fixed the platform-level choices for the Phase 4 flight controller: a pre-certified nRF5340 module on a hand-designed carrier, SWD for debug, DShot for the ESC, turnkey assembly, and no fitted LFXO. It deliberately did not commit to fabricating a specific board — schematic, layout, vendor logistics, and quantities were left to design time.

That design work has since happened. The board is laid out in KiCad as `drone_fc` **v2** under [hardware/electrical/v2/](../../hardware/electrical/v2/): a Raytac MDBT53 (nRF5340, PCB antenna) module, an ICM-42688-P IMU on SPI, an 8-pin JST-SH 4-in-1 ESC interface, an LMR50410 buck plus a 3.3 V LDO regulating from 4S, native USB, and an SWD debug header. A later **v3** revision was started but never finished; it was removed (2026-09-03) because carrying an incomplete second project alongside the fabricated one is a source of confusion, not value. v2 is the revision that goes to fabrication.

This ADR records the concrete build commitment that 0026 left open: which board, which vendor, how many, at what cost, and what the second assembled board can and cannot become.

## Decision

### 1. Fabricate the v2 board, not v3

The revision sent to fabrication is `drone_fc` **v2** ([hardware/electrical/v2/](../../hardware/electrical/v2/)). v3 was an unfinished exploration and has been deleted. There is no partial second project to reconcile against; v2 is the single source of truth for this build.

### 2. Vendor: PCBWay turnkey PCBA (supersedes ADR 0026's Aisler selection)

The board is fabricated and assembled by **PCBWay** as a turnkey PCBA. This **supersedes [ADR 0026](0026-phase4-custom-pcba-nrf5340.md) §Consequences, which named Aisler** (2026-08-31).

The deciding factor is the same one 0026 weighed — external sourcing of the RF module — resolved the other way in practice: PCBWay quoted the full turnkey build *including* sourcing and placing the Raytac module, which removes the consign-the-module logistics that drove the original Aisler preference. The turnkey rationale from [ADR 0026 §9](0026-phase4-custom-pcba-nrf5340.md) is unchanged: a reflow house places the QFN/LGA IMU and the module reliably, keeping hand-solder joints out of bring-up.

### 3. Quantities: 5 fabricated, 2 assembled, 8 modules

- **5 bare boards** — PCBWay's minimum fabrication quantity.
- **2 assembled** — one flight unit and one spare. The spare has standalone value given the crash rate this project expects; a populated flight controller in a drawer is cheap insurance against a shattered board (compare [ADR 0019](0019-airframe-class-3in-4s-printed.md) / the PLA-shatter experience).
- **8 Raytac modules** — the module's minimum order quantity.

This leaves 3 spare bare boards and 6 spare modules. The spare boards enable a later hand or turnkey second-batch build; the spare modules are stock for future revisions. The MOQ overhang is accepted, not engineered around — it is cheaper to hold the surplus than to chase a smaller order.

### 4. Cost

_Figures to be filled from the final PCBWay quote and module order._

| Line item | Qty | Unit | Total |
| --- | ---: | ---: | ---: |
| PCB fabrication (5 boards) | 5 | £TBD | £TBD |
| Turnkey assembly (2 boards) | 2 | £TBD | £TBD |
| Raytac MDBT53 modules | 8 | £TBD | £TBD |
| Shipping / import | — | — | £TBD |
| | | **Total** | **£TBD** |

Cost is recorded here rather than left implicit so the learning-project spend is honest and future revisions have a baseline to compare against.

### 5. The spare can become a remote only via an external ADC

The second assembled board is primarily a spare flight controller. It can *also* serve as the radio remote ([ADR 0002](0002-mcu-and-language.md) currently assigns that role to the second micro:bit), because it is an identical nRF5340 — same antenna, same 802.15.4 PHY — which is the ideal both ends of the link. But not by wiring joysticks directly:

- The four ESC/DShot pins on v2 are **P0.03, P0.16, P0.17, P0.22** (M1–M4). **None are SAADC-capable.**
- The nRF5340's analog inputs are the fixed set AIN0–AIN7 (P0.04–P0.07, P0.25–P0.28). Of these, only four are broken out on v2, and all four are already used — P0.04 (current sense) and P0.27 (VBAT sense) as analog, P0.07 (SPI SCK) and P0.28 (status LED) as digital.

So there is **no path to four analog stick axes on this board**, and adding one needs a respin. The route that needs no respin: the ESC connector exposes five usable digital GPIO (the four motor pins plus P0.08), enough to hang an **external I²C/SPI ADC** (e.g. ADS1115 / MCP3008) that reads the potentiometers, with spare GPIO for arm/mode buttons. The remote is therefore a "spare board plus a small external-ADC harness on the ESC connector," not a direct-wire conversion.

## Fabrication file preparation (implementation notes)

Getting the board to a fabricable state needed two pieces of tooling beyond the KiCad project itself, kept under [hardware/electrical/tools/](../../hardware/electrical/tools/).

### Module footprint: converted from Raytac's Eagle library (vendor-neutral)

The Raytac MDBT53 is not in any stock KiCad library, so its footprint was generated from Raytac's manufacturer-supplied **Eagle** package by [convert_mdbt53_eagle.py](../../hardware/electrical/tools/convert_mdbt53_eagle.py). Three things had to be handled and are worth recording, because they bite silently:

- **Y-axis flip.** Eagle measures Y upward, KiCad downward. Copying pad/graphic coordinates verbatim mirrors the part — a left-right-flipped module that passes visual inspection but fails at assembly. The script negates every Y.
- **Pad-count assertion.** The converter asserts exactly 65 pads before writing anything, so a partial or wrong XML parse fails loudly instead of producing a plausible-looking short footprint.
- **Courtyard / antenna keepout drawn by hand.** Eagle's layer 41 carries RF keepout notes rather than a courtyard outline, so the courtyard rectangle and the "ANTENNA AREA" keepout are added explicitly rather than copied.

This work is independent of the fab house and stands whatever vendor is chosen.

### CPL / BOM reformatting: these outputs are JLCPCB-format

KiCad's native exports do not match a fab assembly template directly, so two scripts reshape them:

- [convert_cpl_for_jlcpcb.py](../../hardware/electrical/tools/convert_cpl_for_jlcpcb.py) turns KiCad's `Ref,Val,Package,PosX,PosY,Rot,Side` placement export into JLCPCB's `Designator,Mid X,Mid Y,Layer,Rotation` — renaming columns, normalising rotation to `% 360`, and capitalising the layer name.
- [make_minimal_bom.py](../../hardware/electrical/tools/make_minimal_bom.py) reduces the KiCad BOM to JLCPCB's `Comment,Designator,Footprint`, stripping the footprint-library prefix.

The committed [drone_fc-cpl-jlcpcb.csv](../../hardware/electrical/v2/) and [drone_fc-bom-minimal.csv](../../hardware/electrical/v2/) are therefore **JLCPCB-format**, produced while JLCPCB was still a candidate vendor. They do **not** transfer directly to a PCBWay turnkey order:

- PCBWay turnkey sources parts against **manufacturer part numbers**. The JLCPCB-minimal BOM carries none — but the full KiCad export [drone_fc-bom.csv](../../hardware/electrical/v2/) (`Refs,Value,Footprint,MPN,Qty,DNP`) does, so a PCBWay BOM should be built from that, not the minimal one.
- Gerbers for PCBWay were exported separately ([drone_fc-pcbway-gerbers.zip](../../hardware/electrical/v2/)); the placement/BOM files still need regenerating to PCBWay's expected format before the order.

## Consequences

- **Commits real spend and lead time** on 5 boards, 2 assemblies, and 8 modules — sequenced in parallel with continued micro:bit tuning, per [ADR 0026](0026-phase4-custom-pcba-nrf5340.md), not blocking it.
- **Supersedes the Aisler choice** in [ADR 0026](0026-phase4-custom-pcba-nrf5340.md); that line is marked superseded there.
- **v3 is gone** — one board revision in the tree, no ambiguity about what was built.
- **MOQ overhang:** 3 spare bare boards and 6 spare modules on the shelf. Not waste — stock for the next revision or a hand-built second batch.
- **A flight-ready spare** exists from day one, and a remote is reachable from it without a respin via an external ADC (§5).
- **Establishes the fabrication baseline** — vendor, quantities, and cost for this board are on record for future revisions to compare against.

## Open questions

- Final cost figures (§4) — pending the PCBWay quote and module order.
- Lead time and the module consign-vs-vendor-source logistics with PCBWay.
- **Regenerate the placement and BOM files in PCBWay's required format** before ordering — the committed CPL/BOM are JLCPCB-format, and a PCBWay turnkey BOM must be built from the MPN-bearing [drone_fc-bom.csv](../../hardware/electrical/v2/), not the minimal JLCPCB BOM (see fabrication file notes).
- **Pre-fab verification of the reversed 8-pin JST-SH ESC pinout** — the v2 symbol numbers pin 1 as the motor end, not the power end ([hardware/electrical/parts-list.md](../../hardware/electrical/parts-list.md)). Getting VBAT (pin 8, up to 16.8 V) onto an MCU GPIO is destructive, not merely confusing. Continuity check from the ESC battery pad through the harness before the order is placed.
- Board bring-up order and a power-on checklist — its own doc once boards arrive.
