//! bshell — BraveOS shell: top panel (per the BraveOS design), radial dock,
//! app grid and power menu. One process, four X windows.

use std::cell::RefCell;
use std::rc::Rc;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as WrapExt;

use bui::canvas::Canvas;
use bui::config::Config;
use bui::geom::Rect;
use bui::pngutil::Image;
use bui::xapp::{App, Handler, PtrKind, WinOpts};

const PANEL_H: i32 = 40;
const DOCK_W: i32 = 480;
const DOCK_H: i32 = 172;
const DOCK_CY_OFF: i32 = 240; // dome circle center below the window's top edge
const DOCK_R_INNER: i32 = 170;
const DOCK_R_OUTER: i32 = 235;
const GRID_W: i32 = 620;
const GRID_H: i32 = 430;
const POWER_W: i32 = 240;
const POWER_H: i32 = 236;

fn sh(cmd: &str) -> String {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

fn spawn(args: &[&str]) {
    if let Err(e) = std::process::Command::new(args[0])
        .args(&args[1..])
        .env("DISPLAY", std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into()))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        eprintln!("bshell: spawn {:?} failed: {}", args, e);
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum DockHit {
    None,
    Files,
    Settings,
    Brave,
    Terminal,
    Grid,
    Moon,
    Wifi,
    PowerBtn,
    Lock,
}

#[derive(Clone, Debug)]
struct AppEntry {
    name: String,
    exec: String,
    icon: String,
}

struct Task {
    win: u32,
    title: String,
    class: String,
    desktop: u32,
}

struct Shell {
    panel: u32,
    dock: u32,
    grid: u32,
    power: u32,
    grid_open: bool,
    power_open: bool,
    desktops: u32,
    current: u32,
    tasks: Vec<Task>,
    active: u32,
    wifi: Option<String>,
    bt: bool,
    battery: Option<(u8, bool)>,
    clock: String,
    apps: Vec<AppEntry>,
    stamp: u64,
    brave: Option<Image>,
    root: u32,
}

fn config_stamp() -> u64 {
    std::fs::metadata("/tmp/.bravos-config-stamp")
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

impl Shell {
    fn status_poll(&mut self) {
        // WiFi + network via nmcli (best effort)
        let out = sh("timeout 3 nmcli -t -f TYPE,STATE device status 2>/dev/null");
        let mut wifi = None;
        for ln in out.lines() {
            let mut it = ln.split(':');
            let ty = it.next().unwrap_or("");
            let st = it.next().unwrap_or("");
            if ty == "wifi" {
                wifi = Some(st.to_string());
            }
        }
        self.wifi = wifi;
        // Bluetooth adapter present?
        self.bt = std::path::Path::new("/sys/class/bluetooth").is_dir()
            && !sh("timeout 2 rfkill list bluetooth 2>/dev/null").trim().is_empty();
        // battery
        self.battery = None;
        if let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("BAT") {
                    let cap = std::fs::read_to_string(format!("{}/capacity", e.path().display()))
                        .ok()
                        .and_then(|s| s.trim().parse::<u8>().ok())
                        .unwrap_or(0);
                    let st = std::fs::read_to_string(format!("{}/status", e.path().display()))
                        .unwrap_or_default();
                    self.battery = Some((cap, st.trim() == "Charging"));
                    break;
                }
            }
        }
        self.clock = sh("date +'%l:%M %p'").trim().to_string();
    }

    fn apps_scan(&mut self) {
        let mut apps = Vec::new();
        for dir in ["/usr/share/applications", "/usr/local/share/applications"] {
            if let Ok(rd) = std::fs::read_dir(dir) {
                for e in rd.flatten() {
                    let f = e.file_name().to_string_lossy().to_string();
                    if !f.ends_with(".desktop") {
                        continue;
                    }
                    let content = std::fs::read_to_string(e.path()).unwrap_or_default();
                    let mut name = None;
                    let mut exec = None;
                    let mut icon = None;
                    let mut nodisplay = false;
                    for ln in content.lines() {
                        if let Some(v) = ln.strip_prefix("Name=") {
                            name = Some(v.trim().to_string());
                        } else if let Some(v) = ln.strip_prefix("Exec=") {
                            exec = Some(v.trim().to_string());
                        } else if let Some(v) = ln.strip_prefix("Icon=") {
                            icon = Some(v.trim().to_string());
                        } else if ln.trim() == "NoDisplay=true" {
                            nodisplay = true;
                        }
                    }
                    if let (Some(n), Some(ex)) = (name, exec) {
                        if !nodisplay {
                            apps.push(AppEntry {
                                name: n,
                                exec: ex,
                                icon: icon.unwrap_or_else(|| "app".into()),
                            });
                        }
                    }
                }
            }
        }
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        apps.dedup_by(|a, b| a.name == b.name);
        self.apps = apps;
    }

    fn refresh_tasks(&mut self, conn: &RustConnection) {
        let client_list = atom(conn, "_NET_CLIENT_LIST");
        let active_atom = atom(conn, "_NET_ACTIVE_WINDOW");
        let name_atom = atom(conn, "_NET_WM_NAME");
        let desktop_atom = atom(conn, "_NET_WM_DESKTOP");
        self.tasks.clear();
        let wins = read_u32_prop(conn, self.root, client_list, 8192);
        for w in wins {
            let title = read_string_prop(conn, w, name_atom)
                .or_else(|| read_string_prop(conn, w, 39))
                .unwrap_or_else(|| "window".into());
            let class = read_string_prop(conn, w, 67).unwrap_or_default();
            let desktop = read_u32_prop(conn, w, desktop_atom, 8).first().copied().unwrap_or(0);
            self.tasks.push(Task { win: w, title, class, desktop });
        }
        self.active = read_u32_prop(conn, self.root, active_atom, 8).first().copied().unwrap_or(0);
        let nd = atom(conn, "_NET_NUMBER_OF_DESKTOPS");
        let cur = atom(conn, "_NET_CURRENT_DESKTOP");
        self.desktops = read_u32_prop(conn, self.root, nd, 8).first().copied().unwrap_or(2).max(1);
        self.current = read_u32_prop(conn, self.root, cur, 8).first().copied().unwrap_or(0);
    }
}

