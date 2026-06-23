use crate::{config::schema::WeatherConfig, sensors::model::SensorSnapshot};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Response {
    current: Current,
}

#[derive(Debug, Deserialize)]
struct Current {
    temperature_2m: f32,
    relative_humidity_2m: f32,
    weather_code: u16,
    wind_speed_10m: f32,
}

pub fn read(config: &WeatherConfig) -> Result<SensorSnapshot> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(4))
        .user_agent("TelemetryForge/0.2")
        .build()?;
    let response = client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", config.latitude.to_string()),
            ("longitude", config.longitude.to_string()),
            (
                "current",
                "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m".into(),
            ),
            ("timezone", "auto".into()),
        ])
        .send()
        .context("weather request failed")?
        .error_for_status()
        .context("weather service returned an error")?
        .json::<Response>()
        .context("invalid weather response")?;
    Ok(SensorSnapshot {
        weather_temperature: Some(response.current.temperature_2m),
        weather_humidity: Some(response.current.relative_humidity_2m),
        weather_wind_speed: Some(response.current.wind_speed_10m),
        weather_code: Some(response.current.weather_code),
        weather_condition: Some(condition(response.current.weather_code).into()),
        ..Default::default()
    })
}

pub fn condition(code: u16) -> &'static str {
    match code {
        0 => "Clear",
        1 => "Mostly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55 | 56 | 57 => "Drizzle",
        61 | 63 | 65 | 66 | 67 => "Rain",
        71 | 73 | 75 | 77 => "Snow",
        80..=82 => "Rain showers",
        85 | 86 => "Snow showers",
        95..=99 => "Thunderstorm",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::condition;

    #[test]
    fn maps_wmo_weather_codes() {
        assert_eq!(condition(0), "Clear");
        assert_eq!(condition(63), "Rain");
        assert_eq!(condition(95), "Thunderstorm");
    }
}
