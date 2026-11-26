//! Font handling: filesystem discovery, fontdue rasterization, rustybuzz
//! shaping, per-character fallback across installed script families.
//! Pure Rust — no C dependencies (no pkg-config / freetype / fontconfig).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use fontdue::Font;
use fontdue::FontSettings;

pub struct CachedGlyph {
    pub w: i32,
    pub h: i32,
    pub left: i32,
    pub top: i32,
    pub advance: i32,
    pub bitmap: Vec<u8>, // 8-bit coverage, w*h
}

struct FaceEntry {
    family: String,
    font: Font,
    hb: rustybuzz::Face<'static>,
    upem: u16,
    ascender: i16,
    descender: i16,
}

/// A discovered-but-maybe-not-loaded face (huge CJK files load lazily).
#[derive(Clone)]
struct FaceSlot {
    family: String,
    path: std::path::PathBuf,
    index: u32,
    big: bool,
}

pub struct Fonts {
    slots: RefCell<Option<Vec<FaceSlot>>>, // scanned on first use
    faces: RefCell<Vec<Option<FaceEntry>>>, // loaded faces, index == slot position
    fallback_for: RefCell<HashMap<char, usize>>,
    cache: RefCell<HashMap<(usize, u32, u32, bool), Arc<CachedGlyph>>>,
}

/// Preference order for the UI font stack. Everything else is appended.
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
    "Liberation Sans",
    "WenQuanYi Zen Hei",
];

fn collect_font_files(dir: &str, out: &mut Vec<std::path::PathBuf>, depth: i32) {
    if depth > 4 || out.len() > 400 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_font_files(path.to_str().unwrap_or(""), out, depth + 1);
        } else {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if matches!(ext.as_str(), "ttf" | "otf" | "ttc") {
                out.push(path);
            }
        }
    }
}

fn font_files() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(d) = std::env::var("BRAVOS_FONTS_DIR") {
        dirs.push(d);
    }
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(format!("{}/.fonts", home));
    }
    dirs.push("/usr/local/share/fonts".into());
    dirs.push("/usr/share/fonts".into());
    let mut out = Vec::new();
    for d in dirs {
        collect_font_files(&d, &mut out, 0);
    }
    out
}

/// Number of faces inside a font file (1 for ttf/otf, N for .ttc).
fn face_count(bytes: &[u8]) -> usize {
    if bytes.len() > 12 && &bytes[0..4] == b"ttcf" {
        u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize
    } else {
        1
    }
}

fn read_family_name(bytes: &[u8], index: u32) -> Option<String> {
    let face = rustybuzz::Face::from_slice(bytes, index)?;
    // name_id 16 = typographic family, 1 = family. (2 is the subfamily
    // "Regular"/"Bold" — NOT the family.)
    let mut typographic = None;
    let mut family = None;
    for name in face.names() {
        if !name.is_unicode() {
            continue;
        }
        match name.name_id {
            16 => typographic = name.to_string(),
            1 => family = name.to_string(),
            _ => {}
        }
    }
    typographic.or(family)
}

