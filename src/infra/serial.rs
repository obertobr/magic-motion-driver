//! Serial port discovery and communication for the CP2102 receiver.

use std::time::Duration;
use serialport::{SerialPort, SerialPortType};

use crate::domain::protocol::{BAUD_RATE, RECEIVER_USB_PID, RECEIVER_USB_VID};

/// Discovers the COM port of the Silicon Labs CP2102 bridge.
pub fn find_receiver_port(override_port: Option<&str>) -> Option<String> {
    if let Some(p) = override_port {
        if !p.trim().is_empty() {
            return Some(p.trim().to_string());
        }
    }

    let ports = serialport::available_ports().ok()?;

    // 1. Match by VID & PID (0x10C4:0xEA60)
    for p in &ports {
        if let SerialPortType::UsbPort(ref usb) = p.port_type {
            if usb.vid == RECEIVER_USB_VID && usb.pid == RECEIVER_USB_PID {
                return Some(p.port_name.clone());
            }
        }
    }

    // 2. Fallback: match by name or manufacturer
    for p in &ports {
        if let SerialPortType::UsbPort(ref usb) = p.port_type {
            let mfg = usb.manufacturer.as_deref().unwrap_or("").to_lowercase();
            let prod = usb.product.as_deref().unwrap_or("").to_lowercase();
            if mfg.contains("silicon labs") || prod.contains("cp210") {
                return Some(p.port_name.clone());
            }
        }
    }

    // 3. Fallback: on Windows, if only one COM port is available besides COM1
    #[cfg(windows)]
    {
        let coms: Vec<_> = ports
            .iter()
            .filter(|p| !p.port_name.eq_ignore_ascii_case("COM1"))
            .collect();
        if coms.len() == 1 {
            return Some(coms[0].port_name.clone());
        }
    }

    None
}

/// Opens the serial port configured for the LG Magic Motion receiver.
pub fn open_port(port_name: &str) -> Result<Box<dyn SerialPort>, serialport::Error> {
    serialport::new(port_name, BAUD_RATE)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(15))
        .open()
}
