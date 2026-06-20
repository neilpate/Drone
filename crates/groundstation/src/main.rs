//! Ground station: single-slider throttle sender + telemetry plotter.
//!
//! Sends the slider value (0.0..=1.0) as a postcard + COBS framed
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

use firmware_types::{DroneState, TelemetryState, Throttle};

use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use gilrs::{Button, EventType, GamepadId, Gilrs};

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
const SERIES_TEMPERATURE: usize = 1;
const SERIES_DRONE_STATE: usize = 2;

struct App {
    port_name: String,
    throttle: f32,
    tx: Option<mpsc::Sender<f32>>,
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
            tx: None,
            telemetry_rx: None,
            status: "Not connected".to_string(),
            start: Instant::now(),
            series: vec![
                Series::new("Throttle (0..1)", egui::Color32::from_rgb(100, 170, 255)),
                Series::new(
                    "Temperature (\u{b0}C)",
                    egui::Color32::from_rgb(255, 140, 0),
                ),
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
            self.series[SERIES_TEMPERATURE].push(t, telemetry.temperature.as_celsius() as f64);
            self.series[SERIES_DRONE_STATE].push(t, drone_state_code(telemetry.drone_state));
            self.last = Some(telemetry);
        }
    }

    /// Open the serial port and start the I/O thread, wiring both the throttle
    /// command channel (UI -> thread) and the telemetry channel (thread -> UI).
    fn connect(&mut self, ctx: egui::Context) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<f32>();
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

    /// Poll the gamepad and map the right trigger (0.0..=1.0) to throttle,
    /// forwarding it to the serial thread. Driven each frame; requests a
    /// repaint so polling continues without other UI events.
    fn poll_gamepad(&mut self, ctx: &egui::Context) {
        let Some(gilrs) = self.gilrs.as_mut() else {
            return;
        };
        // Keep ticking at ~60 Hz so trigger movement feels live and a
        // hot-plugged pad is noticed.
        ctx.request_repaint_after(Duration::from_millis(16));

        let mut new_throttle = None;
        while let Some(event) = gilrs.next_event() {
            self.active_gamepad = Some(event.id);
            if let EventType::ButtonChanged(Button::RightTrigger2, value, _) = event.event {
                new_throttle = Some(value.clamp(0.0, 1.0));
            }
            gilrs.update(&event);
        }

        self.gamepad_name = self
            .active_gamepad
            .or_else(|| gilrs.gamepads().next().map(|(id, _)| id))
            .map(|id| gilrs.gamepad(id).name().to_string());

        if let Some(throttle) = new_throttle {
            self.throttle = throttle;
            if let Some(tx) = &self.tx {
                let _ = tx.send(self.throttle);
            }
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
                    Some(name) => ui.label(format!("{name}  (right trigger \u{2192} throttle)")),
                    None => ui.label("none detected"),
                };
            });

            ui.add_space(4.0);
            let response = ui.add(
                egui::Slider::new(&mut self.throttle, 0.0..=1.0)
                    .text("Throttle")
                    .fixed_decimals(3),
            );
            if response.changed()
                && let Some(tx) = &self.tx
            {
                let _ = tx.send(self.throttle);
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
                    "seq {}  |  throttle {:.3}  |  temp {:.1} \u{b0}C  |  state {:?}",
                    last.sequence_number,
                    last.pilot_command.throttle.as_normalised(),
                    last.temperature.as_celsius(),
                    last.drone_state,
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

fn drone_state_code(state: DroneState) -> f64 {
    match state {
        DroneState::Initialising => 0.0,
        DroneState::Armed => 1.0,
        DroneState::Degraded => 2.0,
        DroneState::Fault => 3.0,
    }
}

// Do both directions in a single thread to avoid needing to share the port between threads.
fn serial_io_thread(
    mut port: Box<dyn serialport::SerialPort>,
    rx: mpsc::Receiver<f32>,
    telemetry_tx: mpsc::Sender<TelemetryState>,
    ctx: egui::Context,
) {
    let mut buf = [0u8; MAX_SEND_BUFFER_SIZE]; // serialization scratch
    let mut raw = [0u8; 256]; // chunk from each read
    let mut cobs: CobsAccumulator<64> = CobsAccumulator::new();

    loop {
        // 1. Send any pending throttle commands (non-blocking drain).
        while let Ok(value) = rx.try_recv() {
            let throttle = Throttle::from_normalised(value);

            let groundstation_command = firmware_types::GroundstationCommand { throttle };

            let framed = postcard::to_slice_cobs(&groundstation_command, &mut buf).unwrap();
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
