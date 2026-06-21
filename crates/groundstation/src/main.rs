//! Ground station: throttle + roll/pitch/yaw command sender + telemetry plotter.
//!
//! Sends the control values as a postcard + COBS framed
//! `GroundstationCommand` over a serial port at 115 200 8N1, and receives
//! postcard + COBS framed `TelemetryState` on the same port. A single I/O
//! thread handles both directions (see `serial_io_thread`) and forwards each
//! decoded `TelemetryState` to the UI thread, which appends it to a set of
//! time series and draws them in a live plot. Each signal can be shown or
//! hidden with a checkbox.

use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use postcard::accumulator::{CobsAccumulator, FeedResult};

use firmware_types::{GroundstationCommand, Pitch, Roll, TelemetryState, Throttle, Yaw};

use groundstation::{drone_state_code, encode_command, stick_to_deflection, trigger_to_throttle};

use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};

const MAX_SEND_BUFFER_SIZE: usize = 32;

/// Maximum number of samples retained per signal (the last N frames).
const MAX_POINTS: usize = 10_000;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Drone Ground Station",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

/// One plotted signal: a name, a colour, a visibility toggle and its samples.
#[derive(Debug)]
struct Series {
    name: &'static str,
    color: egui::Color32,
    visible: bool,
    /// `[t_seconds, value]` points in capture order.
    points: Vec<[f64; 2]>,
}

impl Series {
    fn new(name: &'static str, color: egui::Color32) -> Self {
        Self {
            name,
            color,
            visible: true,
            points: Vec::new(),
        }
    }

    fn push(&mut self, t: f64, value: f64) {
        self.points.push([t, value]);
        if self.points.len() > MAX_POINTS {
            let overflow = self.points.len() - MAX_POINTS;
            self.points.drain(0..overflow);
        }
    }
}

// Indices into `App::series`. Kept as constants so sample-pushing and drawing
// stay in sync without a map lookup.
const SERIES_THROTTLE: usize = 0;
const SERIES_ROLL: usize = 1;
const SERIES_PITCH: usize = 2;
const SERIES_YAW: usize = 3;
const SERIES_DRONE_STATE: usize = 4;

struct App {
    port_name: String,
    throttle: f32,
    roll: f32,
    pitch: f32,
    yaw: f32,
    tx: Option<mpsc::Sender<GroundstationCommand>>,
    telemetry_rx: Option<mpsc::Receiver<TelemetryState>>,
    status: String,
    start: Instant,
    series: Vec<Series>,
    last: Option<TelemetryState>,
    gilrs: Option<Gilrs>,
    active_gamepad: Option<GamepadId>,
    gamepad_name: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            port_name: "COM7".to_string(),
            throttle: 0.0,
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            tx: None,
            telemetry_rx: None,
            status: "Not connected".to_string(),
            start: Instant::now(),
            series: vec![
                Series::new("Throttle (0..1)", egui::Color32::from_rgb(100, 170, 255)),
                Series::new("Roll (-1..1)", egui::Color32::from_rgb(230, 80, 80)),
                Series::new("Pitch (-1..1)", egui::Color32::from_rgb(200, 120, 255)),
                Series::new("Yaw (-1..1)", egui::Color32::from_rgb(230, 200, 60)),
                Series::new("Drone state (0..3)", egui::Color32::from_rgb(80, 200, 120)),
            ],
            last: None,
            gilrs: Gilrs::new().ok(),
            active_gamepad: None,
            gamepad_name: None,
        }
    }
}

impl App {
    /// Build the command from the four current control values.
    fn command(&self) -> GroundstationCommand {
        GroundstationCommand {
            throttle: Throttle::from_normalised(self.throttle),
            roll: Roll::from_normalised(self.roll),
            pitch: Pitch::from_normalised(self.pitch),
            yaw: Yaw::from_normalised(self.yaw),
        }
    }

