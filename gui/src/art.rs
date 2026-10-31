//! Generated artwork so the UI always looks finished, even offline or while images load.

use image::{imageops, RgbaImage};

pub fn hash(s: &str) -> u64 {
    // FNV-1a
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn rng(seed: u64, n: u64) -> f32 {
    // small deterministic hash -> [0,1)
    let mut x = seed ^ n.wrapping_mul(0x9E3779B97F4A7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// HSV (h in degrees, s/v in 0..1) -> RGB 0..255
pub fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
}

fn put(buf: &mut [u8], w: usize, x: usize, y: usize, c: [f32; 3]) {
    let i = (y * w + x) * 4;
    buf[i] = c[0].clamp(0.0, 255.0) as u8;
    buf[i + 1] = c[1].clamp(0.0, 255.0) as u8;
    buf[i + 2] = c[2].clamp(0.0, 255.0) as u8;
    buf[i + 3] = 255;
}

/// Colourful abstract poster for a title with no artwork. Returns RGBA8.
pub fn placeholder_poster(seed: &str, w: usize, h: usize) -> Vec<u8> {
    let s = hash(seed);
    let hue = rng(s, 1) * 360.0;
    let hue2 = hue + 35.0 + rng(s, 2) * 60.0;
    let top = hsv(hue, 0.62, 0.55);
    let bottom = hsv(hue2, 0.55, 0.12);
    let glow = hsv(hue2 + 20.0, 0.45, 1.0);
    let gx = 0.15 + rng(s, 3) * 0.7;
    let gy = 0.1 + rng(s, 4) * 0.35;
    let angle = rng(s, 5) * std::f32::consts::PI;
    let (sa, ca) = angle.sin_cos();

    let mut buf = vec![0u8; w * h * 4];
    for y in 0..h {
        let fy = y as f32 / h as f32;
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let mut c = [0.0f32; 3];
            for i in 0..3 {
                c[i] = top[i] + (bottom[i] - top[i]) * fy.powf(0.85);
            }
            // soft radial glow
            let dx = fx - gx;
            let dy = (fy - gy) * 1.2;
            let d = (dx * dx + dy * dy).sqrt();
            let g = (1.0 - d * 2.2).max(0.0).powi(2) * 0.55;
            // diagonal light band
            let t = (fx - 0.5) * ca + (fy - 0.5) * sa;
            let band = (1.0 - (t - 0.1).abs() * 7.0).max(0.0).powi(2) * 0.16;
            for i in 0..3 {
                c[i] += glow[i] * (g + band);
            }
            // grain
            let n = (rng(s ^ 0xABCD, (y * w + x) as u64) - 0.5) * 7.0;
            for v in c.iter_mut() {
                *v += n;
            }
            put(&mut buf, w, x, y, c);
        }
    }
    buf
}

/// Dark "aurora" background (used for the hero when there is no artwork). RGBA8.
pub fn aurora(w: usize, h: usize, seed: &str) -> Vec<u8> {
    let s = hash(seed);
    let base = [12.0f32, 12.0, 20.0];
    struct Blob {
        x: f32,
        y: f32,
        r: f32,
        col: [f32; 3],
    }
    let hue = rng(s, 1) * 360.0;
    let blobs = [
        Blob { x: 0.78, y: 0.25, r: 0.55, col: hsv(hue, 0.75, 0.85) },
        Blob { x: 0.45, y: 0.85, r: 0.60, col: hsv(hue + 140.0, 0.70, 0.55) },
        Blob { x: 0.12, y: 0.30, r: 0.45, col: hsv(hue + 250.0, 0.65, 0.60) },
        Blob { x: 0.95, y: 0.90, r: 0.40, col: hsv(hue + 60.0, 0.60, 0.50) },
    ];
    let mut buf = vec![0u8; w * h * 4];
    let aspect = w as f32 / h as f32;
    for y in 0..h {
        let fy = y as f32 / h as f32;
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let mut c = base;
            for b in &blobs {
                let dx = (fx - b.x) * aspect;
                let dy = fy - b.y;
                let d = (dx * dx + dy * dy).sqrt() / b.r;
                let k = (1.0 - d).max(0.0);
                let k = k * k * 0.95;
                for (ci, bc) in c.iter_mut().zip(b.col.iter()) {
                    *ci += bc * k;
                }
            }
            let n = (rng(s ^ 0x1234, (y * w + x) as u64) - 0.5) * 5.0;
            for v in c.iter_mut() {
                *v += n;
            }
            put(&mut buf, w, x, y, c);
        }
    }
    buf
}

pub struct Backdrop {
    pub rgba: Vec<u8>,
    pub w: usize,
    pub h: usize,
    /// Average colour of the artwork (0..255), used to tint glows.
    pub avg: [u8; 3],
}

/// Build a soft, blurred 16:9 backdrop from a poster (RGBA8). Cheap enough for a worker thread.
pub fn backdrop_from_poster(rgba: &[u8], w: usize, h: usize) -> Option<Backdrop> {
    let img = RgbaImage::from_raw(w as u32, h as u32, rgba.to_vec())?;
    let (tw, th) = (480u32, 270u32);
    // cover-crop to 16:9 around the upper-middle of the poster (faces/title art live there)
    let target_ratio = tw as f32 / th as f32;
    let src_ratio = w as f32 / h as f32;
    let (cw, ch) = if src_ratio < target_ratio {
        (w as u32, (w as f32 / target_ratio) as u32)
    } else {
        ((h as f32 * target_ratio) as u32, h as u32)
    };
    let cx = (w as u32 - cw) / 2;
    let cy = ((h as u32 - ch) as f32 * 0.30) as u32;
    let cropped = imageops::crop_imm(&img, cx, cy, cw.max(1), ch.max(1)).to_image();
    let small = imageops::resize(&cropped, tw / 3, th / 3, imageops::FilterType::Triangle);
    let blurred = imageops::blur(&small, 5.0);
    let out = imageops::resize(&blurred, tw, th, imageops::FilterType::Triangle);

    let mut sum = [0u64; 3];
    let mut n = 0u64;
    for p in out.pixels() {
        sum[0] += p[0] as u64;
        sum[1] += p[1] as u64;
        sum[2] += p[2] as u64;
        n += 1;
    }
    let n = n.max(1);
    Some(Backdrop {
        rgba: out.into_raw(),
        w: tw as usize,
        h: th as usize,
        avg: [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8],
    })
}

/// Downscale a decoded image to at most `max_w` x `max_h` (keeping aspect). Returns RGBA8 + size.
pub fn fit_rgba(img: &image::DynamicImage, max_w: u32, max_h: u32) -> (Vec<u8>, usize, usize) {
    let (w, h) = (img.width(), img.height());
    let scale = (max_w as f32 / w as f32).min(max_h as f32 / h as f32).min(1.0);
    let (nw, nh) = (
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
    );
    let rgba = if scale < 1.0 {
        imageops::resize(&img.to_rgba8(), nw, nh, imageops::FilterType::Triangle)
    } else {
        img.to_rgba8()
    };
    let (rw, rh) = (rgba.width() as usize, rgba.height() as usize);
    (rgba.into_raw(), rw, rh)
}
