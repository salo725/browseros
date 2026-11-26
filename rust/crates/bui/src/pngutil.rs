//! PNG decoding to RGBA8 buffers (pure Rust `png` crate).

use png::{ColorType, Decoder};

/// A decoded image: straight-alpha RGBA8.
#[derive(Clone)]
pub struct Image {
    pub w: i32,
    pub h: i32,
    pub data: Vec<u8>, // RGBA8, w*h*4
}

impl Image {
    pub fn new(w: i32, h: i32) -> Self {
        Image { w, h, data: vec![0u8; (w * h * 4).max(0) as usize] }
    }

    pub fn decode(path: &str) -> Option<Image> {
        let file = std::fs::File::open(path).ok()?;
        let dec = Decoder::new(file);
        let mut reader = dec.read_info().ok()?;
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).ok()?;
        let (w, h) = (info.width as i32, info.height as i32);
        let img = match info.color_type {
            ColorType::Rgba => Image { w, h, data: buf[..(w * h * 4) as usize].to_vec() },
            ColorType::Rgb => {
                let mut data = Vec::with_capacity((w * h * 4) as usize);
                for px in buf[..(w * h * 3) as usize].chunks_exact(3) {
                    data.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
                Image { w, h, data }
            }
            ColorType::Grayscale => {
                let mut data = Vec::with_capacity((w * h * 4) as usize);
                for g in &buf[..(w * h) as usize] {
                    data.extend_from_slice(&[*g, *g, *g, 255]);
                }
                Image { w, h, data }
            }
            ColorType::GrayscaleAlpha => {
                let mut data = Vec::with_capacity((w * h * 4) as usize);
                for ga in buf[..(w * h * 2) as usize].chunks_exact(2) {
                    data.extend_from_slice(&[ga[0], ga[0], ga[0], ga[1]]);
                }
                Image { w, h, data }
            }
            ColorType::Indexed => return None,
            _ => return None,
        };
        Some(img)
    }

    /// Bilinear resize.
    pub fn resized(&self, nw: i32, nh: i32) -> Image {
        let (sw, sh) = (self.w as f32, self.h as f32);
        let (nw, nh) = (nw.max(1) as usize, nh.max(1) as usize);
        let mut out = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            let fy = (y as f32 + 0.5) * sh / nh as f32 - 0.5;
            let y0 = fy.floor().max(0.0) as usize;
            let y1 = (y0 + 1).min(self.h as usize - 1);
            let wy = (fy - fy.floor()).clamp(0.0, 1.0);
            for x in 0..nw {
                let fx = (x as f32 + 0.5) * sw / nw as f32 - 0.5;
                let x0 = fx.floor().max(0.0) as usize;
                let x1 = (x0 + 1).min(self.w as usize - 1);
                let wx = (fx - fx.floor()).clamp(0.0, 1.0);
                let base = y * nw * 4 + x * 4;
                for c in 0..4 {
                    let p00 = self.data[(y0 * self.w as usize + x0) * 4 + c] as f32;
                    let p01 = self.data[(y0 * self.w as usize + x1) * 4 + c] as f32;
                    let p10 = self.data[(y1 * self.w as usize + x0) * 4 + c] as f32;
                    let p11 = self.data[(y1 * self.w as usize + x1) * 4 + c] as f32;
                    let top = p00 + (p01 - p00) * wx;
                    let bot = p10 + (p11 - p10) * wx;
                    out[base + c] = (top + (bot - top) * wy).clamp(0.0, 255.0) as u8;
                }
            }
        }
        Image { w: nw as i32, h: nh as i32, data: out }
    }

    /// Scale-preserving cover: resize so the image covers `w x h`, centered crop.
    pub fn cover(&self, w: i32, h: i32) -> Image {
        let scale = (w as f32 / self.w as f32).max(h as f32 / self.h as f32);
        let nw = (self.w as f32 * scale).ceil() as i32;
        let nh = (self.h as f32 * scale).ceil() as i32;
        let big = self.resized(nw, nh);
        let x0 = ((nw - w) / 2).max(0) as usize;
        let y0 = ((nh - h) / 2).max(0) as usize;
        let mut out = Image::new(w, h);
        for y in 0..h.min(nh) as usize {
            let src = ((y0 + y) * nw as usize + x0) * 4;
            let dst = (y * w as usize) * 4;
            let row = (w as usize * 4).min(big.data.len() - src);
            out.data[dst..dst + row].copy_from_slice(&big.data[src..src + row]);
        }
        out
    }
}
