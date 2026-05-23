# ADR 0006 — Mechanical CAD: Fusion 360

- **Status:** Accepted
- **Date:** 2026-05-23
- **Related:** [00-vision.md](../00-vision.md), [ADR 0001](0001-platform-airframe-stack.md)

## Context

3D-printed parts appear from Phase 1 onwards: IMU mount on the micro:bit, motor-clamp bench fixture, rigid 4-motor bench mount in Phase 2, flight frame in Phase 3, custom-PCBA carrier in Phase 4, etc. We need a single CAD tool committed early so parts are interchangeable, version-controllable, and not trapped in a format we can't open later.

The user is already familiar with **Fusion 360** and has successfully modelled and printed parts before. The question is whether that familiarity is the deciding factor or whether a better-fit alternative justifies the switching cost.

Candidates considered:

- **Fusion 360.** Industry-standard hobby/prosumer parametric CAD. Free personal-use tier (with caveats; see below). User already proficient.
- **FreeCAD 1.0+.** Open source, free forever, no vendor lock-in. The 1.0 release (late 2024) finally addressed the longstanding topological-naming problem and is genuinely usable. UX still rougher than Fusion; learning curve from scratch.
- **OnShape.** Cloud-native, slick UX, free *only if all documents are public*. Acceptable for a public hobby project, awkward the moment privacy is wanted.
- **OpenSCAD.** Code-based parametric CAD. Excellent for simple geometric brackets (motor mounts, standoffs), text-files version-control beautifully, terrible for organic shapes (canopies, fairings).
- **Solidworks / Inventor / Plasticity.** Wrong cost / scope tier for a hobby project.

## Decision

- **Fusion 360** is the mechanical CAD tool for the project.
- **Source files (`.f3d`) live in the repo** alongside firmware and docs, under a top-level `Mech/` (or equivalent) folder when the first part lands.
- **Every committed `.f3d` ships with an exported `.step`** alongside it. STEP is the vendor-neutral interchange format; this is our escape hatch if Autodesk's free tier becomes unworkable.
- **Print-ready `.stl` (or `.3mf`) files are also committed** for parts we've actually printed, so anyone (including future-us on a fresh machine) can re-print without re-running CAD.

## Why Fusion 360

- **Familiarity dominates.** This is a hobby learning project where the *firmware and electronics* are the learning target. CAD is a means to an end. Spending the early phases re-learning a new CAD tool would burn time on the wrong problem.
- **Successful track record.** User has already shipped working printed parts in Fusion. Known-good toolchain.
- **Mainstream format support.** STEP, STL, 3MF export. The output files are portable even if the source isn't.
- **Reasonable hobby tier.** Free personal-use license covers everything we need.

## Why not the alternatives (now)

- **FreeCAD.** Genuinely tempting on principle (open source, no vendor risk). Real cost is the learning-curve tax during exactly the phases where we want focus on firmware. Reconsider if Autodesk meaningfully degrades the free tier.
- **OnShape.** Public-documents-only restriction is a soft no for a project that may include build details we don't want indexed and republished by AI scrapers.
- **OpenSCAD.** May still appear *alongside* Fusion for trivial parametric brackets if it's the obviously-right tool for a specific part. Not the primary CAD.

## Consequences

### What this commits us to

- A `Mech/` folder structure for committing CAD sources, STEP exports, and print-ready meshes. Layout decided when the first part lands.
- Committing both source (`.f3d`) and neutral (`.step`) for every meaningful part — small ongoing discipline tax, big portability win.
- A future hardware / mechanical doc (`Doc/03-mechanical.md` or similar) when the parts count justifies one.

### What this rules out (for now)

- Treating CAD as out-of-scope for the repo. Parts that exist physically must exist in version control.
- Source-file lock-in. The STEP-alongside-source rule means we are never one Autodesk policy change away from losing the geometry.

### What stays open

- **Folder layout** for `Mech/` — decided when the first part lands.
- **Slicer choice** (PrusaSlicer / OrcaSlicer / Bambu Studio / Cura) — orthogonal to CAD; printer-dependent; not worth an ADR.
- **Whether to also use OpenSCAD** for trivial parametric brackets — case-by-case, no global rule.
- **Migration plan to FreeCAD** if Autodesk's free tier becomes unworkable. The STEP exports are the contingency; the trigger and process are not pre-decided.
