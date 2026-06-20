//! Ground station: single-slider throttle sender + telemetry receiver.
//!
//! Sends the slider value (0.0..=1.0) as a postcard + COBS framed
//! `GroundstationCommand` over a serial port at 115 200 8N1, and receives
//! postcard + COBS framed `TelemetryState` on the same port. A single I/O
//! thread handles both directions (see `serial_io_thread`).

use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use postcard::accumulator::{CobsAccumulator, FeedResult};

use firmware_types::{TelemetryState, Throttle};

use eframe::egui;

const MAX_SEND_BUFFER_SIZE: usize = 32;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Drone Ground Station",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[derive(Debug)]
struct App {
    port_name: String,
    throttle: f32,
    tx: Option<mpsc::Sender<f32>>,
    status: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            port_name: "COM7".to_string(),
            throttle: 0.0,
            tx: None,
            status: "Not connected".to_string(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Drone Ground Station");
            ui.add_space(8.0);

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
                    let (tx, rx) = mpsc::channel::<f32>();

                    let port = match serialport::new(&self.port_name, 115_200)
                        .timeout(Duration::from_millis(50))
                        .open()
                    {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("failed to open {}: {}", self.port_name, e);
                            return;
                        }
                    };

                    thread::spawn(move || serial_io_thread(port, rx));

                    self.tx = Some(tx);
                    self.status = format!("Sending to {}", self.port_name);
                }
            });

            ui.label(&self.status);
            ui.add_space(16.0);

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
        });
    }
}

// Do both directions in a single thread to avoid needing to share the port between threads.
fn serial_io_thread(mut port: Box<dyn serialport::SerialPort>, rx: mpsc::Receiver<f32>) {
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
                            println!("telemetry: {data:?}");
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
