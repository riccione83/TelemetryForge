use ab_glyph::FontArc;
use anyhow::{Context, Result};

pub fn load(name: &str) -> Result<FontArc> {
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
            return FontArc::try_from_vec(bytes).context("Invalid Windows font");
        }
    }
    anyhow::bail!("No supported Windows font found (Segoe UI/Arial)")
}
