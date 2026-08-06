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
struct Log {
    columns: HashMap<String, Vec<f64>>,
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

        let mut n = 0;
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let cells: Vec<&str> = line.split('\t').collect();
            for (i, name) in names.iter().enumerate() {
                let v = cells
                    .get(i)
                    .and_then(|c| c.trim().parse::<f64>().ok())
                    .unwrap_or(f64::NAN);
                columns.get_mut(*name).unwrap().push(v);
            }
            n += 1;
        }
        Ok(Self { columns, n })
    }

    /// The whole column as a slice, or an empty slice if the name is absent.
    fn col(&self, name: &str) -> &[f64] {
        self.columns.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// The string form of a column for categorical fields (e.g. drone_state),
    /// re-read from the raw file is overkill; instead we bucket by the numeric
    /// code the ground station emits — but drone_state is a `{:?}` string, so we
    /// keep a parallel string column loader for it below.
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

    // drone_state is a categorical `{:?}` string; reload just that column as text.
    let state_counts = load_state_counts(&path);

    let t = log.col("t_s");
    let throttle = log.col("throttle");

    // Segment into active vs idle by throttle.
    let active: Vec<usize> = (0..log.n)
        .filter(|&i| throttle.get(i).copied().unwrap_or(0.0) > ACTIVE_THROTTLE)
        .collect();
    let idle: Vec<usize> = (0..log.n)
        .filter(|&i| throttle.get(i).copied().unwrap_or(0.0) <= ACTIVE_THROTTLE)
        .collect();

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
    println!(
        "throttle max={thr_max:.3}  active samples={} (>{ACTIVE_THROTTLE})  idle samples={}",
        active.len(),
        idle.len()
    );

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

    if log.has("gyro_fx_dps") {
        println!("Gyro raw vs filtered (dps), active phase:");
        print_stat(
            "gyro_x raw",
            &stats_over(log.col("gyro_x_dps"), &active),
            "dps",
        );
        print_stat(
            "gyro_x filt",
            &stats_over(log.col("gyro_fx_dps"), &active),
            "dps",
        );
        print_stat(
            "gyro_y raw",
            &stats_over(log.col("gyro_y_dps"), &active),
            "dps",
        );
        print_stat(
            "gyro_y filt",
            &stats_over(log.col("gyro_fy_dps"), &active),
            "dps",
        );
    }

    ExitCode::SUCCESS
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

/// drone_state is written as a `{:?}` string, so bucket it by re-reading the
/// file and counting the categorical column directly.
fn load_state_counts(path: &str) -> Vec<(String, usize)> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let names: Vec<&str> = header.split('\t').collect();
    let Some(state_idx) = names.iter().position(|n| *n == "drone_state") else {
        return Vec::new();
    };
    let mut counts: HashMap<String, usize> = HashMap::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(cell) = line.split('\t').nth(state_idx) {
            *counts.entry(cell.to_string()).or_insert(0) += 1;
        }
    }
    let mut v: Vec<(String, usize)> = counts.into_iter().collect();
    v.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    v
}
