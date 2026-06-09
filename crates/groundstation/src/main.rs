//! Ground station: single-slider throttle sender.
//!
//! Sends the slider value (0.0..=1.0) as a decimal string + LF over a serial
//! port at 115 200 8N1 whenever the slider changes.

use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use eframe::egui;

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
                    let port = self.port_name.clone();
                    thread::spawn(move || serial_thread(port, rx));
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
            if response.changed() {
                if let Some(tx) = &self.tx {
                    let _ = tx.send(self.throttle);
                }
            }
        });
    }
}

fn serial_thread(port_name: String, rx: mpsc::Receiver<f32>) {
    let mut port = match serialport::new(&port_name, 115_200)
        .timeout(Duration::from_millis(100))
        .open()
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("failed to open {port_name}: {e}");
            return;
        }
    };

    while let Ok(value) = rx.recv() {
        let line = format!("{value:.4}\n");
        if let Err(e) = port.write_all(line.as_bytes()) {
            eprintln!("write error: {e}");
            return;
        }
    }
}
