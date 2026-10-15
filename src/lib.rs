//! LG Magic Motion user-mode driver for Windows.
//!
//! Layering:
//! - [`domain`]: pure protocol, buttons, actions, motion and config models (zero I/O, 100% testable).
//! - [`infra`]: Windows native `SendInput`, serial port transport, system tray.
//! - [`presentation`]: eframe/egui GUI control panel.

pub mod domain;
pub mod infra;
pub mod presentation;
