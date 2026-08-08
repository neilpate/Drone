//! Telemetry log analyser.
//!
//! Post-processes a `telemetry-*.tsv` capture (written by the ground station's
//! "Log to TSV" button) into a readable diagnostic report: phase segmentation,
//! per-channel statistics over the active-throttle window, attitude drift
//! direction and rate, motor asymmetry, and demand saturation.
//!
//! Usage:
//!   cargo run --bin analyze -- <path-to-telemetry.tsv>
//!
//! The parser is header-driven (columns are looked up by name), so it tolerates
//! column reordering or additions as the telemetry frame evolves.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::ExitCode;

/// Throttle above which a sample is considered "active" (motors meaningfully
/// spinning), used to segment the idle and active phases of a run.
const ACTIVE_THROTTLE: f64 = 0.05;

/// A parsed column-oriented view of the TSV: name -> values, with one entry per
/// data row. Missing/blank cells parse as NaN so per-column stats can skip them.
/// Every cell is also retained as a string so categorical fields (drone_state,
/// control_mode) can be used to filter rows.
struct Log {
    columns: HashMap<String, Vec<f64>>,
    strings: HashMap<String, Vec<String>>,
    n: usize,
}

impl Log {
    fn load(path: &str) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        let mut lines = text.lines();
        let header = lines.next().ok_or("empty file")?;
        let names: Vec<&str> = header.split('\t').collect();

        let mut columns: HashMap<String, Vec<f64>> =
            names.iter().map(|n| (n.to_string(), Vec::new())).collect();
        let mut strings: HashMap<String, Vec<String>> =
            names.iter().map(|n| (n.to_string(), Vec::new())).collect();

