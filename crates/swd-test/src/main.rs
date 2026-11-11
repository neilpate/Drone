use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const SPEED_KHZ: f64 = 100.0;
const MAX_POINTS: usize = 1_000_000;

struct Scope {
    connection: BufReader<TcpStream>,
}

impl Scope {
    fn connect(address: &str) -> Result<Self> {
        let address = address.parse()?;
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(Self {
            connection: BufReader::new(stream),
        })
    }

    fn command(&mut self, command: &str) -> Result<()> {
        self.connection
            .get_mut()
            .write_all(format!("{command}\n").as_bytes())?;
        Ok(())
    }

    fn query(&mut self, command: &str) -> Result<String> {
        self.command(command)?;
        let mut reply = String::new();
        let count = self
            .connection
            .read_line(&mut reply)
            .map_err(|error| format!("SCPI query {command}: {error}"))?;
        if count == 0 {
            return Err("Scope closed the connection".into());
        }
        Ok(reply.trim().to_owned())
    }

    fn check_errors(&mut self) -> Result<()> {
        let reply = self.query(":SYSTem:ERRor?")?;
        if reply.split(',').next() != Some("0") {
            return Err(format!("Scope SCPI error: {reply}").into());
        }
        Ok(())
    }

    fn configure(&mut self) -> Result<()> {
        self.command(":STOP")?;
        for channel in 1..=2 {
            for setting in ["DISPlay ON", "COUPling DC", "SCALe 1", "OFFSet -1.5"] {
                self.command(&format!(":CHANnel{channel}:{setting}"))?;
            }
        }
        for command in [
            ":ACQuire:TYPE NORMal",
            ":ACQuire:MDEPth 1M",
            ":TIMebase:MODE MAIN",
            ":TIMebase:MAIN:SCALe 0.001",
            ":TIMebase:MAIN:OFFSet 0",
            ":TRIGger:MODE EDGE",
            ":TRIGger:EDGE:SOURce CHANnel1",
            ":TRIGger:EDGE:SLOPe POSitive",
            ":TRIGger:EDGE:LEVel 1.5",
        ] {
            self.command(command)?;
        }
        self.check_errors()
    }

    fn wait_for(&mut self, expected: &[&str], timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        loop {
            let state = self.query(":TRIGger:STATus?")?;
            if expected.contains(&state.as_str()) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "Scope timeout: expected {expected:?}, got {state}. \
                     Check CH1 wiring and trigger; no SWCLK capture was established."
                )
                .into());
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn waveform(&mut self, channel: u8) -> Result<Waveform> {
        self.command(&format!(":WAVeform:SOURce CHANnel{channel}"))?;
        self.command(":WAVeform:MODE RAW")?;
        self.command(":WAVeform:FORMat BYTE")?;
        // DHO804 reports the selected transfer window, not the full record length.
        self.command(":WAVeform:STARt 1")?;
        self.command(&format!(":WAVeform:STOP {MAX_POINTS}"))?;
        let reply = self.query(":WAVeform:PREamble?")?;
        let fields: Vec<&str> = reply.split(',').collect();
        if fields.len() != 10 || fields[0] != "0" {
            return Err(format!("Unsupported BYTE waveform preamble: {reply}").into());
        }
        let points: usize = fields[2].parse()?;
        let interval: f64 = fields[4].parse()?;
        let time_origin: f64 = fields[5].parse()?;
        let time_reference: f64 = fields[6].parse()?;
        let increment: f64 = fields[7].parse()?;
        let origin: f64 = fields[8].parse()?;
        let reference: f64 = fields[9].parse()?;
        if !(2..=MAX_POINTS).contains(&points)
            || !interval.is_finite()
            || interval <= 0.0
            || !increment.is_finite()
            || increment <= 0.0
            || !origin.is_finite()
            || !reference.is_finite()
            || !time_origin.is_finite()
            || !time_reference.is_finite()
        {
            return Err(format!("Invalid waveform preamble: {reply}").into());
        }
        let mut samples = Vec::with_capacity(points);
        for start in (1..=points).step_by(200_000) {
            let stop = (start + 199_999).min(points);
            self.command(&format!(":WAVeform:STARt {start}"))?;
            self.command(&format!(":WAVeform:STOP {stop}"))?;
            self.command(":WAVeform:DATA?")?;
            let data = read_block(&mut self.connection, stop - start + 1)?;
            samples.extend(
                data.into_iter()
                    .map(|code| (f64::from(code) - origin - reference) * increment),
            );
        }
        self.check_errors()?;
        Ok(Waveform {
            samples,
            interval,
            start: time_origin - time_reference * interval,
        })
    }
}

