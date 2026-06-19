# ADR 0019 — Airframe and propulsion class: 3" ducted, 4S, fully 3D-printed frame

- **Status:** Proposed
- **Date:** 2026-06-14
- **Related:** [ADR 0001](0001-platform-airframe-stack.md) (quadcopter, real hardware, learning-first), [ADR 0006](0006-mechanical-cad-fusion360.md) (Fusion 360 for mechanical CAD), [00-vision.md](../00-vision.md) "Definition of success" (Phase 3 commits flight hardware)

## Context

[ADR 0001](0001-platform-airframe-stack.md) committed to a real-hardware quadcopter with rolled-from-scratch firmware, but explicitly left frame class, motors, ESCs, battery, and radio link open. The Phase 3 milestone in [00-vision.md](../00-vision.md) requires those choices to be *committed* before flight hardware is bought, because every part downstream of the frame depends on the prop class and cell count.

Phase 1 firmware bring-up is in progress on bench hardware (micro:bit v2 + ICM-42688 + bench PSU, no airframe). Bench testing remains workable for several more weeks without an airframe. But three things make this the right time to lock the class in:

- **Motor / ESC product cycles take weeks.** Specific parts go out of stock; deciding the *class* now lets the eventual order happen on a known catalogue.
- **The frame is going to be designed in-house and 3D-printed**, not bought. The frame design depends on motor mounting hole pattern and AUW estimate, both of which are pinned by the class.
- **The bench enclosure** required by Phase 2 ([07-safety.md A.8](../07-safety.md), referenced from 00-vision) needs to be sized for the airframe. Building the enclosure ahead of the first props-on full-power run only works if the airframe size is known.

The relevant axes are:

1. **Prop class** (2.5" / 3" / 3.5" / 4" / 5" / 7"). Drives motor stator size, ESC current rating, battery capacity, frame size, noise, safety profile, and how badly a crash hurts.
2. **Battery cell count** (3S / 4S / 6S). Drives motor KV, ESC voltage rating, and ecosystem breadth.
3. **Frame construction** (carbon-fibre kit / hybrid / fully printed). Drives stiffness, weight, crash cost, and how much of the build the author owns.
4. **Frame style** (naked X / ducted / cinewhoop). Drives safety, noise, and structural stiffness of the printed frame.

All four are coupled — it's one decision, not four.

## Decision

The first flight airframe is:

- **Prop class: 3"** (three-inch propellers, tri-blade).
- **Battery: 4S LiPo** (~14.8 V nominal, capacity 450–650 mAh, XT30 connector).
- **Motors: 1507-class brushless**, ~3500–3800 KV (specific part number deferred to ordering time).
- **ESC: 4-in-1 stack ESC**, 25–35 A continuous on 4S, DShot300/600 capable, 20×20 mm or 25.5×25.5 mm mounting standard (specific part number deferred).
- **Frame style: ducted** (cinewhoop-style), four ducts on a centre plate.
- **Frame construction: fully 3D-printed**, in PETG (PLA acceptable for prototypes; PETG for the build that flies). Modular: separate base / duct / canopy parts, each independently replaceable after a crash.

Estimated all-up weight: ~200 g with battery and electronics. Estimated thrust budget: ~500 g at full throttle (4× motors). Thrust-to-weight ratio: ~2.5:1 — comfortable margin.

This is the **class** decision only. The specific frame `.f3d` lives under `hardware/mechanical/` and evolves freely without re-opening this ADR. The specific motor / ESC / battery part numbers are recorded at ordering time as a `hardware/electrical/parts-list.md` (or similar) — also not in scope of this ADR.

## Why this shape

### Why 3" specifically

- **Garden-flyable.** A 3" cinewhoop with ducts is honest-to-god safe to fly in a small back-garden environment. 5" is not. 4" is borderline. 2.5" is safe but payload-constrained.
- **Thrust budget survives a heavy build.** The author's first flight controller is a micro:bit-strapped-to-the-frame stack with hand-wired peripherals. That's substantially heavier than a purpose-built FC. At 3" / 4S / 1507, the thrust budget absorbs the extra weight without making the quad sluggish (T:W ≈ 2.5:1 even on the high end of the AUW estimate).
- **Crash damage is bounded.** A 3" prop in PLA / PETG arms in a duct does not break much when it hits the ground. The author *will* crash it many times during PID tuning and firmware bring-up; cheap-to-replace airframe is essential.
- **Largest deep-resource pool for the size.** "3" cinewhoop" is the most popular printed-frame class on Printables / Thingiverse / FPV community forums. Reference designs, motor recommendations, and tuning notes are abundant.

