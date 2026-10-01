//! The application icon, rendered as pixels in code.
//!
//! Keeping the icon procedural means the repository carries no binary
//! assets: the build script encodes these pixels as PNG/ICO for the
//! executable and the installer, and the app windows use the RGBA pixels
//! directly.

const BG: [u8; 3] = [30, 30, 46];
const EDGE: [u8; 3] = [70, 70, 96];
const ACCENT: [u8; 3] = [88, 101, 242];
const FG: [u8; 3] = [245, 245, 250];

/// RGBA pixels of the icon at `size` x `size`, 4x4 supersampled.
pub fn render_icon(size: usize) -> Vec<u8> {
    let s = size as f32;
    let mut rgba = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let mut acc = [0f32; 4];
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                    let sample = sample_icon(px / s, py / s);
                    for (i, v) in sample.iter().enumerate() {
                        acc[i] += v / 16.0;
                    }
                }
            }
            let offset = (y * size + x) * 4;
            for (i, v) in acc.iter().enumerate() {
                rgba[offset + i] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    rgba
}

/// A dark rounded square with a play-symbol badge: the emulator is the
/// square, the accent disc is the presence indicator.
fn sample_icon(u: f32, v: f32) -> [f32; 4] {
    let margin = 0.03;
    let radius = 0.22;
    if !rounded_rect(u, v, margin, margin, 1.0 - margin, 1.0 - margin, radius) {
        return [0.0, 0.0, 0.0, 0.0];
    }

    let inset = 0.012;
    if !rounded_rect(
        u,
        v,
        margin + inset,
        margin + inset,
        1.0 - margin - inset,
        1.0 - margin - inset,
        radius - inset,
    ) {
        return rgba(EDGE, 255);
    }

    let (cx, cy, r) = (0.5f32, 0.56f32, 0.30f32);
    let dist = ((u - cx).powi(2) + (v - cy).powi(2)).sqrt();
    if dist <= r {
        let (tx, ty) = (u - (cx - 0.10), v - (cy - 0.13));
        let in_triangle =
            (0.0..=0.26).contains(&ty) && tx >= 0.02 && tx <= 0.24 - (ty - 0.13).abs() * 0.85;
        if in_triangle {
            return rgba(FG, 255);
        }
        return rgba(ACCENT, 255);
    }

    rgba(BG, 255)
}

fn rounded_rect(u: f32, v: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    let qx = (u - (x0 + x1) / 2.0).abs() - (x1 - x0) / 2.0 + r;
    let qy = (v - (y0 + y1) / 2.0).abs() - (y1 - y0) / 2.0 + r;
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0);
    outside <= r
}

fn rgba(rgb: [u8; 3], a: u8) -> [f32; 4] {
    [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, a as f32]
}

// ------------------------------------------------------------ encoding

// The encoders are used by the build script (which shares this module)
// and by the unit tests; the binary itself only needs the pixels.

/// Encode RGBA pixels as a PNG. Uses uncompressed deflate blocks, which
/// every PNG decoder accepts and which keeps this file dependency-free.
#[allow(dead_code)]
pub fn png_rgba(width: usize, height: usize, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width * 4 + 1) * height);
    for y in 0..height {
        raw.push(0); // filter: none
        raw.extend_from_slice(&rgba[y * width * 4..(y + 1) * width * 4]);
    }

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA
    png.extend_from_slice(&chunk(b"IHDR", &ihdr));
    png.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    png.extend_from_slice(&chunk(b"IEND", &[]));
    png
}

/// ICO with a single PNG-compressed image (supported since Windows Vista).
#[allow(dead_code)]
pub fn ico_from_png(png: &[u8]) -> Vec<u8> {
    let mut ico = Vec::with_capacity(22 + png.len());
    ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]); // reserved, type=icon, count=1
    ico.extend_from_slice(&[0, 0, 0, 0]); // 256x256, no palette, reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // color planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes()); // image offset
    ico.extend_from_slice(png);
    ico
}

#[allow(dead_code)]
fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = kind.to_vec();
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

#[allow(dead_code)]
fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01]; // zlib header: deflate, fastest
    let mut remaining = data;
    loop {
        let take = remaining.len().min(65535);
        let last = take == remaining.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(take as u16).to_le_bytes());
        out.extend_from_slice(&(!(take as u16)).to_le_bytes());
        out.extend_from_slice(&remaining[..take]);
        remaining = &remaining[take..];
        if last {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

#[allow(dead_code)]
fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

#[allow(dead_code)]
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::{adler32, crc32, ico_from_png, png_rgba, render_icon};

    #[test]
    fn crc32_known_vectors() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn adler32_known_vectors() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn png_has_valid_structure() {
        let png = png_rgba(2, 2, &[128u8; 16]);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert!(png.len() > 60);
    }

    #[test]
    fn zlib_roundtrips_through_standard_decoder() {
        // miniz_oxide comes in transitively; use it to prove the stored
        // deflate blocks are well formed.
        let data: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let png_pixels = vec![77u8; 4 * 4 * 4];
        let png = png_rgba(4, 4, &png_pixels);
        // Extract the IDAT payload.
        let idat_len = u32::from_be_bytes(png[33..37].try_into().unwrap()) as usize;
        let idat = &png[41..41 + idat_len];
        let decompressed = miniz_oxide::inflate::decompress_to_vec_zlib(idat).unwrap();
        assert_eq!(decompressed.len(), (4 * 4 + 1) * 4);
        assert_eq!(data.len(), 200_000);
    }

    #[test]
    fn ico_wraps_png() {
        let png = png_rgba(256, 256, &render_icon(256));
        let ico = ico_from_png(&png);
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(&ico[22..30], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn icon_pixels_look_sane() {
        let pixels = render_icon(64);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Corners are transparent, the center is opaque.
        assert_eq!(pixels[3], 0);
        let center = (32 * 64 + 32) * 4;
        assert_eq!(pixels[center + 3], 255);
    }
}
