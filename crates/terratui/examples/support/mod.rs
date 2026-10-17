use std::sync::Arc;
use terratui::terrabox::graphics::RgbaImage;
/// Repository-owned procedural art: no downloads or decoder needed.
pub fn sample(index: usize) -> Arc<RgbaImage> {
    let (width, height) = (160u32, 96u32);
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let dx = x as i32 - 80;
            let dy = y as i32 - 48;
            let color = match index % 3 {
                0 if dx * dx + dy * dy < 38 * 38 => [20, 185, (100 + x / 2) as u8, 255],
                0 => [
                    30,
                    60,
                    (100 + y) as u8,
                    if (x / 8 + y / 8) % 2 == 0 { 80 } else { 160 },
                ],
                1 if y > 60 + x / 10 => [24, 100, 90, 255],
                1 if y > 45 + x / 6 => [34, 140, 115, 255],
                1 => [230, (80 + y) as u8, (45 + y) as u8, 255],
                _ => [
                    (x * 255 / width) as u8,
                    (y * 255 / height) as u8,
                    200,
                    if dx * dx + dy * dy < 42 * 42 { 255 } else { 70 },
                ],
            };
            pixels.extend_from_slice(&color);
        }
    }
    Arc::new(RgbaImage::new(width, height, pixels).expect("procedural RGBA dimensions"))
}
