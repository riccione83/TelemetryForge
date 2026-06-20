use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat, RgbImage};
use std::io::Cursor;

pub fn png_bytes(image: &RgbImage) -> Result<Vec<u8>> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image.clone())
        .write_to(&mut bytes, ImageFormat::Png)
        .context("Could not encode the PNG preview")?;
    Ok(bytes.into_inner())
}

pub fn parse_colour(value: &str) -> image::Rgb<u8> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() == 6 {
        if let Ok(value) = u32::from_str_radix(hex, 16) {
            return image::Rgb([
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
            ]);
        }
    }
    image::Rgb([255, 255, 255])
}
