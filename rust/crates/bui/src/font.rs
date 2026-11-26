//! Font handling: fontconfig lookup, freetype rasterization, rustybuzz
//! shaping, per-character fallback across all installed script families.

use std::collections::HashMap;
use std::sync::Arc;

use freetype::face::LoadFlag;
use freetype::Library;

pub struct CachedGlyph {
    pub w: i32,
    pub h: i32,
    pub left: i32,
    pub top: i32,
    pub advance: i32, // 26.6
    pub bitmap: Vec<u8>, // 8-bit coverage, w*h
}

struct FaceEntry {
    family: String,
    ft: freetype::Face,
    hb: rustybuzz::Face<'static>,
    upem: u16,
    ascender: i16,
    descender: i16,
}

pub struct Fonts {
    lib: Library,
    faces: Vec<FaceEntry>,
    fallback_for: HashMap<char, usize>,
    cache: HashMap<(usize, u32, u32, bool), Arc<CachedGlyph>>, // face, size, gid, bold
}

const FAMILY_ORDER: &[&str] = &[
    "Noto Sans",
    "DejaVu Sans",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans Arabic",
    "Noto Sans Hebrew",
    "Noto Sans Devanagari",
    "Noto Sans Thai",
    "Noto Sans Lao",
    "Noto Sans Khmer",
    "Noto Sans Myanmar",
    "Noto Sans Georgian",
    "Noto Sans Armenian",
    "Noto Sans Ethiopic",
    "Noto Sans Bengali",
    "Noto Sans Tamil",
    "Noto Sans Sinhala",
    "Noto Sans Burmese",
    "Liberation Sans",
    "WenQuanYi Zen Hei",
];