2.5" was rejected as too constrained on payload — borderline T:W with this build's electronics weight, and not enough headroom for a future Phase 6+ analog FPV camera ([00-vision.md](../00-vision.md) "Beyond Phase 5"). 4" was rejected as the point where printed arms genuinely start to flex enough to interact with the IMU during flight; carbon-fibre arms become the right answer at that size, which conflicts with the "made it myself" frame goal below. 5"+ is out of garden-flying scope.

### Why 4S

- **Universal default for the size.** Tutorials, motor catalogues, and ESC product pages at 1507-class assume 4S unless otherwise stated.
- **Cheaper batteries than 6S** at the relevant capacity. 4S 450–650 mAh packs are £8–12 each.
- **Lower current at the same power than 3S.** At 3S the motors would need higher KV and the ESCs would see ~33% more current, pushing into uncomfortably hot territory for a printed-frame build with limited airflow.
- **6S is overkill at 3"** and the available 1507-class motor KVs assume 4S.

### Why ducted

A duct is a circular shroud (a ring) around each propeller, integrated into the frame structure. The arguments for it specifically *given* a 3D-printed frame:

- **Structural stiffening.** The duct is a closed ring — it resists out-of-plane bending much better than a printed open arm of the same mass. The duct *helps* the printed-frame stiffness problem rather than fighting it.
- **Finger / wall / face protection.** The duct stops anything from touching the prop tips. The author lives with people; the drone will be flown indoors during bench tests; this is non-negotiable.
- **Quieter.** Duct flow reshapes the prop tip vortex; cinewhoops sound like vacuums, not chainsaws. Garden flying with neighbours benefits.
- **Bumper for free.** A duct is also a crash bumper. Hitting a wall doesn't immediately mean broken props.
- **Slight thrust efficiency improvement at hover.** Net positive, even on a short printed duct, but not the main reason to choose this.

The cost is mass (a ducted printed frame is ~2× the mass of a naked printed frame of the same span) and forward-flight drag. Neither matters for this build's mission profile (hover, tune, bench-test, garden) and the thrust budget at 3"/4S/1507 absorbs the mass.

### Why fully printed (not hybrid CF + printed)

The author can already 3D print, has Fusion 360 ([ADR 0006](0006-mechanical-cad-fusion360.md)), and has explicitly stated that **"made every part of it"** is a goal. The drone is the vehicle for learning ([ADR 0001](0001-platform-airframe-stack.md)). Buying a CF arm kit reduces the build's surface area for learning by removing the entire mechanical-design layer. That's the wrong trade for this project.

The classic objection to fully-printed frames is **stiffness at 5"+ with naked arms**. That objection does not apply at 3" with ducted arms (see "Why ducted" above) — the duct ring, not the arm material, dominates the stiffness budget at this size.

The remaining cost is iteration time when the frame breaks: ~1–2 hours of print time per arm vs. instantly-replaceable CF parts. That cost is acceptable. It is also a learning-positive cost — every reprint is a chance to refine the design.

## Consequences

### What this commits us to

