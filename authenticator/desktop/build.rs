fn main() {
    // Only embed a Windows resource (app icon) when compiling for Windows.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_icon();
    }
}

fn embed_windows_icon() {
    use std::io::Write as _;

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let ico_path = std::path::Path::new(&out_dir).join("app.ico");

    // ── Generate a 32×32 RGBA icon (accent-blue ring on dark background) ──
    const SIZE: u32 = 32;
    let c = SIZE as f32 / 2.0;
    let r = c - 0.5_f32;
    let ring_w = 5.0_f32;

    let mut pixels: Vec<u8> = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 + 0.5 - c;
            let dy = y as f32 + 0.5 - c;
            let d = (dx * dx + dy * dy).sqrt();

            if d > r {
                pixels.extend_from_slice(&[0, 0, 0, 0]); // transparent
            } else if d >= r - ring_w {
                pixels.extend_from_slice(&[122, 162, 247, 255]); // accent blue
            } else {
                pixels.extend_from_slice(&[26, 26, 38, 255]); // dark fill
            }
        }
    }

    // ── Write ICO file using the `ico` crate ──────────────────────────────
    let image = ico::IconImage::from_rgba_data(SIZE, SIZE, pixels);
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    dir.add_entry(ico::IconDirEntry::encode(&image).expect("Failed to encode icon image"));
    {
        let file = std::fs::File::create(&ico_path).expect("Failed to create app.ico");
        let mut buf = std::io::BufWriter::new(file);
        dir.write(&mut buf).expect("Failed to write app.ico");
        buf.flush().expect("Failed to flush app.ico");
    }

    // ── Embed the icon into the .exe via winres ───────────────────────────
    let mut res = winres::WindowsResource::new();
    res.set_icon(ico_path.to_str().expect("icon path is not valid UTF-8"));
    if let Err(e) = res.compile() {
        // Emit a warning rather than hard-failing so a missing RC compiler
        // doesn't break cross-compilation or CI environments.
        println!("cargo:warning=winres could not embed icon: {e}");
    }
}
