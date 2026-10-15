//! Infrastructure layer: Windows API, serial communication, and driver background thread.

pub mod autostart;
pub mod driver;
pub mod input;
pub mod serial;

pub use driver::{DriverCommand, DriverHandle, DriverTelemetry};
