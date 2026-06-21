use anyhow::{Context, Result};
use serialport::{FlowControl, SerialPort};
use std::{
    io::{Read, Write},
    time::Duration,
};

pub struct SerialTransport {
    port: Box<dyn SerialPort>,
}

impl SerialTransport {
    pub fn open(name: &str) -> Result<Self> {
        let port = serialport::new(name, 115_200)
            .flow_control(FlowControl::Hardware)
            .timeout(Duration::from_secs(2))
            .open()
            .with_context(|| format!("Could not open serial port {name}"))?;
        Ok(Self { port })
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.port.write_all(data)?;
        Ok(())
    }

    pub fn read_available(&mut self, data: &mut [u8]) -> Result<usize> {
        let available = self.port.bytes_to_read()? as usize;
        if available == 0 {
            return Ok(0);
        }
        let size = available.min(data.len());
        Ok(self.port.read(&mut data[..size])?)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.port.flush()?;
        Ok(())
    }

    pub fn clear_input(&mut self) -> Result<()> {
        self.port.clear(serialport::ClearBuffer::Input)?;
        Ok(())
    }
}