fn read_block(reader: &mut impl Read, expected: usize) -> Result<Vec<u8>> {
    let mut prefix = [0; 2];
    reader.read_exact(&mut prefix)?;
    if prefix[0] != b'#' || !(b'1'..=b'9').contains(&prefix[1]) {
        return Err("Invalid SCPI definite-length block header".into());
    }
    let mut length = vec![0; usize::from(prefix[1] - b'0')];
    reader.read_exact(&mut length)?;
    let length: usize = std::str::from_utf8(&length)?.parse()?;
    if length != expected {
        return Err(format!("Expected {expected} waveform bytes, received {length}").into());
    }
    let mut data = vec![0; length];
    reader.read_exact(&mut data)?;
    let mut ending = [0];
    reader.read_exact(&mut ending)?;
    if ending[0] == b'\r' {
        reader.read_exact(&mut ending)?;
    }
    if ending[0] != b'\n' {
        return Err("Missing SCPI block terminator".into());
    }
    Ok(data)
}

struct Waveform {
    samples: Vec<f64>,
    interval: f64,
    start: f64,
}

struct Measurements {
    low: f64,
    high: f64,
    rising: Vec<usize>,
    falling: usize,
}

fn measure(waveform: &Waveform) -> Result<Measurements> {
    if waveform.samples.len() < 100
        || !waveform.interval.is_finite()
        || waveform.interval <= 0.0
        || waveform.samples.iter().any(|sample| !sample.is_finite())
    {
        return Err("Not enough valid waveform samples".into());
    }
    let mut sorted = waveform.samples.clone();
    sorted.sort_by(f64::total_cmp);
    let low = sorted[sorted.len() / 100];
    let high = sorted[sorted.len() * 99 / 100];
    let mut rising = Vec::new();
    let mut falling = 0;
    let mut is_high = waveform.samples[0] >= 1.5;
    // Fixed Schmitt thresholds reject noise on a flat or floating input.
    for (index, &voltage) in waveform.samples.iter().enumerate() {
        if !is_high && voltage >= 2.0 {
            rising.push(index);
            is_high = true;
        } else if is_high && voltage <= 0.8 {
            falling += 1;
            is_high = false;
        }
    }
    Ok(Measurements {
        low,
        high,
        rising,
        falling,
    })
}

fn levels_ok(measurements: &Measurements) -> bool {
    (-0.3..=0.5).contains(&measurements.low) && (2.7..=3.6).contains(&measurements.high)
}

fn measure_active_data(data: &Waveform, rising: &[usize]) -> Result<Measurements> {
    let first = *rising
        .first()
        .ok_or("No SWCLK edges for data measurement")?;
    let last = *rising.last().ok_or("No SWCLK edges for data measurement")?;
    let samples = data
        .samples
        .get(first..=last)
        .ok_or("SWCLK edges are outside the SWDIO record")?;
    // Exclude long idle periods so sparse SWDIO bits still contribute to rail estimates.
    measure(&Waveform {
        samples: samples.to_vec(),
        interval: data.interval,
        start: data.start + first as f64 * data.interval,
    })
}

fn clock_frequency(measurements: &Measurements, interval: f64) -> Result<f64> {
    if measurements.rising.len() < 11 || measurements.falling < 10 {
        return Err("Fewer than 10 complete SWCLK periods".into());
    }
    let mut periods: Vec<f64> = measurements
        .rising
        .windows(2)
        .map(|pair| (pair[1] - pair[0]) as f64 * interval)
        .collect();
    periods.sort_by(f64::total_cmp);
    // Burst gaps are outliers; use the median instead of averaging over idle time.
    Ok(1.0 / periods[periods.len() / 2])
}

fn timing_ok(frequency: f64, interval: f64) -> bool {
    (frequency / (SPEED_KHZ * 1000.0) - 1.0).abs() <= 0.2 && interval * frequency <= 0.05
}

