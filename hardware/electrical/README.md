# hardware/electrical/

Electrical and electronic design: schematics, PCB layout, fabrication outputs, wiring diagrams.

**Primary content (Phase 4 onwards):** custom nRF5340 carrier-board PCBA — the successor to the BBC micro:bit v2 for Phases 4–5 ([ADR 0002](../../doc/decisions/0002-mcu-and-language.md)).

**Tool:** KiCad (assumed, not yet ADR'd — decision lands with the Phase 4 PCBA design ADR).

**Expected committed file types:**

- `*.kicad_pro`, `*.kicad_sch`, `*.kicad_pcb`, `*.kicad_sym`, `*.kicad_mod` — KiCad project, schematic, board, symbol library, footprint library.
- `*.step` — 3D export of the assembled board, for mechanical fit checks against `hardware/mechanical/`.
- Fabrication outputs (Gerbers, drill files, BOM, pick-and-place) under a `fab/` subfolder per board revision, generated on tag.

**Naming:** by board, e.g. `flight-controller-rev-a/`, `flight-controller-rev-b/`. New revision = new folder, never overwrite.

Empty until Phase 4.
