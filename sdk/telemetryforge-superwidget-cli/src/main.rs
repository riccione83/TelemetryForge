use anyhow::{Context, Result};
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipWriter};

fn main() -> Result<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 3 || !matches!(arguments[0].as_str(), "pack" | "pack-wat") {
        anyhow::bail!(
            "Usage: telemetryforge-superwidget <pack|pack-wat> <project-dir> <output.superwidget>"
        );
    }
    pack(
        &arguments[0],
        Path::new(&arguments[1]),
        Path::new(&arguments[2]),
    )
}

fn pack(mode: &str, project: &Path, output: &Path) -> Result<()> {
    let manifest = fs::read(project.join("manifest.yaml")).context("manifest.yaml is missing")?;
    let module = if mode == "pack-wat" {
        wat::parse_file(project.join("widget.wat")).context("widget.wat is invalid")?
    } else {
        fs::read(project.join("widget.wasm")).context("widget.wasm is missing")?
    };
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut archive = ZipWriter::new(File::create(output)?);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    archive.start_file("manifest.yaml", options)?;
    archive.write_all(&manifest)?;
    archive.start_file("widget.wasm", options)?;
    archive.write_all(&module)?;
    archive.finish()?;
    println!("{}", output.display());
    Ok(())
}