fn swd_bits(clock: &Waveform, data: &Waveform, rising: &[usize]) -> Result<Vec<u8>> {
    if clock.samples.len() != data.samples.len()
        || clock.interval != data.interval
        || (clock.start - data.start).abs() > clock.interval * 0.5
    {
        return Err("Channel sample grids do not align; cannot decode SWD".into());
    }
    rising
        .iter()
        .map(|&index| {
            let voltage = data.samples[index];
            if voltage <= 0.8 {
                Ok(0)
            } else if voltage >= 2.0 {
                Ok(1)
            } else {
                Err(format!("Ambiguous SWDIO level {voltage:.3} V at clock sample {index}").into())
            }
        })
        .collect()
}

fn lsb_value(bits: &[u8]) -> u16 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (index, &bit)| value | (u16::from(bit) << index))
}

#[derive(Debug)]
struct SwdRequest {
    selection_start: usize,
    reset_clocks: usize,
    request_start: usize,
    header: u8,
    ack: Option<u16>,
}

fn decode_swd(bits: &[u8]) -> Result<SwdRequest> {
    // Anchor decoding to reset + JTAG-to-SWD selection, not arbitrary 8-bit matches.
    let selection_start = (50..bits.len().saturating_sub(15))
        .find(|&index| {
            bits[index - 50..index].iter().all(|&bit| bit == 1)
                && lsb_value(&bits[index..index + 16]) == 0xe79e
        })
        .ok_or("No reset (>=50 high clocks) followed by JTAG-to-SWD code 0xE79E")?;
    let reset_start = selection_start + 16;
    let reset_clocks = bits[reset_start..]
        .iter()
        .take_while(|&&bit| bit == 1)
        .count();
    if reset_clocks < 50 {
        return Err("Missing >=50 high-clock SWD line reset after selection".into());
    }
    let idle_start = reset_start + reset_clocks;
    let idle_clocks = bits[idle_start..]
        .iter()
        .take_while(|&&bit| bit == 0)
        .count();
    if idle_clocks < 2 {
        return Err("Missing >=2 low idle clocks after SWD line reset".into());
    }
    let request_start = idle_start + idle_clocks;
    let header_bits = bits
        .get(request_start..request_start + 8)
        .ok_or("Capture ends before a complete SWD request")?;
    if header_bits[0] != 1
        || header_bits[6] != 0
        || header_bits[7] != 1
        || header_bits[1..=5].iter().copied().sum::<u8>() % 2 != 0
    {
        return Err("Invalid SWD request start/stop/park bits or even parity".into());
    }
    let header = lsb_value(header_bits) as u8;
    if header != 0xa5 {
        return Err(format!("Expected DPIDR read request 0xA5, decoded 0x{header:02X}").into());
    }
    // One turnaround clock follows the request; an unconnected line is not an ACK.
    let ack = bits
        .get(request_start + 9..request_start + 12)
        .map(lsb_value);
    Ok(SwdRequest {
        selection_start,
        reset_clocks,
        request_start,
        header,
        ack,
    })
}

fn stimulate(scope: &mut Scope, probe: &str) -> Result<()> {
    println!("Requesting SWD at {SPEED_KHZ:.0} kHz. A no-target error is expected:");
    let mut child = Command::new("probe-rs")
        .args([
            "info",
            "--probe",
            probe,
            "--protocol",
            "swd",
            "--speed",
            "100",
            "--non-interactive",
        ])
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(15);
    let result = (|| -> Result<()> {
        loop {
            if let Some(status) = child.try_wait()? {
                println!("probe-rs exited with {status} (not the electrical test verdict).");
                return Ok(());
            }
            if scope.query(":TRIGger:STATus?")? == "STOP" {
                println!(
                    "Burst captured; stopping probe-rs without waiting for no-target retries."
                );
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("No capture within 15 seconds of starting probe-rs".into());
            }
            thread::sleep(Duration::from_millis(50));
        }
    })();
    if child.try_wait()?.is_none() {
        child.kill()?;
    }
    child.wait()?;
    result
}

