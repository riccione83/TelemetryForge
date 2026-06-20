use super::transport::SerialTransport;
use crate::config::schema::Orientation;
use anyhow::{bail, Result};
use image::RgbImage;
use std::{thread, time::Duration};

const DISPLAY_BITMAP: u8 = 197;
const SET_BRIGHTNESS: u8 = 110;
const SET_ORIENTATION: u8 = 121;
const SCREEN_ON: u8 = 109;
const HELLO: u8 = 69;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayRevision {
    Turing35,
    UsbMonitor35,
    UsbMonitor5,
    UsbMonitor7,
    Unknown,
}

fn command(cmd: u8, x: u16, y: u16, ex: u16, ey: u16) -> [u8; 6] {
    [
        (x >> 2) as u8,
        (((x & 3) << 6) + (y >> 4)) as u8,
        (((y & 15) << 4) + (ex >> 6)) as u8,
        (((ex & 63) << 2) + (ey >> 8)) as u8,
        (ey & 255) as u8,
        cmd,
    ]
}

pub fn set_brightness(io: &mut SerialTransport, percent: u8) -> Result<()> {
    let percent = percent.min(100);
    let absolute = 255 - ((percent as f32 / 100.0) * 255.0) as u16;
    io.write_all(&command(SET_BRIGHTNESS, absolute, 0, 0, 0))
}

pub fn initialize(io: &mut SerialTransport) -> Result<DisplayRevision> {
    // UsbMonitor V2 screens expect the HELLO exchange before regular
    // commands. Original Turing 3.5 screens simply do not answer it.
    io.clear_input()?;
    io.write_all(&[HELLO; 6])?;
    io.flush()?;
    thread::sleep(Duration::from_millis(120));

    let mut response = [0u8; 6];
    let size = io.read(&mut response)?;
    io.clear_input()?;
    let revision = match response {
        [1, 1, 1, 1, 1, 1] => DisplayRevision::UsbMonitor35,
        [2, 2, 2, 2, 2, 2] => DisplayRevision::UsbMonitor5,
        [3, 3, 3, 3, 3, 3] => DisplayRevision::UsbMonitor7,
        _ if size == 0 => DisplayRevision::Turing35,
        _ => DisplayRevision::Unknown,
    };
    io.write_all(&command(SCREEN_ON, 0, 0, 0, 0))?;
    io.flush()?;
    thread::sleep(Duration::from_millis(50));
    Ok(revision)
}

pub fn set_orientation(
    io: &mut SerialTransport,
    orientation: Orientation,
    width: u32,
    height: u32,
) -> Result<()> {
    let value = match orientation {
        Orientation::Portrait => 0,
        Orientation::ReversePortrait => 1,
        Orientation::Landscape => 2,
        Orientation::ReverseLandscape => 3,
    };
    let mut payload = [0u8; 16];
    payload[..6].copy_from_slice(&command(SET_ORIENTATION, 0, 0, 0, 0));
    payload[6] = value + 100;
    payload[7] = (width >> 8) as u8;
    payload[8] = width as u8;
    payload[9] = (height >> 8) as u8;
    payload[10] = height as u8;
    io.write_all(&payload)
}

pub fn send_image(io: &mut SerialTransport, image: &RgbImage, x: u16, y: u16) -> Result<()> {
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 || width > u16::MAX as u32 || height > u16::MAX as u32 {
        bail!("Dimensioni frame non valide");
    }
    let ex = x + width as u16 - 1;
    let ey = y + height as u16 - 1;
    io.write_all(&command(DISPLAY_BITMAP, x, y, ex, ey))?;

    let mut rgb565 = Vec::with_capacity((width * height * 2) as usize);
    for pixel in image.pixels() {
        let [r, g, b] = pixel.0;
        let packed = ((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3);
        rgb565.extend_from_slice(&packed.to_le_bytes());
    }
    for chunk in rgb565.chunks(width as usize * 8) {
        io.write_all(chunk)?;
    }
    io.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_bitmap_command_like_upstream() {
        assert_eq!(
            command(DISPLAY_BITMAP, 0, 0, 479, 319),
            [0, 0, 7, 125, 63, 197]
        );
    }

    #[test]
    fn rgb565_is_little_endian() {
        let mut image = RgbImage::new(1, 1);
        image.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        let [r, g, b] = image.get_pixel(0, 0).0;
        let packed = ((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3);
        assert_eq!(packed.to_le_bytes(), [0x00, 0xF8]);
    }

    #[test]
    fn hello_command_matches_upstream() {
        assert_eq!([HELLO; 6], [69, 69, 69, 69, 69, 69]);
    }
}
