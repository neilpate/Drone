# iFlight XING2 1404 Unibell 3800 KV — motor datasheet

Captured 2026-08-22. iFlight does not publish a standalone PDF for this motor; the
specifications below are the manufacturer table as listed by the retailer we
ordered from. This file is the repo's authoritative copy so the numbers survive
link rot.

- **Source:** <https://www.fpv24.com/en/iflight/iflight-xing2-1404-unibell-3800kv-fpv-motor>
- **Order ref:** IFL-X009438
- **EAN:** 4260691178198
- **Manufacturer:** Huizhou iFlight Intelligent Technology Ltd.

## Specifications

| Parameter | Value |
| --- | --- |
| Power supply | 3–4S |
| KV | 3800 |
| Configuration | 9N12P (9 stator slots, 12 magnetic poles) |
| Stator diameter | 14 mm |
| Stator length | 4 mm |
| Shaft diameter | 1.5 mm |
| Dimensions | ⌀19.9 × 13.5 mm |
| Weight | 9.1 g |
| No-load current @ 5 V | ≤ 0.47 A |
| Maximum power | 253.4 W |
| Internal resistance | 0.220 Ω |
| Maximum current | 15.84 A |

Package contents: 1× motor, 8× M2×7 screw.

## Firmware-relevant note

**9N12P → 12 poles → 6 pole pairs.** ESC telemetry reports *electrical* RPM; the
mechanical shaft RPM is electrical RPM ÷ pole pairs. This constant is used in
`ERPM::POLE_PAIRS` ([crates/firmware-types/src/erpm.rs](../../../crates/firmware-types/src/erpm.rs)):

```
mechanical_rpm = (telemetry_erpm_field × 100) ÷ 6
```
