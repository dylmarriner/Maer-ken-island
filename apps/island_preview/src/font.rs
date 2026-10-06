//! A 5×7 bitmap font for gallery labels: digits and the few letters the
//! labels use. Each glyph is seven rows of five bits, most significant bit
//! on the left.

fn glyph(ch: char) -> Option<[u8; 7]> {
    Some(match ch {
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        'D' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        ' ' => [0; 7],
        _ => return None,
    })
}

/// Draws `text` into an RGB image at (`x`, `y`) with pixel size `scale`.
/// Characters without a glyph are skipped.
#[allow(clippy::too_many_arguments)]
pub fn draw_text(
    rgb: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    text: &str,
    colour: [u8; 3],
    scale: usize,
) {
    for (i, ch) in text.chars().enumerate() {
        let Some(rows) = glyph(ch) else { continue };
        let ox = x + i * 6 * scale;
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..5 {
                if bits & (0x10 >> col) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let (px, py) = (ox + col * scale + dx, y + row * scale + dy);
                        if px < width && py < height {
                            let k = 3 * (py * width + px);
                            rgb[k..k + 3].copy_from_slice(&colour);
                        }
                    }
                }
            }
        }
    }
}
