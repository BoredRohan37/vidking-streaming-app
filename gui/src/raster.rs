//! Tiny CPU renderer for egui.
//!
//! egui produces textured, vertex-coloured triangles. This module rasterises them into a
//! `0x00RRGGBB` framebuffer so the GUI runs on machines without a GPU or OpenGL 2.0
//! (for example old Windows 7 PCs). Blending is done in gamma space, like egui's own
//! painters, with premultiplied alpha.

use egui::epaint::{ImageData, Primitive, Vertex};
use egui::{ClippedPrimitive, TextureFilter, TextureId, TexturesDelta};
use std::collections::HashMap;

struct Tex {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
    nearest: bool,
}

#[derive(Default)]
pub struct Raster {
    textures: HashMap<TextureId, Tex>,
}

impl Raster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply texture uploads. Call before `paint`.
    pub fn set_textures(&mut self, delta: &TexturesDelta) {
        for (id, deltas) in &delta.set {
          for d in deltas.iter() {
            let ImageData::Color(img) = &d.image;
            let [w, h] = img.size;
            let nearest = d.options.magnification == TextureFilter::Nearest;
            match d.pos {
                None => {
                    let px: Vec<[u8; 4]> = img.pixels.iter().map(|c| c.to_array()).collect();
                    self.textures.insert(*id, Tex { w, h, px, nearest });
                }
                Some([x0, y0]) => {
                    if let Some(tex) = self.textures.get_mut(id) {
                        for row in 0..h {
                            let ty = y0 + row;
                            if ty >= tex.h {
                                break;
                            }
                            for col in 0..w {
                                let tx = x0 + col;
                                if tx >= tex.w {
                                    break;
                                }
                                tex.px[ty * tex.w + tx] = img.pixels[row * w + col].to_array();
                            }
                        }
                    }
                }
            }
          }
        }
    }

    /// Free textures egui no longer needs. Call after `paint`.
    pub fn free_textures(&mut self, delta: &TexturesDelta) {
        for id in &delta.free {
            self.textures.remove(id);
        }
    }

    /// Paint primitives into `fb` (`width * height` pixels, `0x00RRGGBB`).
    pub fn paint(
        &self,
        fb: &mut [u32],
        width: usize,
        height: usize,
        ppp: f32,
        prims: &[ClippedPrimitive],
    ) {
        if fb.len() < width * height || width == 0 || height == 0 {
            return;
        }
        for prim in prims {
            let Primitive::Mesh(mesh) = &prim.primitive else {
                continue;
            };
            let Some(tex) = self.textures.get(&mesh.texture_id) else {
                continue;
            };
            let cr = prim.clip_rect;
            let clip = (
                ((cr.min.x * ppp).floor().max(0.0)) as i32,
                ((cr.min.y * ppp).floor().max(0.0)) as i32,
                ((cr.max.x * ppp).ceil().min(width as f32)) as i32,
                ((cr.max.y * ppp).ceil().min(height as f32)) as i32,
            );
            if clip.2 <= clip.0 || clip.3 <= clip.1 {
                continue;
            }
            #[allow(clippy::chunks_exact_to_as_chunks)]
            for tri in mesh.indices.chunks_exact(3) {
                let (Some(a), Some(b), Some(c)) = (
                    mesh.vertices.get(tri[0] as usize),
                    mesh.vertices.get(tri[1] as usize),
                    mesh.vertices.get(tri[2] as usize),
                ) else {
                    continue;
                };
                draw_triangle(fb, width, clip, tex, ppp, a, b, c);
            }
        }
    }
}

#[inline]
fn col4(v: &Vertex) -> [f32; 4] {
    let c = v.color.to_array();
    [c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32]
}

