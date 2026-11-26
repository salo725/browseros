//! RGBA canvas with the BraveOS "2026 glass" drawing helpers.

use crate::geom::{blend, box_blur, lerp_rgb};
use crate::pngutil::Image;

pub struct Canvas {
    pub w: i32,
    pub h: i32,
    pub data: Vec<u8>, // straight-alpha RGBA8
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Canvas {
        Canvas { w, h, data: vec![0u8; (w * h * 4).max(0) as usize] }
    }

    #[inline]
    pub fn px(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = ((y * self.w + x) * 4) as usize;
        let dst = [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]];
        let out = blend(dst, c);
        self.data[i..i + 4].copy_from_slice(&out);
    }

    /// Blend an 8-bit coverage bitmap (e.g. glyph raster) at (x, y).
    pub fn blit_alpha(&mut self, bm: &[u8], w: i32, h: i32, x: i32, y: i32, color: [u8; 4]) {
        for row in 0..h {
            let py = y + row;
            if py < 0 || py >= self.h {
                continue;
            }
            for col in 0..w {
                let px = x + col;
                if px < 0 || px >= self.w {
                    continue;
                }
                let a = bm[(row * w + col) as usize] as u32;
                if a == 0 {
                    continue;
                }
                let mut c = color;
                c[3] = ((c[3] as u32) * a / 255).min(255) as u8;
                self.px(px, py, c);
            }
        }
    }

    pub fn fill(&mut self, c: [u8; 4]) {
        for i in 0..(self.w * self.h) as usize {
            self.data[i * 4..i * 4 + 4].copy_from_slice(&c);
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.px(xx, yy, c);
            }
        }
    }

    pub fn rounded(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, c: [u8; 4]) {
        let r = r.min(w / 2).min(h / 2).max(0);
        for yy in 0..h {
            for xx in 0..w {
                let dx = if xx < r { r - xx } else if xx >= w - r { xx - (w - r - 1) } else { 0 };
                let dy = if yy < r { r - yy } else if yy >= h - r { yy - (h - r - 1) } else { 0 };
                if dx * dx + dy * dy <= r * r {
                    self.px(x + xx, y + yy, c);
                }
            }
        }
    }

    /// Rounded-rect outline.
    pub fn rounded_outline(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, c: [u8; 4]) {
        let r = r.min(w / 2).min(h / 2).max(1);
        // four straight edges
        for xx in x + r..x + w - r {
            self.px(xx, y, c);
            self.px(xx, y + h - 1, c);
        }
        for yy in y + r..y + h - r {
            self.px(x, yy, c);
            self.px(x + w - 1, yy, c);
        }
        // quarter arcs
        let mut arc = |cx: i32, cy: i32, sx: i32, sy: i32| {
            for a in 0..90i32 {
                let rad = (a as f32) * std::f32::consts::PI / 180.0;
                let px = cx + sx * (r as f32 * rad.cos()) as i32;
                let py = cy + sy * (r as f32 * rad.sin()) as i32;
                self.px(px, py, c);
                // anti-alias the diagonal a bit
                self.px(px + sx, py, [c[0], c[1], c[2], c[3] / 2]);
                self.px(px, py + sy, [c[0], c[1], c[2], c[3] / 2]);
            }
        };
        arc(x + r, y + r, -1, -1);
        arc(x + w - 1 - r, y + r, 1, -1);
        arc(x + r, y + h - 1 - r, -1, 1);
        arc(x + w - 1 - r, y + h - 1 - r, 1, 1);
    }

    /// Vertical gradient fill with alpha.
    pub fn v_gradient(&mut self, x: i32, y: i32, w: i32, h: i32, top: [u8; 4], bottom: [u8; 4]) {
        for yy in 0..h {
            let t = ((yy * 255) / h.max(1)).clamp(0, 255);
            let c = [
                lerp_rgb([top[0], top[1], top[2]], [bottom[0], bottom[1], bottom[2]], t as u32)[0],
                lerp_rgb([top[0], top[1], top[2]], [bottom[0], bottom[1], bottom[2]], t as u32)[1],
                lerp_rgb([top[0], top[1], top[2]], [bottom[0], bottom[1], bottom[2]], t as u32)[2],
                top[3] + ((bottom[3] as i32 - top[3] as i32) * t as i32 / 255).clamp(0, 255) as u8,
            ];
            self.rect(x, y + yy, w, 1, c);
        }
    }

    /// Glass pill/card: translucent dark fill + soft top highlight + hairline.
    pub fn glass(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, alpha: u8, composited: bool) {
        let a = if composited { alpha } else { (alpha as u16 + 40).min(255) as u8 };
        self.rounded(x, y, w, h, r, [10, 10, 16, a]);
        self.rounded_outline(x, y, w, h, r, [255, 255, 255, if composited { 26 } else { 40 }]);
    }

    /// Draw an RGBA image at (x, y).
    pub fn image(&mut self, img: &Image, x: i32, y: i32) {
        for yy in 0..img.h {
            for xx in 0..img.w {
                let i = ((yy * img.w + xx) * 4) as usize;
                let c = [img.data[i], img.data[i + 1], img.data[i + 2], img.data[i + 3]];
                if c[3] > 0 {
                    self.px(x + xx, y + yy, c);
                }
            }
        }
    }

    /// Draw an image tinted with a single color (for monochrome status icons).
    pub fn image_tinted(&mut self, img: &Image, x: i32, y: i32, color: [u8; 4]) {
        for yy in 0..img.h {
            for xx in 0..img.w {
                let i = ((yy * img.w + xx) * 4) as usize;
                let a = img.data[i + 3] as u32;
                if a == 0 {
                    continue;
                }
                let mut c = color;
                c[3] = ((color[3] as u32) * a / 255).min(255) as u8;
                self.px(x + xx, y + yy, c);
            }
        }
    }

    /// Antialiased line (approximate: supersample by drawing 4 offsets).
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: [u8; 4]) {
        let dx = (x1 - x0) as f32;
        let dy = (y1 - y0) as f32;
        let steps = (dx.abs() + dy.abs()).ceil().max(1.0) as i32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = (x0 as f32 + dx * t) as i32;
            let y = (y0 as f32 + dy * t) as i32;
            self.px(x, y, c);
            let mut soft = c;
            soft[3] = c[3] / 2;
            self.px(x + 1, y, soft);
            self.px(x, y + 1, soft);
        }
    }

    /// Antialiased circle outline.
    pub fn circle_outline(&mut self, cx: i32, cy: i32, r: i32, c: [u8; 4]) {
        let steps = (r * 7).max(32);
        for i in 0..steps {
            let a = i as f32 * 2.0 * std::f32::consts::PI / steps as f32;
            let x = cx + (r as f32 * a.cos()) as i32;
            let y = cy + (r as f32 * a.sin()) as i32;
            self.px(x, y, c);
            let mut soft = c;
            soft[3] = c[3] / 2;
            self.px(x + 1, y, soft);
        }
    }

    /// Filled circle.
    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, c: [u8; 4]) {
        for yy in -r..=r {
            for xx in -r..=r {
                if xx * xx + yy * yy <= r * r {
                    self.px(cx + xx, cy + yy, c);
                }
            }
        }
    }

    /// Soft radial glow (for the Brave dock button).
    pub fn glow(&mut self, cx: i32, cy: i32, r: i32, color: [u8; 3]) {
        let size = (2 * r + 2) as usize;
        let mut buf = vec![0u8; size * size * 4];
        for y in 0..(2 * r + 2) {
            for x in 0..(2 * r + 2) {
                let d = ((x - r - 1) * (x - r - 1) + (y - r - 1) * (y - r - 1)) as f32;
                let t = 1.0 - d / (r * r) as f32;
                let a = (t.clamp(0.0, 1.0).powf(2.2) * 255.0) as u8;
                let i = ((y * (2 * r + 2) + x) * 4) as usize;
                buf[i] = color[0];
                buf[i + 1] = color[1];
                buf[i + 2] = color[2];
                buf[i + 3] = a;
            }
        }
        // soften
        box_blur(&mut buf, 2 * r + 2, 2 * r + 2, r / 3 + 1);
        let img = Image { w: 2 * r + 2, h: 2 * r + 2, data: buf };
        self.image(&img, cx - r - 1, cy - r - 1);
    }

    /// Return the current buffer as X11-friendly ARGB32 scanlines (BGRX order).
    pub fn to_argb(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.data.len());
        for px in self.data.chunks_exact(4) {
            out.push(px[2]); // B
            out.push(px[1]); // G
            out.push(px[0]); // R
            out.push(px[3]); // A
        }
        out
    }
}
