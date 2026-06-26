# Measuring CPU load with no OS: the idle-spinner method

_Captured 2026-06-26 after building `load_profiler` — a way to answer "how busy is the chip?" when there is no operating system keeping scheduler accounting for you._

## The question

On a hosted OS, "CPU load" is something the kernel hands you for free. `top` reads it out of per-process accounting the scheduler already maintains: the kernel knows exactly how many ticks each task ran because it context-switches them itself.

On bare metal with Embassy there is no such bookkeeping. The executor is cooperative — tasks run until they `.await`, and nobody is counting cycles. So how do you measure "what fraction of the time is the CPU doing real work?" without instrumenting every task by hand?

## The principle: measure the idle, infer the busy

You cannot easily measure the busy time directly (it is spread across many tasks and interrupts). But you can measure the *idle* time, and load is just the complement:

$$\text{load} = 1 - \frac{\text{idle time}}{\text{wall-clock time}}$$

The trick is to make the idle time visible. Put a task at the **lowest possible priority** that does nothing but a fixed, known chunk of busy-work, over and over. Because it is the lowest priority, it only ever runs on cycles that no real work wanted — i.e. on the idle cycles. So:

- Time the fixed chunk once while the system is otherwise idle. Call that $T_0$ (the baseline — how long the work takes when it gets the whole CPU).
- Time the same fixed chunk again while the system is doing real work. Call that $T_1$. It will be *longer*, because higher-priority work kept preempting the spinner, stretching its wall-clock duration.

The ratio $T_0 / T_1$ is the fraction of wall-clock time the spinner actually got the CPU — the idle fraction. So:

$$\text{load} = 1 - \frac{T_0}{T_1}$$

If the system is idle, $T_1 = T_0$ and load is 0. If real work consumed half the CPU during the window, the spinner takes twice as long, $T_0/T_1 = 0.5$, load is 50%. No per-task instrumentation, no scheduler hooks — one spinner and one clock.

