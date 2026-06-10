# GPIO state during reset and flashing

_Captured 2026-06-10 after watching the drone's motor spin every time the firmware was flashed. Not a firmware bug — the firmware wasn't running yet._

## The observation

Each time `cargo run` (or the probe-rs debug launch) reflashed the drone, the motor briefly spun up before the new firmware took control. Once the firmware was running, behaviour was correct: motor off at boot, follows the slider thereafter. So whatever was happening only happened *between* code being live.

## The cause

On the nRF52833, every GPIO pin starts life **input, no pull, hi-Z** at reset. The chip stays that way until your code calls `Output::new(...)` (or whatever peripheral takes ownership of the pin and configures it). For us that means there is a window — power-on, every reset, the entire flashing sequence — where the motor PWM pin (`P0_17`) is just floating.

Floating into what? In our case, an **L9110 H-bridge input**. The L9110 is active-low: a logic LOW on the input drives the motor. A floating pin with no external pull is decided by leakage and weak internal pulls in the receiving chip; for the L9110 that came out at "low enough to count as on". So:

```
reset / flashing window:
  nRF GPIO          = hi-Z (floating)
  L9110 sees        = ~LOW
  L9110 interprets  = "drive motor"
  motor             = spins

after firmware initialises PWM:
  nRF GPIO          = push-pull, driving inactive level
  L9110 sees        = whatever firmware says
  motor             = stops
```

The "inversion" in our PWM code (`set_duty(0, MAX_DUTY)` to mean "off") is itself a workaround for the L9110 being active-low. That workaround relies on the firmware being in control of the pin. The reset window is exactly the time when it isn't.

## The general rule

The safe state of a peripheral and the un-driven (hi-Z) state of the controlling GPIO must agree. If they don't, the system has a "no firmware = unsafe" failure mode, which is exactly the wrong way round.

Two ways to make them agree:

1. **External pull resistor** on the GPIO line, in the direction that means "safe" to the peripheral. For an active-low driver, pull *up* to VCC; for an active-high driver, pull *down* to GND. 10 kΩ is the standard value. The pull-up is weak (~330 µA at 3V3 for 10 kΩ); the firmware's push-pull output will overwhelm it instantly when it takes over.
2. **Driver chips with an explicit enable / sleep input** (DRV8833 `nSLEEP`, TB6612 `STBY`, BLHeli ESCs that won't arm without a valid PWM signal). Pull the enable line to "off" with another resistor and the driver is electrically inert until firmware actively turns it on. This is what proper motor drivers exist for.

Option 1 is "fix the symptom", option 2 is "use hardware that doesn't have the symptom". The first is right for hobby breadboards; the second is right for the actual drone (Phase 3 onwards, where four BLDC ESCs replace the L9110 and bring their own enable semantics for free).

## Why software can't fully fix this

You can shrink the window — grab the pin as a plain `Output::new(..., Level::High, ...)` at the very start of your BSP, before the PWM peripheral takes it — and that gets you down from "until PWM init finishes" to "from a few hundred microseconds after reset". But you can't get to zero, because the pin is hi-Z *before any code runs at all*. Power-on reset, brown-out, reset button, attach-and-flash from probe-rs: all of these have a window where no instructions are executing yet. Hardware has to win that round.

This is the same reason I/O-expander chips, level shifters, and motor drivers all have their own enable / output-enable / `OE#` pins with pull-down defaults. Whoever designed those expected exactly this scenario.

## Mapping to the bench setup

Two changes covered the issue completely:

- 10 kΩ pull-up from `P0_17` to 3V3 on the breadboard. Now during flashing the line sits at logic high, the L9110 sees "off", the motor stays still.
- One-line comment in `firmware-drone/src/board/microbit_v2.rs` next to the PWM init explaining that the pull-up is a hardware-firmware contract, not optional.

The Phase-3 brushless ESCs don't need this — they require a valid PWM signal in a specific range to arm, so a floating input is just "no signal", which they treat as "stay disarmed". Different problem class, no resistor needed.

## Related: pin reset behaviour across MCUs

For reference, the rule "GPIO is hi-Z at reset" is essentially universal in modern microcontrollers:

- **ARM Cortex-M** (nRF52, STM32, RP2040, NXP, etc.): all GPIO start as input, no pull. Some chips (STM32) let you configure boot-time pull behaviour in option bytes; few do this in practice.
- **AVR / 8-bit**: same — DDR register clears to all-input on reset.
- **PIC**: same.
- **ESP32**: most pins same; a small number of "strapping pins" have boot-time pulls and you must not put functional loads on them. Different problem, same family of gotchas.

So this lesson generalises beyond nRF: any time an MCU drives an active-something external chip that does not have its own enable line, a pull resistor in the safe direction is mandatory hardware.

## Sources

- [Nordic nRF52833 product specification](https://infocenter.nordicsemi.com/topic/ps_nrf52833/dif.html), GPIO chapter — confirms input/no-pull default at reset.
- [L9110 datasheet](https://www.elecrow.com/download/L9110.pdf) — input/output truth table; forward/reverse driven by IA/IB inputs being asserted low.
- General embedded folklore: every MCU vendor's "design considerations" app note has a paragraph that boils down to this.
