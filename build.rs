//! Build script: renders the application icon and embeds it in the
//! Windows executable.
//!
//! The icon pixels are generated in code (see `src/icon.rs`) so the
//! repository stays free of binary assets. The PNG is written next to the
//! sources for the installer, and the ICO (which wraps the PNG) is embedded
//! in the executable.

#[path = "src/icon.rs"]
mod icon;

use std::path::Path;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pixels = icon::render_icon(256);
    let png = icon::png_rgba(256, 256, &pixels);

    write_if_changed(&manifest_dir.join("assets/icon.png"), &png);
    write_if_changed(
        &manifest_dir.join("assets/icon.ico"),
        &icon::ico_from_png(&png),
    );

    // Embed the icon in the Windows executable. The build script runs on
    // the host, so check the target explicitly.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("app-icon.rc", embed_resource::NONE);
    }
}

fn write_if_changed(path: &Path, bytes: &[u8]) {
    if std::fs::read(path)
        .map(|existing| existing == bytes)
        .unwrap_or(false)
    {
        return;
    }
    std::fs::write(path, bytes).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}
