# hardware/mechanical/

Mechanical CAD: airframe, motor mounts, sensor mounts, test enclosure, anything 3D-printed or machined.

**Tool:** Fusion 360 ([ADR 0006](../../doc/decisions/0006-mechanical-cad-fusion360.md)).

**Committed file types** (per ADR 0006):

- `*.f3d` — Fusion 360 native source. The authoritative copy.
- `*.stl` / `*.3mf` — print-ready meshes for parts that go to the printer. Mark slicer settings in the filename or a sibling note if it matters.

**Naming:** a descriptive part name with matching stems across file types. Add a `-vN` suffix if you keep multiple generations of a part, and bump it when the geometry changes meaningfully.

## Parts

| Part | Files | Notes |
| --- | --- | --- |
| Simple Motor Mount | [`.f3d`](Simple%20Motor%20Mount.f3d) (Fusion source), [`.stl`](Simple%20Motor%20Mount.stl) (print mesh) | First printed part. Flat plate carrying the iFlight XING2 1404 (M2) motor bolt pattern. |
