//! BraveOS user configuration (~/.config/bravos/config.json).

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::json;

#[derive(Clone)]
pub struct Config {
    pub language: String,       // "auto" or BCP47-ish code e.g. "de", "zh-CN"
    pub accent: String,         // hex "#f04e23"
    pub wallpaper: String,      // file name in /usr/share/bravos/wallpapers
    pub dark: bool,
    pub layouts: Vec<String>,   // active XKB layout codes (first = default)
    pub repeat_delay_ms: u32,
    pub repeat_rate_ms: u32,
    pub mouse_speed: f64,       // -1.0 .. 1.0
    pub natural_scroll: bool,
    pub left_handed: bool,
    pub blank_minutes: u32,     // screen blank, 0 = never
    pub workspaces: u32,
    pub shortcuts: HashMap<String, String>, // action -> combo e.g. "super+enter"
    pub region: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut sh = HashMap::new();
        sh.insert("terminal".into(), "super+enter".into());
        sh.insert("browser".into(), "super+b".into());
        sh.insert("files".into(), "super+e".into());
        sh.insert("settings".into(), "super+s".into());
        sh.insert("close".into(), "alt+f4".into());
        sh.insert("switcher".into(), "alt+tab".into());
        sh.insert("switcher_back".into(), "alt+shift+tab".into());
        sh.insert("workspace_next".into(), "super+right".into());
        sh.insert("workspace_prev".into(), "super+left".into());
        sh.insert("show_desktop".into(), "super+d".into());
        sh.insert("maximize".into(), "super+up".into());
        sh.insert("restore".into(), "super+down".into());
        sh.insert("fullscreen".into(), "super+f".into());
        sh.insert("snap_left".into(), "super+shift+left".into());
        sh.insert("snap_right".into(), "super+shift+right".into());
        sh.insert("lock".into(), "super+l".into());
        sh.insert("screenshot".into(), "print".into());
        sh.insert("app_grid".into(), "super+a".into());
        Config {
            language: "auto".into(),
            accent: "#f04e23".into(),
            wallpaper: "braveos.png".into(),
            dark: true,
            layouts: vec!["us".into()],
            repeat_delay_ms: 300,
            repeat_rate_ms: 40,
            mouse_speed: 0.0,
            natural_scroll: false,
            left_handed: false,
            blank_minutes: 5,
            workspaces: 2,
            shortcuts: sh,
            region: "auto".into(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".config/bravos/config.json")
    }

    pub fn load() -> Config {
        let mut cfg = Config::default();
        if let Ok(s) = std::fs::read_to_string(Self::path()) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                cfg.apply_json(&v);
            }
        }
        cfg
    }

    pub fn apply_json(&mut self, v: &serde_json::Value) {
        if let Some(x) = v["language"].as_str() { self.language = x.into(); }
        if let Some(x) = v["accent"].as_str() { self.accent = x.into(); }
        if let Some(x) = v["wallpaper"].as_str() { self.wallpaper = x.into(); }
        if let Some(x) = v["dark"].as_bool() { self.dark = x; }
        if let Some(x) = v["region"].as_str() { self.region = x.into(); }
        if let Some(a) = v["layouts"].as_array() {
            let ls: Vec<String> = a.iter().filter_map(|x| x.as_str().map(String::from)).collect();
            if !ls.is_empty() { self.layouts = ls; }
        }
        if let Some(x) = v["repeat_delay_ms"].as_u64() { self.repeat_delay_ms = x as u32; }
        if let Some(x) = v["repeat_rate_ms"].as_u64() { self.repeat_rate_ms = x as u32; }
        if let Some(x) = v["mouse_speed"].as_f64() { self.mouse_speed = x; }
        if let Some(x) = v["natural_scroll"].as_bool() { self.natural_scroll = x; }
        if let Some(x) = v["left_handed"].as_bool() { self.left_handed = x; }
        if let Some(x) = v["blank_minutes"].as_u64() { self.blank_minutes = x as u32; }
        if let Some(x) = v["workspaces"].as_u64() { self.workspaces = (x as u32).clamp(1, 9); }
        if let Some(m) = v["shortcuts"].as_object() {
            for (k, val) in m {
                if let Some(s) = val.as_str() {
                    self.shortcuts.insert(k.clone(), s.to_string());
                }
            }
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "language": self.language,
            "region": self.region,
            "accent": self.accent,
            "wallpaper": self.wallpaper,
            "dark": self.dark,
            "layouts": self.layouts,
            "repeat_delay_ms": self.repeat_delay_ms,
            "repeat_rate_ms": self.repeat_rate_ms,
            "mouse_speed": self.mouse_speed,
            "natural_scroll": self.natural_scroll,
            "left_handed": self.left_handed,
            "blank_minutes": self.blank_minutes,
            "workspaces": self.workspaces,
            "shortcuts": self.shortcuts,
        })
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&p, serde_json::to_string_pretty(&self.to_json()).unwrap_or_default());
        // Notify every BraveOS app (shell, wm, settings) that config changed.
        // We signal via SIGUSR1-style: simplest robust IPC is touching a file
        // everyone polls on tick.
        let _ = std::fs::write(PathBuf::from("/tmp/.bravos-config-stamp"), "now");
    }

    pub fn hex_rgb(hex: &str) -> [u8; 3] {
        let h = hex.trim_start_matches('#');
        if h.len() >= 6 {
            let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(240);
            let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(78);
            let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(35);
            [r, g, b]
        } else {
            [240, 78, 35]
        }
    }
}
