# Flight hardware parts list

The committed bill of materials for the first flight airframe. This realises the
class decision in [ADR 0019](../../doc/decisions/0019-airframe-class-3in-4s-printed.md):
3" ducted, 4S, 1507-class / ~3500–3800 KV motors, 4-in-1 DShot ESC. Per that ADR,
specific part numbers live here as a parts-list update, not as a new ADR. Deviations
from the class spec are called out below and tracked back to the ADR.

All propulsion parts ordered from [FPV24](https://www.fpv24.com/) (DE) in one round
on 2026-06-28.

## What this round buys, and why only this

This is the **props-off motor bring-up** round, not a fly-away round. It buys the three
propulsion parts and nothing else, because everything else needed to spin a motor on the
bench is already owned or printed:

- **Flight controller** — BBC micro:bit v2, already owned ([ADR 0002](../../doc/decisions/0002-mcu-and-language.md)).
- **micro:bit edge-connector breakout** — already owned (currently used to wire the IMU and a test motor).
- **Power for bring-up** — bench PSU, already owned. Props-off, unloaded motors draw little; a
  5–10 A bench supply is sufficient. Props-on tethered hover is *not* in scope for this round
  (see "Deferred" below) — that draws tens of amps and needs the flight battery.
- **Frame** — 3D-printed PETG, in-house design ([ADR 0019](../../doc/decisions/0019-airframe-class-3in-4s-printed.md)),
  printed not bought.

## Propulsion (ordered 2026-06-28, FPV24)

| Part | Selection | Qty | Unit | Line |
| --- | --- | --- | ---: | ---: |
| Motor | iFlight XING2 1404 Unibell 3800 KV | 4 | €16.90 | €67.60 |
| Propeller | HQ Durable 3030 (T3X3X3), tri-blade PC | 3 sets | €2.89 | €8.67 |
| ESC | Sequre Blueson A1 65 A 6S AM32 4-in-1 | 1 | €35.90 | €35.90 |
| | | | **Total** | **€112.17** |

### Motor — iFlight XING2 1404 Unibell 3800 KV

- **Link:** <https://www.fpv24.com/en/iflight/iflight-xing2-1404-unibell-3800kv-fpv-motor> (order ref `IFL-X009438`)
- **Key specs:** 1404 stator, 3800 KV, 3–4S, 9.1 g, 1.5 mm bare prop shaft (press-fit), 9N12P.
- **Why:** 3800 KV sits squarely in the ADR 0019 ~3500–3800 KV band for 3"/4S. The Unibell
  bell is a single moulded piece (one fewer fastener to lose). Press-fit 1.5 mm shaft pairs
  directly with the HQ 3030 prop below.
- **Deviation from ADR 0019:** the ADR specifies a **1507-class** stator; this is a **1404**
  (smaller). Accepted — see the ADR amendment note for 2026-06-28. Thrust budget remains ample
  for the ~200 g AUW target (T:W comfortably > 2:1), and the lighter motor only helps the AUW
  ceiling the ADR sets (~280 g / T:W 1.8 before the class is reconsidered).

### Propeller — HQ Durable 3030 (T3X3X3)

- **Link:** <https://www.fpv24.com/en/hqprop/hq-durable-prop-3030-triple-blade-t3x3x3-purple-4-pieces-pc-fpv-propeller-3-inch> (order ref `HQP-T3X3X3LU-PC`)
- **Key specs:** 3" diameter, 3" pitch, tri-blade, polycarbonate (durable compound),
  press-fit bore (1.9/1.4/1.9 mm) matching the 1.5 mm motor shaft.
- **Why:** 3"/tri-blade matches the ADR 0019 prop class. The "Durable" PC compound flexes rather
  than shatters on impact — the right choice for a learner build that will crash repeatedly during
  PID tuning. Three sets ordered for spares.
- **Naming note:** the leading **T** in `T3X3X3` denotes **tri-blade**, not a mount type. The
  motors include the M2 screws used to bolt the *motor* to the frame; the prop itself is press-fit,
  no prop screw.

### ESC — Sequre Blueson A1 65 A 6S AM32 4-in-1

- **Link:** <https://www.fpv24.com/en/sequre/sequre-blueson-a1-65a-6s-am32-esc> (order ref `SEQ-BLUESON-A1-6S-65A-AM32`)
- **Key specs:** 4-in-1, AM32 firmware, AT32F421 MCU, 2–6S (4S mid-range), 65 A continuous,
  20×20 mm mount, 19 g, current sensor. Ships with wire harness, capacitor, soft-mount grommets,
  and an XT60 lead.
- **Why:** 4-in-1 and DShot-capable as ADR 0019 requires. **AM32** is open-source firmware and,
  critically for this project, supports **standard PWM (1000–2000 µs)** as well as DShot300/600 —
  which keeps the planned PWM-first-then-DShot bring-up path open (start with the simple, forgiving
  protocol on the nRF52 PWM peripheral, switch the same board to DShot later). 65 A on 4S is well
  over-specced for four 1404s, giving thermal headroom in a poorly-ventilated printed frame.
- **No BEC.** This board has **no 5 V regulator output**. That is fine on the bench (the micro:bit
  runs from USB), but a separate switching 5 V BEC will be needed before any untethered flight. See
  "Deferred".
- **Deviation from ADR 0019:** the ADR class spec is 25–35 A; this is 65 A. Higher current rating
  is harmless headroom, not a class violation — the constraint was a *minimum* capability.
- **Naming gotcha:** the product name contains no "4-in-1", but the spec table confirms it is one.
  Earlier in selection a T-Motor F35A was briefly mis-read as a 4-in-1 (it is a *single* ESC). Lesson
  recorded: verify the product page's type field, never trust the model name alone.

#### ESC options considered and rejected

- **T-Motor Velox V45A Lite 4-in-1** — <https://droneshop.nl/tmotor-velox-v45a-lite-4in1-esc>
  (€52.95, droneshop.nl). BLHeli_S. Rejected: separate vendor (split shipping) and pricier for no
  PWM-path advantage over AM32.
- **iFlight Borg 60RS** — <https://www.fpv24.com/en/iflight/iflight-borg-60rs-esc> (€56.90, FPV24).
  BLHeli_32, 9.5 g. Rejected: uses an **FPC ribbon connector** designed to mate with a stacked iFlight
  flight controller — awkward to hand-wire to a micro:bit; rated **4–8S** (4S is its floor, it is built
  for 5–8S); lists only DShot/OneShot/Multishot, **no standard PWM**, which closes the easy bring-up
  path. Its advantages (light, bidirectional DShot) are things this project does not need.

## Already owned (not purchased)

- **Flight controller:** BBC micro:bit v2 (×2 — second is the ground-station / remote placeholder).
- **micro:bit edge-connector breakout** — used today for IMU + test-motor wiring.
- **Bench PSU** — powers the ESC battery pads (~14.8 V, current-limited) during props-off bring-up.

## Bench bring-up notes (props OFF)

- ESC battery pads ← bench PSU set to ~14.8 V (4S nominal), current-limited.
- micro:bit ← USB from the PC.
- **Tie ESC ground and micro:bit ground together** — the motor signal wires need a common ground
  reference or the DShot/PWM signal is meaningless.
- **Always props-off on the bench.** Props go on only inside a tethered enclosure once the firmware
  is trusted (Phase 2 safety prerequisite).

## Deferred (until tethered / free flight)

Not bought this round. Needed only once the build moves from props-off bench bring-up to
props-on power:

- **Flight battery** — 4S LiPo, 450–650 mAh, XT30, per ADR 0019. Provisional candidate:
  CNHL MiniStar 4S 650 mAh 70C (XT30U), ~€12.50 at FPV24 (re-verify at order time). Required for
  props-on hover, which a bench PSU cannot supply (tens of amps peak across four motors).
- **LiPo balance charger** — required before the first battery charge; a fire-safety prerequisite,
  not a convenience.
- **5 V switching BEC** — the Blueson A1 has no BEC. A buck (switching, not linear) BEC rated for
  ≥4S input is needed to power the micro:bit from the pack in untethered flight.

## References

- [ADR 0019](../../doc/decisions/0019-airframe-class-3in-4s-printed.md) — airframe and propulsion class (parent decision).
- [ADR 0002](../../doc/decisions/0002-mcu-and-language.md) — micro:bit v2 as flight controller for Phases 1–3.
- [hardware/mechanical/](../mechanical/) — the 3D-printed frame design (to be created).
