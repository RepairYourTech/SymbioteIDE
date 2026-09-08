fn main() {
    // Reproducible generated proof icon, kept with ignored build artifacts.
    std::fs::create_dir_all("target").unwrap();
    let file = std::fs::File::create("target/proof-icon.png").unwrap();
    let mut encoder = png::Encoder::new(file, 32, 32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer
        .write_image_data(&[42, 105, 160, 255].repeat(32 * 32))
        .unwrap();
    drop(writer);
    tauri_build::build();
}
