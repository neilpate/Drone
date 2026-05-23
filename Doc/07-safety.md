# 07 — Safety

_Status: Living document. Mandatory for Phase 3 free flight; many sections become binding earlier._

This is the doc that says **what we will not do, what we will always do, and how we will know our safety mechanisms actually work** — both in meatspace and in the firmware. It is split into:

- **Part A — Physical safety.** Hazards we expose ourselves to, and the rules and equipment that mitigate them.
- **Part B — Firmware safety.** Code-level rules the firmware must enforce, regardless of what the pilot or link is doing.
- **Part C — Verification.** How we prove the safety mechanisms work before relying on them.
- **Pre-flight checklist.** The actual sequential checks read before powering up, from Phase 3 onwards.
- **Incident log.** Where near-misses and failures are recorded and learned from.

The author is **a single operator working alone, indoors initially, with no prior LiPo or multirotor experience**. The rules below are calibrated for that, and lean conservative. They can be relaxed deliberately and individually, never silently.

---

## Part A — Physical safety

### A.1 Hazard inventory

| Hazard | Worst credible outcome | Primary mitigation |
|---|---|---|
| Spinning propeller | Finger laceration / amputation; eye injury from shed prop | Props off bench; safety glasses always when motors can spin; arming model in firmware |
| Bare-shaft brushless motor running unexpectedly | Skin pinch, projectile risk if grub screw exits | Bench PSU current-limited; firmware arming model; physical clamp on motor |
| LiPo thermal runaway (vent-with-flame) | Fire; toxic smoke; property damage | Hobby-grade balance charger; LiPo charging bag; non-flammable surface; never unattended; physical isolation from flammables |
| LiPo short circuit (dropped tool, damaged insulation) | As above | Tidy bench; insulated tools; inspect leads before every use; XT/JST connectors not bare wire |
| LiPo overdischarge / overcharge | Cell damage, future thermal event | Firmware low-voltage cut (Part B); never store at 4.2 V/cell or below 3.5 V/cell |
| Soldering iron burns / fume inhalation | Burns; respiratory irritation | Iron in stand always; fume extractor or open window; tin held with pliers; aware of where the tip is |
| Crash into person / pet / property | Bruise, cut, broken object | Indoor flight area cleared (A.6); solo flight only when alone in the space; conservative envelope |
| Crash damaging the drone | Lost work, broken parts | Tether (Phase 3); soft / open space; small-prop class for early flights (A.7) |
| RF interference (2.4 GHz) | Loss of link → failsafe trigger; interference with home Wi-Fi / Bluetooth | Failsafe behaviour (Part B); avoid co-existing 2.4 GHz devices on the same channel during testing |

Anything not on this list that turns up in practice goes in the **incident log** (below) and gets added here.

### A.2 Personal protective equipment (PPE)

Minimum kit, always present at the bench / flight area:

- **Safety glasses (ANSI Z87.1 or EN 166).** Worn any time motors can physically spin (regardless of whether props are fitted). Cheap, no excuse.
- **Closed-toe shoes** on flight days.
- **No loose sleeves, no dangling jewellery, no ties** when working near spinning hardware.
- **Long hair tied back.**
- **First-aid kit within reach.** Plasters, sterile pads, burn gel.
- **Phone within reach** with emergency number ready (lone-worker discipline; see A.5).

### A.3 Bench / flight rules per phase

| Phase | Motors | Props | Battery on board | PPE minimum | Location | Notes |
|---|---|---|---|---|---|---|
| **1** | 1 | **No** | No (bench PSU) | Glasses | Workshop bench, motor clamped | Single motor; if it runs away, it spins in a clamp. Lowest-risk phase. |
| **2** | 4 | **No** | No (bench PSU) | Glasses | Workshop bench, frame physically restrained | Four motors but no thrust. Mixer / DShot validation. |
| **3 — first flights** | 4 | **Yes** | Yes (LiPo) | Glasses; closed-toe shoes; long sleeves removed | **Cleared indoor area** (A.6); tether attached | Free-flight regime begins. Tether is mandatory until controlled hover is repeatable. |
| **3 — post-tether** | 4 | Yes | Yes | As above | Cleared indoor area | Tether removed only after pre-flight failsafe verification (Part C). |
| **3 — outdoor** | 4 | Yes | Yes | As above + weather-appropriate | Outdoor (A.6) | Only after stable indoor flight is repeatable. Conservative conditions (A.7). |
| **4** | 4 | No | No (bench PSU) | Glasses | Workshop bench | New hardware bring-up. Back to bench discipline. |
| **5** | 4 | Yes | Yes | As Phase 3 | As Phase 3 | First flight on new hardware. Re-run the entire Part-C verification before flight. |

