# `embassy-time`, the tick rate, and tickless timers

_Captured 2026-06-16 while putting a placeholder `Timer::after_secs(1).await` in `serial_link_tx` and wondering how the compiler knows what a "second" is on a bare-metal target._

## The observation

You can write:

```rust
Timer::after_secs(1).await;
```

in a `no_std` firmware crate and it just works — the chip sleeps for one real-world second. There is no global "set the clock to 64 MHz" call anywhere in our code, no manual tick counting, no SysTick interrupt that we configured. It's all units of actual time, like on a hosted OS.

How does the firmware know what a second is, and how does it sleep for one without burning CPU?

## Three things to keep separate

The confusion comes from conflating three different "clocks" that all coexist on the chip:

1. **CPU clock** — 64 MHz on nRF52833. Decides how fast instructions execute. Embassy doesn't care about this for timekeeping. The chip would still tell correct time if the CPU were halted entirely.
2. **High-frequency clock (HFCLK)** — 16 / 64 MHz, drives peripherals like SPI, UART, the radio. Also not the timekeeping source.
3. **Low-frequency clock (LFCLK)** — 32_768 Hz, always. This is a *watch crystal* frequency, 2^15 Hz, chosen so it divides cleanly into seconds. **This** is what the RTC peripheral counts, and **this** is what `embassy-time` ultimately rests on.

The 32_768 Hz rate is a hardware fact of the nRF52: the RTC peripheral is physically wired to LFCLK and cannot run at any other rate. Nordic picked it because every quartz wristwatch on earth runs at that frequency and the parts are cheap and accurate.

## How `embassy-time` knows the rate

`embassy-time` exposes a `TICK_HZ` constant. Its value is set at **compile time** by a Cargo feature: `tick-hz-1_000`, `tick-hz-32_768`, `tick-hz-1_000_000`, etc. Exactly one must be active. We don't select one explicitly, so we get the default (1 MHz on `embassy-time` 0.3).

Then `Duration::from_secs` is a `const fn`:

```rust
pub const fn from_secs(secs: u64) -> Self {
    Self { ticks: secs * TICK_HZ }
}
```

So `Timer::after_secs(1)` compiles down to a constant tick count. The unit lives only at the call site; the runtime sees plain integers.

The C equivalent would be `#define TICKS_PER_SEC 32768` and writing `sleep(1 * TICKS_PER_SEC)` everywhere. Same idea, made invisible by named constructors.

The matching half is the **driver**: `embassy-nrf`'s built-in time driver knows it's driving an RTC at 32_768 Hz, and converts between embassy ticks and RTC ticks. If `TICK_HZ` and the driver's actual rate disagree, every timer in the program is silently wrong by that ratio — no error, no warning. Picking `tick-hz-32_768` for nRF gives zero conversion (perfect 1:1) and is slightly more efficient; the default 1 MHz makes the driver do scaling math and limits resolution to ~30 µs (one RTC tick).

## Tickless: no periodic interrupt

The really clever part. A naive design takes a SysTick interrupt every 1 ms (or every RTC tick) and decrements counters in the ISR. That wakes the CPU constantly, which on a battery-powered device is a power disaster.

Embassy is **tickless**. The chain is:

1. The RTC's 24-bit counter ticks at 32_768 Hz **in hardware**, with the CPU asleep. No interrupt, no CPU involvement, microamps of current.
2. The RTC also has **compare registers** (`CC[0]`, `CC[1]`, …). Write a target counter value into one, and the hardware fires an interrupt **only when the counter reaches that exact value**. Nothing in between.
3. When you `Timer::after_secs(1).await`, the driver reads the current counter, adds 32_768, writes that into a CC register, the executor calls `cortex_m::asm::wfi()` (Wait For Interrupt), CPU sleeps.
4. CPU sleeps for the **entire second**. Wakes exactly once when CC matches.

The C-RTOS equivalent ("tick-based scheduler") would take ~1000 SysTick interrupts in that second to discover the same fact. Embassy takes zero. This is *the* core reason async embedded is more power-efficient than threaded embedded, not just more ergonomic.

## Multiple timers: one hardware register is enough

What if several tasks have timers pending? The driver only has one CC register pointed at the *next* deadline; the others are remembered in a small software queue (size set by the `generic-queue-N` feature — we use `generic-queue-8`).

```
let t1 = Timer::after_secs(10);
let t2 = Timer::after_secs(3);
let t3 = Timer::after_secs(7);
```

```
t=0   : driver sees pending {10, 3, 7}, sets CC = 3s, CPU sleeps
t=3s  : RTC fires, wakes t2's task, finds min {10, 7}, sets CC = 7s, sleeps
t=7s  : RTC fires, wakes t3's task, finds min {10}, sets CC = 10s, sleeps
t=10s : RTC fires, wakes t1's task, queue empty, sleeps forever
```

Each `Timer` future stores its own deadline in its own task's state. The driver only has to find the minimum and reprogram CC after each wake. For 8 slots a linear scan beats a heap; bigger fleets would use a min-heap.

## Counter overflow

24-bit counter at 32_768 Hz wraps every ~512 seconds. The driver handles this by taking one overflow interrupt every wrap to extend the counter into a 64-bit virtual time in software. So strictly speaking there *is* one periodic interrupt — but at ~0.002 Hz, not 32 kHz. Below the noise.

## Takeaway

- `Timer::after_secs(1)` is "set this hardware compare register and sleep" — not "spin a counter".
- The unit is compile-time constant arithmetic; the runtime never sees seconds.
- One hardware compare register suffices for many pending timers, because the driver only needs the soonest.
- Power per timer ≈ one CPU wake. Idle ≈ literally just RTC ticking, ~1 µA.