    /// Send the current command to the serial thread, if connected.
    fn send_command(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(self.command());
        }
    }

    /// Drain any telemetry delivered by the serial thread into the series.
    fn ingest_telemetry(&mut self) {
        let Some(rx) = &self.telemetry_rx else {
            return;
        };
        let mut samples = Vec::new();
        while let Ok(telemetry) = rx.try_recv() {
            samples.push(telemetry);
        }
        for telemetry in samples {
            let t = self.start.elapsed().as_secs_f64();
            self.series[SERIES_THROTTLE]
                .push(t, telemetry.pilot_command.throttle.as_normalised() as f64);
            self.series[SERIES_ROLL].push(t, telemetry.pilot_command.roll.as_normalised() as f64);
            self.series[SERIES_PITCH].push(t, telemetry.pilot_command.pitch.as_normalised() as f64);
            self.series[SERIES_YAW].push(t, telemetry.pilot_command.yaw.as_normalised() as f64);
            self.series[SERIES_DRONE_STATE].push(t, drone_state_code(telemetry.drone_state));
            self.last = Some(telemetry);
        }
    }

    /// Open the serial port and start the I/O thread, wiring both the throttle
    /// command channel (UI -> thread) and the telemetry channel (thread -> UI).
    fn connect(&mut self, ctx: egui::Context) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<GroundstationCommand>();
        let (telemetry_tx, telemetry_rx) = mpsc::channel::<TelemetryState>();

        let port = match serialport::new(&self.port_name, 115_200)
            .timeout(Duration::from_millis(50))
            .open()
        {
            Ok(p) => p,
            Err(e) => {
                self.status = format!("failed to open {}: {}", self.port_name, e);
                eprintln!("failed to open {}: {}", self.port_name, e);
                return;
            }
        };

        thread::spawn(move || serial_io_thread(port, cmd_rx, telemetry_tx, ctx));

        self.tx = Some(cmd_tx);
        self.telemetry_rx = Some(telemetry_rx);
        self.start = Instant::now();
        for series in &mut self.series {
            series.points.clear();
        }
        self.status = format!("Connected to {}", self.port_name);
    }

    /// Poll the gamepad and map controls: right trigger to throttle, and the
    /// Mode 2 sticks to yaw/roll/pitch. Driven each frame; requests a repaint
    /// so polling continues without other UI events.
    fn poll_gamepad(&mut self, ctx: &egui::Context) {
        let Some(gilrs) = self.gilrs.as_mut() else {
            return;
        };
        // Keep ticking at ~60 Hz so trigger movement feels live and a
        // hot-plugged pad is noticed.
        ctx.request_repaint_after(Duration::from_millis(16));

        let mut changed = false;
        while let Some(event) = gilrs.next_event() {
            self.active_gamepad = Some(event.id);
            match event.event {
                // Throttle stays on the right trigger (springs to idle), not a
                // self-centring stick (ADR 0021).
                EventType::ButtonChanged(Button::RightTrigger2, value, _) => {
                    self.throttle = trigger_to_throttle(value);
                    changed = true;
                }
                // Mode 2 sticks: left X = yaw, right X = roll, right Y = pitch.
                EventType::AxisChanged(Axis::LeftStickX, value, _) => {
                    self.yaw = stick_to_deflection(value);
                    changed = true;
                }
                EventType::AxisChanged(Axis::RightStickX, value, _) => {
                    self.roll = stick_to_deflection(value);
                    changed = true;
                }
                // gilrs reports stick-up as positive; invert so pushing the
                // stick forward commands nose-down (forward flight).
                EventType::AxisChanged(Axis::RightStickY, value, _) => {
                    self.pitch = stick_to_deflection(-value);
                    changed = true;
                }
                _ => {}
            }
            gilrs.update(&event);
        }

        self.gamepad_name = self
            .active_gamepad
            .or_else(|| gilrs.gamepads().next().map(|(id, _)| id))
            .map(|id| gilrs.gamepad(id).name().to_string());

        if changed {
            self.send_command();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ingest_telemetry();
        self.poll_gamepad(ctx);

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.heading("Drone Ground Station");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Port:");
                ui.add_enabled(
                    self.tx.is_none(),
                    egui::TextEdit::singleline(&mut self.port_name).desired_width(120.0),
                );
                let connect_label = if self.tx.is_some() {
                    "Connected"
                } else {
                    "Connect"
                };
                if ui
                    .add_enabled(self.tx.is_none(), egui::Button::new(connect_label))
                    .clicked()
                {
                    self.connect(ui.ctx().clone());
                }
                ui.label(&self.status);
            });

            ui.horizontal(|ui| {
                ui.label("Gamepad:");
                match &self.gamepad_name {
                    Some(name) => ui.label(format!(
                        "{name}  (R2 \u{2192} throttle, left stick \u{2192} yaw, right stick \u{2192} roll/pitch)"
                    )),
                    None => ui.label("none detected"),
                };
            });

            ui.add_space(4.0);
            let mut changed = ui
                .add(
                    egui::Slider::new(&mut self.throttle, 0.0..=1.0)
                        .text("Throttle")
                        .fixed_decimals(3),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut self.roll, -1.0..=1.0)
                        .text("Roll")
                        .fixed_decimals(3),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut self.pitch, -1.0..=1.0)
                        .text("Pitch")
                        .fixed_decimals(3),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut self.yaw, -1.0..=1.0)
                        .text("Yaw")
                        .fixed_decimals(3),
                )
                .changed();
            if changed {
                self.send_command();
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Show:");
                for series in &mut self.series {
                    ui.checkbox(&mut series.visible, series.name);
                }
                if ui.button("Clear").clicked() {
                    for series in &mut self.series {
                        series.points.clear();
                    }
                }
            });

            if let Some(last) = &self.last {
                ui.add_space(2.0);
                ui.label(format!(
                    "seq {}  |  throttle {:.3}  |  state {:?}  |  rpy {:+.2} {:+.2} {:+.2}",
                    last.sequence_number,
                    last.pilot_command.throttle.as_normalised(),
                    last.drone_state,
                    last.pilot_command.roll.as_normalised(),
                    last.pilot_command.pitch.as_normalised(),
                    last.pilot_command.yaw.as_normalised(),
                ));
            }
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            Plot::new("telemetry_plot")
                .legend(Legend::default())
                .x_axis_label("time (s)")
                .show(ui, |plot_ui| {
                    for series in &self.series {
                        if series.visible && !series.points.is_empty() {
                            let line = Line::new(PlotPoints::from(series.points.clone()))
                                .name(series.name)
                                .color(series.color);
                            plot_ui.line(line);
                        }
                    }
                });
        });
    }
}

