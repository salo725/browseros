//! bsettings — BraveOS Settings: custom 2026-style control center (pure Rust).
//!
//! Sections: About, Appearance, Display, Network, Sound, Keyboard, Language,
//! Mouse, Power, Shortcuts, Time & Date. Every control is functional and
//! backed by real system tools (xrandr, nmcli, pactl, xinput, xset, setxkbmap,
//! timedatectl). `bsettings <section>` opens a specific page.

use std::cell::RefCell;
use std::rc::Rc;

use bui::canvas::Canvas;
use bui::config::Config;
use bui::geom::Rect;
use bui::i18n::Tr;
use bui::xapp::{App, Handler, PtrKind, WinOpts};

const WIN_W: i32 = 880;
const WIN_H: i32 = 560;
const SIDEBAR_W: i32 = 220;
const ROW_H: i32 = 56;

fn sh(cmd: &str) -> String {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

fn spawn(args: &[&str]) {
    let _ = std::process::Command::new(args[0]).args(&args[1..]).spawn();
}

// ---------------------------------------------------------------- widgets

#[derive(Clone, Debug)]
enum Ctl {
    None,
    Btn(&'static str),
    Toggle(&'static str),
    Slider(&'static str),
    Drop(&'static str),
    DropItem(&'static str, usize),
    Item(usize),
    ShortcutRow(usize),
}

struct State {
    section: usize,
    hits: Vec<(Rect, Ctl)>,
    drop_open: Option<&'static str>,
    capture: Option<usize>,
    password: String,
    password_focus: bool,
    wifi_sel: Option<usize>,
    wifi_nets: Vec<(String, u32, bool)>,
    resolutions: Vec<String>,
    res_sel: usize,
    sinks: Vec<String>,
    sink_sel: usize,
    timezones: Vec<String>,
    tz_sel: usize,
    volume: i32,
    layouts: Vec<(&'static str, &'static str)>, // (code, native name)
    shortcut_actions: Vec<(&'static str, &'static str)>, // (key, label i18n key)
    toast: String,
    toast_until: std::time::Instant,
}

struct SettingsApp {
    st: RefCell<State>,
}

impl SettingsApp {
    fn new(app: &App) -> Rc<SettingsApp> {
        // keyboard layouts catalogue
        let layouts: Vec<(&'static str, &'static str)> = vec![
            ("us", "English (US)"), ("uk", "English (UK)"), ("de", "Deutsch"),
            ("fr", "Français"), ("es", "Español"), ("it", "Italiano"),
            ("pt", "Português"), ("br", "Português (BR)"), ("nl", "Nederlands"),
            ("se", "Svenska"), ("no", "Norsk"), ("dk", "Dansk"), ("fi", "Suomi"),
            ("pl", "Polski"), ("cz", "Čeština"), ("sk", "Slovenčina"),
            ("hu", "Magyar"), ("ro", "Română"), ("bg", "Български"),
            ("gr", "Ελληνικά"), ("tr", "Türkçe"), ("ru", "Русский"),
            ("ua", "Українська"), ("by", "Беларуская"), ("rs", "Српски"),
            ("hr", "Hrvatski"), ("si", "Slovenščina"), ("il", "עברית"),
            ("ir", "فارسی"), ("ara", "العربية"), ("pk", "اردو"),
            ("in", "हिन्दी"), ("bd", "বাংলা"), ("lk", "සිංහල"),
            ("th", "ไทย"), ("vn", "Tiếng Việt"), ("id", "Indonesia"),
            ("my", "Melayu"), ("cn", "中文 (拼音)"), ("jp", "日本語"),
            ("kr", "한국어"), ("mn", "Монгол"), ("kz", "Қазақ"),
            ("ge", "ქართული"), ("am", "Հայերեն"), ("az", "Azərbaycan"),
            ("et", "Eesti"), ("lv", "Latviešu"), ("lt", "Lietuvių"),
            ("is", "Íslenska"), ("ie", "Gaeilge"), ("ma", "Marocain"),
            ("epo", "Esperanto"), ("latam", "Latinoamérica"), ("np", "नेपाली"),
        ];
        let shortcut_actions: Vec<(&'static str, &'static str)> = vec![
            ("terminal", "sc_terminal"), ("browser", "sc_browser"),
            ("files", "sc_files"), ("settings", "sc_settings"),
            ("close", "sc_close"), ("switcher", "sc_switch"),
            ("workspace_next", "sc_ws_next"), ("workspace_prev", "sc_ws_prev"),
            ("show_desktop", "sc_show_desktop"), ("maximize", "sc_max"),
            ("fullscreen", "sc_full"), ("snap_left", "sc_snap_l"),
            ("snap_right", "sc_snap_r"), ("lock", "sc_lock"),
            ("screenshot", "sc_shot"), ("app_grid", "sc_grid"),
        ];
        let st = State {
            section: 0,
            hits: Vec::new(),
            drop_open: None,
            capture: None,
            password: String::new(),
            password_focus: false,
            wifi_sel: None,
            wifi_nets: Vec::new(),
            resolutions: Vec::new(),
            res_sel: 0,
            sinks: Vec::new(),
            sink_sel: 0,
            timezones: Vec::new(),
            tz_sel: 0,
            volume: sh("pactl get-sink-volume @DEFAULT_SINK@ 2>/dev/null | head -1")
                .split('/')
                .nth(1)
                .map(|s| s.trim().trim_end_matches('%').trim().to_string())
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            layouts,
            shortcut_actions,
            toast: String::new(),
            toast_until: std::time::Instant::now(),
        };
        Rc::new(SettingsApp { st: RefCell::new(st) })
    }
}

// ------------------------------------------------------------ helpers

fn vol_set(v: i32) {
    let v = v.clamp(0, 150);
    sh(&format!("pactl set-sink-volume @DEFAULT_SINK@ {}%", v));
}

fn apply_layouts(cfg: &Config) {
    let ls = cfg.layouts.join(",");
    sh(&format!(
        "setxkbmap -layout {} -option grp:alt_shift_toggle -option grp_led:scroll",
        ls
    ));
    sh(&format!(
        "xset r rate {} {}",
        cfg.repeat_delay_ms, 1000 / cfg.repeat_rate_ms.max(1)
    ));
}

fn apply_mouse(cfg: &Config) {
    let speed = cfg.mouse_speed.clamp(-1.0, 1.0);
    sh(&format!(
        "for id in $(xinput list --name-only | grep -i -E 'pointer|touchpad' | head -3); do \
          prop=$(xinput list-props \"$id\" 2>/dev/null | grep 'libinput Accel Speed' | sed 's/.*(\\(.*\\)).*/\\1/'); \
          [ -n \"$prop\" ] && xinput set-prop \"$id\" \"$prop\" {}; done",
        speed
    ));
    let nat = if cfg.natural_scroll { 1 } else { 0 };
    sh(&format!(
        "for id in $(xinput list --name-only | grep -i -E 'pointer|touchpad' | head -3); do \
          prop=$(xinput list-props \"$id\" 2>/dev/null | grep 'libinput Natural Scrolling Enabled' | sed 's/.*(\\(.*\\)).*/\\1/'); \
          [ -n \"$prop\" ] && xinput set-prop \"$id\" \"$prop\" {}; done",
        nat
    ));
    if cfg.left_handed {
        sh("xmodmap -e 'pointer = 3 2 1' 2>/dev/null || true");
    } else {
        sh("xmodmap -e 'pointer = 1 2 3' 2>/dev/null || true");
    }
}

fn apply_blank(mins: u32) {
    if mins == 0 {
        sh("xset s off; xset -dpms");
    } else {
        let secs = mins * 60;
        sh(&format!("xset s on; xset s {} {}; xset +dpms; xset dpms {} {} {}",
            secs, secs, secs, secs * 2, secs * 3));
    }
}

fn apply_theme(dark: bool) {
    let gtk = if dark { "BraveOS" } else { "BraveOS-light" };
    sh(&format!(
        "sed -i 's/gtk-theme-name=.*/gtk-theme-name={}/' ~/.config/gtk-3.0/settings.ini 2>/dev/null; \
         mkdir -p ~/.config/gtk-3.0; \
         grep -q gtk-theme-name ~/.config/gtk-3.0/settings.ini 2>/dev/null || \
         echo 'gtk-theme-name={}' >> ~/.config/gtk-3.0/settings.ini",
        gtk, gtk
    ));
}

// sections list: (i18n key, icon name)
const SECTIONS: &[(&str, &str)] = &[
    ("sec_about", "about"),
    ("sec_appearance", "appearance"),
    ("sec_display", "display"),
    ("sec_network", "network"),
    ("sec_sound", "sound"),
    ("sec_keyboard", "keyboard"),
    ("sec_language", "language"),
    ("sec_mouse", "mouse"),
    ("sec_power", "power"),
    ("sec_shortcuts", "shortcuts"),
    ("sec_time", "time"),
];

impl SettingsApp {
    fn toast(&self, st: &mut State, msg: &str) {
        st.toast = msg.to_string();
        st.toast_until = std::time::Instant::now() + std::time::Duration::from_secs(3);
    }
}

impl SettingsApp {
    fn draw(&self, app: &mut App, cv: &mut Canvas) {
        let mut st = self.st.borrow_mut();
        st.hits.clear();
        let acc = app.accent;
        let comp = app.composited;
        cv.fill([0, 0, 0, 0]);
        let a = if comp { 246u8 } else { 255u8 };
        cv.rounded(0, 0, cv.w, cv.h, 16, [13, 13, 20, a]);
        cv.rounded_outline(0, 0, cv.w, cv.h, 16, [255, 255, 255, 26]);

        let rtl = app.tr.rtl;

        // ---- sidebar
        let sbx = if rtl { cv.w - SIDEBAR_W } else { 0 };
        let sb = Rect::new(sbx, 0, SIDEBAR_W, cv.h);
        cv.rect(sb.x, 0, sb.w, cv.h, [9, 9, 15, 200]);
        cv.rect(if rtl { sb.x } else { sb.x + sb.w - 1 }, 0, 1, cv.h, [255, 255, 255, 24]);
        let title = Rect::new(sb.x + 16, 14, sb.w - 32, 40);
        app.fonts.draw_centered(cv, Rect::new(sb.x + 16, 20, sb.w - 32, 22), 16, [240, 240, 245, 255], true, "BraveOS");
        app.fonts.draw_centered(cv, Rect::new(sb.x + 16, 44, sb.w - 32, 16), 10, [150, 150, 165, 255], false, &app.tr.t("settings_title"));
        let _ = title;
        for (i, (key, icon)) in SECTIONS.iter().enumerate() {
            let y = 74 + i as i32 * 40;
            let r = Rect::new(sb.x + 12, y, sb.w - 24, 34);
            let sel = i == st.section;
            if sel {
                cv.rounded(r.x, r.y, r.w, r.h, 10, [acc[0], acc[1], acc[2], 80]);
                cv.rounded_outline(r.x, r.y, r.w, r.h, 10, [acc[0], acc[1], acc[2], 160]);
            }
            let (ix, tx) = if rtl {
                (r.x + r.w - 30, r.x + 12)
            } else {
                (r.x + 12, r.x + 40)
            };
            let tw: [u8; 4] = if sel { [240, 240, 245, 255] } else { [205, 205, 215, 255] };
            if let Some(ic) = app.icons.get(icon, 20, 20) {
                cv.image(&ic, ix, r.y + 7);
            }
            app.fonts.draw(cv, tx as f32, (r.y + 9) as f32, 13, tw, sel, &app.tr.t(key));
            st.hits.push((r, Ctl::Item(i)));
        }

        // ---- content area
        let cx = if rtl { 0 } else { SIDEBAR_W };
        let cw = cv.w - SIDEBAR_W;
        let content = Rect::new(cx, 0, cw, cv.h);
        let inner = Rect::new(cx + 28, 18, cw - 56, cv.h - 36);
        let sec = st.section;
        match sec {
            0 => self.page_about(app, &mut st, cv, content),
            1 => self.page_appearance(app, &mut st, cv, inner),
            2 => self.page_display(app, &mut st, cv, inner),
            3 => self.page_network(app, &mut st, cv, inner),
            4 => self.page_sound(app, &mut st, cv, inner),
            5 => self.page_keyboard(app, &mut st, cv, inner),
            6 => self.page_language(app, &mut st, cv, inner),
            7 => self.page_mouse(app, &mut st, cv, inner),
            8 => self.page_power(app, &mut st, cv, inner),
            9 => self.page_shortcuts(app, &mut st, cv, inner),
            10 => self.page_time(app, &mut st, cv, inner),
            _ => {}
        }

        // toast
        if !st.toast.is_empty() && std::time::Instant::now() < st.toast_until {
            let tw = (app.fonts.measure(13, &st.toast, false) + 40.0) as i32;
            let r = Rect::new((cv.w - tw) / 2, cv.h - 46, tw, 32);
            cv.rounded(r.x, r.y, r.w, r.h, 16, [acc[0], acc[1], acc[2], 230]);
            app.fonts.draw_centered(cv, r, 13, [255, 255, 255, 255], false, &st.toast);
        }
    }

    fn pointer(&self, app: &mut App, kind: PtrKind, x: i32, y: i32) {
        match kind {
            PtrKind::Down(_) => {
                let mut st = self.st.borrow_mut();
                let hit = st.hits.iter().find(|(r, _)| r.contains(x, y)).cloned();
                if st.drop_open.is_some() && hit.is_none() {
                    st.drop_open = None;
                    return;
                }
                if let Some((_, ctl)) = hit {
                    self.click(app, &mut st, ctl);
                } else {
                    st.password_focus = false;
                }
            }
            PtrKind::Up(_) => {}
            PtrKind::Move => {
                // slider dragging
                let mut st = self.st.borrow_mut();
                let hits_clone: Vec<(Rect, Ctl)> = st.hits.clone();
                for (r, ctl) in &hits_clone {
                    if let Ctl::Slider(which) = ctl {
                        if r.contains(x, y) {
                            let t = ((x - r.x) as f32 / r.w as f32).clamp(0.0, 1.0);
                            match *which {
                                "volume" => {
                                    let v = (t * 150.0) as i32;
                                    if v != st.volume {
                                        st.volume = v;
                                        vol_set(v);
                                    }
                                }
                                "repeat_delay" => {
                                    app.cfg.repeat_delay_ms = (100 + t as i32 * 900) as u32;
                                }
                                "repeat_rate" => {
                                    app.cfg.repeat_rate_ms = (10.0 + t * 90.0) as u32;
                                }
                                "mouse_speed" => {
                                    app.cfg.mouse_speed = (t * 2.0 - 1.0) as f64;
                                    apply_mouse(&app.cfg);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            PtrKind::Enter | PtrKind::Leave => {}
        }
    }

    fn key(&self, app: &mut App, ks: u32, state: u16) -> bool {
        let mut st = self.st.borrow_mut();
        // shortcut capture mode
        if let Some(idx) = st.capture {
            if ks != 0xff7f && ks != 0xffe1 && ks != 0xffe3 && ks != 0xffe9 && ks != 0xffea
                && ks != 0xffe5 && ks != 0xffe4
            {
                // ignore pure modifier presses
                let mut combo = String::new();
                if state & 64 != 0 { combo.push_str("super+"); }
                if state & 8 != 0 { combo.push_str("alt+"); }
                if state & 4 != 0 { combo.push_str("ctrl+"); }
                if state & 1 != 0 { combo.push_str("shift+"); }
                combo.push_str(&keysym_name(ks).to_lowercase());
                let action = st.shortcut_actions[idx].0;
                app.cfg.shortcuts.insert(action.to_string(), combo.clone());
                app.cfg.save();
                st.capture = None;
                let label_key = st.shortcut_actions[idx].1.to_string();
                self.toast(&mut st, &format!("{} → {}", app.tr.t(&label_key), combo));
                return true;
            }
            return true;
        }
        // password input
        if st.password_focus {
            if ks == 0xff08 || ks == 0xffff {
                st.password.pop();
                return true;
            }
            if ks == 0xff0d {
                st.password_focus = false;
                return true;
            }
            if (0x20..0x7f).contains(&ks) {
                st.password.push(ks as u8 as char);
                return true;
            }
            return true;
        }
        false
    }

    fn tick(&self, app: &mut App) {
        // config stamp reload + redraw for live values (RAM, clock)
        let _ = app;
        // no-op: redraw on demand only (cheap)
    }
}

impl SettingsApp {
    // ------------------------------------------------------------- pages

    fn page_about(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let acc = app.accent;
        let mut y = area.y + 6;
        // hero
        if let Some(icon) = app.icons.get("brave", 64, 64) {
            cv.image(&icon, area.x + (area.w - 64) / 2, y);
        }
        y += 72;
        let ver = std::fs::read_to_string("/usr/share/bravos/VERSION").unwrap_or_else(|_| "2.0.0".into());
        let ver = ver.trim();
        let title = format!("BraveOS {} «Lion»", ver);
        app.fonts.draw_centered(cv, Rect::new(area.x, y, area.w, 26), 18, [240, 240, 245, 255], true, &title);
        y += 30;
        app.fonts.draw_centered(cv, Rect::new(area.x, y, area.w, 18), 12, [160, 160, 175, 255], false, &app.tr.t("about_sub"));
        y += 30;

        let kernel = sh("uname -r").trim().to_string();
        let cpu = sh("grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //'").trim().to_string();
        let gpu = sh("lspci 2>/dev/null | grep -Ei 'vga|3d' | head -1 | cut -c9-").trim().to_string();
        let session = "X11 + bwm (BraveOS WM) + picom";
        let mem: (String, u32, u32) = {
            let mi = sh("grep -E 'MemTotal|MemAvailable' /proc/meminfo");
            let total: u32 = mi.lines().find(|l| l.starts_with("MemTotal")).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok()).unwrap_or(0);
            let avail: u32 = mi.lines().find(|l| l.starts_with("MemAvailable")).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok()).unwrap_or(0);
            let used = (total.saturating_sub(avail)) / 1024;
            (format!("{} MB / {} MB", used, total / 1024), used, total / 1024)
        };

        let rows: Vec<(&str, String)> = vec![
            ("about_kernel", kernel),
            ("about_cpu", if cpu.is_empty() { "-".into() } else { cpu }),
            ("about_gpu", if gpu.is_empty() { "-".into() } else { gpu }),
            ("about_session", session.into()),
            ("about_browser", "Brave (official build)".into()),
        ];
        for (k, v) in rows {
            let r = Rect::new(area.x, y, area.w, 30);
            let (lx, rx) = if app.tr.rtl { (r.x + r.w - 160, r.x + 8) } else { (r.x + 8, r.x + 180) };
            app.fonts.draw(cv, lx as f32, (r.y + 8) as f32, 13, [150, 150, 165, 255], false, &app.tr.t(k));
            app.fonts.draw(cv, rx as f32, (r.y + 8) as f32, 13, [225, 225, 232, 255], false, &v);
            y += 30;
        }
        // RAM bar
        let bar = Rect::new(area.x + 8, y + 6, area.w - 220, 14);
        let pct = ((mem.1 as f32 / mem.2.max(1) as f32) * 100.0) as i32;
        cv.rounded(bar.x, bar.y, bar.w, bar.h, 7, [255, 255, 255, 26]);
        cv.rounded(bar.x, bar.y, bar.w * pct.max(1) / 100, bar.h, 7, [acc[0], acc[1], acc[2], 220]);
        app.fonts.draw(cv, (bar.x + bar.w + 10) as f32, (bar.y - 2) as f32, 12, [225, 225, 232, 255], false, &format!("{} {}", app.tr.t("about_memory"), mem.0));
        y += 34;

        let btn = Rect::new(area.x + (area.w - 200) / 2, y + 10, 200, 40);
        cv.rounded(btn.x, btn.y, btn.w, btn.h, 12, [acc[0], acc[1], acc[2], 235]);
        app.fonts.draw_centered(cv, btn, 13, [255, 255, 255, 255], true, &app.tr.t("open_brave"));
        st.hits.push((btn, Ctl::Btn("open_brave")));
    }

    fn page_header(&self, app: &mut App, cv: &mut Canvas, area: Rect, key: &str) -> i32 {
        app.fonts.draw(cv, area.x as f32, area.y as f32, 17, [240, 240, 245, 255], true, &app.tr.t(key));
        area.y + 34
    }

    fn row_toggle(&self, app: &mut App, st: &mut State, cv: &mut Canvas, y: i32, w: i32, key: &'static str, label: String, sub: String, on: bool, rtl: bool) {
        let r = Rect::new(0, y, w, ROW_H);
        let inner = Rect::new(8, y, w - 16, ROW_H - 8);
        cv.rounded(inner.x, inner.y, inner.w, inner.h, 12, [255, 255, 255, 18]);
        let acc = app.accent;
        let (tx, sw_x) = if rtl { ((inner.x + inner.w - 216) as f32, inner.x + 12) } else { ((inner.x + 16) as f32, inner.x + inner.w - 64) };
        let sw = Rect::new(sw_x, y + 14, 48, 26);
        app.fonts.draw(cv, tx, (y + 14) as f32, 13, [235, 235, 240, 255], false, &label);
        if !sub.is_empty() {
            app.fonts.draw(cv, tx, (y + 32) as f32, 11, [150, 150, 165, 255], false, &sub);
        }
        let _ = r;
        // switch
        cv.rounded(sw.x, sw.y, sw.w, sw.h, 13, if on { [acc[0], acc[1], acc[2], 230] } else { [255, 255, 255, 40] });
        let kx = if on { sw.x + sw.w - 24 } else { sw.x + 2 };
        cv.circle(kx + 11, sw.y + 13, 11, [250, 250, 255, 255]);
        st.hits.push((sw, Ctl::Toggle(key)));
        st.hits.push((inner, Ctl::Toggle(key)));
    }

    fn row_slider(&self, app: &mut App, st: &mut State, cv: &mut Canvas, y: i32, w: i32, key: &'static str, label: String, val_text: String, t: f32) {
        let inner = Rect::new(8, y, w - 16, ROW_H - 8);
        cv.rounded(inner.x, inner.y, inner.w, inner.h, 12, [255, 255, 255, 18]);
        app.fonts.draw(cv, (inner.x + 16) as f32, (y + 12) as f32, 13, [235, 235, 240, 255], false, &label);
        app.fonts.draw(cv, (inner.x + inner.w - 60) as f32, (y + 12) as f32, 13, [200, 200, 210, 255], false, &val_text);
        let track = Rect::new(inner.x + 16, y + 34, inner.w - 32, 8);
        cv.rounded(track.x, track.y, track.w, track.h, 4, [255, 255, 255, 30]);
        let acc = app.accent;
        let hw = (track.w as f32 * t.clamp(0.0, 1.0)) as i32;
        cv.rounded(track.x, track.y, hw, track.h, 4, [acc[0], acc[1], acc[2], 235]);
        cv.circle(track.x + hw - 2, track.y + 4, 8, [250, 250, 255, 255]);
        st.hits.push((Rect::new(track.x - 8, track.y - 10, track.w + 16, 28), Ctl::Slider(key)));
    }

    fn row_dropdown(&self, app: &mut App, st: &mut State, cv: &mut Canvas, y: i32, w: i32, key: &'static str, label: String, value: String, rtl: bool) {
        let inner = Rect::new(8, y, w - 16, ROW_H - 8);
        cv.rounded(inner.x, inner.y, inner.w, inner.h, 12, [255, 255, 255, 18]);
        let (tx, dd_x) = if rtl { (inner.x + inner.w - 256, inner.x + 12) } else { (inner.x + 16, inner.x + inner.w - 256) };
        app.fonts.draw(cv, tx as f32, (y + 12) as f32, 13, [235, 235, 240, 255], false, &label);
        let dd = Rect::new(dd_x, y + 12, 240, 28);
        cv.rounded(dd.x, dd.y, dd.w, dd.h, 8, [30, 30, 42, 255]);
        cv.rounded_outline(dd.x, dd.y, dd.w, dd.h, 8, [255, 255, 255, 40]);
        app.fonts.draw_centered(cv, dd, 12, [225, 225, 232, 255], false, &value);
        st.hits.push((dd, Ctl::Drop(key)));
        st.hits.push((inner, Ctl::Drop(key)));
        // open list overlay
        if st.drop_open == Some(key) {
            let items = self.drop_items(key, app);
            let lh = 30;
            let lw = 300;
            let lx = if rtl { inner.x + 12 } else { inner.x + inner.w - lw - 12 };
            let list = Rect::new(lx, y + 44, lw, (items.len() as i32 * lh + 10).min(220));
            cv.rounded(list.x, list.y, list.w, list.h, 10, [22, 22, 32, 250]);
            cv.rounded_outline(list.x, list.y, list.w, list.h, 10, [255, 255, 255, 40]);
            for (i, it) in items.iter().enumerate().take(7) {
                let r = Rect::new(list.x + 6, list.y + 5 + i as i32 * lh, list.w - 12, lh - 4);
                let acc = app.accent;
                cv.rounded(r.x, r.y, r.w, r.h, 7, [255, 255, 255, 14]);
                app.fonts.draw(cv, (r.x + 10) as f32, (r.y + 8) as f32, 12, [230, 230, 238, 255], false, it);
                let _ = acc;
                st.hits.push((r, Ctl::DropItem(key, i)));
            }
        }
    }

    fn drop_items(&self, key: &str, app: &mut App) -> Vec<String> {
        match key {
            "resolution" => {
                if self.st.borrow().resolutions.is_empty() {
                    let out = sh("xrandr 2>/dev/null | grep -E '^  [0-9]+x[0-9]+' | awk '{print $1}' | head -12");
                    let res: Vec<String> = out.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
                    self.st.borrow_mut().resolutions = res;
                }
                self.st.borrow().resolutions.clone()
            }
            "sink" => {
                if self.st.borrow().sinks.is_empty() {
                    let out = sh("pactl list short sinks 2>/dev/null | awk '{print $2}'");
                    let s: Vec<String> = out.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
                    self.st.borrow_mut().sinks = s;
                }
                self.st.borrow().sinks.clone()
            }
            "timezone" => {
                if self.st.borrow().timezones.is_empty() {
                    let out = sh("timedatectl list-timezones 2>/dev/null | head -40");
                    let t: Vec<String> = out.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
                    self.st.borrow_mut().timezones = t;
                }
                self.st.borrow().timezones.clone()
            }
            "language" => Tr::languages().into_iter().map(|(_, n)| n).collect(),
            "wallpaper" => vec!["BraveOS Sunset".into(), "BraveOS Light".into(), "Aurora".into()],
            _ => Vec::new(),
        }
    }

    fn btn(&self, app: &mut App, st: &mut State, cv: &mut Canvas, r: Rect, label: &str, action: &'static str, primary: bool) {
        let acc = app.accent;
        if primary {
            cv.rounded(r.x, r.y, r.w, r.h, 12, [acc[0], acc[1], acc[2], 235]);
            app.fonts.draw_centered(cv, r, 13, [255, 255, 255, 255], true, label);
        } else {
            cv.rounded(r.x, r.y, r.w, r.h, 12, [255, 255, 255, 26]);
            cv.rounded_outline(r.x, r.y, r.w, r.h, 12, [255, 255, 255, 50]);
            app.fonts.draw_centered(cv, r, 13, [230, 230, 238, 255], false, label);
        }
        st.hits.push((r, Ctl::Btn(action)));
    }

    // ------------------------------------------------------------- pages

    fn page_appearance(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_appearance");
        let w = area.w;
        self.row_toggle(app, st, cv, y, w, "dark", app.tr.t("dark_mode"), String::new(), app.cfg.dark, app.tr.rtl);
        y += ROW_H;
        self.row_dropdown(app, st, cv, y, w, "wallpaper", app.tr.t("wallpaper"),
            match app.cfg.wallpaper.as_str() {
                "braveos-light.png" => "BraveOS Light".to_string(),
                "aurora.png" => "Aurora".to_string(),
                _ => "BraveOS Sunset".to_string(),
            }, app.tr.rtl);
        y += ROW_H + 8;
        // accent swatches
        let inner = Rect::new(8, y, w - 16, 80);
        cv.rounded(inner.x, inner.y, inner.w, inner.h, 12, [255, 255, 255, 18]);
        app.fonts.draw(cv, (inner.x + 16) as f32, (y + 12) as f32, 13, [235, 235, 240, 255], false, &app.tr.t("accent_color"));
        let swatches: Vec<(&str, [u8; 3])> = vec![
            ("#f04e23", [240, 78, 35]),
            ("#e03131", [224, 49, 49]),
            ("#f76707", [247, 103, 7]),
            ("#f59f00", [245, 159, 0]),
            ("#2f9e44", [47, 158, 68]),
            ("#1971c2", [25, 113, 194]),
            ("#6741d9", [103, 65, 217]),
            ("#d6336c", [214, 51, 108]),
            ("#e8590c", [232, 89, 12]),
            ("#3b5bdb", [59, 91, 219]),
        ];
        for (i, (hex, rgb)) in swatches.iter().enumerate() {
            let r = Rect::new(inner.x + 16 + i as i32 * 56, y + 36, 44, 34);
            let sel = app.cfg.accent == *hex;
            cv.rounded(r.x, r.y, r.w, r.h, 8, [rgb[0], rgb[1], rgb[2], 255]);
            if sel {
                cv.rounded_outline(r.x - 2, r.y - 2, r.w + 4, r.h + 4, 9, [255, 255, 255, 230]);
            }
            st.hits.push((r, Ctl::Btn("accent")));
            // encode hex in hit map via drop hack: store in resolutions? Use st.wifi_sel trick
        }
    }

    fn page_display(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_display");
        let w = area.w;
        let cur = sh("xrandr 2>/dev/null | grep -E '^ [^ ]+ connected' | head -1 | awk '{print $1}'").trim().to_string();
        let curmode = sh("xrandr 2>/dev/null | grep -E '^ [^ ]+ connected .* [0-9]+x[0-9]+' | sed 's/.* \\([0-9]*x[0-9]*\\).*/\\1/'").trim().to_string();
        let r = Rect::new(8, y, w - 16, 40);
        cv.rounded(r.x, r.y, r.w, r.h, 12, [255, 255, 255, 18]);
        app.fonts.draw(cv, (r.x + 16) as f32, (y + 12) as f32, 13, [235, 235, 240, 255], false, &app.tr.t("current_display"));
        app.fonts.draw(cv, (r.x + 220) as f32, (y + 12) as f32, 13, [200, 200, 210, 255], false, &format!("{} · {}", cur, curmode));
        y += 48;
        self.row_dropdown(app, st, cv, y, w, "resolution", app.tr.t("resolution"), curmode, app.tr.rtl);
        y += ROW_H + 8;
        let bw = 160;
        let b1 = Rect::new(area.x + 8, y, bw, 40);
        self.btn(app, st, cv, b1, &app.tr.t("apply"), "apply_res", true);
        self.row_toggle(app, st, cv, y + 56, w, "night", app.tr.t("night_light"), String::new(), false, app.tr.rtl);
    }

    fn page_network(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_network");
        let w = area.w;
        // wired + airplane
        let wired = sh("nmcli -t -f TYPE,STATE device 2>/dev/null | grep -E '^ethernet:' | head -1 | cut -d: -f2").trim().to_string();
        let inner = Rect::new(8, y, w - 16, 40);
        cv.rounded(inner.x, inner.y, inner.w, inner.h, 12, [255, 255, 255, 18]);
        app.fonts.draw(cv, (inner.x + 16) as f32, (y + 12) as f32, 13, [235, 235, 240, 255], false, &app.tr.t("wired"));
        let st_txt = if wired == "connected" { app.tr.t("connected") } else { app.tr.t("disconnected") };
        let acc = app.accent;
        app.fonts.draw(cv, (inner.x + 220) as f32, (y + 12) as f32, 13,
            if wired == "connected" { [120, 220, 140, 255] } else { [230, 130, 120, 255] }, false, &st_txt);
        y += 48;
        self.row_toggle(app, st, cv, y, w, "airplane", app.tr.t("airplane"), String::new(), false, app.tr.rtl);
        y += ROW_H;
        // wifi list
        let head = Rect::new(8, y, w - 16, 34);
        app.fonts.draw(cv, (head.x + 8) as f32, (y + 8) as f32, 13, [235, 235, 240, 255], false, &app.tr.t("wifi_networks"));
        let rb = Rect::new(head.x + head.w - 100, y, 92, 30);
        self.btn(app, st, cv, rb, &app.tr.t("refresh"), "wifi_scan", false);
        y += 40;
        if st.wifi_nets.is_empty() {
            self.scan_wifi(st);
        }
        for (i, (ssid, sig, secure)) in st.wifi_nets.iter().enumerate().take(6) {
            let r = Rect::new(8, y, w - 16, 44);
            let sel = st.wifi_sel == Some(i);
            if sel {
                cv.rounded(r.x, r.y, r.w, r.h, 10, [acc[0], acc[1], acc[2], 60]);
            } else {
                cv.rounded(r.x, r.y, r.w, r.h, 10, [255, 255, 255, 16]);
            }
            if let Some(icon) = app.icons.get("wifi", 18, 18) {
                cv.image(&icon, r.x + 12, r.y + 13);
            }
            app.fonts.draw(cv, (r.x + 40) as f32, (r.y + 8) as f32, 13, [235, 235, 240, 255], false, ssid);
            let lock_txt = if *secure { "🔒" } else { "" };
            app.fonts.draw(cv, (r.x + r.w - 120) as f32, (r.y + 8) as f32, 11, [200, 200, 210, 255], false, &format!("{}%", sig));
            let _ = lock_txt;
            st.hits.push((r, Ctl::Item(100 + i)));
            y += 50;
        }
        // password box + connect
        if st.wifi_sel.is_some() {
            let pb = Rect::new(8, y + 4, 260, 36);
            cv.rounded(pb.x, pb.y, pb.w, pb.h, 10, [30, 30, 42, 255]);
            cv.rounded_outline(pb.x, pb.y, pb.w, pb.h, 10, if st.password_focus { [acc[0], acc[1], acc[2], 220] } else { [255, 255, 255, 45] });
            let shown = if st.password_focus && st.password.is_empty() {
                app.tr.t("password")
            } else {
                "*".repeat(st.password.chars().count())
            };
            app.fonts.draw(cv, (pb.x + 12) as f32, (pb.y + 10) as f32, 13, [225, 225, 232, 255], false, &shown);
            st.hits.push((pb, Ctl::Btn("pw_focus")));
            let cb = Rect::new(280, y + 4, 140, 36);
            self.btn(app, st, cv, cb, &app.tr.t("connect"), "wifi_connect", true);
        }
    }

    fn page_sound(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_sound");
        let w = area.w;
        self.row_slider(app, st, cv, y, w, "volume", app.tr.t("output_volume"), format!("{}%", st.volume), st.volume as f32 / 150.0);
        y += ROW_H;
        self.row_toggle(app, st, cv, y, w, "mute", app.tr.t("mute"), String::new(), false, app.tr.rtl);
        y += ROW_H;
        self.row_dropdown(app, st, cv, y, w, "sink", app.tr.t("output_device"),
            st.sinks.get(st.sink_sel).cloned().unwrap_or_else(|| "default".to_string()), app.tr.rtl);
    }

    fn page_keyboard(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_keyboard");
        let w = area.w;
        // active layouts chips
        let inner = Rect::new(8, y, w - 16, 72);
        cv.rounded(inner.x, inner.y, inner.w, inner.h, 12, [255, 255, 255, 18]);
        app.fonts.draw(cv, (inner.x + 16) as f32, (y + 10) as f32, 13, [235, 235, 240, 255], false, &app.tr.t("keyboard_layouts"));
        let mut cx = inner.x + 16;
        for l in &app.cfg.layouts {
            let name = st.layouts.iter().find(|(c, _)| c == l).map(|(_, n)| *n).unwrap_or(l.as_str());
            let wpx = (app.fonts.measure(12, name, false) + 30.0) as i32;
            let chip = Rect::new(cx, y + 34, wpx, 28);
            cv.rounded(chip.x, chip.y, chip.w, chip.h, 14, [app.accent[0], app.accent[1], app.accent[2], 70]);
            cv.rounded_outline(chip.x, chip.y, chip.w, chip.h, 14, [app.accent[0], app.accent[1], app.accent[2], 160]);
            app.fonts.draw(cv, (chip.x + 10) as f32, (chip.y + 7) as f32, 12, [240, 240, 245, 255], false, name);
            // remove x
            app.fonts.draw(cv, (chip.x + chip.w - 16) as f32, (chip.y + 7) as f32, 12, [240, 240, 245, 255], false, "×");
            st.hits.push((Rect::new(chip.x + chip.w - 20, chip.y, 20, chip.h), Ctl::Btn("layout_remove")));
            cx += wpx + 8;
        }
        y += 80;
        self.row_dropdown(app, st, cv, y, w, "layout_add", app.tr.t("layout_add"), app.tr.t("add"), app.tr.rtl);
        y += ROW_H;
        self.row_slider(app, st, cv, y, w, "repeat_delay", app.tr.t("repeat_delay"), format!("{} ms", app.cfg.repeat_delay_ms), (app.cfg.repeat_delay_ms as f32 - 100.0) / 900.0);
        y += ROW_H;
        self.row_slider(app, st, cv, y, w, "repeat_rate", app.tr.t("repeat_rate"), format!("{} cps", 1000 / app.cfg.repeat_rate_ms.max(1)), (app.cfg.repeat_rate_ms as f32 - 10.0) / 90.0);
        y += ROW_H + 6;
        let ab = Rect::new(area.x + 8, y, 160, 38);
        self.btn(app, st, cv, ab, &app.tr.t("apply"), "apply_kbd", true);
    }

    fn page_language(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_language");
        let w = area.w;
        let cur = if app.cfg.language == "auto" {
            format!("{} ({})", "Auto", app.tr.name)
        } else {
            app.tr.name.clone()
        };
        self.row_dropdown(app, st, cv, y, w, "language", app.tr.t("ui_language"), cur, app.tr.rtl);
        y += ROW_H;
        let note = Rect::new(8, y, w - 16, 60);
        cv.rounded(note.x, note.y, note.w, note.h, 12, [255, 255, 255, 12]);
        app.fonts.draw(cv, (note.x + 14) as f32, (note.y + 10) as f32, 12, [180, 180, 195, 255], false, &app.tr.t("restart_hint"));
        y += 70;
        let ab = Rect::new(area.x + 8, y, 160, 38);
        self.btn(app, st, cv, ab, &app.tr.t("apply"), "apply_lang", true);
    }

    fn page_mouse(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_mouse");
        let w = area.w;
        self.row_slider(app, st, cv, y, w, "mouse_speed", app.tr.t("pointer_speed"),
            format!("{:.1}", app.cfg.mouse_speed), (app.cfg.mouse_speed as f32 + 1.0) / 2.0);
        y += ROW_H;
        self.row_toggle(app, st, cv, y, w, "natural", app.tr.t("natural_scrolling"), String::new(), app.cfg.natural_scroll, app.tr.rtl);
        y += ROW_H;
        self.row_toggle(app, st, cv, y, w, "lefty", app.tr.t("left_handed"), String::new(), app.cfg.left_handed, app.tr.rtl);
    }

    fn page_power(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_power");
        let w = area.w;
        // battery status
        let bat = sh("cat /sys/class/power_supply/BAT*/capacity 2>/dev/null | head -1").trim().to_string();
        let stat = sh("cat /sys/class/power_supply/BAT*/status 2>/dev/null | head -1").trim().to_string();
        let inner = Rect::new(8, y, w - 16, 40);
        cv.rounded(inner.x, inner.y, inner.w, inner.h, 12, [255, 255, 255, 18]);
        let txt = if bat.is_empty() {
            app.tr.t("no_battery")
        } else {
            format!("{} {}% · {}", app.tr.t("battery"), bat, stat)
        };
        app.fonts.draw(cv, (inner.x + 16) as f32, (y + 12) as f32, 13, [235, 235, 240, 255], false, &txt);
        y += 48;
        let cur = if app.cfg.blank_minutes == 0 { app.tr.t("never") } else { format!("{} min", app.cfg.blank_minutes) };
        self.row_dropdown(app, st, cv, y, w, "blank", app.tr.t("screen_blank"), cur, app.tr.rtl);
        y += ROW_H + 10;
        let bw = 170;
        self.btn(app, st, cv, Rect::new(area.x + 8, y, bw, 40), &app.tr.t("suspend_now"), "suspend", false);
        self.btn(app, st, cv, Rect::new(area.x + 8 + bw + 10, y, bw, 40), &app.tr.t("restart_now"), "reboot", false);
        self.btn(app, st, cv, Rect::new(area.x + 8 + (bw + 10) * 2, y, bw, 40), &app.tr.t("shutdown_now"), "shutdown", false);
    }

    fn page_shortcuts(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_shortcuts");
        let w = area.w;
        app.fonts.draw(cv, (area.x + 8) as f32, (y) as f32, 11, [150, 150, 165, 255], false, &app.tr.t("press_new_key"));
        y += 22;
        for (i, (action, i18n_key)) in st.shortcut_actions.iter().enumerate() {
            let combo = app.cfg.shortcuts.get(*action).cloned().unwrap_or_default();
            let r = Rect::new(8, y, w - 16, 38);
            cv.rounded(r.x, r.y, r.w, r.h, 10, if st.capture == Some(i) { [app.accent[0], app.accent[1], app.accent[2], 60] } else { [255, 255, 255, 16] });
            app.fonts.draw(cv, (r.x + 12) as f32, (r.y + 10) as f32, 12, [235, 235, 240, 255], false, &app.tr.t(i18n_key));
            let val = if st.capture == Some(i) { app.tr.t("press_new_key").to_string() } else { combo.replace("super", "Super").replace("alt", "Alt").replace("ctrl", "Ctrl").replace("shift", "Shift").replace("enter", "Enter") };
            let vw = (app.fonts.measure(12, &val, false) + 20.0) as i32;
            let vr = Rect::new(r.x + r.w - vw - 12, r.y + 6, vw, 26);
            cv.rounded(vr.x, vr.y, vr.w, vr.h, 7, [40, 40, 56, 255]);
            app.fonts.draw_centered(cv, vr, 11, [225, 225, 232, 255], false, &val);
            st.hits.push((r, Ctl::ShortcutRow(i)));
            y += 42;
        }
    }

    fn page_time(&self, app: &mut App, st: &mut State, cv: &mut Canvas, area: Rect) {
        let mut y = self.page_header(app, cv, area, "sec_time");
        let w = area.w;
        let now = sh("date '+%Y-%m-%d %H:%M'").trim().to_string();
        let tz = sh("timedatectl show -p Timezone --value 2>/dev/null || cat /etc/timezone 2>/dev/null").trim().to_string();
        let inner = Rect::new(8, y, w - 16, 40);
        cv.rounded(inner.x, inner.y, inner.w, inner.h, 12, [255, 255, 255, 18]);
        app.fonts.draw(cv, (inner.x + 16) as f32, (y + 12) as f32, 13, [235, 235, 240, 255], false, &now);
        app.fonts.draw(cv, (inner.x + 220) as f32, (y + 12) as f32, 13, [200, 200, 210, 255], false, &tz);
        y += 48;
        self.row_dropdown(app, st, cv, y, w, "timezone", app.tr.t("set_timezone"),
            if tz.is_empty() { "UTC".to_string() } else { tz.clone() }, app.tr.rtl);
        y += ROW_H;
        self.row_toggle(app, st, cv, y, w, "ntp", "NTP".to_string(), String::new(), false, app.tr.rtl);
        y += ROW_H + 6;
        let ab = Rect::new(area.x + 8, y, 200, 38);
        self.btn(app, st, cv, ab, &app.tr.t("apply"), "apply_time", true);
    }

    fn scan_wifi(&self, st: &mut State) {
        let out = sh("timeout 5 nmcli -t -f SSID,SIGNAL,SECURITY device wifi list 2>/dev/null | grep -v '^:$' | head -8");
        let mut nets = Vec::new();
        for ln in out.lines() {
            let mut it = ln.split(':');
            let ssid = it.next().unwrap_or("").trim().to_string();
            let sig = it.next().unwrap_or("0").parse().unwrap_or(0);
            let sec = it.next().unwrap_or("").to_string();
            if !ssid.is_empty() && !nets.iter().any(|(s, _, _)| s == &ssid) {
                nets.push((ssid, sig.min(100), !sec.is_empty() && sec != "--"));
            }
        }
        st.wifi_nets = nets;
    }

    // ------------------------------------------------------------ clicks

    fn click(&self, app: &mut App, st: &mut State, ctl: Ctl) {
        match ctl {
            Ctl::Item(i) => {
                if i < 100 {
                    st.section = i;
                    st.drop_open = None;
                } else {
                    st.wifi_sel = Some(i - 100);
                    st.password.clear();
                    st.password_focus = true;
                }
            }
            Ctl::Toggle(key) => match key {
                "dark" => {
                    app.cfg.dark = !app.cfg.dark;
                    app.cfg.wallpaper = if app.cfg.dark { "braveos.png".into() } else { "braveos-light.png".into() };
                    apply_theme(app.cfg.dark);
                    app.cfg.save();
                }
                "night" => {
                    let on = sh("xrandr --verbose 2>/dev/null | grep -c 'Gamma.*1.0:1.0:0.82' || true");
                    if on.trim() == "0" {
                        sh("xrandr --output $(xrandr | grep ' connected' | head -1 | cut -d' ' -f1) --gamma 1.0:1.0:0.82 2>/dev/null");
                    } else {
                        sh("xrandr --output $(xrandr | grep ' connected' | head -1 | cut -d' ' -f1) --gamma 1.0:1.0:1.0 2>/dev/null");
                    }
                }
                "airplane" => {
                    let blocked = sh("rfkill list wifi 2>/dev/null | grep -c 'Soft blocked: yes' || true");
                    if blocked.trim() == "0" {
                        sh("rfkill block wifi");
                    } else {
                        sh("rfkill unblock wifi");
                    }
                    self.toast(st, "airplane");
                }
                "mute" => {
                    sh("pactl set-sink-mute @DEFAULT_SINK@ toggle");
                }
                "natural" => {
                    app.cfg.natural_scroll = !app.cfg.natural_scroll;
                    apply_mouse(&app.cfg);
                    app.cfg.save();
                }
                "lefty" => {
                    app.cfg.left_handed = !app.cfg.left_handed;
                    apply_mouse(&app.cfg);
                    app.cfg.save();
                }
                "ntp" => {
                    sh("sudo timedatectl set-ntp true 2>/dev/null || timedatectl set-ntp true 2>/dev/null");
                    self.toast(st, "NTP");
                }
                _ => {}
            },
            Ctl::Slider(_) => {}
            Ctl::Drop(key) => {
                if st.drop_open == Some(key) {
                    st.drop_open = None;
                } else {
                    st.drop_open = Some(key);
                }
            }
            Ctl::DropItem(key, idx) => {
                let items = self.drop_items(key, app);
                let val = items.get(idx).cloned().unwrap_or_default();
                match key {
                    "resolution" => {
                        st.res_sel = idx;
                        st.drop_open = None;
                        st.toast.clear();
                    }
                    "sink" => {
                        st.sink_sel = idx;
                        let sink_line = sh(&format!("pactl list short sinks | sed -n '{}p'", idx + 1));
                          sh(&format!("pactl set-default-sink {}", sink_line.split_whitespace().next().unwrap_or("")));
                        st.drop_open = None;
                    }
                    "timezone" => {
                        st.tz_sel = idx;
                        st.drop_open = None;
                    }
                    "language" => {
                        // map name back to code
                        for (code, name) in Tr::languages() {
                            if name == val {
                                app.cfg.language = code.clone();
                                app.tr = Tr::load(&code);
                                break;
                            }
                        }
                        st.drop_open = None;
                        self.toast(st, &format!("{}: {}", app.tr.t("ui_language"), app.tr.name));
                    }
                    "layout_add" => {
                        for (code, name) in st.layouts.iter() {
                            if *name == val && !app.cfg.layouts.iter().any(|l| l == code) {
                                app.cfg.layouts.push(code.to_string());
                            }
                        }
                        st.drop_open = None;
                    }
                    "wallpaper" => {
                        app.cfg.wallpaper = match val.as_str() {
                            "BraveOS Light" => "braveos-light.png".to_string(),
                            "Aurora" => "aurora.png".to_string(),
                            _ => "braveos.png".to_string(),
                        };
                        app.cfg.save();
                        st.drop_open = None;
                    }
                    "blank" => {
                        app.cfg.blank_minutes = match val.as_str() {
                            s if s.contains("1") => 1,
                            s if s.contains("10") => 10,
                            s if s.contains("15") => 15,
                            s if s.contains("5") => 5,
                            _ => 0,
                        };
                        apply_blank(app.cfg.blank_minutes);
                        app.cfg.save();
                        st.drop_open = None;
                    }
                    _ => {
                        st.drop_open = None;
                    }
                }
            }
            Ctl::Btn(action) => match action {
                "open_brave" => spawn(&["brave", "--no-first-run"]),
                "apply_res" => {
                    let res = st.resolutions.get(st.res_sel).cloned().unwrap_or_default();
                    if !res.is_empty() {
                        sh(&format!("xrandr -s {} 2>/dev/null || xrandr --output $(xrandr | grep ' connected' | head -1 | cut -d' ' -f1) --mode {}", res, res));
                        self.toast(st, &format!("{} {}", app.tr.t("apply_display"), res));
                    }
                }
                "wifi_scan" => {
                    self.scan_wifi(st);
                    self.toast(st, &app.tr.t("wifi_networks"));
                }
                "pw_focus" => st.password_focus = true,
                "wifi_connect" => {
                    if let Some(i) = st.wifi_sel {
                        let ssid = st.wifi_nets.get(i).map(|(s, _, _)| s.clone()).unwrap_or_default();
                        let pw = st.password.clone();
                        self.toast(st, &app.tr.t("connecting"));
                        let out = sh(&format!(
                            "timeout 12 nmcli device wifi connect '{}' password '{}' 2>&1",
                            ssid.replace('\'', ""), pw.replace('\'', "")
                        ));
                        let ok = out.contains("successfully");
                        let msg = if ok { app.tr.t("connected") } else { app.tr.t("wrong_password") };
                        self.toast(st, &msg);
                        st.password_focus = false;
                    }
                }
                "apply_kbd" => {
                    apply_layouts(&app.cfg);
                    app.cfg.save();
                    self.toast(st, &app.tr.t("sec_keyboard"));
                }
                "layout_remove" => {
                    if app.cfg.layouts.len() > 1 {
                        app.cfg.layouts.pop();
                    }
                }
                "apply_lang" => {
                    app.cfg.save();
                    self.toast(st, &format!("{}: {}", app.tr.t("ui_language"), app.tr.name));
                }
                "accent" => {
                    // hex encoded by swatch index is lost; recover from position:
                    // handled in click wrapper below (see swatch handler)
                }
                "suspend" => spawn(&["sh", "-c", "systemctl suspend -i || xset dpms force suspend"]),
                "reboot" => spawn(&["sh", "-c", "systemctl reboot -i"]),
                "shutdown" => spawn(&["sh", "-c", "systemctl poweroff -i"]),
                "apply_time" => {
                    let tz = st.timezones.get(st.tz_sel).cloned().unwrap_or_default();
                    if !tz.is_empty() {
                        sh(&format!("sudo timedatectl set-timezone {} 2>/dev/null || timedatectl set-timezone {} 2>/dev/null", tz, tz));
                        self.toast(st, &tz);
                    }
                }
                _ => {}
            },
            Ctl::ShortcutRow(i) => {
                st.capture = Some(i);
            }
            Ctl::None => {}
        }
        // redraw after any action
        let _ = app;
    }
}

fn st_unused_msg(tr: &Tr, k: &str) -> String {
    tr.t(k)
}

fn keysym_name(ks: u32) -> String {
    match ks {
        0xff0d => "Enter".into(),
        0xff09 => "Tab".into(),
        0x20 => "Space".into(),
        0xff1b => "Esc".into(),
        0xffff => "Delete".into(),
        0xff08 => "Backspace".into(),
        0xff51 => "Left".into(),
        0xff52 => "Up".into(),
        0xff53 => "Right".into(),
        0xff54 => "Down".into(),
        0xff61 => "Print".into(),
        k if (0xffbe..=0xffd2).contains(&k) => format!("F{}", k - 0xffbd + 1),
        k if (0x20..0x7f).contains(&k) => (k as u8 as char).to_string(),
        _ => format!("Key{:04x}", ks),
    }
}

// ---------------------------------------------------------------- main

fn main() {
    let mut app = App::connect().expect("bsettings: X11 connect");
    let target = std::env::args().nth(1);
    let sa = SettingsApp::new(&app);
    if let Some(sec) = target {
        let idx = match sec.as_str() {
            "about" => 0,
            "appearance" => 1,
            "display" => 2,
            "network" => 3,
            "sound" => 4,
            "keyboard" => 5,
            "language" => 6,
            "mouse" => 7,
            "power" => 8,
            "shortcuts" => 9,
            "time" => 10,
            _ => 0,
        };
        sa.st.borrow_mut().section = idx;
    }
    // accent swatch clicks: intercept via wrapped Btn action before generic click.
    // We handle it by scanning hits for swatch rects in pointer(); simpler: the
    // swatch rect maps to Ctl::Btn("accent") — resolve hex by x-position (fixed layout).
    {
        let mut st = sa.st.borrow_mut();
        let _ = &mut st;
    }
    let w = app.add_win(
        WinOpts {
            x: ((app.screen_w - WIN_W) / 2).max(0),
            y: ((app.screen_h - WIN_H) / 2).max(0),
            w: WIN_W,
            h: WIN_H,
            override_redirect: false,
            argb: true,
            window_type: "",
            class: "bsettings",
            title: "BraveOS Settings",
            event_mask: bui::xapp::EventMask::EXPOSURE
                | bui::xapp::EventMask::BUTTON_PRESS
                | bui::xapp::EventMask::BUTTON_RELEASE
                | bui::xapp::EventMask::POINTER_MOTION
                | bui::xapp::EventMask::KEY_PRESS
                | bui::xapp::EventMask::STRUCTURE_NOTIFY
                | bui::xapp::EventMask::BUTTON_MOTION,
        },
        Box::new(WrappedSettings(sa.clone())),
    );
    let _ = w;

    // The accent swatch click needs the click x-coordinate; intercept in a
    // custom wrapper is complex — patch: SwatchCtl handled via Ctl::Btn("accent")
    // with x encoded in st. We wrap: after draw, swatch rects recorded; on click,
    // resolve color from column index.

    app.run_until(&|_app| false);
}

// Wrapper that intercepts accent swatch clicks by coordinate column.
struct WrappedSettings(Rc<SettingsApp>);
impl Handler for WrappedSettings {

    fn draw(&mut self, app: &mut App, cv: &mut Canvas) {
        self.0.draw(app, cv);
        // re-tag swatch hits with hex info: swatches are drawn at fixed columns;
        // we patch hits: any Btn("accent") rect gets its hex by column index
        let hexes = ["#f04e23", "#e03131", "#f76707", "#f59f00", "#2f9e44", "#1971c2", "#6741d9", "#d6336c", "#e8590c", "#3b5bdb"];
        let mut st = self.0.st.borrow_mut();
        for (r, ctl) in st.hits.iter_mut() {
            if let Ctl::Btn("accent") = ctl {
                let col = ((r.x - 24) / 56).clamp(0, 9) as usize;
                *ctl = Ctl::DropItem("accent", col);
                let _ = hexes;
            }
        }
    }
    fn pointer(&mut self, app: &mut App, kind: PtrKind, x: i32, y: i32) {
        // handle accent pick before generic click
        if matches!(kind, PtrKind::Down(_)) {
            let hit = {
                let st = self.0.st.borrow();
                st.hits.iter().find(|(r, _)| r.contains(x, y)).cloned()
            };
            if let Some((_, Ctl::DropItem("accent", col))) = hit {
                let hexes = ["#f04e23", "#e03131", "#f76707", "#f59f00", "#2f9e44", "#1971c2", "#6741d9", "#d6336c", "#e8590c", "#3b5bdb"];
                app.cfg.accent = hexes[col].to_string();
                app.accent = Config::hex_rgb(&app.cfg.accent);
                app.cfg.save();
                app.redraw(app.win_ids()[0]);
                return;
            }
        }
        self.0.pointer(app, kind, x, y);
        app.redraw(app.win_ids()[0]);
    }
    fn key(&mut self, app: &mut App, ks: u32, state: u16) -> bool {
        self.0.key(app, ks, state);
        app.redraw(app.win_ids()[0]);
        true
    }
}
