# hardware/electrical/

Electrical and electronic design: schematics, PCB layout, fabrication outputs, wiring diagrams.

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
