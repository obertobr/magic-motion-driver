//! Pure domain layer: models, protocols, actions, motion engine, and configuration.
//!
//! This layer contains zero OS dependencies, GUI calls, or I/O.

pub mod action;
pub mod button;
pub mod config;
pub mod gravity;
pub mod motion;
pub mod protocol;

pub use action::{Action, MediaKey, VirtualKey};
pub use button::{Button, ButtonSet};
pub use config::Config;
pub use gravity::{AccelCalibration, RollEstimator};
pub use motion::{MotionProcessor, MotionSettings};
pub use protocol::{Frame, FrameDecoder, BAUD_RATE, FRAME_LEN, RECEIVER_USB_PID, RECEIVER_USB_VID};