        let mut n = 0;
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let cells: Vec<&str> = line.split('\t').collect();
            for (i, name) in names.iter().enumerate() {
                let cell = cells.get(i).map(|c| c.trim()).unwrap_or("");
                let v = cell.parse::<f64>().unwrap_or(f64::NAN);
                columns.get_mut(*name).unwrap().push(v);
                strings.get_mut(*name).unwrap().push(cell.to_string());
            }
            n += 1;
        }
        Ok(Self {
            columns,
            strings,
            n,
        })
    }

    /// The whole column as a slice, or an empty slice if the name is absent.
    fn col(&self, name: &str) -> &[f64] {
        self.columns.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// The string form of a column, or an empty slice if the name is absent.
    fn col_str(&self, name: &str) -> &[String] {
        self.strings.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// True if a numeric column of that name is present.
    fn has(&self, name: &str) -> bool {
        self.columns.contains_key(name)
    }
}

/// Summary statistics over a set of sample indices for one column.
struct Stats {
    n: usize,
    mean: f64,
    sd: f64,
    min: f64,
    max: f64,
}

fn stats_over(values: &[f64], idx: &[usize]) -> Stats {
    let mut vs: Vec<f64> = idx
        .iter()
        .filter_map(|&i| values.get(i).copied())
        .filter(|v| !v.is_nan())
        .collect();
    if vs.is_empty() {
        return Stats {
            n: 0,
            mean: f64::NAN,
            sd: f64::NAN,
            min: f64::NAN,
            max: f64::NAN,
        };
    }
    let n = vs.len();
    let mean = vs.iter().sum::<f64>() / n as f64;
    let var = vs.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n as f64;
    let sd = var.sqrt();
    vs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Stats {
        n,
        mean,
        sd,
        min: vs[0],
        max: vs[n - 1],
    }
}

/// Least-squares slope of `y` against `t` over the given indices, in y-units per
/// second. Used to quantify a steady drift (e.g. attitude walking off in one
/// direction) rather than reacting to instantaneous noise.
fn slope_per_s(t: &[f64], y: &[f64], idx: &[usize]) -> f64 {
    let pts: Vec<(f64, f64)> = idx
        .iter()
        .filter_map(|&i| Some((*t.get(i)?, *y.get(i)?)))
        .filter(|(a, b)| !a.is_nan() && !b.is_nan())
        .collect();
    let n = pts.len();
    if n < 2 {
        return f64::NAN;
    }
    let nf = n as f64;
    let mt = pts.iter().map(|(a, _)| a).sum::<f64>() / nf;
    let my = pts.iter().map(|(_, b)| b).sum::<f64>() / nf;
    let mut num = 0.0;
    let mut den = 0.0;
    for (a, b) in &pts {
        num += (a - mt) * (b - my);
        den += (a - mt) * (a - mt);
    }
    if den == 0.0 { f64::NAN } else { num / den }
}

/// Estimate the dominant oscillation in a signal over the given sample indices.
///
/// The amplitude statistics (sd) can't tell a slow wander from a fast buzz, and
/// a small-amplitude high-frequency limit cycle is invisible in the angle sd
/// while dominating the gyro. This computes a periodogram to read the actual
/// frequency: a naive DFT evaluated at the real (jittery) sample timestamps, on
/// the linearly-detrended, Hann-windowed signal, scanned over a fixed grid up
/// to Nyquist (capped at 30 Hz). Detrending removes the slow drift so its
/// low-frequency leakage doesn't swamp the fast peak we're hunting.
///
/// Returns `(peak_frequency_hz, peak_amplitude)` where amplitude is the
/// approximate zero-to-peak of the dominant sinusoid, in the signal's units.
fn dominant_frequency(t: &[f64], y: &[f64], idx: &[usize]) -> Option<(f64, f64)> {
    let pts: Vec<(f64, f64)> = idx
        .iter()
        .filter_map(|&i| Some((*t.get(i)?, *y.get(i)?)))
        .filter(|(a, b)| !a.is_nan() && !b.is_nan())
        .collect();
    let n = pts.len();
    if n < 16 {
        return None;
    }
    let nf = n as f64;

    // Linear detrend: subtract the least-squares line so a steady drift doesn't
    // pile power at DC and leak into the low-frequency bins.
    let mt = pts.iter().map(|(a, _)| a).sum::<f64>() / nf;
    let my = pts.iter().map(|(_, b)| b).sum::<f64>() / nf;
    let mut num = 0.0;
    let mut den = 0.0;
    for (a, b) in &pts {
        num += (a - mt) * (b - my);
        den += (a - mt) * (a - mt);
    }
    let slope = if den == 0.0 { 0.0 } else { num / den };
    let intercept = my - slope * mt;

    let span = pts[n - 1].0 - pts[0].0;
    if span <= 0.0 {
        return None;
    }
    let t0 = pts[0].0;

    // Detrend and apply a Hann window (reduces spectral leakage from the finite,
    // non-integer-period record).
    let mut xs: Vec<(f64, f64)> = Vec::with_capacity(n);
    let mut wsum = 0.0;
    for (k, (a, b)) in pts.iter().enumerate() {
        let detr = b - (intercept + slope * a);
        let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * k as f64 / (nf - 1.0)).cos();
        wsum += w;
        xs.push((*a, detr * w));
    }

    let mean_dt = span / (nf - 1.0);
    let f_nyquist = 0.5 / mean_dt;
    let f_min = 1.0;
    let f_max = f_nyquist.min(30.0);
    if f_max <= f_min {
        return None;
    }

    let step = 0.25;
    let mut best_f = f_min;
    let mut best_power = -1.0;
    let mut f = f_min;
    while f <= f_max {
        let mut re = 0.0;
        let mut im = 0.0;
        for (a, x) in &xs {
            let phase = 2.0 * std::f64::consts::PI * f * (a - t0);
            re += x * phase.cos();
            im -= x * phase.sin();
        }
        let power = re * re + im * im;
        if power > best_power {
            best_power = power;
            best_f = f;
        }
        f += step;
    }

    // Coherent-gain correction for the Hann window: A ~= 2*|X| / sum(w).
    let amplitude = 2.0 * best_power.sqrt() / wsum;
    Some((best_f, amplitude))
}

