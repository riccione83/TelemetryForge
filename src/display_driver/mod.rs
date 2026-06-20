pub mod detection;
pub mod protocol;
pub mod transport;

use crate::config::schema::DisplayConfig;
use anyhow::Result;
use transport::SerialTransport;

pub struct DisplaySession {
    transport: SerialTransport,
}

impl DisplaySession {
    pub fn connect(config: &DisplayConfig) -> Result<Self> {
        let port = detection::resolve_port(&config.port)?;
        let mut transport = SerialTransport::open(&port)?;
        let revision = protocol::initialize(&mut transport)?;
        tracing::info!(port = %port, ?revision, "display initialized");
        protocol::set_orientation(
            &mut transport,
            config.orientation,
            config.width,
            config.height,
        )?;
        protocol::set_brightness(&mut transport, config.brightness)?;
        Ok(Self { transport })
    }

    pub fn send_region(&mut self, rgb: &image::RgbImage, x: u16, y: u16) -> Result<()> {
        protocol::send_image(&mut self.transport, rgb, x, y)
    }

    pub fn set_brightness(&mut self, percent: u8) -> Result<()> {
        protocol::set_brightness(&mut self.transport, percent)
    }
}

pub fn send_frame(config: &DisplayConfig, rgb: &image::RgbImage) -> Result<()> {
    let mut session = DisplaySession::connect(config)?;
    session.send_region(rgb, 0, 0)?;
    Ok(())
}

pub fn apply_brightness(config: &DisplayConfig) -> Result<()> {
    let mut session = DisplaySession::connect(config)?;
    session.set_brightness(config.brightness)
}
