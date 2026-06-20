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

    pub fn read(&mut self, data: &mut [u8]) -> Result<usize> {
        match self.port.read(data) {
            Ok(size) => Ok(size),
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => Ok(0),
            Err(error) => Err(error.into()),
        }
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