/// Phase (in degrees) by which `lead` leads `lag` at frequency `f`, over the
/// given sample indices. Computed from the cross-spectrum:
/// `arg(DFT(lead) * conj(DFT(lag)))`, both mean-removed and Hann-windowed.
///
/// Diagnostic use: angle is the time-integral of rate, and integration adds a
/// -90 deg phase, so a rate signal should lead its angle by +90 deg (they are
/// in quadrature, not antiphase). A reading near +-180 deg means the attitude
/// estimate carries an extra ~quarter-cycle of lag at the oscillation frequency
/// - which turns proportional feedback into positive feedback and can drive the
/// very limit cycle we are chasing.
fn phase_between(t: &[f64], lead: &[f64], lag: &[f64], idx: &[usize], f: f64) -> Option<f64> {
    let pts: Vec<(f64, f64, f64)> = idx
        .iter()
        .filter_map(|&i| Some((*t.get(i)?, *lead.get(i)?, *lag.get(i)?)))
        .filter(|(a, b, c)| !a.is_nan() && !b.is_nan() && !c.is_nan())
        .collect();
    let n = pts.len();
    if n < 16 {
        return None;
    }
    let nf = n as f64;
    let mean_lead = pts.iter().map(|(_, b, _)| b).sum::<f64>() / nf;
    let mean_lag = pts.iter().map(|(_, _, c)| c).sum::<f64>() / nf;
    let t0 = pts[0].0;

    let (mut lead_re, mut lead_im, mut lag_re, mut lag_im) = (0.0, 0.0, 0.0, 0.0);
    for (k, (a, b, c)) in pts.iter().enumerate() {
        let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * k as f64 / (nf - 1.0)).cos();
        let phase = 2.0 * std::f64::consts::PI * f * (a - t0);
        let (cs, sn) = (phase.cos(), phase.sin());
        let lv = (b - mean_lead) * w;
        let mv = (c - mean_lag) * w;
        lead_re += lv * cs;
        lead_im -= lv * sn;
        lag_re += mv * cs;
        lag_im -= mv * sn;
    }
    // cross = lead * conj(lag)
    let cross_re = lead_re * lag_re + lead_im * lag_im;
    let cross_im = lead_im * lag_re - lead_re * lag_im;
    Some(cross_im.atan2(cross_re).to_degrees())
}

fn print_stat(label: &str, s: &Stats, unit: &str) {
    if s.n == 0 {
        println!("  {label:<20} (no data)");
        return;
    }
    println!(
        "  {label:<20} mean={:>9.3} sd={:>8.3} min={:>9.3} max={:>9.3} {unit}",
        s.mean, s.sd, s.min, s.max
    );
}

