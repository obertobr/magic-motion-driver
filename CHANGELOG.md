# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-11-02

### Added
- **111 Hz RF Serial Stream Decoder**: Full state-machine decoder (`FrameDecoder`) for the LG Magic Motion AN-MR200 / AN-MR100 USB receiver (`Silicon Labs CP2102`, `VID 0x10C4`, `PID 0xEA60`, `115200 8-N-1`) with automatic stream resynchronization and checksum validation.
- **Roll-Compensated Air Mouse Engine**: Real-time 2D cursor projection using accelerometer gravity estimation (`AccelCalibration::LG_AN_MR200`) to keep screen vertical/horizontal movement aligned with gravity regardless of wrist tilt, plus dynamic trust weighting to suppress linear hand acceleration and click tremors.
- **Interactive GUI Control Panel (`eframe`/`egui`)**:
  - **Buttons Tab**: Live visual highlight of pressed remote buttons on the AN-MR200 diagram, 22+ built-in actions (mouse clicks, scroll, media transport, system volume, browser navigation, window management), and a custom multi-key keyboard shortcut recorder.
  - **Cursor & Sensitivity Tab**: Real-time sliders for cursor speed, exponential smoothing filter, deadzone, axis inversion, roll compensation toggle, and 1.5-second gyroscope rest-center calibration.
  - **System Tab**: Windows startup auto-run (`HKCU\...\Run`), target monitor clamping (free multi-monitor or locked to any specific display), and language selector.
  - **Raw Data & 3D Studio Tab**: Live connection status & frame rate (`Hz`), bipolar gyroscope and accelerometer (`g`) bars, 5-channel real-time oscilloscope (`Gyro X/Y`, `Accel X/Y/Z`), singularity-free 3D wireframe attitude visualization powered by a **Direction Cosine Matrix (DCM)** complementary filter (`SO(3)` orthonormalization), and lossless 111 Hz CSV sensor recorder with hardware frame timestamps.
- **Windows System Tray Integration**: Live green/red connection icon, tooltip with active COM port, quick cursor toggle, panel restore, and instant termination.
- **Internationalization (i18n)**: Full English (`en-US`) and Portuguese (`pt-BR`) localization with automatic first-run Windows UI language detection (`GetUserDefaultUILanguage`).

