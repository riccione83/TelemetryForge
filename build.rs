fn main() {
    let png = std::path::Path::new("icons/icon.png");
    let ico = std::path::Path::new("icons/icon.ico");
    if png.exists() && !ico.exists() {
        let image = image::open(png)
            .expect("failed to read icons/icon.png")
            .resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
        image
            .save_with_format(ico, image::ImageFormat::Ico)
            .expect("failed to generate icons/icon.ico");
    }
    tauri_build::build()
}
