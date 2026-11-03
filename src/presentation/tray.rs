//! System tray icon and context menu with live status feedback.

use std::sync::mpsc::{Receiver, Sender};
use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::domain::i18n::Language;
use crate::infra::driver::DriverCommand;
use crate::presentation::i18n::get_messages;

pub struct SystemTray {
    tray_icon: TrayIcon,
    status_item: MenuItem,
    item_open: MenuItem,
    item_toggle: MenuItem,
    item_quit: MenuItem,
    icon_connected: Icon,
    icon_disconnected: Icon,
    last_connected: Option<bool>,
    last_port: String,
    last_lang: Option<Language>,
    menu_open_id: tray_icon::menu::MenuId,
    menu_toggle_id: tray_icon::menu::MenuId,
    menu_quit_id: tray_icon::menu::MenuId,
    menu_rx: Receiver<MenuEvent>,
    tray_rx: Receiver<TrayIconEvent>,
}

impl std::fmt::Debug for SystemTray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemTray")
            .field("last_connected", &self.last_connected)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    OpenPanel,
    Quit,
}

impl SystemTray {
    pub fn new(ctx: egui::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let menu = Menu::new();

        let status_item = MenuItem::new("Status: Desconectado", false, None);
        let item_open = MenuItem::new("Abrir Painel", true, None);
        let item_toggle = MenuItem::new("Ativar / Pausar Mira", true, None);
        let item_quit = MenuItem::new("Sair", true, None);

        let menu_open_id = item_open.id().clone();
        let menu_toggle_id = item_toggle.id().clone();
        let menu_quit_id = item_quit.id().clone();

        menu.append(&status_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&item_open)?;
        menu.append(&item_toggle)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&item_quit)?;

        let (menu_tx, menu_rx) = std::sync::mpsc::channel();
        let ctx_menu = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = menu_tx.send(event);
            ctx_menu.request_repaint();
        }));

        let (tray_tx, tray_rx) = std::sync::mpsc::channel();
        let ctx_tray = ctx;
        TrayIconEvent::set_event_handler(Some(move |event| {
            let _ = tray_tx.send(event);
            ctx_tray.request_repaint();
        }));

        let icon_connected = create_colored_tray_icon(true)?;
        let icon_disconnected = create_colored_tray_icon(false)?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_menu_on_right_click(true)
            .with_tooltip("LG Magic Motion: Desconectado")
            .with_icon(icon_disconnected.clone())
            .build()?;

        Ok(Self {
            tray_icon,
            status_item,
            item_open,
            item_toggle,
            item_quit,
            icon_connected,
            icon_disconnected,
            last_connected: Some(false),
            last_port: String::new(),
            last_lang: None,
            menu_open_id,
            menu_toggle_id,
            menu_quit_id,
            menu_rx,
            tray_rx,
        })
    }

    /// Dynamically updates the tray icon, tooltip, and menu status text according to connection state and language.
    pub fn update_status(&mut self, connected: bool, port: &str, lang: Language) {
        if self.last_connected != Some(connected) || self.last_port != port || self.last_lang != Some(lang) {
            self.last_connected = Some(connected);
            self.last_port = port.to_string();
            self.last_lang = Some(lang);

            let tr = get_messages(lang);

            self.item_open.set_text(tr.tray_open_panel);
            self.item_toggle.set_text(tr.tray_toggle_cursor);
            self.item_quit.set_text(tr.tray_quit);

            let (tooltip, menu_txt) = if connected {
                let p = if port.is_empty() { tr.status_connected } else { port };
                (
                    format!("{}: {} ({p})", tr.app_title, tr.status_connected),
                    format!("{}: {} ({p})", tr.conn_status, tr.status_connected),
                )
            } else {
                (
                    format!("{}: {}", tr.app_title, tr.status_disconnected),
                    format!("{}: {}", tr.conn_status, tr.status_disconnected),
                )
            };

            let target_icon = if connected {
                &self.icon_connected
            } else {
                &self.icon_disconnected
            };

            let _ = self.tray_icon.set_icon(Some(target_icon.clone()));
            let _ = self.tray_icon.set_tooltip(Some(tooltip));
            self.status_item.set_text(menu_txt);
        }
    }

    /// Checks if a tray menu or icon interaction event occurred.
    pub fn process_events(&self, driver_cmd_tx: &Sender<DriverCommand>) -> Option<TrayAction> {
        let mut action = None;

        // Drain all menu events
        while let Ok(event) = self.menu_rx.try_recv() {
            if event.id == self.menu_open_id {
                action = Some(TrayAction::OpenPanel);
            } else if event.id == self.menu_toggle_id {
                let _ = driver_cmd_tx.send(DriverCommand::ToggleCursor);
            } else if event.id == self.menu_quit_id {
                action = Some(TrayAction::Quit);
            }
        }

        // Drain all icon click / double click events
        while let Ok(event) = self.tray_rx.try_recv() {
            match event {
                TrayIconEvent::Click { button: tray_icon::MouseButton::Left, button_state: tray_icon::MouseButtonState::Up, .. }
                | TrayIconEvent::DoubleClick { button: tray_icon::MouseButton::Left, .. } => {
                    action = Some(TrayAction::OpenPanel);
                }
                _ => {}
            }
        }

        action
    }
}

/// Generates a crisp 32x32 RGBA icon (Green when connected, Red when disconnected) in memory.
fn create_colored_tray_icon(connected: bool) -> Result<Icon, Box<dyn std::error::Error>> {
    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);

    let cx = 16.0f32;
    let cy = 16.0f32;

    // Palette:
    // Connected: Emerald Green (#28DC64)
    // Disconnected: Coral Red (#E63C3C)
    let (fg_r, fg_g, fg_b) = if connected {
        (0x28, 0xDC, 0x64)
    } else {
        (0xE6, 0x3C, 0x3C)
    };

    let (bg_r, bg_g, bg_b) = if connected {
        (0x10, 0x30, 0x1C)
    } else {
        (0x30, 0x14, 0x14)
    };

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if (dist - 12.0).abs() <= 2.0 || dist <= 3.5 || (dx.abs() <= 1.0 && dist <= 14.0) || (dy.abs() <= 1.0 && dist <= 14.0) {
                // Foreground crosshair / outer ring / central dot
                rgba.extend_from_slice(&[fg_r, fg_g, fg_b, 0xFF]);
            } else if dist <= 14.0 {
                // Dark tinted background circular disc
                rgba.extend_from_slice(&[bg_r, bg_g, bg_b, 0xCC]);
            } else {
                // Transparent
                rgba.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
            }
        }
    }

    Ok(Icon::from_rgba(rgba, SIZE, SIZE)?)
}
