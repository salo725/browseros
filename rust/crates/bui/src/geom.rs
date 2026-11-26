//! Geometry helpers shared by all BraveOS UI components.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
    pub fn shrink(&self, d: i32) -> Rect {
        Rect::new(self.x + d, self.y + d, (self.w - 2 * d).max(0), (self.h - 2 * d).max(0))
    }
}

/// Blend `src` over `dst` (both 0xAARRGGBB premultiplied-NO, straight alpha).
pub fn blend(dst: [u8; 4], src: [u8; 4]) -> [u8; 4] {
    let sa = src[3] as u32;
    if sa == 255 {
        return src;
    }
    if sa == 0 {
        return dst;
    }
    let da = dst[3] as u32;
    let out_a = sa + da * (255 - sa) / 255;
    let mut out = [0u8; 4];
    for c in 0..3 {
        let s = src[c] as u32;
        let d = dst[c] as u32;
        let v = if out_a == 0 {
            0
        } else {
            (s * sa + d * da * (255 - sa) / 255) / out_a
        };
        out[c] = v.min(255) as u8;
    }
    out[3] = out_a.min(255) as u8;
    out
}

/// Interpolate two RGB colors by t (0..=255).
pub fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: u32) -> [u8; 3] {
    let t = t.min(255);
    [
        (a[0] as u32 + ((b[0] as u32 - a[0] as u32) * t) / 255).min(255) as u8,
        (a[1] as u32 + ((b[1] as u32 - a[1] as u32) * t) / 255).min(255) as u8,
        (a[2] as u32 + ((b[2] as u32 - a[2] as u32) * t) / 255).min(255) as u8,
    ]
}

/// Simple box blur in-place over an RGBA buffer (used for glow effects).
pub fn box_blur(buf: &mut [u8], w: i32, h: i32, radius: i32) {
    if radius <= 0 || w <= 0 || h <= 0 {
        return;
    }
    let (w, h) = (w as usize, h as usize);
    let r = radius as usize;
    let mut tmp = vec![0u8; buf.len()];
    // horizontal pass
    for y in 0..h {
        for c in 0..4 {
            let mut sum = 0u32;
            let row = y * w;
            for x in 0..w {
                if x == 0 {
                    for dx in 0..=r {
                        let xi = (dx).min(w - 1);
                        sum += buf[(row + xi) * 4 + c] as u32;
                    }
                    let n = (r + 1) as u32;
                    tmp[(row) * 4 + c] = (sum / n) as u8;
                } else {
                    let add_x = (x + r).min(w - 1);
                    let rem_x = x.saturating_sub(r + 1);
                    sum += buf[(row + add_x) * 4 + c] as u32;
                    sum -= buf[(row + rem_x) * 4 + c] as u32;
                    tmp[(row + x) * 4 + c] = (sum / (r + 1) as u32) as u8;
                }
            }
        }
    }
    // vertical pass into buf
    for x in 0..w {
        for c in 0..4 {
            let mut sum = 0u32;
            for y in 0..h {
                if y == 0 {
                    for dy in 0..=r {
                        let yi = (dy).min(h - 1);
                        sum += tmp[(yi * w + x) * 4 + c] as u32;
                    }
                    buf[(x) * 4 + c] = (sum / (r + 1) as u32) as u8;
                } else {
                    let add_y = (y + r).min(h - 1);
                    let rem_y = y.saturating_sub(r + 1);
                    sum += tmp[(add_y * w + x) * 4 + c] as u32;
                    sum -= tmp[(rem_y * w + x) * 4 + c] as u32;
                    buf[(y * w + x) * 4 + c] = (sum / (r + 1) as u32) as u8;
                }
            }
        }
    }
}
