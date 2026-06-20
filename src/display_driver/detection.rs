use anyhow::{bail, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DisplayPort {
    pub port: String,
    pub name: String,
    pub serial_number: Option<String>,
    pub likely_turzx: bool,
}

pub fn list() -> Result<Vec<DisplayPort>> {
    let ports = serialport::available_ports()?;
    let mut ports = ports
        .into_iter()
        .map(|port| {
            let (name, serial_number, likely_turzx) = match port.port_type {
                serialport::SerialPortType::UsbPort(info) => {
                    let likely = info.serial_number.as_deref() == Some("USB35INCHIPSV2")
                        || (info.vid == 0x1a86 && info.pid == 0x5722)
                        || info.product.as_deref().is_some_and(|name| {
                            let name = name.to_ascii_lowercase();
                            name.contains("turing")
                                || name.contains("turzx")
                                || name.contains("usb monitor")
                                || name.contains("ch340")
                        });
                    let name = info.product.unwrap_or_else(|| {
                        format!("Dispositivo USB {:04X}:{:04X}", info.vid, info.pid)
                    });
                    (name, info.serial_number, likely)
                }
                serialport::SerialPortType::BluetoothPort => {
                    ("Bluetooth serial port".into(), None, false)
                }
                serialport::SerialPortType::PciPort => ("PCI serial port".into(), None, false),
                serialport::SerialPortType::Unknown => ("Serial port".into(), None, false),
            };
            DisplayPort {
                port: port.port_name,
                name,
                serial_number,
                likely_turzx,
            }
        })
        .collect::<Vec<_>>();
    ports.sort_by(|a, b| {
        b.likely_turzx
            .cmp(&a.likely_turzx)
            .then_with(|| a.port.cmp(&b.port))
    });
    Ok(ports)
}

pub fn resolve_port(configured: &str) -> Result<String> {
    if !configured.eq_ignore_ascii_case("AUTO") {
        return Ok(configured.to_string());
    }
    let ports = list()?;
    if let Some(display) = ports.iter().find(|port| port.likely_turzx) {
        Ok(display.port.clone())
    } else if ports.len() == 1 {
        // Many clones do not expose known USB identifiers. If only one serial
        // port exists, AUTO can select it without ambiguity.
        Ok(ports[0].port.clone())
    } else if ports.is_empty() {
        bail!("No COM port detected. Check the USB data cable and display serial driver.")
    } else {
        bail!("No port was automatically identified as TURZX. Select the COM port manually.")
    }
}
