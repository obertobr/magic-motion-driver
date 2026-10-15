//! System tray icon and context menu.

use std::sync::mpsc::Sender;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::infra::driver::DriverCommand;

pub struct SystemTray {
    _tray_icon: TrayIcon,
    menu_open_id: tray_icon::menu::MenuId,
    menu_toggle_id: tray_icon::menu::MenuId,
    menu_recenter_id: tray_icon::menu::MenuId,
    menu_quit_id: tray_icon::menu::MenuId,
}

impl std::fmt::Debug for SystemTray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemTray").finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    OpenPanel,
    Quit,
}

impl SystemTray {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let menu = Menu::new();

        let item_open = MenuItem::new("Abrir Painel", true, None);
        let item_toggle = MenuItem::new("Ativar / Pausar Mira", true, None);
        let item_recenter = MenuItem::new("Recalibrar Centro", true, None);
        let item_quit = MenuItem::new("Sair", true, None);

        let menu_open_id = item_open.id().clone();
        let menu_toggle_id = item_toggle.id().clone();
        let menu_recenter_id = item_recenter.id().clone();
        let menu_quit_id = item_quit.id().clone();

        menu.append(&item_open)?;
        menu.append(&item_toggle)?;
        menu.append(&item_recenter)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&item_quit)?;

        let icon = create_tray_icon()?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("LG Magic Motion Driver")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            _tray_icon: tray_icon,
            menu_open_id,
            menu_toggle_id,
            menu_recenter_id,
            menu_quit_id,
        })
    }

    /// Checks if a tray menu event occurred.
    pub fn process_events(&self, driver_cmd_tx: &Sender<DriverCommand>) -> Option<TrayAction> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.menu_open_id {
                return Some(TrayAction::OpenPanel);
            } else if event.id == self.menu_toggle_id {
                let _ = driver_cmd_tx.send(DriverCommand::ToggleCursor);
            } else if event.id == self.menu_recenter_id {
                let _ = driver_cmd_tx.send(DriverCommand::Recenter);
            } else if event.id == self.menu_quit_id {
                return Some(TrayAction::Quit);
            }
        }

        if let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
            if matches!(event, tray_icon::TrayIconEvent::DoubleClick { .. }) {
                return Some(TrayAction::OpenPanel);
            }
        }

        None
    }
}

/// Generates a crisp 32x32 RGBA icon (cyan target circle) in memory.
fn create_tray_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);

    let cx = 16.0f32;
    let cy = 16.0f32;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if (dist - 12.0).abs() <= 2.0 || dist <= 3.5 || (dx.abs() <= 1.0 && dist <= 14.0) || (dy.abs() <= 1.0 && dist <= 14.0) {
                // Bright cyan
                rgba.extend_from_slice(&[0x00, 0xDC, 0xFF, 0xFF]);
            } else if dist <= 14.0 {
                // Dark translucent background
                rgba.extend_from_slice(&[0x18, 0x18, 0x24, 0xAA]);
            } else {
                // Transparent
                rgba.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
            }
        }
    }

    Ok(Icon::from_rgba(rgba, SIZE, SIZE)?)
}
