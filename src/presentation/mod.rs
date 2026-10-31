//! Presentation layer for the desktop UI and system tray.

pub mod hotspots;
pub mod mesh3d;
pub mod tray;
pub mod view;

use eframe::egui::{self, ViewportBuilder};

use crate::domain::config::Config;
use crate::infra::driver::DriverHandle;
use view::DriverApp;

pub fn run_gui_app(
    mut driver: DriverHandle,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("LG Magic Motion Painel")
            .with_inner_size([720.0, 660.0])
            .with_min_inner_size([640.0, 580.0])
            .with_active(true),
        ..Default::default()
    };

    let cmd_tx = driver.cmd_sender.clone();
    let telem_rx = driver.take_telemetry_receiver();

    let res = eframe::run_native(
        "LG Magic Motion Painel",
        native_options,
        Box::new(move |cc| {
            // Dark visuals
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            let app = DriverApp::new(cc, config, cmd_tx, telem_rx);

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
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.update_background(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.render(ui);
    }
}