Rule: **props go on no earlier than they're needed for the phase.** If you can do today's work without props, the props stay off.

### A.4 LiPo discipline

The single highest physical-safety risk in this project. Treat LiPos with the respect you would treat a small can of petrol.

**Charging:**

- Use a **hobby-grade balance charger** (e.g. ISDT, SkyRC, iCharger — entry-level models are fine). Not the bundled charger from a toy.
- Charge **inside a LiPo charging bag** or, better, a metal Bat-Safe-style box. On a **non-flammable surface** (tile, concrete, baking tray) **away from anything flammable** (curtains, paper, cardboard).
- **Never leave a charging LiPo unattended.** If you need to leave the room, stop the charge.
- Charge at **1C or below** unless the pack is explicitly rated for higher. (For a 1500 mAh pack, that's 1.5 A.)
- Always charge via the **balance lead**, not just the main lead. The charger watches per-cell voltage.

**Storage:**

- Store packs at **storage voltage: 3.80–3.85 V per cell** if they will not be used within 24–48 hours. Modern chargers have a "Storage" mode that does this automatically.
- **Never store fully charged** (4.20 V/cell) for more than a day. Cell life collapses.
- **Never store discharged** (below 3.50 V/cell). Risk of permanent damage and copper-shunt formation.
- Store in the LiPo bag / metal box, ideally at room temperature, away from sources of heat or ignition.

**Inspection (before every use):**

- Look for **swelling, puffiness, or any deviation from a flat rectangular shape.** A swollen pack is damaged — retire it, do not fly it, do not even charge it again.
- Check the **leads and balance connector** for nicks, exposed copper, scorch marks.
- Check the **wrap** for tears.
- If in doubt, retire the pack. Packs are cheap. Fires are not.

**Damage / event response:**

- If a pack starts to **swell, smoke, or feel hot during charging**: stop the charger, move the pack (using tongs or oven mitts) into the LiPo bag / metal box / outside onto concrete, retreat, observe. Have a Class D / lithium-rated extinguisher or a bucket of sand within reach if possible. Water is acceptable on a LiPo *fire* (cooling the cells) but useless on a vent-with-flame in progress.
- After any event the pack is retired regardless of whether it appears intact.

**Disposal:**

- Discharge to **below 2 V/cell** (traditional method: submerge in salt water for ~24 hours, in a non-metal container, outside or in a well-ventilated area). Once discharged it is safe to take to a battery recycling point (hardware stores, council recycling, hobby shops often have bins).

**External references** (worth reading before first charge): *RC Groups LiPo FAQ*; *Bat-Safe / Bat-Box product documentation*; *manufacturer's datasheet for your specific charger*.

### A.5 Working solo

The author works alone. The buddy system that catches mistakes in workshops is absent here, so the discipline replacing it is stricter, not looser.

- **Tell someone before each bench / flight session.** Quick text: "starting drone work" / "done, safe". They don't have to be present, they have to know if you go silent.
- **Phone within reach, charged, unlocked.** Emergency number ready on the lockscreen.
- **Any injury → done for the day.** A cut, a burn, dizziness, eye irritation, anything: stop the session, no exceptions. A second mistake is much more likely than the first.
- **No working on the drone tired or distracted.** This is rule-zero of solo lab work.
- **Charging supervision is non-negotiable.** Not "I'll just pop to the kitchen". Stop the charge if you need to leave the room.

### A.6 Flight environment

**Indoor (first flights, Phase 3 early):**

- **Minimum cleared space:** 4 m × 4 m × 2.5 m ceiling. Smaller works only with the smallest prop class (A.7).
- **Floor:** carpet, mat, or other soft surface. No glass, no ceramic, no tripping hazards.
- **Walls clear of fragile objects** in the cone the drone could reach.
- **No pets, no people, no children** in the room. Door closed.
- **No ceiling fans, no air conditioning, no open windows.** Anything that moves air is a problem at this scale.
- **One bright, even light source.** Strong shadows confuse human pilot perception.

**Outdoor (later in Phase 3):**

- **Open space, no people within the maximum-credible-departure radius.** For a small drone, "garden, nobody else present" is enough. For anything larger, a club field.
- **No buildings, cars, livestock, or trees within ~10 m** of the planned flight area.
- **Daytime only.**
- **Aware of UK CAA / local regulations.** Below the CAA's 250 g registration threshold and outside flight-restriction zones is the simplest legal posture; verify before flying anything heavier.

**No-go conditions (cancel the session):**

- Wind above ~5 m/s (≈ 10 mph). Leaves moving on trees = too much.
- Rain, fog, or any precipitation.
- Pilot is tired, ill, distracted, or has been drinking.
- LiPo damaged or out of storage spec (A.4).
- Any safety equipment missing (PPE, tether, extinguisher proximity).
- Any failsafe (Part B) has not been verified working in this session.
- Unanticipated audience appears (neighbour, family, passer-by). Stop, communicate, only resume if the area is genuinely clear.

### A.7 Frame and battery class — bias toward small for first flights

The Phase 3 frame / motor / battery selection is still open ([00-vision.md](00-vision.md)). For a solo, indoor, first-time pilot, **lean toward the smallest viable prop class**:

- **2–3″ ducted (tinywhoop / cinewhoop / micro) class.** 1S–2S LiPo, ~250–500 mAh, ~80–150 g all-up. Ducted props mean a wall strike is a thump, not a hole.
- Versus **5″ class.** 4S LiPo, ~1300–1800 mAh, ~500–700 g. Fast, capable, *and* the standard cause of hobby-FPV finger amputations.

A small frame is more forgiving of every kind of mistake: crashes, runaway thrust, prop strikes, LiPo size (smaller cell, smaller event). It also costs less to crash repeatedly while learning. The trade-off — less headroom for Phase-6+ analog FPV payload — is real but acceptable.

**Recommendation feeding back into the Phase 3 ADR:** start at ~3″ ducted, escalate only if needed. Not yet locked in.

---

## Part B — Firmware safety

These are rules the firmware must enforce regardless of pilot input or link state. They are architectural commitments, not implementation details — every actor design ([ADR 0004](decisions/0004-concurrency-embassy-channels.md)) must accommodate them.

### B.1 Arming model

- **The drone is _inert_ until explicitly armed.** "Inert" means motor outputs are forced to zero / disabled at the lowest possible layer (ideally a PWM peripheral disable, not a software zero).
- **Arming requires _all_ of the following, simultaneously, for a debounce period (~500 ms):**
  - Throttle stick at idle / minimum.
  - IMU reporting healthy samples (no NaN, no saturation, INT1 firing at the expected rate).
  - Link present (recent valid command packet received).
  - No active fault flags.
  - Explicit arming command from the pilot (a dedicated button / stick gesture, not the throttle).
- **Disarming is instant, from any source.** Pilot disarm command, link loss beyond the configured threshold, any fault flag → motors inert immediately. There is no "graceful disarm".
- **Re-arming after disarm requires the full arming sequence again.** No automatic re-arm.

### B.2 Throttle and command shaping

- **Throttle slew rate is limited in firmware.** Pilot stick to zero-to-full no faster than ~500 ms regardless of stick movement.
- **Attitude / rate setpoints are clamped** to configurable maxima (e.g. max 30° lean angle in self-level mode, max 200°/s rate in rate mode for early flights). Limits start tight and are widened only with deliberate intent.
- **All pilot inputs pass through a deadzone** so a centred stick is truly centred.

### B.3 Link-loss failsafe

- **Phase 1–2 (bench):** loss of valid command packets for > 200 ms → throttle to zero, disarm.
- **Phase 3+ (flight):** loss of valid command packets for > 200 ms → controlled descent at a fixed sink rate, disarm on ground contact (detected via accelerometer + low throttle output). **No RTL** — we have no position estimate.
- The failsafe **must be testable on the bench** by physically powering down the ground micro:bit. It will be, before every flight session (Part C).

### B.4 Sensor-fault failsafe

- **IMU NaN, infinity, or out-of-range value** in any axis → disarm; latch fault flag; require power cycle to clear.
- **IMU INT1 timeout** (no interrupt for > 5 × expected period) → disarm; latch fault flag.
- **IMU saturation** (any axis pegged at ±max for > N samples) → disarm; latch fault flag. (Tunable; gyro saturation in flight is real for aggressive manoeuvres but unexpected in the early phases.)
- Faults are **latched, not transient.** A glitchy IMU does not get to recover mid-flight.

### B.5 Battery monitoring and low-battery behaviour

- **Battery voltage is sampled at ≥ 10 Hz** through a divider into an ADC.
- **Three thresholds, configurable per-pack class:**
  - **Warning** (≈ 3.6 V/cell): telemetry flag raised, pilot notified via ground UI.
  - **Forced land** (≈ 3.4 V/cell): pilot inputs progressively biased toward descent; full throttle no longer reaches max.
  - **Cut** (≈ 3.2 V/cell): immediate disarm regardless of altitude. Yes, this can crash the drone — better a crash than a damaged pack and a fire on the charger an hour later.
- Voltage sampling **accounts for sag under load.** A pack at 3.5 V resting may drop to 3.0 V under full throttle; thresholds compare against a filtered-and-load-compensated value, not raw ADC.

### B.6 Watchdog

- **Hardware watchdog timer enabled in flight builds.** Control-loop task must pet it on every cycle. Failure to pet → chip reset → drone falls (and that's the correct behaviour: a hung controller is more dangerous than a falling drone).
- **Disabled in bench builds** to allow interactive debugging without spurious resets. Build-flavour gated (B.8).

### B.7 Panic behaviour

- `panic-probe` (the default with `defmt-rtt`) **halts the CPU on panic**, which is excellent on the bench (you can attach a debugger and inspect) and **catastrophic in flight** (motors keep their last PWM duty cycle).
- **Flight builds must use a panic handler that resets the chip,** falling through to the watchdog or directly calling `cortex_m::peripheral::SCB::sys_reset()`. Build-flavour gated (B.8).
- On reset, the chip boots **inert / disarmed** (B.1), and the failsafe code path takes over.

### B.8 Build flavours / safety modes

Three Cargo feature flags, mutually exclusive, exactly one of which must be active for a build to succeed:

| Feature | Use | Watchdog | Panic handler | Motor outputs |
|---|---|---|---|---|
| `bench` | Phases 1, 2, 4 (no props, no battery) | Disabled | Halt | Enabled |
| `tethered` | Phase 3 / 5 first flights (battery + tether) | Enabled | Reset | Enabled |
| `flight` | Phase 3 / 5 untethered flight | Enabled | Reset | Enabled, full envelope |

`bench` and `flight` must be visually distinguishable in the ground-station UI (different colour band, large text) so the pilot cannot accidentally fly a `bench` build.

---

## Part C — Verification discipline

A failsafe is **not trusted until it has been deliberately triggered and observed to work**, in the current build, on this hardware, in this session. This is the discipline that distinguishes safety theatre from actual safety.

### C.1 Bench-prove every failsafe before Phase 3

Before the first flight, each of the following must have been triggered intentionally on the bench, observed to behave as specified in Part B, and the result recorded:

| Failsafe | Bench test |
|---|---|
| Arming model (B.1) | Try to arm with throttle high → must refuse. Try to arm with IMU disconnected → must refuse. |
| Disarming (B.1) | Send disarm during armed bench-spin → motors must stop within one control cycle. |
| Throttle slew (B.2) | Step pilot input from 0 to 100% → motor output must take ≥ 500 ms to reach 100%. |
| Attitude clamp (B.2) | Send 90° setpoint in self-level mode → output limited to configured max. |
| Link loss, bench (B.3) | Power down ground µbit during armed bench-spin → motors must stop within 200 ms. |
| Link loss, flight (B.3) | (Tethered, props on, in flight) power down ground µbit → must enter controlled descent. |
| IMU NaN (B.4) | Inject a NaN into the fusion input → must latch fault, disarm. |
| IMU timeout (B.4) | Disconnect IMU INT1 line during operation → must latch fault, disarm within 5 × period. |
| Battery thresholds (B.5) | Apply a slowly-decreasing voltage to the battery sense input → all three thresholds trigger at correct points. |
| Watchdog (B.6) | Insert a long `block_on(pending::<()>)` in the control-loop task → chip must reset. |
| Panic reset (B.7) | Insert a `panic!()` in a non-critical task → chip must reset, drone boots disarmed. |

These tests should be **automated where possible** (host-runnable unit tests on the `core` modules per [ADR 0007](decisions/0007-testing-and-ci-strategy.md)). The on-hardware tests become HIL tests once Tier 2 CI lands.

### C.2 Pre-flight gate

The first flight of Phase 3 (and Phase 5) requires every test in C.1 to have been re-run on **the exact firmware build that will fly**, in the current session. No "we tested it yesterday".

### C.3 Incident learning

Every safety-relevant incident (vented LiPo, runaway motor, unexpected disarm, etc.) is recorded in the incident log (below) with: what happened, what was expected, what changed in the firmware / docs / process as a result. Showcase signal, but more importantly, the only way mistakes turn into knowledge.

---

## Pre-flight checklist (Phase 3 onwards)

Read aloud in order. Anything that doesn't pass → stop, fix, restart.

1. **Pilot fit.** Rested, sober, not distracted, not rushing.
2. **Area clear.** Cleared zone (A.6); no people, pets, valuables in cone; door closed.
3. **PPE on.** Glasses on. Closed-toe shoes. Sleeves clear.
4. **LiPo inspection.** No swelling, no nicks, leads intact. Voltage ≥ 3.7 V/cell.
5. **Drone inspection.** All four motor screws tight. Props correct rotation, no cracks, no chips. No loose wires. Frame intact.
6. **Build flavour confirmed.** Ground UI displays the expected build flavour (`tethered` or `flight`) — not `bench`.
7. **Tether attached** (Phase 3 first flights).
8. **Ground micro:bit + PC running.** Telemetry flowing. RSSI healthy.
9. **Failsafe verification done this session** (C.2). Confirmed by checking the verification log.
10. **Extinguisher / fire blanket within reach** (especially indoor first flights).
11. **Phone in pocket, somebody knows the session is starting.**
12. **Arming sequence.** Throttle min. Arm. Verify motors idle. **Take off.**

---

## Incident log

Format for each entry:

```
### YYYY-MM-DD — Short title
**Phase:** N
**What happened:** factual description.
**What was expected:** what should have happened.
**Root cause:** as best understood.
**Changes:** what was changed in firmware / hardware / docs / process.
**Status:** Resolved / Open / Monitoring.
```

(No entries yet.)

---

## Open questions

These will be resolved as Phase 3 prep firms up. Each becomes either a section update here or its own ADR.

- **Exact frame / motor / prop / battery class.** Recommendation (A.7) is to start small (≈ 3″ ducted); not yet locked in.
- **Specific charger model.** Hobby-grade balance charger; brand TBD.
- **LiPo storage box.** LiPo bag minimum; Bat-Safe-class metal box preferred. TBD.
- **Tether design.** Catch line, not power line. Attachment point on frame. Length. Material. TBD.
- **Battery-voltage sense circuit.** Divider ratio, ADC channel, filter time constant. Specified in the firmware safety ADR when written.
- **Failsafe ADR.** Part B is the working specification; will be formalised in an ADR before Phase 3 (per [02-architecture.md](02-architecture.md)).
- **Local regulatory status.** Confirm UK CAA registration thresholds and any flight-restriction zones for the planned outdoor location.

---

## References

- LiPo handling: manufacturer datasheet for the specific charger; community LiPo FAQs (RC Groups, RCGroups Wiki, Oscar Liang's LiPo guide).
- UK drone regulations: <https://register-drones.caa.co.uk/>.
- Embedded watchdog patterns: `embedded-hal` watchdog traits; nRF52833 reference manual §6.36 (WDT).
- Embassy panic-on-flight discussion: `embassy-rs/embassy` issues + discussions on panic handling for production firmware.
