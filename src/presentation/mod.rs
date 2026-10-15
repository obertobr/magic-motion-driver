//! Presentation layer for the desktop UI and system tray.

pub mod hotspots;
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

            // Wake up egui event loop immediately when tray icon or menu items are interacted with
            let ctx_icon = cc.egui_ctx.clone();
            tray_icon::TrayIconEvent::set_event_handler(Some(move |_| {
                ctx_icon.request_repaint();
            }));

            let ctx_menu = cc.egui_ctx.clone();
            tray_icon::menu::MenuEvent::set_event_handler(Some(move |_| {
                ctx_menu.request_repaint();
            }));

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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.app.should_exit {
            std::process::exit(0);
        }
        self.app.poll_telemetry();
        self.app.render(ui);
        if self.app.should_exit {
            std::process::exit(0);
        }
    }
}