// Do both directions in a single thread to avoid needing to share the port between threads.
fn serial_io_thread(
    mut port: Box<dyn serialport::SerialPort>,
    rx: mpsc::Receiver<GroundstationCommand>,
    telemetry_tx: mpsc::Sender<TelemetryState>,
    ctx: egui::Context,
) {
    let mut buf = [0u8; MAX_SEND_BUFFER_SIZE]; // serialization scratch
    let mut raw = [0u8; 256]; // chunk from each read
    let mut cobs: CobsAccumulator<64> = CobsAccumulator::new();

    loop {
        // 1. Send any pending commands (non-blocking drain).
        while let Ok(command) = rx.try_recv() {
            let framed = encode_command(command, &mut buf).unwrap();
            if let Err(e) = port.write_all(framed) {
                eprintln!("write error: {e}");
            }
        }

        // 2. Read whatever telemetry is available, then loop back to writes.
        match port.read(&mut raw) {
            Ok(0) => {}
            Ok(n) => {
                let mut window = &raw[..n];
                while !window.is_empty() {
                    window = match cobs.feed::<TelemetryState>(window) {
                        FeedResult::Consumed => break,        // buffered, need more bytes
                        FeedResult::OverFull(rest) => rest,   // frame too big -> resync
                        FeedResult::DeserError(rest) => rest, // garbage -> resync
                        FeedResult::Success { data, remaining } => {
                            if telemetry_tx.send(data).is_err() {
                                return; // UI gone
                            }
                            ctx.request_repaint();
                            remaining
                        }
                    };
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                eprintln!("read error: {e}");
                return;
            }
        }
    }
}
