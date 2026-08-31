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

#### FC connector pinout (8-pin JST-SH)

The ESC-to-FC harness is an 8-pin **JST SH** assembly. Board side is `SM08B-SRSS-TB`
(side-entry / right-angle SMT header, 1.0 mm pitch; the top-entry sibling is `BM08B-SRSS-TB`);
the cable housing is `SHR-08V-S-B`.

**This is not the standard 4-in-1 pinout.** The `ESC_4in1_Connector` schematic symbol's pin
numbering was deliberately **reversed** to match this project's custom cable, so pin 1 is the
motor end, not the power end:

| Pin | Signal | Harness wire |
| ---: | --- | --- |
| 1 | M4 | white |
| 2 | M3 | white |
| 3 | M2 | white |
| 4 | M1 | white |
| 5 | Telemetry (serial) | green |
| 6 | Current sense (analog) | yellow |
| 7 | GND | black |
| 8 | VBAT | red |

**Read the wire colours, not the pad order.** The four motor lines are all white and the pad
silkscreen is only legible from one side of the ESC, so any judgement based on "left" or "right"
depends on which face you are looking at — that mirroring already caused one false conclusion
while drawing the schematic. The colours are viewpoint-independent; the pin numbers are not.
Definitive check before fab: continuity from the ESC battery pad through the harness to the free
connector identifies pin 8 (VBAT) unambiguously.

Getting this reversed puts pack voltage (up to 16.8 V on 4S) onto an MCU GPIO. Compare
[ADR 0023](../../doc/decisions/0023-motor-numbering-layout-rotation.md), where reversed ESC signal
leads silently inverted roll and pitch; on the custom PCBA the same class of error is destructive
rather than merely confusing.

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

## Flight battery and charging (selected 2026-07-26, FPV24)

The props-on power round: the flight battery, a charger, and the connector rework needed to mate
them to the ESC. This is the step a bench PSU cannot do — it browns out under motor load and cannot
sink the motors' regenerative current, tripping its over-voltage protection (recorded in the
learning note [imu-vibration-and-bench-psu-power.md](../../doc/learning/imu-vibration-and-bench-psu-power.md)).
Realises the battery class from [ADR 0019](../../doc/decisions/0019-airframe-class-3in-4s-printed.md).

| Part | Selection | Qty | Unit |
| --- | --- | --- | ---: |
| Battery | CNHL MiniStar 4S 650 mAh 70C (XT30U) | 2–3 | €12.50 |
| Charger | ToolkitRC M4 Pocket 1–4S 5A 80W (RTC-TK11800) | 1 | €32.90 |

### Battery — CNHL MiniStar 4S 650 mAh 70C (XT30U)

- **Link:** <https://www.fpv24.com/en/cnhl/cnhl-ministar-lipo-battery-650mah-148v-4s-70c-xt30u>
- **Key specs:** 4S (14.8 V nominal), 650 mAh, 70C, XT30U connector, ~72 g.
- **Why:** top of the ADR 0019 450–650 mAh band for the most loiter time to observe behaviour during
  PID tuning; 70C (≈45 A continuous) covers the gentle hover current of a ~200 g craft with headroom
  for tuning-burst transients. XT30 is the correct connector for this current/weight class (see the
  connector note below). Two or three packs so a charging pack never blocks a bench session.

### Charger — ToolkitRC M4 Pocket 1–4S 5A 80W

- **Link:** <https://www.fpv24.com/en/toolkitrc/toolkitrc-m4-pocket-lipo-1-4s-5a-charger-dc-7-25v-usb-c>
- **Key specs:** 1–4S balance charge, 1–5 A / 80 W, 400 mA balancer, output **XT30 & XT60**, input
  **XT60 (7–25 V)** or **USB-C PD (5–20 V)**, 75 g.
- **Why:** 1–4S matches the 4S-only platform exactly (no wasted 6S capability); the native **XT30
  output** mates the CNHL packs with no charge-lead adapter; the **USB-C PD input** lets it charge
  from a laptop/phone PD brick, or it can run off the existing bench PSU via its XT60 input. 80 W / 5 A
  is ample for a 650 mAh pack (1C = 0.65 A). Preferred over a bigger 6S charger (e.g. SkyRC B6neo)
  precisely because it is 4S-native, XT30-native, and USB-C-powerable.
- **Still needed alongside:** a **LiPo-safe charging/storage bag** (fire-safety prerequisite), and —
  if powering the charger from the bench PSU rather than USB-C PD — a bench-PSU → XT60 input lead.

### Connector note — ESC pigtail XT60 → XT30

The Blueson A1 ESC ships with an **XT60** pigtail, but the battery class is **XT30** (lighter, and
correctly sized for a 3"/4S whoop's current). Plan: **re-terminate the ESC pigtail to XT30** so it
mates the packs directly. Interim option for bench use: an XT30↔XT60 adapter lead. Do not buy XT60
packs to match the pigtail — XT60 is oversized for this class.

## Deferred (until untethered / free flight)

Not bought yet. Needed only once the build moves from tethered to free flight:

- **5 V switching BEC** — the Blueson A1 has no BEC. A buck (switching, not linear) BEC rated for
  ≥4S input is needed to power the micro:bit from the pack in untethered flight.

## References

- [ADR 0019](../../doc/decisions/0019-airframe-class-3in-4s-printed.md) — airframe and propulsion class (parent decision).
- [ADR 0002](../../doc/decisions/0002-mcu-and-language.md) — micro:bit v2 as flight controller for Phases 1–3.
- [hardware/mechanical/](../mechanical/) — the 3D-printed frame design (to be created).
