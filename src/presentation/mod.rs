//! Presentation layer for the desktop UI and system tray.

pub mod hotspots;
pub mod tray;
pub mod view;

use eframe::egui::{self, ViewportBuilder};

use crate::domain::config::Config;
use crate::infra::driver::{DriverHandle, DriverTelemetry};
use view::DriverApp;

pub fn run_gui_app(
    mut driver: DriverHandle,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("LG Magic Motion Driver")
            .with_inner_size([720.0, 660.0])
            .with_min_inner_size([640.0, 580.0])
            .with_active(true),
        ..Default::default()
    };

    let cmd_tx = driver.cmd_sender.clone();
    let telem_rx = driver.take_telemetry_receiver();

    let res = eframe::run_native(
        "LG Magic Motion Driver",
        native_options,
        Box::new(move |cc| {
            // Dark visuals
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            let app = DriverApp::new(cc, config, cmd_tx);

            if let Some(rx) = telem_rx {
                let ctx = cc.egui_ctx.clone();
                std::thread::Builder::new()
                    .name("telem-forwarder".to_string())
                    .spawn(move || {
                        while let Ok(telem) = rx.recv() {
                            ctx.request_repaint();
                            ctx.data_mut(|d| {
                                d.insert_temp(egui::Id::new("latest_telemetry"), telem);
                            });
                        }
                    })
                    .expect("spawn telem forwarder");
            }

            Ok(Box::new(AppWrapper { app }))
        }),
    );

    driver.stop();
    res.map_err(|e| e.to_string().into())
}

struct AppWrapper {
    app: DriverApp,
}

impl eframe::App for AppWrapper {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let latest_telem: Option<DriverTelemetry> = ui.ctx().data_mut(|d| {
            d.get_temp(egui::Id::new("latest_telemetry"))
        });

        if let Some(telem) = latest_telem {
            self.app.update_telemetry(telem);
        }

        self.app.render(ui);
    }
}
