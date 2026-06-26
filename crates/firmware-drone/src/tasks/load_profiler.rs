use embassy_time::{Duration, Instant};

use firmware_types::CpuLoad;

use crate::signals::cpu_load;

/// Spin iterations per profiling sample. Sized so one pass takes on the order of
/// a second at zero load, which sets both the load-averaging window and the
/// reporting cadence. The exact value is not critical: `calibrate` measures the
/// real zero-load duration for whatever value this is. Tune it to change the
/// window length. Measured at ~0.498 us/iteration on the nRF52833 at 64 MHz, so
/// 2_008_500 iterations gives a ~1 s window.
const SPIN_ITERATIONS: u32 = 2_008_500;

/// One unit of profiler work. `#[inline(never)]` plus `black_box` pin the
/// per-iteration cost and stop the optimiser from collapsing the loop into a
/// constant, so calibration and the runtime loop measure the same thing.
#[inline(never)]
fn spin_once(counter: u32) -> u32 {
    core::hint::black_box(counter.wrapping_add(1))
}

/// Run one spin window and return how long it took (wall-clock). The RTC behind
/// `Instant` is free-running, so the elapsed time includes any cycles stolen by
/// higher-priority work that preempted the spin.
fn spin_window() -> Duration {
    let start = Instant::now();
    let mut counter: u32 = 0;
    for _ in 0..SPIN_ITERATIONS {
        counter = spin_once(counter);
    }
    core::hint::black_box(counter);
    start.elapsed()
}

/// Zero-load baseline `T0`: how long one spin window takes with nothing else
/// running. Call this in `main` *before* any other task is spawned, so the CPU
/// is otherwise idle (only the time-driver tick runs, which is part of the
/// genuine baseline).
pub fn calibrate() -> Duration {
    spin_window()
}

/// Lowest-priority task and the sole thread-mode task. It busy-spins and never
/// awaits, so it only runs on cycles no higher-priority work wanted. Each window
/// it re-times the same spin; under load the window stretches from the baseline
/// `T0` to `T1`, and `1 - T0/T1` is the system load.
#[embassy_executor::task]
pub async fn load_profiler(baseline: Duration) -> ! {
    defmt::info!("load profiler task: started");

    let baseline_us = baseline.as_micros().max(1);

    loop {
        let elapsed_us = spin_window().as_micros().max(1);

        // elapsed_us >= baseline_us under contention. Work in basis points
        // (hundredths of a percent) so sub-1% loads are visible: integer percent
        // truncated everything below 1% to 0. Over a ~1 s window the 30.5 us
        // embassy-time tick gives ~0.003% granularity, so two decimals is
        // meaningful. Computed in u64 to avoid overflow on the `* 10_000`.
        let load_bp = 10_000u64.saturating_sub(baseline_us * 10_000 / elapsed_us);
        let load = CpuLoad::from_percentage((load_bp as f32) / 100.0);

        defmt::info!("system load: {}.{:02} %", load_bp / 100, load_bp % 100);

        cpu_load::set(load);
    }
}
