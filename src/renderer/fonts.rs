use ab_glyph::FontArc;
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

static FONT_CACHE: LazyLock<Mutex<HashMap<String, FontArc>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn load(name: &str) -> Result<FontArc> {
    if let Some(font) = FONT_CACHE.lock().expect("font cache poisoned").get(name) {
        return Ok(font.clone());
    }
    let requested = match name {
        "Arial" => r"C:\Windows\Fonts\arial.ttf",
        "Arial Bold" => r"C:\Windows\Fonts\arialbd.ttf",
        "Bahnschrift" => r"C:\Windows\Fonts\bahnschrift.ttf",
        "Calibri" => r"C:\Windows\Fonts\calibri.ttf",
        "Calibri Bold" => r"C:\Windows\Fonts\calibrib.ttf",
        "Consolas" => r"C:\Windows\Fonts\consola.ttf",
        "Consolas Bold" => r"C:\Windows\Fonts\consolab.ttf",
        "Courier New" => r"C:\Windows\Fonts\cour.ttf",
        "Impact" => r"C:\Windows\Fonts\impact.ttf",
        "Segoe UI Bold" => r"C:\Windows\Fonts\segoeuib.ttf",
        "Segoe UI Symbol" => r"C:\Windows\Fonts\seguisym.ttf",
        "Tahoma" => r"C:\Windows\Fonts\tahoma.ttf",
        "Trebuchet MS" => r"C:\Windows\Fonts\trebuc.ttf",
        "Verdana" => r"C:\Windows\Fonts\verdana.ttf",
        _ => r"C:\Windows\Fonts\segoeui.ttf",
    };
    for path in [
        requested,
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\arial.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            let font = FontArc::try_from_vec(bytes).context("Invalid Windows font")?;
            FONT_CACHE
                .lock()
                .expect("font cache poisoned")
                .insert(name.to_owned(), font.clone());
            return Ok(font);
        }
    }
    anyhow::bail!("No supported Windows font found (Segoe UI/Arial)")
}