impl Fonts {
    pub fn new() -> Fonts {
        Fonts {
            slots: RefCell::new(None),
            faces: RefCell::new(Vec::new()),
            fallback_for: RefCell::new(HashMap::new()),
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Discover installed fonts on first use (cheap for apps that never
    /// draw text, e.g. the wallpaper tool).
    fn ensure_scan(&self) {
        if self.slots.borrow().is_some() {
            return;
        }

        let mut found: Vec<(String, std::path::PathBuf, u32, bool)> = Vec::new();
        for path in font_files() {
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let big = bytes.len() > 12_000_000;
            let n = face_count(&bytes);
            for i in 0..n.min(8) {
                if let Some(fam) = read_family_name(&bytes, i as u32) {
                    found.push((fam, path.clone(), i as u32, big));
                    break;
                }
            }
        }
        // order slots by FAMILY_ORDER preference, one face per family
        let mut slots: Vec<FaceSlot> = Vec::new();
        let mut take = |faces: &mut Vec<FaceSlot>, fam: &str| {
            if faces.iter().any(|f| f.family == fam) {
                return;
            }
            if let Some((_, path, idx, big)) =
                found.iter().find(|(f, _, _, _)| f == fam)
            {
                faces.push(FaceSlot {
                    family: fam.to_string(),
                    path: path.clone(),
                    index: *idx,
                    big: *big,
                });
            }
        };
        for pref in FAMILY_ORDER {
            take(&mut slots, pref);
        }
        for (fam, _, _, _) in found.iter() {
            if !slots.iter().any(|f| &f.family == fam) {
                take(&mut slots, fam);
            }
        }
        let mut loaded = Vec::with_capacity(slots.len());
        for _slot in slots.iter() {
            // ALL faces load lazily on first use: eagerly loading (and
            // leaking) every font file here cost the shell ~300 MB RSS on
            // a fonts-noto-core system. face_for_char() loads a face the
            // first time a character actually needs it.
            loaded.push(None);
        }
        *self.faces.borrow_mut() = loaded;
        *self.slots.borrow_mut() = Some(slots);
    }

    fn load_slot(slot: &FaceSlot) -> Option<FaceEntry> {
        let bytes = std::fs::read(&slot.path).ok()?;
        let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let hb = rustybuzz::Face::from_slice(leaked, slot.index)?;
        let upem = (hb.units_per_em().max(16)) as u16;
        let ascender = hb.ascender();
        let descender = hb.descender();
        let font = Font::from_bytes(
            std::borrow::Cow::Borrowed(leaked),
            FontSettings { collection_index: slot.index, scale: 40.0, load_substitutions: false },
        )
        .ok()?;
        Some(FaceEntry {
            family: slot.family.clone(),
            font,
            hb,
            upem,
            ascender,
            descender,
        })
    }

    /// Debug: family + glyph id + rasterize metrics for a char.
    pub fn glyph_debug(&self, c: char, size: u32) -> String {
        let fi = self.face_for_char(c);
        let faces = self.faces.borrow();
        let f = faces[fi].as_ref().unwrap();
        let gid = f.hb.glyph_index(c);
        let gid_v = gid.map(|g| g.0).unwrap_or(u16::MAX);
        let raster = if gid_v != u16::MAX {
            let (m, cov) = f.font.rasterize_indexed(gid_v as u16, size as f32);
            format!("gid={} box={}x{} adv={:.1} cov_len={} lit={}",
                gid_v, m.width, m.height, m.advance_width,
                cov.len(), cov.iter().filter(|&&v| v > 40).count())
        } else {
            "no glyph".into()
        };
        format!("face{} [{}] upem={} '{}' {}", fi, f.family, f.upem, c, raster)
    }

    fn face_for_char(&self, c: char) -> usize {
        self.ensure_scan();
        if let Some(&i) = self.fallback_for.borrow().get(&c) {
            return i;
        }
        // already-loaded faces first
        {
            let faces = self.faces.borrow();
            for (i, f) in faces.iter().enumerate() {
                if let Some(f) = f {
                    if f.hb.glyph_index(c).is_some() {
                        self.fallback_for.borrow_mut().insert(c, i);
                        return i;
                    }
                }
            }
        }
        // lazy-load deferred (huge) faces until one covers the char
        let n_slots = self.slots.borrow().as_ref().map(|v| v.len()).unwrap_or(0);
        for i in 0..n_slots {
            let loaded = self.faces.borrow()[i].is_some();
            if loaded {
                continue;
            }
            let slot = self.slots.borrow().as_ref().unwrap()[i].clone();
            if let Some(entry) = Self::load_slot(&slot) {
                let covers = entry.hb.glyph_index(c).is_some();
                self.faces.borrow_mut()[i] = Some(entry);
                if covers {
                    self.fallback_for.borrow_mut().insert(c, i);
                    return i;
                }
            }
        }
        self.fallback_for.borrow_mut().insert(c, 0);
        0
    }

    fn glyph(&self, face_idx: usize, size: u32, gid: u32, bold: bool) -> Arc<CachedGlyph> {
        let key = (face_idx, size, gid, bold);
        if let Some(g) = self.cache.borrow().get(&key) {
            return g.clone();
        }
        let faces = self.faces.borrow();
        let face = faces[face_idx].as_ref().expect("face loaded");
        let (m, cov) = face.font.rasterize_indexed(gid as u16, size as f32);
        let adv = m.advance_width.round() as i32;
        let g = Arc::new(CachedGlyph {
            w: m.width as i32,
            h: m.height as i32,
            left: m.xmin as i32,
            top: m.ymin as i32, // bottom edge relative to baseline (up positive)
            advance: adv,
            bitmap: cov,
        });
        self.cache.borrow_mut().insert(key, g.clone());
        g
    }

    fn runs<'a>(&self, text: &'a str) -> Vec<(usize, &'a str)> {
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

    #[allow(clippy::type_complexity)]
    fn shape_run(&self, face_idx: usize, size: u32, text: &str, base_cluster: usize)
        -> Vec<(u32, usize, i32, i32, i32)>
    {
        let faces = self.faces.borrow();
        let face = faces[face_idx].as_ref().expect("face loaded");
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

    pub fn measure(&self, size: u32, text: &str, bold: bool) -> f32 {
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

    pub fn ascent(&self, size: u32) -> i32 {
        let faces = self.faces.borrow();
        match faces.iter().flatten().next() {
            Some(f) => ((f.ascender as f32) * size as f32 / f.upem as f32).ceil() as i32,
            None => size as i32,
        }
    }

    pub fn descent(&self, size: u32) -> i32 {
        let faces = self.faces.borrow();
        match faces.iter().flatten().next() {
            Some(f) => ((-f.descender as f32) * size as f32 / f.upem as f32).ceil() as i32,
            None => (size as i32) / 4,
        }
    }

    pub fn draw_baseline(
        &self,
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
                let baseline = (y - yo as f32).floor() as i32;
                // fontdue: ymin = bottom edge vs baseline (up positive)
                let top = baseline - (g.top + g.h);
                let extra = if bold { 1 } else { 0 };
                for b in 0..=extra {
                    cv.blit_alpha(&g.bitmap, g.w, g.h, px + g.left + b, top, color);
                }
                pen += adv as f32;
            }
            if bold {
                pen += 1.0;
            }
        }
        pen
    }

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

    pub fn ellipsize(&self, size: u32, text: &str, bold: bool, max_w: f32) -> String {
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