fn main() -> ExitCode {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: analyze <path-to-telemetry.tsv>");
            return ExitCode::FAILURE;
        }
    };

    let log = match Log::load(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    // drone_state and control_mode are categorical `{:?}` strings; count them.
    let state_counts = count_categorical(log.col_str("drone_state"));
    let mode_counts = count_categorical(log.col_str("control_mode"));

    let t = log.col("t_s");
    let throttle = log.col("throttle");
    let state = log.col_str("drone_state");
    let mode = log.col_str("control_mode");
    let have_mode = log
        .strings
        .get("control_mode")
        .is_some_and(|v| !v.is_empty());

    // The analysis window is genuine closed-loop flight: throttle up, Armed, and
    // (if the column exists) Stabilized. Mixing in Manual or failsafe samples
    // would blend open- and closed-loop behaviour and make the stats meaningless.
    let is_closed_loop = |i: usize| -> bool {
        let thr_ok = throttle.get(i).copied().unwrap_or(0.0) > ACTIVE_THROTTLE;
        let armed = state.get(i).map(|s| s == "Armed").unwrap_or(true);
        let stab = !have_mode || mode.get(i).map(|m| m == "Stabilized").unwrap_or(true);
        thr_ok && armed && stab
    };
    let active: Vec<usize> = (0..log.n).filter(|&i| is_closed_loop(i)).collect();
    let idle: Vec<usize> = (0..log.n)
        .filter(|&i| throttle.get(i).copied().unwrap_or(0.0) <= ACTIVE_THROTTLE)
        .collect();
    // Throttle-up samples that were excluded from analysis because they were
    // Manual or in a failsafe state - reported so a muddied capture is obvious.
    let excluded_active = (0..log.n)
        .filter(|&i| {
            throttle.get(i).copied().unwrap_or(0.0) > ACTIVE_THROTTLE && !is_closed_loop(i)
        })
        .count();

    let duration = if log.n >= 2 { t[log.n - 1] - t[0] } else { 0.0 };
    let mean_dt_ms = if log.n >= 2 {
        duration / (log.n - 1) as f64 * 1000.0
    } else {
        f64::NAN
    };
    let thr_max = throttle
        .iter()
        .cloned()
        .filter(|v| !v.is_nan())
        .fold(f64::MIN, f64::max);

    println!("=== Telemetry analysis: {path} ===");
    println!(
        "rows={}  duration={:.2}s  mean sample interval={:.2} ms",
        log.n, duration, mean_dt_ms
    );
    print!("drone_state:");
    for (name, count) in &state_counts {
        print!("  {name}={count}");
    }
    println!();
    if mode_counts.is_empty() {
        println!("control_mode: (not logged - old capture; cannot confirm the loop was closed)");
    } else {
        print!("control_mode:");
        for (name, count) in &mode_counts {
            print!("  {name}={count}");
        }
        println!();
        // A capture dominated by Manual is open-loop and useless for tuning; say so loudly.
        let total: usize = mode_counts.iter().map(|(_, c)| c).sum();
        let manual = mode_counts
            .iter()
            .find(|(n, _)| n == "Manual")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        if total > 0 && manual * 2 >= total {
            println!(
                "  WARNING: {:.0}% of samples are Manual (open-loop). The controller was not \
                 running for these - not a valid tuning capture.",
                100.0 * manual as f64 / total as f64
            );
        }
    }
    println!(
        "throttle max={thr_max:.3}  closed-loop active samples={} (throttle>{ACTIVE_THROTTLE}, Armed{})  idle samples={}",
        active.len(),
        if have_mode { ", Stabilized" } else { "" },
        idle.len()
    );
    if excluded_active > 0 {
        println!(
            "  note: {excluded_active} throttle-up samples excluded from analysis (Manual or failsafe)"
        );
    }

    if active.is_empty() {
        println!("\nNo active-throttle samples; nothing to analyse. Was this an idle capture?");
        return ExitCode::SUCCESS;
    }

    let active_span = t[*active.last().unwrap()] - t[active[0]];
    println!(
        "\n--- ACTIVE PHASE ({:.2}s, n={}) ---",
        active_span,
        active.len()
    );

    // Gains in force during the capture (logged per-row by the ground station).
    // If a gain was tuned live mid-capture, the varied range is shown.
    if log.has("kp_roll") {
        let g = |name: &str| -> String {
            let s = stats_over(log.col(name), &active);
            if s.n == 0 {
                "n/a".to_string()
            } else if (s.max - s.min).abs() < 1e-9 {
                format!("{:.4}", s.mean)
            } else {
                format!("{:.4} (varied {:.4}..{:.4})", s.mean, s.min, s.max)
            }
        };
        println!("Gains (as logged):");
        println!(
            "  roll  kp={}  ki={}  kd={}",
            g("kp_roll"),
            g("ki_roll"),
            g("kd_roll")
        );
        println!(
            "  pitch kp={}  ki={}  kd={}",
            g("kp_pitch"),
            g("ki_pitch"),
            g("kd_pitch")
        );
        println!(
            "  yaw   kp={}  ki={}  kd={}",
            g("kp_yaw"),
            g("ki_yaw"),
            g("kd_yaw")
        );
    }

    println!("Attitude (deg):");
    print_stat(
        "attitude_roll",
        &stats_over(log.col("attitude_roll_deg"), &active),
        "deg",
    );
    print_stat(
        "attitude_pitch",
        &stats_over(log.col("attitude_pitch_deg"), &active),
        "deg",
    );

    // Drift: the headline number for "goes off in one direction".
    let roll_drift = slope_per_s(t, log.col("attitude_roll_deg"), &active);
    let pitch_drift = slope_per_s(t, log.col("attitude_pitch_deg"), &active);
    println!("Attitude drift (least-squares slope over active phase):");
    println!(
        "  roll  {:+.2} deg/s  ({})",
        roll_drift,
        drift_dir_roll(roll_drift)
    );
    println!(
        "  pitch {:+.2} deg/s  ({})",
        pitch_drift,
        drift_dir_pitch(pitch_drift)
    );

    println!("Controller demand (-1..1):");
    print_stat(
        "demand_roll",
        &stats_over(log.col("demand_roll"), &active),
        "",
    );
    print_stat(
        "demand_pitch",
        &stats_over(log.col("demand_pitch"), &active),
        "",
    );
    print_stat(
        "demand_yaw",
        &stats_over(log.col("demand_yaw"), &active),
        "",
    );
    println!("Demand saturation (|demand| >= 0.99):");
    println!(
        "  roll  {:.0}%",
        saturation_pct(log.col("demand_roll"), &active)
    );
    println!(
        "  pitch {:.0}%",
        saturation_pct(log.col("demand_pitch"), &active)
    );
    println!(
        "  yaw   {:.0}%",
        saturation_pct(log.col("demand_yaw"), &active)
    );

    println!("Motor outputs (0..1):");
    let m1 = stats_over(log.col("motor1"), &active);
    let m2 = stats_over(log.col("motor2"), &active);
    let m3 = stats_over(log.col("motor3"), &active);
    let m4 = stats_over(log.col("motor4"), &active);
    print_stat("motor1 (rear-right)", &m1, "");
    print_stat("motor2 (front-right)", &m2, "");
    print_stat("motor3 (rear-left)", &m3, "");
    print_stat("motor4 (front-left)", &m4, "");
    print_motor_asymmetry(m1.mean, m2.mean, m3.mean, m4.mean);

    report_gyro_and_oscillation(&log, t, &active);

    // The whole active phase blends ground handling, aborted attempts, and the
    // throttle chop with the actual airborne time. Isolate the longest sustained
    // high-throttle window as the most likely real flight and report it on its
    // own, so hover behaviour is not averaged together with ground transients.
    match longest_flight_segment(throttle, &is_closed_loop, log.n) {
        Some(flight) => {
            let span = t[*flight.last().unwrap()] - t[flight[0]];
            let tag = if span < MIN_FLIGHT_S {
                " (brief hop)"
            } else {
                ""
            };
            println!(
                "\n--- FLIGHT SEGMENT{tag} ({:.2}s, n={}, t={:.1}..{:.1}s, throttle>{}) ---",
                span,
                flight.len(),
                t[flight[0]],
                t[*flight.last().unwrap()],
                FLIGHT_THROTTLE
            );
            println!("Attitude (deg):");
            print_stat(
                "attitude_roll",
                &stats_over(log.col("attitude_roll_deg"), &flight),
                "deg",
            );
            print_stat(
                "attitude_pitch",
                &stats_over(log.col("attitude_pitch_deg"), &flight),
                "deg",
            );
            let rd = slope_per_s(t, log.col("attitude_roll_deg"), &flight);
            let pd = slope_per_s(t, log.col("attitude_pitch_deg"), &flight);
            println!(
                "Attitude drift:  roll {rd:+.2} deg/s ({}),  pitch {pd:+.2} deg/s ({})",
                drift_dir_roll(rd),
                drift_dir_pitch(pd)
            );
            let m1 = stats_over(log.col("motor1"), &flight);
            let m2 = stats_over(log.col("motor2"), &flight);
            let m3 = stats_over(log.col("motor3"), &flight);
            let m4 = stats_over(log.col("motor4"), &flight);
            print_motor_asymmetry(m1.mean, m2.mean, m3.mean, m4.mean);
            report_gyro_and_oscillation(&log, t, &flight);
        }
        None => {
            println!(
                "\nNo flight segment (throttle>{FLIGHT_THROTTLE}, >={MIN_FLIGHT_SAMPLES} samples) \
                 detected - likely all ground handling."
            );
        }
    }

    ExitCode::SUCCESS
}