fn sample(tex: &Tex, u: f32, v: f32) -> [f32; 4] {
    let fw = tex.w as f32;
    let fh = tex.h as f32;
    if tex.nearest {
        let x = ((u * fw).floor() as i32).clamp(0, tex.w as i32 - 1) as usize;
        let y = ((v * fh).floor() as i32).clamp(0, tex.h as i32 - 1) as usize;
        let p = tex.px[y * tex.w + x];
        return [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32];
    }
    let x = u * fw - 0.5;
    let y = v * fh - 0.5;
    let x0f = x.floor();
    let y0f = y.floor();
    let fx = x - x0f;
    let fy = y - y0f;
    let maxx = tex.w as i32 - 1;
    let maxy = tex.h as i32 - 1;
    let x0 = (x0f as i32).clamp(0, maxx) as usize;
    let x1 = (x0f as i32 + 1).clamp(0, maxx) as usize;
    let y0 = (y0f as i32).clamp(0, maxy) as usize;
    let y1 = (y0f as i32 + 1).clamp(0, maxy) as usize;
    let p00 = tex.px[y0 * tex.w + x0];
    let p10 = tex.px[y0 * tex.w + x1];
    let p01 = tex.px[y1 * tex.w + x0];
    let p11 = tex.px[y1 * tex.w + x1];
    let w00 = (1.0 - fx) * (1.0 - fy);
    let w10 = fx * (1.0 - fy);
    let w01 = (1.0 - fx) * fy;
    let w11 = fx * fy;
    let mut out = [0.0f32; 4];
    for i in 0..4 {
        out[i] = p00[i] as f32 * w00 + p10[i] as f32 * w10 + p01[i] as f32 * w01 + p11[i] as f32 * w11;
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn draw_triangle(
    fb: &mut [u32],
    stride: usize,
    clip: (i32, i32, i32, i32),
    tex: &Tex,
    ppp: f32,
    a: &Vertex,
    b: &Vertex,
    c: &Vertex,
) {
    let (ax, ay) = (a.pos.x * ppp, a.pos.y * ppp);
    let (bx, by) = (b.pos.x * ppp, b.pos.y * ppp);
    let (cx, cy) = (c.pos.x * ppp, c.pos.y * ppp);

    let area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    if area.abs() < 1e-8 || !area.is_finite() {
        return;
    }

    let min_x = (ax.min(bx).min(cx).floor() as i32).max(clip.0);
    let max_x = (ax.max(bx).max(cx).ceil() as i32).min(clip.2);
    let min_y = (ay.min(by).min(cy).floor() as i32).max(clip.1);
    let max_y = (ay.max(by).max(cy).ceil() as i32).min(clip.3);
    if max_x <= min_x || max_y <= min_y {
        return;
    }

    let inv = 1.0 / area;
    // Barycentric weights are affine in (px, py): w = k + px*dx + py*dy.
    let ka = (bx * cy - by * cx) * inv;
    let dxa = (by - cy) * inv;
    let dya = (cx - bx) * inv;
    let kb = (cx * ay - cy * ax) * inv;
    let dxb = (cy - ay) * inv;
    let dyb = (ax - cx) * inv;

    let ca = col4(a);
    let cb = col4(b);
    let cc = col4(c);
    let same_color = ca == cb && cb == cc;

    let const_uv = a.uv == b.uv && b.uv == c.uv;
    let const_tex = if const_uv {
        Some(sample(tex, a.uv.x, a.uv.y))
    } else {
        None
    };

    for y in min_y..max_y {
        let py = y as f32 + 0.5;
        let row = y as usize * stride;
        for x in min_x..max_x {
            let px = x as f32 + 0.5;
            let wa = ka + px * dxa + py * dya;
            let wb = kb + px * dxb + py * dyb;
            let wc = 1.0 - wa - wb;
            if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                continue;
            }

            let t = match const_tex {
                Some(t) => t,
                None => {
                    let u = wa * a.uv.x + wb * b.uv.x + wc * c.uv.x;
                    let v = wa * a.uv.y + wb * b.uv.y + wc * c.uv.y;
                    sample(tex, u, v)
                }
            };

            let vc = if same_color {
                ca
            } else {
                [
                    wa * ca[0] + wb * cb[0] + wc * cc[0],
                    wa * ca[1] + wb * cb[1] + wc * cc[1],
                    wa * ca[2] + wb * cb[2] + wc * cc[2],
                    wa * ca[3] + wb * cb[3] + wc * cc[3],
                ]
            };

            // Premultiplied texel * premultiplied vertex colour.
            let k = 1.0 / 255.0;
            let sr = t[0] * vc[0] * k;
            let sg = t[1] * vc[1] * k;
            let sb = t[2] * vc[2] * k;
            let sa = t[3] * vc[3] * k;
            if sa <= 0.0 && sr <= 0.0 && sg <= 0.0 && sb <= 0.0 {
                continue;
            }

            let idx = row + x as usize;
            let dst = fb[idx];
            let inv_a = 1.0 - (sa * k).min(1.0);
            let dr = ((dst >> 16) & 0xFF) as f32;
            let dg = ((dst >> 8) & 0xFF) as f32;
            let db = (dst & 0xFF) as f32;
            let r = (sr + dr * inv_a).min(255.0) as u32;
            let g = (sg + dg * inv_a).min(255.0) as u32;
            let bl = (sb + db * inv_a).min(255.0) as u32;
            fb[idx] = (r << 16) | (g << 8) | bl;
        }
    }
}