fn run() -> Result<bool> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "Usage: swd-test [scope-ip:port] [probe-selector]\n\
             Defaults: 192.168.2.30:5555 and 2e8a:000c:E66594D0637D4534\n\
             Target DISCONNECTED; CH1=SWCLK, CH2=SWDIO, both grounds=Pico GND.\n\
             Match scope attenuation settings to the physical probes.\n\
             Changes acquisition/channel/trigger settings; leaves the scope stopped.\n\
             Tests levels, clock timing and the initial SWD selection/DPIDR request.\n\
             Does not verify target communication."
        );
        return Ok(true);
    }
    if args.len() > 2 {
        return Err("Too many arguments; use --help".into());
    }
    let address = args.first().map_or("192.168.2.30:5555", String::as_str);
    let probe = args
        .get(1)
        .map_or("2e8a:000c:E66594D0637D4534", String::as_str);
    println!("Target must be DISCONNECTED. CH1=SWCLK; CH2=SWDIO.");
    let mut scope = Scope::connect(address)?;
    let identity = scope.query("*IDN?")?;
    println!("Scope: {identity}");
    if !identity.starts_with("RIGOL TECHNOLOGIES,DHO804,") {
        return Err("This tool has only been verified for the Rigol DHO804".into());
    }
    scope.check_errors()?;
    for channel in 1..=2 {
        let attenuation = scope.query(&format!(":CHANnel{channel}:PROBe?"))?;
        println!("CH{channel} probe attenuation: {attenuation}x (must match probe switch).");
    }
    scope.configure()?;
    let capture = (|| -> Result<(Waveform, Waveform)> {
        scope.command(":SINGle")?;
        scope.wait_for(&["WAIT"], Duration::from_secs(5))?;
        stimulate(&mut scope, probe)?;
        scope.wait_for(&["STOP"], Duration::from_secs(5))?;
        Ok((scope.waveform(1)?, scope.waveform(2)?))
    })();
    let stopped = scope.command(":STOP");
    let (clock, data) = capture?;
    stopped?;
    let clock_measurements = measure(&clock)?;
    let bits = swd_bits(&clock, &data, &clock_measurements.rising)?;
    let data_measurements = measure_active_data(&data, &clock_measurements.rising)?;
    let mut passed = true;
    for (name, measurements) in [
        ("SWCLK", &clock_measurements),
        ("SWDIO", &data_measurements),
    ] {
        let valid =
            levels_ok(measurements) && measurements.rising.len() >= 2 && measurements.falling >= 2;
        println!(
            "{} {name}: low={:.3} V, high={:.3} V, rising={}, falling={}",
            if valid { "PASS" } else { "FAIL" },
            measurements.low,
            measurements.high,
            measurements.rising.len(),
            measurements.falling,
        );
        passed &= valid;
    }
    let frequency = clock_frequency(&clock_measurements, clock.interval)?;
    let timing_ok = timing_ok(frequency, clock.interval);
    println!(
        "{} SWCLK: {:.2} kHz; requested {SPEED_KHZ:.0} kHz (+/-20%); {:.1} samples/period",
        if timing_ok { "PASS" } else { "FAIL" },
        frequency / 1000.0,
        1.0 / (clock.interval * frequency),
    );
    passed &= timing_ok;
    match decode_swd(&bits) {
        Ok(request) => {
            println!(
                "PASS SWD: 0xE79E JTAG-to-SWD at clock {}, then {} reset clocks; \
                 0x{:02X} DP read DPIDR at clock {} (start/stop/park and even parity valid).",
                request.selection_start + 1,
                request.reset_clocks,
                request.header,
                request.request_start + 1,
            );
            if let Some(ack) = request.ack {
                println!(
                    "ACK-slot sampled value: 0b{ack:03b}; target disconnected, \
                     so this is NOT a verified acknowledgement."
                );
            }
        }
        Err(error) => {
            println!("FAIL SWD protocol: {error}");
            passed = false;
        }
    }
    println!(
        "{}: initial outgoing SWD sequence checked; target response and later transfers NOT verified.",
        if passed { "PASS" } else { "FAIL" },
    );
    Ok(passed)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("ERROR: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn block_preserves_binary_newlines_and_accepts_crlf() {
        let mut bytes = Cursor::new(b"#14\0\n\r\xff\r\n");
        assert_eq!(read_block(&mut bytes, 4).unwrap(), [0, 10, 13, 255]);
    }

    #[test]
    fn block_rejects_bad_headers_lengths_and_truncation() {
        for input in [
            &b"oops"[..],
            &b"#0"[..],
            &b"#15abcde\n"[..],
            &b"#14abc"[..],
            &b"#14abcdX"[..],
        ] {
            assert!(read_block(&mut Cursor::new(input), 4).is_err());
        }
    }

    #[test]
    fn clock_measurement_ignores_idle_gaps() {
        let mut samples = vec![0.0; 1000];
        for cycle in 0..50 {
            samples.extend(vec![3.3; 50]);
            samples.extend(vec![0.0; 50]);
            if cycle == 25 {
                samples.extend(vec![0.0; 1000]);
            }
        }
        let waveform = Waveform {
            samples,
            interval: 1e-7,
            start: 0.0,
        };
        let measurements = measure(&waveform).unwrap();
        assert!(levels_ok(&measurements));
        assert_eq!(measurements.rising.len(), 50);
        let frequency = clock_frequency(&measurements, waveform.interval).unwrap();
        assert!((frequency - 100_000.0).abs() < 1.0);
    }

    #[test]
    fn timing_requires_requested_frequency_and_sufficient_sampling() {
        assert!(timing_ok(100_000.0, 1e-7));
        assert!(timing_ok(85_000.0, 1e-7));
        assert!(!timing_ok(10_000.0, 1e-7));
        assert!(!timing_ok(125_000.0, 1e-7));
        assert!(!timing_ok(100_000.0, 1e-6));
    }

    #[test]
    fn levels_require_low_and_high_rails() {
        let measurements = |low, high| Measurements {
            low,
            high,
            rising: Vec::new(),
            falling: 0,
        };
        assert!(levels_ok(&measurements(-0.1, 3.3)));
        assert!(!levels_ok(&measurements(0.7, 3.3)));
        assert!(!levels_ok(&measurements(0.0, 1.8)));
        assert!(!levels_ok(&measurements(0.0, 5.0)));
    }

    #[test]
    fn flat_noise_and_wrong_voltage_do_not_pass() {
        for samples in [
            vec![0.0; 1000],
            vec![3.3; 1000],
            (0..1000).map(|i| 1.5 + (i % 2) as f64 * 0.05).collect(),
            (0..1000)
                .map(|i| if i % 2 == 0 { 0.0 } else { 5.0 })
                .collect(),
        ] {
            let waveform = Waveform {
                samples,
                interval: 1e-7,
                start: 0.0,
            };
            let measurements = measure(&waveform).unwrap();
            assert!(
                !levels_ok(&measurements)
                    || clock_frequency(&measurements, waveform.interval).is_err()
            );
        }
    }

    fn protocol_bits() -> Vec<u8> {
        let mut bits = vec![1; 56];
        bits.extend((0..16).map(|index| ((0xe79e >> index) & 1) as u8));
        bits.extend([1; 56]);
        bits.extend([0; 8]);
        bits.extend((0..8).map(|index| ((0xa5 >> index) & 1) as u8));
        bits.extend([1; 4]);
        bits
    }

    #[test]
    fn decodes_reset_selection_and_dpidr_request() {
        let request = decode_swd(&protocol_bits()).unwrap();
        assert_eq!(request.selection_start, 56);
        assert_eq!(request.reset_clocks, 56);
        assert_eq!(request.request_start, 136);
        assert_eq!(request.header, 0xa5);
        assert_eq!(request.ack, Some(7));
    }

    #[test]
    fn data_rail_measurement_excludes_long_idle_periods() {
        let mut data = Waveform {
            samples: vec![3.3; 100_000],
            interval: 1e-7,
            start: 0.0,
        };
        data.samples[50_000..50_100].fill(0.0);
        assert!(!levels_ok(&measure(&data).unwrap()));
        assert!(levels_ok(
            &measure_active_data(&data, &[49_500, 50_500]).unwrap()
        ));
        assert!(measure_active_data(&data, &[]).is_err());
    }

    #[test]
    fn rejects_corrupt_selection_header_and_short_capture() {
        for index in [56, 136, 141, 142, 143] {
            let mut bits = protocol_bits();
            bits[index] ^= 1;
            assert!(decode_swd(&bits).is_err(), "bit {index}");
        }
        assert!(decode_swd(&[1; 236]).is_err());
        assert!(decode_swd(&protocol_bits()[..140]).is_err());
    }

    #[test]
    fn samples_data_on_clock_rising_edges_and_checks_alignment() {
        let clock = Waveform {
            samples: vec![0.0; 100],
            interval: 1e-7,
            start: 0.0,
        };
        let mut data = Waveform {
            samples: vec![0.0; 100],
            interval: 1e-7,
            start: 0.0,
        };
        data.samples[20] = 3.3;
        assert_eq!(swd_bits(&clock, &data, &[10, 20]).unwrap(), [0, 1]);
        data.samples[20] = 1.5;
        assert!(swd_bits(&clock, &data, &[20]).is_err());
        data.start = 1e-6;
        assert!(swd_bits(&clock, &data, &[10]).is_err());
    }
}
