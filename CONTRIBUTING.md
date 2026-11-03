# Contributing to LG Magic Motion Driver

Thank you for your interest in contributing to **LG Magic Motion Driver**!

## Architecture Overview

This project follows **Hexagonal / Clean Architecture** with strict layer separation:

- **`src/domain/`** — Pure domain logic, protocol frame decoding (`protocol.rs`), button bitmask/navigation decoding (`button.rs`), roll-compensated air-mouse kinematics (`motion.rs`, `gravity.rs`), 3D Direction Cosine Matrix attitude estimation (`attitude.rs`), configuration schema (`config.rs`), and language domain models (`i18n.rs`). **No I/O, Win32 APIs, or GUI dependencies are allowed in `domain`.**
- **`src/infra/`** — Hardware and OS adapters: CP210x USB serial discovery and streaming (`serial.rs`), high-frequency 111 Hz worker thread (`driver.rs`), native Win32 `SendInput` and multi-monitor clamping (`input.rs`), and Windows Registry autostart (`autostart.rs`).
- **`src/presentation/`** — `eframe`/`egui` desktop control panel (`view.rs`), 3D wireframe renderer (`mesh3d.rs`), interactive remote hotspots (`hotspots.rs`), system tray controller (`tray.rs`), and static translation catalogs (`i18n.rs`).

## Development Setup

### Prerequisites
- **Windows 10 or Windows 11**
- **Rust 1.85+** (2024 Edition)
- [Silicon Labs CP210x USB to UART Bridge VCP Drivers](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers) (for hardware testing)

### Running Tests

All domain, protocol, motion, and presentation logic is covered by unit and integration tests (including real captured RF frames in `tests/fixtures/captured_frames.txt`):

```powershell
cargo test
```

### Formatting & Linting

Please ensure your code is formatted with `rustfmt` before opening a Pull Request:

```powershell
cargo fmt --check
cargo clippy
```

## Submitting Changes

1. Fork the repository and create a feature branch (`git checkout -b feat/my-feature`).
2. Add unit tests for any new domain logic or protocol behavior.
3. Verify that `cargo test` passes cleanly.
4. Open a Pull Request with a clear description of the change.