impl Fonts {
    /// Build the fallback list from fontconfig; missing families are skipped.
    pub fn new() -> Fonts {
        let lib = Library::init().expect("freetype init");
        let mut faces = Vec::new();
        let fc = fontconfig::Fontconfig::new();
        if let Some(fc) = fc {
            for fam in FAMILY_ORDER {
                // resolve regular weight for the family
                let Ok(m) = fc.find(fam, None) else { continue };
                if faces.iter().any(|f: &FaceEntry| f.family == *fam) {
                    continue;
                }
                if let Some(entry) = Self::load_face(&lib, m.path.to_str().unwrap_or(""), fam) {
                    faces.push(entry);
                }
            }
        }
        // Last-resort: scan well-known files directly (headless test hosts).
        if faces.is_empty() {
            for p in [
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            ] {
                if let Some(entry) = Self::load_face(&lib, p, "fallback") {
                    faces.push(entry);
                    break;
                }
            }
        }
        Fonts {
            lib,
            faces,
            fallback_for: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    fn load_face(lib: &Library, path: &str, family: &str) -> Option<FaceEntry> {
        // For TTC collections prefer an SC/TC face, else index 0.
        let mut best_index: isize = 0;
        let probe = lib.new_face(path, 0).ok()?;
        let num = probe.num_faces();
        if num > 1 {
            for i in 0..num {
                if let Ok(f) = lib.new_face(path, i as isize) {
                    if let Some(name) = f.family_name() {
                        if name.contains("SC") {
                            best_index = i as isize;
                            break;
                        }
                    }
                }
            }
        }
        let ft = lib.new_face(path, best_index).ok()?;
        let bytes = std::fs::read(path).ok()?;
        let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let hb = rustybuzz::Face::from_slice(leaked, best_index.max(0) as u32)?;
        let upem = (hb.units_per_em().max(16)) as u16;
        let ascender = hb.ascender();
        let descender = hb.descender();
        Some(FaceEntry {
            family: family.to_string(),
            ft,
            hb,
            upem,
            ascender,
            descender,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    fn face_for_char(&mut self, c: char) -> usize {
        if let Some(&i) = self.fallback_for.get(&c) {
            return i;
        }
        let mut idx = 0; // primary UI face is the default
        for (i, f) in self.faces.iter().enumerate() {
            if f.hb.glyph_index(c).is_some() {
                idx = i;
                break;
            }
        }
        self.fallback_for.insert(c, idx);
        idx
    }

    fn glyph(&mut self, face_idx: usize, size: u32, gid: u32, bold: bool) -> Arc<CachedGlyph> {
        let key = (face_idx, size, gid, bold);
        if let Some(g) = self.cache.get(&key) {
            return g.clone();
        }
        let face = &mut self.faces[face_idx];
        let _ = face.ft.set_pixel_sizes(0, size);
        let _ = face.ft.load_glyph(gid, LoadFlag::RENDER | LoadFlag::TARGET_NORMAL);
        let slot = face.ft.glyph();
        let bm = slot.bitmap();
        let (bw, bh) = (bm.width() as i32, bm.rows() as i32);
        let src: Vec<u8> = bm.buffer().to_vec();
        let n = (bw * bh).max(0) as usize;
        let mut bitmap = src;
        bitmap.truncate(n);
        bitmap.resize(n, 0);
        let g = Arc::new(CachedGlyph {
            w: bw,
            h: bh,
            left: slot.bitmap_left(),
            top: slot.bitmap_top(),
            advance: slot.advance().x as i32,
            bitmap,
        });
        self.cache.insert(key, g.clone());
        g
    }

    /// Split text into runs of (face_index, &str) by per-char coverage.
    fn runs<'a>(&mut self, text: &'a str) -> Vec<(usize, &'a str)> {
        let mut out = Vec::new();
        let mut cur: Option<usize> = None;
        let mut start = 0usize;
        for (i, c) in text.char_indices() {
            let f = self.face_for_char(c);
            match cur {
                Some(f0) if f0 == f => {}
                _ => {
                    if let Some(f0) = cur {
                        out.push((f0, &text[start..i]));
                    }
                    cur = Some(f);
                    start = i;
                }
            }
        }
        if let Some(f0) = cur {
            out.push((f0, &text[start..]));
        }
        out
    }

    /// Shape a run and return (glyph_id, cluster_offset, x_offset, y_offset, x_advance) in pixels.
    #[allow(clippy::type_complexity)]
    fn shape_run(&mut self, face_idx: usize, size: u32, text: &str, base_cluster: usize)
        -> Vec<(u32, usize, i32, i32, i32)>
    {
        let face = &self.faces[face_idx];
        let scale = size as f32 / face.upem as f32;
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(text);
        let shaped = rustybuzz::shape(&face.hb, &[], buf);
        let infos = shaped.glyph_infos();
        let poss = shaped.glyph_positions();
        let mut out = Vec::with_capacity(infos.len());
        for (info, pos) in infos.iter().zip(poss.iter()) {
            out.push((
                info.glyph_id as u32,
                base_cluster + info.cluster as usize,
                (pos.x_offset as f32 * scale) as i32,
                (pos.y_offset as f32 * scale) as i32,
                (pos.x_advance as f32 * scale) as i32,
            ));
        }
        out
    }

    /// Measure the pixel width of a string.
    pub fn measure(&mut self, size: u32, text: &str, bold: bool) -> f32 {
        let mut w = 0f32;
        for (fi, run) in self.runs(text) {
            for (_gid, _c, _xo, _yo, adv) in self.shape_run(fi, size, run, 0) {
                w += adv as f32;
            }
            if bold && !run.is_empty() {
                w += 1.0;
            }
        }
        w
    }

    /// Pixel ascent of the primary face at `size`.
    pub fn ascent(&self, size: u32) -> i32 {
        match self.faces.first() {
            Some(f) => ((f.ascender as f32) * size as f32 / f.upem as f32).ceil() as i32,
            None => size as i32,
        }
    }

    pub fn descent(&self, size: u32) -> i32 {
        match self.faces.first() {
            Some(f) => ((-f.descender as f32) * size as f32 / f.upem as f32).ceil() as i32,
            None => (size as i32) / 4,
        }
    }

    /// Draw a string with its baseline at (x, y). Returns final x.
    pub fn draw_baseline(
        &mut self,
        cv: &mut crate::canvas::Canvas,
        x: f32,
        y: f32,
        size: u32,
        color: [u8; 4],
        bold: bool,
        text: &str,
    ) -> f32 {
        let mut pen = x;
        for (fi, run) in self.runs(text) {
            let shaped = self.shape_run(fi, size, run, 0);
            for (gid, _cluster, xo, yo, adv) in shaped {
                let g = self.glyph(fi, size, gid, false);
                let px = (pen + xo as f32).floor() as i32;
                let py = (y - yo as f32).floor() as i32;
                let extra = if bold { 1 } else { 0 };
                for b in 0..=extra {
                    cv.blit_alpha(
                        &g.bitmap, g.w, g.h,
                        px + g.left + b, py - g.top, color,
                    );
                }
                pen += adv as f32;
            }
            if bold {
                pen += 1.0;
            }
        }
        pen
    }

    /// Draw a string whose top-left box starts at (x, y).
    pub fn draw(
        &mut self,
        cv: &mut crate::canvas::Canvas,
        x: f32,
        y: f32,
        size: u32,
        color: [u8; 4],
        bold: bool,
        text: &str,
    ) -> f32 {
        self.draw_baseline(cv, x, y + self.ascent(size) as f32, size, color, bold, text)
    }

    /// Draw a string centered in a box.
    pub fn draw_centered(
        &mut self,
        cv: &mut crate::canvas::Canvas,
        box_: crate::geom::Rect,
        size: u32,
        color: [u8; 4],
        bold: bool,
        text: &str,
    ) {
        let w = self.measure(size, text, bold);
        let a = self.ascent(size) as f32;
        let d = self.descent(size) as f32;
        let bx = box_.x as f32 + (box_.w as f32 - w) / 2.0;
        let by = box_.y as f32 + (box_.h as f32 - (a + d)) / 2.0;
        self.draw_baseline(cv, bx, by + a, size, color, bold, text);
    }

    /// Ellipsize text to fit `max_w`, appending "…".
    pub fn ellipsize(&mut self, size: u32, text: &str, bold: bool, max_w: f32) -> String {
        if self.measure(size, text, bold) <= max_w {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        let mut lo = 0usize;
        let mut hi = chars.len();
        while lo < hi {
            let mid = (lo + hi + 1) / 2;
            let s: String = chars[..mid].iter().collect();
            if self.measure(size, &(s + "…"), bold) <= max_w {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let mut s: String = chars[..lo].iter().collect();
        s.push('…');
        s
    }
}
