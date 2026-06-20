pub mod advanced_widgets;
pub mod background;
pub mod canvas;
pub mod fonts;

use crate::{config::schema::AppConfig, sensors::model::SensorSnapshot};
use anyhow::Result;
use image::RgbImage;

pub fn render(config: &AppConfig, sensors: &SensorSnapshot) -> Result<RgbImage> {
    let mut frame = background::create(config)?;
    advanced_widgets::draw_all(&mut frame, config, sensors)?;
    Ok(frame)
}