This is the embedded equivalent of the classic "idle task counter" trick from RTOS land (FreeRTOS' `vApplicationIdleHook` counting loops), refined to use a real timer instead of a loop count so the result is in honest wall-clock units.

## Why the spinner must be the *sole* lowest-priority task

The whole method rests on one property: **every cycle the spinner is not running is a cycle stolen by real work.** That only holds if nothing else shares its priority tier.

Our executor layout (see [main.rs](../../crates/firmware-drone/src/main.rs)):

| Tier | What runs there | Priority |
|------|-----------------|----------|
| Peripheral ISRs | RADIO, RTC, TEMP — the HAL's own interrupt handlers | P0 (highest) |
| `InterruptExecutor` on `SWI0_EGU0` | all 7 real tasks (supervisor, remote_link, sensors, telemetry, …) | P6 |
| Thread mode | `load_profiler`, and nothing else | lowest |

The preemption chain is strictly: **P0 ISRs ▸ P6 tasks ▸ thread-mode spinner.** On Cortex-M, thread mode (no interrupt active) is by definition the lowest priority — it runs only when no exception is pending.

If a second task lived in thread mode, the two would be *cooperative siblings*: the spinner would have to `.await`/yield to let the other run, and that other task's time would look like "idle" to the spinner (it ran while the spinner was parked, not preempted). The measurement would under-report load. By being **alone at the bottom**, the spinner has no cooperative peers — the only thing that can take the CPU away from it is genuine preemption by a higher tier, which is exactly what we want to count.

This is also why the spinner **never `.await`s**. An `.await` would yield to the executor voluntarily; we want it to surrender the CPU *only* under preemption, never cooperatively.

## The free-running clock captures the stolen cycles for free

`spin_window` brackets the fixed work with `Instant::now()` / `start.elapsed()`:

```rust
fn spin_window() -> Duration {
    let start = Instant::now();
    let mut counter: u32 = 0;
    for _ in 0..SPIN_ITERATIONS {
        counter = spin_once(counter);
    }
    core::hint::black_box(counter);
    start.elapsed()
}
```

`Instant` is backed by the RTC, which ticks at 32_768 Hz in hardware regardless of what the CPU is doing (see [embassy-time-and-tickless-timers.md](embassy-time-and-tickless-timers.md)). So when an ISR or a P6 task preempts the spin mid-loop, the RTC keeps counting through that detour. The `elapsed()` therefore includes every cycle that was stolen — the preemption time is captured automatically, with no need to hook the scheduler. That free-running property is the linchpin: a CPU-cycle counter that froze while the CPU was elsewhere would not work.

## Keeping the work honest: `black_box` and `inline(never)`

The fixed chunk is a loop that increments a counter. A loop with no observable effect is exactly what an optimiser deletes — and if the compiler folds `SPIN_ITERATIONS` increments into a single add, calibration and the runtime loop both measure nothing.

```rust
#[inline(never)]
fn spin_once(counter: u32) -> u32 {
    core::hint::black_box(counter.wrapping_add(1))
}
```

`core::hint::black_box` is an optimisation barrier — it tells the compiler "assume this value is observed, do not reason about it." Combined with `#[inline(never)]`, it pins the per-iteration cost so the loop genuinely executes `SPIN_ITERATIONS` times. Measured at ~0.498 µs/iteration on the nRF52833 at 64 MHz — about 32 cycles per `spin_once` call (the call/return overhead dominates the single add, which is the point: a stable, non-zero unit of work). `SPIN_ITERATIONS = 2_008_500` then gives roughly a 1 s window, which sets both the averaging period and the reporting cadence.

## Calibration is just the same spin, run early

$T_0$ is not a magic constant — it is measured at boot, in `main`, **before any task is spawned**:

```rust
let calibration_baseline = tasks::load_profiler::calibrate();
```

At that moment the CPU is genuinely idle (only the time-driver tick runs, which is a legitimate part of the baseline). So whatever `SPIN_ITERATIONS` happens to be, `calibrate` records its true zero-load duration. The exact iteration count stops mattering — the baseline self-adjusts. That is why the doc comment says "the exact value is not critical."

## Two decimals out of defmt, which has no float precision

A naive integer-percent report (`load = 100 * (1 - T0/T1)`) truncates everything below 1% to `0` — and an idle quadcopter sits well under 1%, so the readout was a permanent, suspicious `0%`. The fix is more resolution, but defmt fights back:

**`defmt` has no float-precision format hint.** `{:.2}` and `{=f32:.2}` both fail to compile with `unknown display hint`. `{:02}` exists but is integer width/zero-padding, not decimal places. (This is a deliberate defmt constraint: float formatting on the host side is kept minimal.)

The workaround is to do the fixed-point scaling yourself — compute the load in **basis points** (hundredths of a percent, 0..10_000) and split it into integer and fractional parts for printing:

```rust
let load_bp = 10_000u64.saturating_sub(baseline_us * 10_000 / elapsed_us);
defmt::info!("system load: {}.{:02} %", load_bp / 100, load_bp % 100);
```

`load_bp / 100` is the whole-percent part, `load_bp % 100` the two-decimal remainder, zero-padded with `{:02}`. The `* 10_000` is done in `u64` to avoid overflow, and `saturating_sub` clamps the (impossible-but-cheap-to-guard) case where rounding makes `elapsed < baseline`. Over a ~1 s window the 30.5 µs embassy-time tick gives ~0.003% granularity, so two decimals is genuinely meaningful, not noise.

## The limit: cooperative starvation looks like a frozen reading

The method has one sharp failure mode, and it is worth understanding because it bit during validation. The spinner only gets to re-time and re-publish *between* windows. If a single P6 task ever busy-spends a **full tick period or more without yielding**, two things happen at once:

1. That task's `Ticker::next()` is always already due, so it never awaits — it monopolises the P6 executor and starves its task-mates cooperatively (no telemetry, etc.).
2. The thread-mode spinner, being strictly lower priority, never runs at all — so it stops printing and its last published load value freezes.

So a *stuck* reading (rather than a high one) is the signature of cooperative starvation in the P6 tier, not of the profiler being broken. During testing, injecting a busy `cortex_m::asm::delay(N)` into a P6 task produced a clean linear load curve at small `N` (≈ `1.6 × N / 640_000` per 10 ms tick — `asm::delay` burns ~1.6 cycles per count on the M4), right up until `N` approached a full tick, at which point the whole thing fell off the cliff into starvation. The profiler measures *preemption* faithfully; it cannot measure a task that refuses to yield, because that pathology takes down the measurement machinery too.

## Caveats

- It is a **statistical, windowed** average over ~1 s, not an instantaneous load. Short bursts get smeared across the window.
- It measures **CPU contention**, i.e. preemption of the lowest tier. Time the CPU spends asleep in `wfi` with nothing pending counts as idle (correct), but the method cannot distinguish "asleep" from "spinning the idle task" — both are idle, which is the right answer anyway.
- The baseline assumes the spinner's per-iteration cost is stable. Flash wait states / branch prediction make it stable enough here, but on a chip with caches the baseline would want re-measuring per build.

## Takeaway

- You can measure CPU load with no scheduler accounting by making the **idle** visible: a lone lowest-priority spinner of fixed work, timed against a free-running clock. Load is $1 - T_0/T_1$.
- The spinner must be the *sole* lowest-priority task and must *never* `.await` — otherwise cooperative peers masquerade as idle.
- The free-running RTC behind `Instant` is what makes stolen cycles count automatically.
- `black_box` + `inline(never)` keep the optimiser from deleting the very work you are timing.
- defmt has no float precision; reach for fixed-point basis points and split the digits by hand.
- A *frozen* reading means a P6 task is hogging its executor, not that the profiler failed.