- **A bench enclosure sized for a ~25–30 cm-span 3" cinewhoop** with battery and electronics (Phase 2 prerequisite per [00-vision.md](../00-vision.md) and `07-safety.md`).
- **A frame design in Fusion 360** under `hardware/mechanical/`, parameterised around a 1507-class motor mounting hole pattern (verified against the specific motor at order time) and a 3" prop diameter with ~1 mm tip-to-duct clearance. Frame revisions evolve under that design without re-opening this ADR.
- **Modular frame structure** — base / four ducts / canopy as separate printed parts bolted together — so a single broken duct does not require reprinting the whole airframe.
- **PETG as the production filament** for the airframe that flies. PLA prints first iterations during geometry development; PETG is what the prototype crashes in.
- **Future ESC and motor orders** are constrained to **DShot-capable, 4S-rated, ~25–35 A** ESCs and **1507-class, ~3500–3800 KV** motors. Specific parts (T-Motor / EMAX / iFlight / BetaFPV / etc.) chosen at order time on the catalogue then current.
- **Battery and charger commitments**: 4S LiPo 450–650 mAh, XT30 connector. A proper LiPo balance charger is a Phase 2 prerequisite — flying without one is a fire risk regardless of frame class. Charger purchase is **not** gated by this ADR; it can land any time.
- **Thrust-to-weight planning headroom**: the design AUW budget is ~200 g; if the actual build comes in over ~280 g (T:W < 1.8) the class is no longer adequate and this ADR gets superseded rather than stretched.

### What this rules out

- **No carbon-fibre arms or plates** in the first-arc airframe. This includes hybrid CF/printed builds. Re-opens only as a superseding ADR if printed-frame dynamics turn out to be a blocker.
- **No 5" / 6S / freestyle build** in the first arc. That is a separate, future quad if the author chooses to build one.
- **No naked X (un-ducted) frame** for the first build. Safety and printed-frame stiffness both push toward ducted.
- **No payload assumptions beyond the planned electronics + battery + a future small analog FPV camera + VTX (~20–30 g extra).** Larger payloads (HD camera, GPS module, additional batteries) are not on the table for this airframe class. Phase 6+ analog FPV remains a planned use ([00-vision.md](../00-vision.md) "Beyond Phase 5") and fits the budget.
- **No motor / ESC / battery purchases before the frame's first physical print and a fit-check.** Ordering motors before mounting holes are validated is how spare-parts boxes happen.

### What stays open

- **Specific motor part number.** 1507-class @ ~3500–3800 KV is the spec; the actual brand and model gets picked at order time and recorded as part of `hardware/electrical/parts-list.md`. Re-decision is not an ADR; it's a parts-list update.
- **Specific ESC part number.** 4-in-1, DShot-capable, 4S, ~25–35 A is the spec; same treatment as motors.
- **Battery capacity within the 450–650 mAh band.** Trade-off is hover time vs. mass; resolved empirically once the frame's actual AUW is known.
- **Radio link.** Still deferred ([AGENTS.md](../../AGENTS.md) "Next open questions"). Second micro:bit is the Phase 1–2 placeholder. The radio choice does not block this ADR.
- **Frame geometry.** Span, duct height, motor pitch, mounting points for the FC stack, battery strap routing — all decided in the Fusion 360 design under `hardware/mechanical/`. This ADR pins the *class* the geometry must satisfy, not the geometry itself.
- **Bench rig vs. flight frame.** It may be useful to print a deliberately-stiff "bench rig" (one motor, one prop, isolated from the flight frame) for early firmware bring-up, separate from the flight airframe. That's a [hardware/](../../hardware/README.md) decision, not an ADR.
- **Phase-4 PCBA mounting in this frame.** The custom nRF5340 PCBA from [00-vision.md](../00-vision.md) Phase 4 is intended to *replace* the micro:bit on this frame. The frame design should leave room for that swap, but the PCBA's outline is its own future ADR.

## References

- [00-vision.md](../00-vision.md) — Phase 3 commits flight hardware; 3" / 4S meets that bar.
- [ADR 0001](0001-platform-airframe-stack.md) — quadcopter, real hardware, learning-first scope (parent).
- [ADR 0006](0006-mechanical-cad-fusion360.md) — frame `.f3d` + `.step` lives in Fusion 360.
- [Printables — 3" cinewhoop frames](https://www.printables.com/search/models?q=3%20inch%20cinewhoop) — reference designs to study before designing one.
- [Oscar Liang's drone build guides](https://oscarliang.com/) — community reference for motor / ESC / battery pairings at common classes.
- BetaFPV / T-Motor / EMAX / iFlight — vendors with reliable spec sheets at the 1507 / 4S / 3" class.
- Implementation: `hardware/mechanical/` (frame design — to be created), `hardware/electrical/parts-list.md` (motor / ESC / battery selections at order time — to be created), bench enclosure (Phase 2 prerequisite — to be designed).