/// Throttle above which a sample is considered part of a flight (rather than
/// ground idle or spool-up). Chosen well above `ACTIVE_THROTTLE` and below a
/// typical hover throttle.
const FLIGHT_THROTTLE: f64 = 0.25;
/// Minimum duration of a sustained high-throttle window for it to be treated as
/// a solid flight rather than a brief hop (below this it is still reported, just
/// tagged, since a short window's oscillation stats are unreliable).
const MIN_FLIGHT_S: f64 = 1.5;
/// Minimum sample count for a flight window to be worth reporting at all.
const MIN_FLIGHT_SAMPLES: usize = 20;

/// The longest contiguous run of closed-loop samples with throttle above
/// `FLIGHT_THROTTLE`, provided it has at least `MIN_FLIGHT_SAMPLES` samples.
/// Without an altitude signal this is the best proxy for "actually airborne": a
/// sustained high-throttle stretch, as opposed to the brief spool-ups and chops
/// of ground handling that otherwise get averaged into the active-phase stats.
fn longest_flight_segment<F: Fn(usize) -> bool>(
    throttle: &[f64],
    is_closed_loop: F,
    n: usize,
) -> Option<Vec<usize>> {
    let mut best: Vec<usize> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    for i in 0..n {
        if throttle.get(i).copied().unwrap_or(0.0) > FLIGHT_THROTTLE && is_closed_loop(i) {
            cur.push(i);
        } else if !cur.is_empty() {
            if cur.len() > best.len() {
                best = std::mem::take(&mut cur);
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() > best.len() {
        best = cur;
    }
    if best.len() < MIN_FLIGHT_SAMPLES {
        return None;
    }
    Some(best)
}

/// Effective sample rate (Hz) over a set of indices, from the first/last
/// timestamp - so the Nyquist note is correct for a sub-window, not just the
/// whole capture.
fn window_sample_hz(t: &[f64], idx: &[usize]) -> f64 {
    if idx.len() < 2 {
        return f64::NAN;
    }
    let span = t[*idx.last().unwrap()] - t[idx[0]];
    if span <= 0.0 {
        return f64::NAN;
    }
    (idx.len() - 1) as f64 / span
}

/// Gyro statistics, the dominant-oscillation periodogram, and the rate-vs-angle
/// phase over a set of sample indices - shared by the active-phase and
/// flight-segment reports so both windows are characterised identically.
fn report_gyro_and_oscillation(log: &Log, t: &[f64], idx: &[usize]) {
    if log.has("gyro_fx_dps") {
        println!("Gyro raw vs filtered (dps):");
        print_stat("gyro_x raw", &stats_over(log.col("gyro_x_dps"), idx), "dps");
        print_stat(
            "gyro_x filt",
            &stats_over(log.col("gyro_fx_dps"), idx),
            "dps",
        );
        print_stat("gyro_y raw", &stats_over(log.col("gyro_y_dps"), idx), "dps");
        print_stat(
            "gyro_y filt",
            &stats_over(log.col("gyro_fy_dps"), idx),
            "dps",
        );
    }

    println!("Dominant oscillation (detrended periodogram, 1 Hz..Nyquist, capped 30 Hz):");
    let sample_hz = window_sample_hz(t, idx);
    println!(
        "  NOTE: telemetry ~{:.0} Hz -> Nyquist ~{:.0} Hz. A peak near that ceiling is unreliable \
         (true oscillation may be higher and aliased). Log gyro at the loop rate to resolve it.",
        sample_hz,
        sample_hz / 2.0
    );
    let osc = |label: &str, col: &str, unit: &str| match dominant_frequency(t, log.col(col), idx) {
        Some((f, amp)) => println!("  {label:<20} {f:>6.2} Hz  ~{amp:.1} {unit} amplitude"),
        None => println!("  {label:<20} (insufficient data)"),
    };
    if log.has("gyro_fx_dps") {
        osc("gyro_x filt", "gyro_fx_dps", "dps");
        osc("gyro_y filt", "gyro_fy_dps", "dps");
    }
    osc("attitude_roll", "attitude_roll_deg", "deg");
    osc("attitude_pitch", "attitude_pitch_deg", "deg");

    const PHASE_MIN_AMP_DPS: f64 = 4.0;
    if log.has("gyro_fx_dps") {
        if let Some((f, amp)) = dominant_frequency(t, log.col("gyro_fx_dps"), idx) {
            if amp < PHASE_MIN_AMP_DPS {
                println!(
                    "  phase: roll calm ({amp:.1} dps < {PHASE_MIN_AMP_DPS:.0}) - no coherent \
                     oscillation, phase not meaningful"
                );
            } else if let Some(ph) = phase_between(
                t,
                log.col("gyro_fx_dps"),
                log.col("attitude_roll_deg"),
                idx,
                f,
            ) {
                println!(
                    "  phase: roll rate leads roll angle by {ph:+.0} deg at {f:.2} Hz \
                     (expect +90; ~+-180 => estimator lag)"
                );
            }
        }
        if let Some((f, amp)) = dominant_frequency(t, log.col("gyro_fy_dps"), idx) {
            if amp < PHASE_MIN_AMP_DPS {
                println!(
                    "  phase: pitch calm ({amp:.1} dps < {PHASE_MIN_AMP_DPS:.0}) - no coherent \
                     oscillation, phase not meaningful"
                );
            } else if let Some(ph) = phase_between(
                t,
                log.col("gyro_fy_dps"),
                log.col("attitude_pitch_deg"),
                idx,
                f,
            ) {
                println!(
                    "  phase: pitch rate leads pitch angle by {ph:+.0} deg at {f:.2} Hz \
                     (expect +90; ~+-180 => estimator lag)"
                );
            }
        }
    }
}

/// +roll = right-side-down (ADR 0021). Describe which way a roll drift tips.
fn drift_dir_roll(slope: f64) -> &'static str {
    if slope.is_nan() {
        "n/a"
    } else if slope > 0.5 {
        "tipping right"
    } else if slope < -0.5 {
        "tipping left"
    } else {
        "roughly level"
    }
}

/// +pitch = nose-up (ADR 0021). Describe which way a pitch drift tips.
fn drift_dir_pitch(slope: f64) -> &'static str {
    if slope.is_nan() {
        "n/a"
    } else if slope > 0.5 {
        "nose rising"
    } else if slope < -0.5 {
        "nose dropping"
    } else {
        "roughly level"
    }
}

fn saturation_pct(values: &[f64], idx: &[usize]) -> f64 {
    let mut sat = 0;
    let mut tot = 0;
    for &i in idx {
        if let Some(v) = values.get(i)
            && !v.is_nan()
        {
            tot += 1;
            if v.abs() >= 0.99 {
                sat += 1;
            }
        }
    }
    if tot == 0 {
        0.0
    } else {
        100.0 * sat as f64 / tot as f64
    }
}

/// Point out which corner is working hardest — a persistent asymmetry is the
/// signature of a physical imbalance (CG, a weak motor, a bent prop) that the
/// controller is fighting.
fn print_motor_asymmetry(m1: f64, m2: f64, m3: f64, m4: f64) {
    let means = [
        ("M1 rear-right", m1),
        ("M2 front-right", m2),
        ("M3 rear-left", m3),
        ("M4 front-left", m4),
    ];
    let hi = means
        .iter()
        .cloned()
        .fold(("", f64::MIN), |a, b| if b.1 > a.1 { b } else { a });
    let lo = means
        .iter()
        .cloned()
        .fold(("", f64::MAX), |a, b| if b.1 < a.1 { b } else { a });
    let spread = hi.1 - lo.1;
    println!(
        "  asymmetry: hardest={} ({:.3}), softest={} ({:.3}), spread={:.3}",
        hi.0, hi.1, lo.0, lo.1, spread
    );
    // Left vs right and front vs rear balance hint at roll / pitch bias.
    let right = (m1 + m2) / 2.0;
    let left = (m3 + m4) / 2.0;
    let front = (m2 + m4) / 2.0;
    let rear = (m1 + m3) / 2.0;
    println!(
        "  left/right balance: L={:.3} R={:.3} (dL-R={:+.3})   front/rear: F={:.3} R={:.3} (dF-R={:+.3})",
        left,
        right,
        left - right,
        front,
        rear,
        front - rear
    );
}

/// Count the values of an already-loaded categorical column. Returns
/// (value, count) sorted most-frequent first; empty if the column is empty.
fn count_categorical(values: &[String]) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for v in values {
        if !v.is_empty() {
            *counts.entry(v.clone()).or_insert(0) += 1;
        }
    }
    let mut v: Vec<(String, usize)> = counts.into_iter().collect();
    v.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    v
}
