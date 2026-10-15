//! High-rate driver worker thread.
//!
//! Handles the 111 Hz serial stream, motion processing, button transitions,
//! native SendInput dispatching, and telemetry to the UI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::domain::action::Action;
use crate::domain::button::{decode_button_code, ButtonSet};
use crate::domain::config::Config;
use crate::domain::motion::{MotionProcessor, MotionSettings};
use crate::domain::protocol::{Frame, FrameDecoder};
use crate::infra::input::{execute_action_press, execute_action_release, send_mouse_move_clamped};
use crate::infra::serial::{find_receiver_port, open_port};

/// Real-time telemetry sent from driver thread to UI.
#[derive(Clone, Debug)]
pub struct DriverTelemetry {
    pub connected: bool,
    pub port_name: String,
    pub rate_hz: f32,
    pub gyro_x: u16,
    pub gyro_y: u16,
    pub gyro_dx: i32,
    pub gyro_dy: i32,
    pub accel_x: u16,
    pub accel_y: u16,
    pub accel_z: u16,
    pub active_buttons: ButtonSet,
    pub cursor_enabled: bool,
    pub center_x: f32,
    pub center_y: f32,
}

impl Default for DriverTelemetry {
    fn default() -> Self {
        Self {
            connected: false,
            port_name: String::new(),
            rate_hz: 0.0,
            gyro_x: 537,
            gyro_y: 537,
            gyro_dx: 0,
            gyro_dy: 0,
            accel_x: 2048,
            accel_y: 2048,
            accel_z: 2048,
            active_buttons: ButtonSet::EMPTY,
            cursor_enabled: true,
            center_x: 537.0,
            center_y: 537.0,
        }
    }
}

/// Commands sent from UI or System Tray to the Driver thread.
#[derive(Debug)]
pub enum DriverCommand {
    UpdateConfig(Config),
    ToggleCursor,
    Recenter,
    Stop,
}

pub struct DriverHandle {
    pub cmd_sender: Sender<DriverCommand>,
    pub telemetry_receiver: Option<Receiver<DriverTelemetry>>,
    is_running: Arc<AtomicBool>,
    worker_thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for DriverHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriverHandle")
            .field("is_running", &self.is_running)
            .finish()
    }
}

impl DriverHandle {
    pub fn start(config: Config) -> Self {
        let (cmd_tx, cmd_rx) = channel::<DriverCommand>();
        let (telem_tx, telem_rx) = channel::<DriverTelemetry>();
        let is_running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&is_running);

        let worker_thread = thread::Builder::new()
            .name("magic-motion-driver".to_string())
            .spawn(move || {
                run_driver_loop(config, cmd_rx, telem_tx, running_clone);
            })
            .expect("spawn driver thread");

