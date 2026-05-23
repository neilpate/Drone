# hardware/mechanical/

Mechanical CAD: airframe, motor mounts, sensor mounts, test enclosure, anything 3D-printed or machined.

**Tool:** Fusion 360 ([ADR 0006](../../doc/decisions/0006-mechanical-cad-fusion360.md)).

**Committed file types** (per ADR 0006):

- `*.f3d` — Fusion 360 native source. The authoritative copy.
- `*.step` — neutral exchange format. Re-exported on every meaningful change so the design is readable without Fusion.
- `*.stl` / `*.3mf` — print-ready meshes for parts that go to the printer. Mark slicer settings in the filename or a sibling note if it matters.

**Naming:** `partname-vN.{f3d,step,stl,3mf}`. Bump `N` when the geometry changes meaningfully; don't bump for cosmetic edits.

Empty until the first part lands.
