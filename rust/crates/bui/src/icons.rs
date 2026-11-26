//! Icon loading: PNG icons from /usr/share/bravos/icons (+ hicolor fallbacks).

use std::collections::HashMap;

use crate::pngutil::Image;

pub struct IconStore {
    cache: HashMap<(String, i32, i32), Image>,
    dirs: Vec<String>,
}

impl IconStore {
    pub fn new() -> IconStore {
        let mut dirs = Vec::new();
        if let Ok(d) = std::env::var("BRAVOS_ICONS_DIR") {
            dirs.push(d);
        }
        dirs.extend([
            "/usr/share/bravos/icons".to_string(),
            "/usr/local/share/icons/hicolor/48x48/apps".to_string(),
            "/usr/share/icons/hicolor/48x48/apps".to_string(),
        ]);
        IconStore { cache: HashMap::new(), dirs }
    }

    /// Load icon by name, scaled to w x h. Names map to `icon-<name>.png`
    /// (BraveOS asset set) or `<name>.png` (generic).
    pub fn get(&mut self, name: &str, w: i32, h: i32) -> Option<Image> {
        let key = (name.to_string(), w, h);
        if let Some(img) = self.cache.get(&key) {
            return Some(img.clone());
        }
        for dir in &self.dirs {
            for cand in [format!("{}/icon-{}.png", dir, name), format!("{}/{}.png", dir, name)] {
                if let Some(img) = Image::decode(&cand) {
                    let scaled = if img.w == w && img.h == h {
                        img
                    } else {
                        img.resized(w, h)
                    };
                    self.cache.insert(key, scaled.clone());
                    return Some(scaled);
                }
            }
        }
        None
    }
}