        Self {
            cmd_sender: cmd_tx,
            telemetry_receiver: Some(telem_rx),
            is_running,
            worker_thread: Some(worker_thread),
        }
    }

    pub fn take_telemetry_receiver(&mut self) -> Option<Receiver<DriverTelemetry>> {
        self.telemetry_receiver.take()
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        let _ = self.cmd_sender.send(DriverCommand::Stop);
        if let Some(handle) = self.worker_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for DriverHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn resolve_monitor_rect(target: Option<usize>) -> Option<[i32; 4]> {
    let target = target?;
    let monitors = crate::infra::input::list_monitors();
    monitors.into_iter().find(|m| m.id == target).map(|m| m.rect)
}

fn run_driver_loop(
    mut config: Config,
    cmd_rx: Receiver<DriverCommand>,
    telem_tx: Sender<DriverTelemetry>,
    is_running: Arc<AtomicBool>,
) {
    let mut processor = MotionProcessor::new(MotionSettings::from(&config.motion));
    let mut decoder = FrameDecoder::new();
    let mut active_buttons = ButtonSet::EMPTY;
    let mut clamp_rect = resolve_monitor_rect(config.target_monitor);

    let mut port_name: Option<String> = None;
    let mut port_stream: Option<Box<dyn serialport::SerialPort>> = None;
    let mut read_buf = [0u8; 256];

    let mut fps_count: u32 = 0;
    let mut fps_timer = Instant::now();
    let mut current_fps = 0.0f32;
    let mut last_telem_send = Instant::now();
    let mut last_reconnect_attempt = Instant::now() - Duration::from_secs(5);
    let mut last_valid_frame: Option<Frame> = None;
    let mut last_dx: i32 = 0;
    let mut last_dy: i32 = 0;

    while is_running.load(Ordering::Relaxed) {
        // 1. Process commands from UI / Tray
        loop {
            match cmd_rx.try_recv() {
                Ok(DriverCommand::UpdateConfig(new_cfg)) => {
                    let port_changed = new_cfg.port_override != config.port_override;
                    clamp_rect = resolve_monitor_rect(new_cfg.target_monitor);
                    processor.set_settings(MotionSettings::from(&new_cfg.motion));
                    config = new_cfg;
                    if port_changed {
                        port_stream = None;
                        port_name = None;
                        decoder.reset();
                        last_reconnect_attempt = Instant::now() - Duration::from_secs(5);
                    }
                }
                Ok(DriverCommand::ToggleCursor) => {
                    processor.toggle_cursor();
                }
                Ok(DriverCommand::Recenter) => {
                    // Recenter to current resting positions
                    processor.set_center(537.0, 537.0);
                }
                Ok(DriverCommand::Stop) => {
                    is_running.store(false, Ordering::SeqCst);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    is_running.store(false, Ordering::SeqCst);
                    break;
                }
            }
        }

        if !is_running.load(Ordering::Relaxed) {
            break;
        }

        // 2. Manage serial connection with auto-reconnect
        if port_stream.is_none() {
            if last_reconnect_attempt.elapsed() >= Duration::from_millis(800) {
                last_reconnect_attempt = Instant::now();
                let found = find_receiver_port(config.port_override.as_deref());
                if let Some(name) = found {
                    match open_port(&name) {
                        Ok(p) => {
                            port_name = Some(name);
                            port_stream = Some(p);
                            decoder.reset();
                        }
                        Err(_) => {
                            port_stream = None;
                        }
                    }
                }
            }

            if port_stream.is_none() {
                last_valid_frame = None;
                last_dx = 0;
                last_dy = 0;
                // Send disconnected telemetry at ~5 Hz
                if last_telem_send.elapsed() >= Duration::from_millis(200) {
                    let _ = telem_tx.send(DriverTelemetry {
                        connected: false,
                        port_name: String::new(),
                        rate_hz: 0.0,
                        cursor_enabled: processor.is_cursor_enabled(),
                        center_x: processor.center_x,
                        center_y: processor.center_y,
                        ..Default::default()
                    });
                    last_telem_send = Instant::now();
                }
                thread::sleep(Duration::from_millis(20));
                continue;
            }
        }

        // 3. Read incoming bytes from serial
        let mut got_bytes = false;
        if let Some(ref mut port) = port_stream {
            match port.read(&mut read_buf) {
                Ok(n) if n > 0 => {
                    decoder.push(&read_buf[..n]);
                    got_bytes = true;
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => {
                    // Port disconnected or error
                    port_stream = None;
                    port_name = None;
                    continue;
                }
            }
        }

        // 4. Process all complete frames received
        while let Some(frame) = decoder.next_frame() {
            fps_count += 1;
            last_valid_frame = Some(frame);

            // A. Motion processing (sub-pixel mouse movement)
            let (dx, dy) = processor.process(frame.gyro_x, frame.gyro_y);
            last_dx = dx;
            last_dy = dy;
            if dx != 0 || dy != 0 {
                send_mouse_move_clamped(dx, dy, clamp_rect);
            }

            // B. Button states & transitions
            let new_buttons = decode_button_code(frame.button_code);
            let pressed = new_buttons.difference(active_buttons);
            let released = active_buttons.difference(new_buttons);

            for btn in pressed.iter() {
                let action = config.get_action(btn);
                match action {
                    Action::ToggleCursor => {
                        processor.toggle_cursor();
                    }
                    Action::Recenter => {
                        processor.set_center(frame.gyro_x as f32, frame.gyro_y as f32);
                    }
                    act => {
                        execute_action_press(act);
                    }
                }
            }

            for btn in released.iter() {
                let action = config.get_action(btn);
                execute_action_release(action);
            }

            active_buttons = new_buttons;
        }

        // 5. Calculate FPS (packet rate Hz)
        let elapsed = fps_timer.elapsed();
        if elapsed >= Duration::from_millis(500) {
            current_fps = (fps_count as f32) / elapsed.as_secs_f32();
            fps_count = 0;
            fps_timer = Instant::now();
        }

        // 6. Send telemetry to UI at ~60 Hz (every 16 ms) or immediately if buttons change
        if last_telem_send.elapsed() >= Duration::from_millis(16) || !active_buttons.is_empty() {
            last_telem_send = Instant::now();
            let (gx, gy, ax, ay, az) = if let Some(ref f) = last_valid_frame {
                (f.gyro_x, f.gyro_y, f.accel[0], f.accel[1], f.accel[2])
            } else {
                (537, 537, 2048, 2048, 2048)
            };

            let _ = telem_tx.send(DriverTelemetry {
                connected: port_stream.is_some(),
                port_name: port_name.clone().unwrap_or_default(),
                rate_hz: current_fps,
                gyro_x: gx,
                gyro_y: gy,
                gyro_dx: last_dx,
                gyro_dy: last_dy,
                accel_x: ax,
                accel_y: ay,
                accel_z: az,
                active_buttons,
                cursor_enabled: processor.is_cursor_enabled(),
                center_x: processor.center_x,
                center_y: processor.center_y,
            });
        }

        if !got_bytes {
            // Sleep briefly to yield CPU when waiting for serial buffer
            thread::sleep(Duration::from_millis(2));
        }
    }

    // Release any pressed buttons on shutdown
    for btn in active_buttons.iter() {
        let action = config.get_action(btn);
        execute_action_release(action);
    }
}