// -------------------------------------------------------------- X helpers

fn atom(conn: &RustConnection, name: &str) -> u32 {
    conn.intern_atom(false, name.as_bytes())
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.atom)
        .unwrap_or(0)
}

fn read_u32_prop(conn: &RustConnection, win: u32, prop: u32, len: u32) -> Vec<u32> {
    conn.get_property(false, win, prop, 0u32, 0, len)
        .ok()
        .and_then(|c| c.reply().ok())
        .and_then(|r| r.value32().map(|it| it.collect()))
        .unwrap_or_default()
}

fn read_string_prop(conn: &RustConnection, win: u32, prop: u32) -> Option<String> {
    conn.get_property(false, win, prop, 0u32, 0, 2048)
        .ok()
        .and_then(|c| c.reply().ok())
        .filter(|r| r.value_len > 0)
        .map(|r| String::from_utf8_lossy(&r.value).to_string())
}

// --------------------------------------------------------------- layout

struct Ui {
    tasks: Vec<(Rect, usize)>,
    pager: Vec<(Rect, u32)>,
    plus: Rect,
    right: Vec<(Rect, &'static str)>,
    logo: Rect,
}

fn panel_layout(w: i32, shell: &Shell) -> Ui {
    let mut right: Vec<(Rect, &'static str)> = Vec::new();
    let mut x = w - 14;
    x -= 30;
    right.push((Rect::new(x + 2, 6, 26, 28), "power"));
    let cw = 74i32;
    x -= cw + 6;
    right.push((Rect::new(x, 6, cw, 28), "clock"));
    x -= 22;
    right.push((Rect::new(x, 10, 1, 20), "sep"));
    x -= 10;
    if shell.battery.is_some() {
        x -= 66;
        right.push((Rect::new(x, 6, 62, 28), "battery"));
        x -= 8;
    }
    x -= 28;
    right.push((Rect::new(x + 2, 6, 24, 28), "bt"));
    x -= 30;
    right.push((Rect::new(x + 2, 6, 24, 28), "wifi"));

    let right_edge = x;

    let mut pager = Vec::new();
    let mut px = 118i32;
    for i in 0..shell.desktops {
        pager.push((Rect::new(px, 7, 28, 26), i + 1));
        px += 34;
    }
    let plus = Rect::new(px, 7, 26, 26);

    let t0 = px + 34;
    let t1 = right_edge - 10;
    let area_w = t1 - t0;
    let mut tasks = Vec::new();
    if !shell.tasks.is_empty() {
        let n = shell.tasks.len() as i32;
        let gap = 6;
        let per = ((area_w - gap * (n - 1)) / n).clamp(60, 210);
        let total = per * n + gap * (n - 1);
        let mut tx = t0 + (area_w - total) / 2;
        for (i, _t) in shell.tasks.iter().enumerate() {
            tasks.push((Rect::new(tx, 5, per, 30), i));
            tx += per + gap;
        }
    }
    let logo = Rect::new(14, 8, 26, 24);
    Ui { tasks, pager, plus, right, logo }
}

fn dome_spans() -> Vec<(i32, i32, i32)> {
    let cx = DOCK_W / 2;
    let cy = DOCK_CY_OFF;
    let r = 285;
    let mut spans = Vec::new();
    for y in 0..DOCK_H {
        let dy = y - cy;
        let d2 = (r * r - dy * dy).max(0);
        let hw = (d2 as f64).sqrt() as i32;
        let x0 = (cx - hw).max(0);
        let x1 = (cx + hw).min(DOCK_W);
        if x1 > x0 {
            spans.push((y, x0, x1 - x0));
        }
    }
    spans
}

fn rounded_spans(w: i32, h: i32, r: i32) -> Vec<(i32, i32, i32)> {
    let mut spans = Vec::new();
    for y in 0..h {
        let dy = if y < r { r - y } else if y >= h - r { y - (h - r - 1) } else { 0 };
        let dx = ((r * r - dy * dy).max(0)) as f64;
        let inr = (dx.sqrt()) as i32;
        let x0 = if y < r || y >= h - r { r - inr } else { 0 };
        let x1 = if y < r || y >= h - r { w - (r - inr) } else { w };
        if x1 > x0 {
            spans.push((y, x0, x1 - x0));
        }
    }
    spans
}

fn dock_buttons() -> Vec<(Rect, DockHit)> {
    let cx = DOCK_W / 2;
    let cy = DOCK_CY_OFF; // relative to window top
    let mut out = Vec::new();
    let mut place = |out: &mut Vec<(Rect, DockHit)>, angle_deg: f32, r: i32, size: i32, hit: DockHit| {
        let a = angle_deg as f32 * std::f32::consts::PI / 180.0;
        let x = cx + ((r as f32) * a.sin()) as i32 - size / 2;
        let y = cy - ((r as f32) * a.cos()) as i32 - size / 2;
        out.push((Rect::new(x, y, size, size), hit));
    };
    let inner = [
        (-38.0f32, DockHit::Files, 44),
        (-19.0, DockHit::Settings, 44),
        (0.0, DockHit::Brave, 64),
        (19.0, DockHit::Terminal, 44),
        (38.0, DockHit::Grid, 44),
    ];
    for (a, hit, size) in inner {
        place(&mut out, a, DOCK_R_INNER, size, hit);
    }
    let outer = [
        (-34.0f32, DockHit::Moon),
        (-17.0, DockHit::Wifi),
        (17.0, DockHit::PowerBtn),
        (34.0, DockHit::Lock),
    ];
    for (a, hit) in outer {
        place(&mut out, a, DOCK_R_OUTER, 34, hit);
    }
    out
}

// -------------------------------------------------------------- drawing

fn draw_panel(app: &mut App, shell: &Shell, cv: &mut Canvas, ui: &Ui) {
    let comp = app.composited;
    let acc = app.accent;
    cv.fill([0, 0, 0, 0]);
    let a = if comp { 225u8 } else { 255u8 };
    if std::env::var("BRAVOS_RED_TEST").is_ok() {
        cv.rect(0, 0, cv.w, PANEL_H, [255, 0, 0, 255]);
    } else {
        cv.rect(0, 0, cv.w, PANEL_H, [24, 24, 34, 255]);
    }
    cv.rect(0, PANEL_H - 1, cv.w, 1, [255, 255, 255, 34]);

    // logo + wordmark
    if let Some(icon) = app.icons.get("brave", 24, 24) {
        cv.image(&icon, ui.logo.x, ui.logo.y);
    }
    cv.rect(46, 10, 1, 20, [255, 255, 255, 40]);
    app.fonts.draw(cv, 56.0, 11.0, 14, [240, 240, 245, 255], true, "BraveOS");

    // pager
    for (r, n) in &ui.pager {
        let active = (*n - 1) == shell.current;
        if active {
            cv.rounded(r.x, r.y, r.w, r.h, 8, [acc[0], acc[1], acc[2], 235]);
            app.fonts.draw_centered(cv, *r, 13, [255, 255, 255, 255], true, &n.to_string());
        } else {
            cv.rounded(r.x, r.y, r.w, r.h, 8, [255, 255, 255, 28]);
            cv.rounded_outline(r.x, r.y, r.w, r.h, 8, [255, 255, 255, 40]);
            app.fonts.draw_centered(cv, *r, 13, [200, 200, 210, 255], false, &n.to_string());
        }
    }
    let p = ui.plus;
    cv.rounded(p.x, p.y, p.w, p.h, 8, [255, 255, 255, 20]);
    app.fonts.draw_centered(cv, p, 15, [200, 200, 210, 255], false, "+");

    // tasks
    for (r, i) in &ui.tasks {
        let t = &shell.tasks[*i];
        let is_active = t.win == shell.active;
        let (bg, ring) = if is_active {
            ([acc[0], acc[1], acc[2], 92], [acc[0], acc[1], acc[2], 235])
        } else {
            ([255, 255, 255, 30], [255, 255, 255, 55])
        };
        cv.rounded(r.x, r.y, r.w, r.h, 10, bg);
        cv.rounded_outline(r.x, r.y, r.w, r.h, 10, ring);
        let icon_name = match t.class.to_lowercase().as_str() {
            "brave" | "brave-browser" | "brave-browser-stable" => "brave",
            "xterm" | "xt" => "terminal",
            "pcmanfm" | "pcmanfm-qt" => "folder",
            "bsettings" => "settings",
            "org.gnome.nautilus" | "nautilus" => "folder",
            _ => "",
        };
        let mut tx = r.x + 8;
        if let Some(icon) = app.icons.get(if icon_name.is_empty() { "app" } else { icon_name }, 18, 18) {
            cv.image(&icon, tx, r.y + 6);
        } else {
            cv.circle(tx + 9, r.y + 15, 8, [acc[0], acc[1], acc[2], 200]);
        }
        tx += 24;
        let maxw = r.w - (tx - r.x) - 8;
        let title = app.fonts.ellipsize(12, &t.title, false, maxw as f32);
        app.fonts.draw(cv, tx as f32, (r.y + 8) as f32, 12, [235, 235, 240, 255], is_active, &title);
    }

    // right side: wifi, bt, battery, clock, power
    for (r, kind) in &ui.right {
        match *kind {
            "sep" => cv.rect(r.x, r.y, 1, r.h, [255, 255, 255, 40]),
            "wifi" => {
                let dim = !matches!(shell.wifi.as_deref(), Some("connected"));
                if let Some(icon) = app.icons.get("wifi", 22, 22) {
                    cv.image_tinted(&icon, r.x, r.y, [235, 235, 240, if dim { 90 } else { 255 }]);
                }
            }
            "bt" => {
                let alpha = if shell.bt { 255u8 } else { 90u8 };
                if let Some(icon) = app.icons.get("bluetooth", 20, 20) {
                    cv.image_tinted(&icon, r.x, r.y, [235, 235, 240, alpha]);
                }
            }
            "battery" => {
                if let Some((cap, _charging)) = shell.battery {
                    if let Some(icon) = app.icons.get("battery", 20, 20) {
                        cv.image_tinted(&icon, r.x, r.y + 4, [235, 235, 240, 255]);
                    }
                    let txt = format!("{}%", cap);
                    app.fonts.draw(cv, (r.x + 24) as f32, (r.y + 6) as f32, 13, [235, 235, 240, 255], false, &txt);
                }
            }
            "clock" => {
                app.fonts.draw_centered(cv, *r, 13, [235, 235, 240, 255], false, &shell.clock);
            }
            "power" => {
                if let Some(icon) = app.icons.get("power", 22, 22) {
                    cv.image_tinted(&icon, r.x, r.y, [235, 235, 240, 255]);
                }
            }
            _ => {}
        }
    }
}

fn draw_dock(app: &mut App, shell: &Shell, cv: &mut Canvas) {
    let comp = app.composited;
    let acc = app.accent;
    cv.fill([0, 0, 0, 0]);
    let cx = cv.w / 2;
    let cy = DOCK_CY_OFF;
    let r = 285;
    // frosted dome: radial falloff; without a compositor alpha is ignored,
    // so we vary COLOR (not just alpha) to keep depth on 24-bit screens
    let a = if comp { 135u8 } else { 235u8 };
    for yy in 0..cv.h {
        for xx in 0..cv.w {
            let dx = (xx - cx) as f32;
            let dy = (yy - cy) as f32;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= r as f32 {
                let t = d / r as f32; // 0 center .. 1 rim
                let alpha = (a as f32 * (0.55 + 0.45 * t * t)) as u8;
                // color varies: lighter lavender near the icon band, darker rim
                let cr = (30.0 - 18.0 * t) as u8;
                let cg = (30.0 - 18.0 * t) as u8;
                let cb = (48.0 - 26.0 * t) as u8;
                cv.px(xx, yy, [cr, cg, cb, alpha]);
            }
        }
    }
    cv.circle_outline(cx, cy, r, [255, 255, 255, 70]);
    cv.circle_outline(cx, cy, r - 10, [255, 255, 255, 26]);

    let buttons = dock_buttons();
    for (rect, hit) in &buttons {
        let size = rect.w;
        let center = rect.center();
        if *hit == DockHit::Brave {
            cv.glow(center.0, center.1, size / 2 + 9, acc);
            cv.circle_outline(center.0, center.1, size / 2 + 4, [acc[0], acc[1], acc[2], 220]);
            cv.circle(center.0, center.1, size / 2, [18, 18, 28, 245]);
            let isz = size - 14;
            if let Some(icon) = shell.brave.as_ref().map(|b| b.resized(isz, isz)) {
                cv.image(&icon, center.0 - isz / 2, center.1 - isz / 2);
            } else if let Some(icon) = app.icons.get("brave", isz, isz) {
                cv.image(&icon, center.0 - isz / 2, center.1 - isz / 2);
            }
        } else {
            cv.circle(center.0, center.1, size / 2, [255, 255, 255, 26]);
            cv.circle_outline(center.0, center.1, size / 2, [255, 255, 255, 80]);
            let (name, tint) = match hit {
                DockHit::Files => ("folder", None),
                DockHit::Settings => ("settings", None),
                DockHit::Terminal => ("terminal", None),
                DockHit::Grid => ("grid", None),
                DockHit::Moon => ("moon", Some([190, 190, 215, 255])),
                DockHit::Wifi => ("wifi", Some([235, 235, 240, 255])),
                DockHit::PowerBtn => ("power", Some([235, 235, 240, 255])),
                DockHit::Lock => ("lock", Some([235, 235, 240, 255])),
                DockHit::Brave | DockHit::None => ("app", None),
            };
            let sz = size - 14;
            if let Some(icon) = app.icons.get(name, sz, sz) {
                match tint {
                    Some(t) => cv.image_tinted(&icon, center.0 - sz / 2, center.1 - sz / 2, t),
                    None => cv.image(&icon, center.0 - sz / 2, center.1 - sz / 2),
                }
            }
        }
    }
    let _ = shell;
}

fn draw_grid(app: &mut App, shell: &Shell, cv: &mut Canvas) {
    let comp = app.composited;
    cv.fill([0, 0, 0, 0]);
    let a = if comp { 240u8 } else { 255u8 };
    cv.rounded(0, 0, cv.w, cv.h, 18, [12, 12, 19, a]);
    cv.rounded_outline(0, 0, cv.w, cv.h, 18, [255, 255, 255, 30]);
    app.fonts.draw_centered(cv, Rect::new(0, 14, cv.w, 24), 15, [240, 240, 245, 255], true, &app.tr.t("app_grid"));
    let acc = app.accent;
    let cols = 5usize;
    let cw = cv.w / cols as i32;
    let ch = 96;
    for (i, e) in shell.apps.iter().enumerate().take(24) {
        let col = (i % cols) as i32;
        let row = (i / cols) as i32;
        let x = col * cw + 8;
        let y = 46 + row * ch;
        let cell = Rect::new(x, y, cw - 16, ch - 10);
        cv.rounded(cell.x, cell.y, cell.w, cell.h, 12, [255, 255, 255, 22]);
        let isz = 44;
        let icx = cell.x + (cell.w - isz) / 2;
        if let Some(icon) = app.icons.get(&e.icon, isz, isz).or_else(|| app.icons.get("app", isz, isz)) {
            cv.image(&icon, icx, cell.y + 8);
        } else {
            cv.circle(cell.x + cell.w / 2, cell.y + 8 + isz / 2, isz / 2, [acc[0], acc[1], acc[2], 170]);
        }
        let label = app.fonts.ellipsize(11, &e.name, false, (cell.w - 8) as f32);
        app.fonts.draw_centered(cv, Rect::new(cell.x, cell.y + 56, cell.w, 16), 11, [225, 225, 232, 255], false, &label);
    }
}

fn draw_power(app: &mut App, cv: &mut Canvas) {
    let comp = app.composited;
    cv.fill([0, 0, 0, 0]);
    let a = if comp { 242u8 } else { 255u8 };
    cv.rounded(0, 0, cv.w, cv.h, 16, [12, 12, 19, a]);
    cv.rounded_outline(0, 0, cv.w, cv.h, 16, [255, 255, 255, 30]);
    let items = [
        ("lock", app.tr.t("pm_lock")),
        ("moon", app.tr.t("pm_suspend")),
        ("restart", app.tr.t("pm_restart")),
        ("power", app.tr.t("pm_shutdown")),
    ];
    for (i, (icon, label)) in items.iter().enumerate() {
        let y = 12 + i as i32 * 54;
        let r = Rect::new(10, y, cv.w - 20, 46);
        cv.rounded(r.x, r.y, r.w, r.h, 10, [255, 255, 255, 18]);
        if let Some(ic) = app.icons.get(icon, 22, 22) {
            cv.image(&ic, r.x + 14, r.y + 12);
        }
        app.fonts.draw(cv, (r.x + 48) as f32, (r.y + 14) as f32, 13, [235, 235, 240, 255], false, label);
    }
}

// ------------------------------------------------------------- handlers

type Shared = Rc<RefCell<Shell>>;

struct PanelH(Shared);
impl Handler for PanelH {
    fn draw(&mut self, app: &mut App, cv: &mut Canvas) {
        let sh = self.0.borrow();
        let ui = panel_layout(cv.w, &sh);
        draw_panel(app, &sh, cv, &ui);
    }
    fn pointer(&mut self, app: &mut App, kind: PtrKind, x: i32, y: i32) {
        if !matches!(kind, PtrKind::Down(1)) {
            return;
        }
        let (task_hit, pager_hit, plus_hit, right_hit, power_win, desktops) = {
            let sh = self.0.borrow();
            let ui = panel_layout(app.screen_w, &sh);
            let task_hit = ui.tasks.iter().find(|(r, _)| r.contains(x, y)).map(|(_, i)| sh.tasks[*i].win);
            let task_active = ui.tasks.iter().find(|(r, _)| r.contains(x, y))
                .map(|(_, i)| sh.tasks[*i].win == sh.active);
            let pager_hit = ui.pager.iter().find(|(r, _)| r.contains(x, y)).map(|(_, n)| *n);
            let plus_hit = ui.plus.contains(x, y);
            let desktops = sh.desktops;
            let right_hit = ui.right.iter().find(|(r, _)| r.contains(x, y)).map(|(_, k)| *k);
            (
                task_hit.map(|w| (w, task_active.unwrap_or(false))),
                pager_hit,
                plus_hit,
                right_hit,
                sh.power,
                desktops,
            )
        };
        if let Some((win, active)) = task_hit {
            if active {
                wm_msg("_NET_WM_STATE", win, 1); // iconify via WM_STATE
            } else {
                wm_msg("_NET_ACTIVE_WINDOW", win, 0);
            }
            return;
        }
        if let Some(n) = pager_hit {
            wm_msg("_NET_CURRENT_DESKTOP", n - 1, 0);
            return;
        }
        if plus_hit {
            wm_msg("_NET_NUMBER_OF_DESKTOPS", desktops + 1, 0);
            return;
        }
        if let Some(kind) = right_hit {
            match kind {
                "power" => {
                    app.set_geom(power_win, app.screen_w - POWER_W - 20, app.screen_h - DOCK_H - 40 - POWER_H - 8, POWER_W, POWER_H);
                }
                "clock" => spawn(&["bsettings", "time"]),
                "battery" => spawn(&["bsettings", "power"]),
                "wifi" => spawn(&["bsettings", "network"]),
                _ => {}
            }
        }
    }
    fn tick(&mut self, app: &mut App) {
        let mut reload = false;
        {
            let mut sh = self.0.borrow_mut();
            sh.status_poll();
            reload = sh.stamp != config_stamp();
            if reload {
                sh.stamp = config_stamp();
            }
        }
        {
            let conn = open_conn();
            self.0.borrow_mut().refresh_tasks(&conn);
            if std::env::var("BRAVOS_DEBUG_TASKS").is_ok() {
                let sh = self.0.borrow();
                eprintln!("bshell: tasks={} desktops={} current={} active={}", sh.tasks.len(), sh.desktops, sh.current, sh.active);
            }
        }
        if reload {
            app.cfg = Config::load();
            app.tr = bui::i18n::Tr::load(&app.cfg.language);
            app.accent = Config::hex_rgb(&app.cfg.accent);
        }
        for id in app.win_ids() {
            if id != self.0.borrow().panel {
                continue;
            }
            app.redraw(id);
        }
    }
    fn property(&mut self, app: &mut App, _atom_id: u32) {
        let conn = open_conn();
        self.0.borrow_mut().refresh_tasks(&conn);
        let panel = self.0.borrow().panel;
        app.redraw(panel);
    }
}

struct DockH(Shared);
impl Handler for DockH {
    fn draw(&mut self, app: &mut App, cv: &mut Canvas) {
        let sh = self.0.borrow();
        draw_dock(app, &sh, cv);
    }
    fn pointer(&mut self, app: &mut App, kind: PtrKind, x: i32, y: i32) {
        if !matches!(kind, PtrKind::Down(1)) {
            return;
        }
        for (r, hit) in dock_buttons() {
            if r.contains(x, y) {
                match hit {
                    DockHit::Files => spawn(&["pcmanfm"]),
                    DockHit::Settings => spawn(&["bsettings"]),
                    DockHit::Brave => spawn(&["brave", "--maximized", "--no-first-run"]),
                    DockHit::Terminal => spawn(&["xterm", "-fa", "Monospace", "-fs", "11"]),
                    DockHit::Grid => {
                        let opening = !self.0.borrow().grid_open;
                        self.0.borrow_mut().grid_open = opening;
                        let id = self.0.borrow().grid;
                        if opening {
                            app.set_geom(id, (app.screen_w - GRID_W) / 2, (app.screen_h - GRID_H) / 2 - 40, GRID_W, GRID_H);
                        } else {
                            app.set_geom(id, -GRID_W * 2, (app.screen_h - GRID_H) / 2 - 40, GRID_W, GRID_H);
                        }
                    }
                    DockHit::Moon => {
                        let mut cfg = Config::load();
                        cfg.dark = !cfg.dark;
                        cfg.wallpaper = if cfg.dark { "braveos.png".into() } else { "braveos-light.png".into() };
                        cfg.save();
                    }
                    DockHit::Wifi => spawn(&["bsettings", "network"]),
                    DockHit::PowerBtn => {
                        let id = self.0.borrow().power;
                        app.set_geom(id, app.screen_w - POWER_W - 20, app.screen_h - DOCK_H - 40 - POWER_H - 8, POWER_W, POWER_H);
                    }
                    DockHit::Lock => spawn(&["xtrlock"]),
                    DockHit::None => {}
                }
                return;
            }
        }
    }
    fn tick(&mut self, app: &mut App) {
        let changed = self.0.borrow().stamp != config_stamp();
        if changed {
            self.0.borrow_mut().stamp = config_stamp();
            app.cfg = Config::load();
            app.tr = bui::i18n::Tr::load(&app.cfg.language);
            app.accent = Config::hex_rgb(&app.cfg.accent);
            for id in app.win_ids() {
                app.redraw(id);
            }
        }
    }
}

struct GridH(Shared);
impl Handler for GridH {
    fn draw(&mut self, app: &mut App, cv: &mut Canvas) {
        let sh = self.0.borrow();
        draw_grid(app, &sh, cv);
    }
    fn pointer(&mut self, app: &mut App, kind: PtrKind, x: i32, y: i32) {
        match kind {
            PtrKind::Down(1) => {
                let sh = self.0.borrow();
                let cols = 5usize;
                let cw = GRID_W / cols as i32;
                let ch = 96;
                let mut launched: Option<String> = None;
                for (i, e) in sh.apps.iter().enumerate().take(24) {
                    let col = (i % cols) as i32;
                    let row = (i / cols) as i32;
                    let cell = Rect::new(col * cw + 8, 46 + row * ch, cw - 16, ch - 10);
                    if cell.contains(x, y) {
                        launched = Some(e.exec.clone());
                        break;
                    }
                }
                if let Some(exec) = launched {
                    drop(sh);
                    self.0.borrow_mut().grid_open = false;
                    let id = self.0.borrow().grid;
                    app.set_geom(id, -GRID_W * 2, (app.screen_h - GRID_H) / 2 - 40, GRID_W, GRID_H);
                    let parts: Vec<String> = exec
                        .split_whitespace()
                        .filter(|p| !(p.starts_with('%') && p.len() == 2))
                        .map(String::from)
                        .collect();
                    let refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
                    if !refs.is_empty() {
                        spawn(&refs);
                    }
                }
            }
            PtrKind::Down(_) => {
                self.0.borrow_mut().grid_open = false;
                let id = self.0.borrow().grid;
                app.set_geom(id, -GRID_W * 2, (app.screen_h - GRID_H) / 2 - 40, GRID_W, GRID_H);
            }
            _ => {}
        }
    }
    fn key(&mut self, app: &mut App, ks: u32, _state: u16) -> bool {
        if ks == 0xff1b {
            self.0.borrow_mut().grid_open = false;
            let id = self.0.borrow().grid;
            app.set_geom(id, -GRID_W * 2, (app.screen_h - GRID_H) / 2 - 40, GRID_W, GRID_H);
            return true;
        }
        false
    }
}

struct PowerH(Shared);
impl Handler for PowerH {
    fn draw(&mut self, app: &mut App, cv: &mut Canvas) {
        draw_power(app, cv);
    }
    fn pointer(&mut self, app: &mut App, kind: PtrKind, x: i32, _y: i32) {
        if !matches!(kind, PtrKind::Down(1)) {
            return;
        }
        let y = _y;
        let idx = ((y - 12) / 54).max(0) as usize;
        let id = self.0.borrow().power;
        app.set_geom(id, -POWER_W * 2, -POWER_H * 2, POWER_W, POWER_H);
        match idx {
            0 => spawn(&["xtrlock"]),
            1 => spawn(&["sh", "-c", "systemctl suspend -i || xset dpms force suspend"]),
            2 => spawn(&["sh", "-c", "systemctl reboot -i"]),
            3 => spawn(&["sh", "-c", "systemctl poweroff -i"]),
            _ => {}
        }
    }
}

// ------------------------------------------------- WM client messages

fn wm_msg(name: &str, data0: u32, _flag: u32) {
    let conn = open_conn();
    let a = atom(&conn, name);
    let root = conn.setup().roots.first().map(|s| s.root).unwrap_or(0);
    let ev = ClientMessageEvent::new(32, root, a, [data0, 0, 0, 0, 0]);
    let _ = conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        &ev,
    );
    let _ = conn.flush();
}

thread_local! {
    static CONN: RefCell<Option<std::rc::Rc<RustConnection>>> = RefCell::new(None);
}

fn open_conn() -> std::rc::Rc<RustConnection> {
    CONN.with(|c| {
        {
            let mut slot = c.borrow_mut();
            if slot.is_none() {
                *slot = RustConnection::connect(None).ok().map(|(conn, _)| std::rc::Rc::new(conn));
            }
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

// ---------------------------------------------------------------- main

fn main() {
    let mut app = App::connect().expect("bshell: X11 connect");
    let root = app.root;
    shell_main(app, root);
}

fn shell_main(mut app: App, root: u32) {
    let brave_icon = Image::decode("/usr/share/bravos/icons/icon-brave.png")
        .or_else(|| Image::decode("/usr/share/icons/hicolor/48x48/apps/icon-brave.png"));

    let shell = Rc::new(RefCell::new(Shell {
        panel: 0,
        dock: 0,
        grid: 0,
        power: 0,
        grid_open: false,
        power_open: false,
        desktops: 2,
        current: 0,
        tasks: Vec::new(),
        active: 0,
        wifi: None,
        bt: false,
        battery: None,
        clock: String::new(),
        apps: Vec::new(),
        stamp: config_stamp(),
        brave: brave_icon,
        root,
    }));
    shell.borrow_mut().status_poll();
    shell.borrow_mut().apps_scan();

    let sw = app.screen_w;
    let shh = app.screen_h;

    let strut = atom(&app.conn, "_NET_WM_STRUT_PARTIAL");

    // -- panel (managed; strut set before mapping so the WM sees it at manage time)
    let panel = app.add_win(
        WinOpts {
            x: 0,
            y: 0,
            w: sw,
            h: PANEL_H,
            override_redirect: false,
            argb: true,
            window_type: "dock",
            class: "bshell-panel",
            title: "bravos-panel",
            event_mask: bui::xapp::EventMask::EXPOSURE
                | bui::xapp::EventMask::BUTTON_PRESS
                | bui::xapp::EventMask::BUTTON_RELEASE
                | bui::xapp::EventMask::POINTER_MOTION
                | bui::xapp::EventMask::ENTER_WINDOW
                | bui::xapp::EventMask::LEAVE_WINDOW
                | bui::xapp::EventMask::PROPERTY_CHANGE
                | bui::xapp::EventMask::STRUCTURE_NOTIFY,
        },
        Box::new(PanelH(shell.clone())),
    );
    app.conn
        .change_property32(
            PropMode::REPLACE,
            panel,
            strut,
            6u32,
            &[0u32, 0, PANEL_H as u32, 0, 0, 0, 0, sw as u32, 0, 0, 0, sw as u32],
        )
        .ok();

    // -- dock (managed, bottom-center, partial strut over the dome area)
    let dock = app.add_win(
        WinOpts {
            x: (sw - DOCK_W) / 2,
            y: shh - DOCK_H,
            w: DOCK_W,
            h: DOCK_H,
            override_redirect: false,
            argb: true,
            window_type: "dock",
            class: "bshell-dock",
            title: "bravos-dock",
            event_mask: bui::xapp::EventMask::EXPOSURE
                | bui::xapp::EventMask::BUTTON_PRESS
                | bui::xapp::EventMask::BUTTON_RELEASE
                | bui::xapp::EventMask::POINTER_MOTION
                | bui::xapp::EventMask::STRUCTURE_NOTIFY,
        },
        Box::new(DockH(shell.clone())),
    );
    app.conn
        .change_property32(
            PropMode::REPLACE,
            dock,
            strut,
            6u32,
            &[
                0, 0, 0, 150,
                0, 0, (sw / 2 - DOCK_W / 2) as u32, (sw / 2 + DOCK_W / 2) as u32,
                0, 0, (shh - 150) as u32, shh as u32,
            ],
        )
        .ok();

    // -- app grid (parked off-screen until opened)
    let grid = app.add_win(
        WinOpts {
            x: -GRID_W * 2,
            y: (shh - GRID_H) / 2 - 40,
            w: GRID_W,
            h: GRID_H,
            override_redirect: false,
            argb: true,
            window_type: "dock",
            class: "bshell-grid",
            title: "bravos-grid",
            event_mask: bui::xapp::EventMask::EXPOSURE
                | bui::xapp::EventMask::BUTTON_PRESS
                | bui::xapp::EventMask::BUTTON_RELEASE
                | bui::xapp::EventMask::KEY_PRESS
                | bui::xapp::EventMask::STRUCTURE_NOTIFY,
        },
        Box::new(GridH(shell.clone())),
    );

    // -- power menu (parked off-screen until opened)
    let power = app.add_win(
        WinOpts {
            x: -POWER_W * 2,
            y: -POWER_H * 2,
            w: POWER_W,
            h: POWER_H,
            override_redirect: false,
            argb: true,
            window_type: "dock",
            class: "bshell-power",
            title: "bravos-power",
            event_mask: bui::xapp::EventMask::EXPOSURE
                | bui::xapp::EventMask::BUTTON_PRESS
                | bui::xapp::EventMask::STRUCTURE_NOTIFY,
        },
        Box::new(PowerH(shell.clone())),
    );

    {
        let mut s = shell.borrow_mut();
        s.panel = panel;
        s.dock = dock;
        s.grid = grid;
        s.power = power;
    }

    // Without a compositor the ARGB alpha is ignored: shape the windows so
    // transparent areas are truly invisible.
    if !app.composited {
        app.set_shape_spans(dock, &dome_spans());
        app.set_shape_spans(grid, &rounded_spans(GRID_W, GRID_H, 18));
        app.set_shape_spans(power, &rounded_spans(POWER_W, POWER_H, 16));
    }

    println!("bshell: ready sw={} sh={} composited={} argb_visual={:?}", sw, shh, app.composited, app.argb_visual.is_some());
    app.conn.flush().ok();

    app.run_until(&|_app| false);
}
