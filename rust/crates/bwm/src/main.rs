//! bwm — BraveOS window manager (pure Rust, X11, x11rb).
//!
//! Floating WM with workspaces, EWMH essentials, comprehensive key/mouse
//! shortcuts, focus borders, alt-tab switcher and a root context menu.
//! Replaces Openbox. Config: ~/.config/bravos/config.json (shortcuts).

use std::collections::HashMap;
use std::process::{Command, Stdio};

use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::shape::ConnectionExt as ShapeExt;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as WrapExt;

use bui::canvas::Canvas;
use bui::config::Config;
use bui::font::Fonts;
use bui::geom::Rect;

#[derive(Clone)]
struct Client {
    win: Window,
    frame: Window,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    desktop: u32,
    maximized: bool,
    fullscreen: bool,
    minimized: bool,
    title: String,
    class: String,
    pre_max: Option<(i32, i32, i32, i32)>,
}

#[derive(Clone, Copy)]
struct KeysymEntry {
    keysym: u32,
    keycode: u8,
}

struct Wm {
    conn: RustConnection,
    root: Window,
    screen_w: i32,
    screen_h: i32,
    clients: Vec<Client>,
    focused: Option<Window>,
    current_desktop: u32,
    desktops: u32,
    workarea: (i32, i32, i32, i32),
    show_desktop_mode: bool,
    cfg: Config,
    fonts: Fonts,
    atoms: HashMap<&'static str, u32>,
    keysym_index: HashMap<u32, u8>,
    keysyms: Vec<u32>,
    keycode_min: u32,
    keysyms_per_keycode: usize,
    menu_open: bool,
    menu_sel: i32,
    menu_items: Vec<(&'static str, &'static str)>, // (key, arg)
    menu_win: Window,
    menu_cvs: Option<Canvas>,
    gc: Gcontext,
    border_glow: bool,
    switcher_state: Option<(usize, Vec<Window>)>,
    // pass-through windows (panels/docks), with a desktop flag
    shell_wins: Vec<(Window, bool)>,
    top_strut: i32,
    bottom_strut: i32,
    utf8_atom: u32,
    argb_visual: Option<u32>,
}

const ATOM_CARDINAL: u32 = 6;
const ATOM_ATOM: u32 = 4;
const ATOM_UTF8: u32 = 93; // interned at startup
const BORDER: i32 = 2;
// raw modifier bits (ModMask field is private in x11rb 0.14)
const MOD_SHIFT: u16 = 1;
const MOD_LOCK: u16 = 2;
const MOD_CTRL: u16 = 4;
const MOD_ALT: u16 = 8;
const MOD_NUM: u16 = 16;
const MOD_SUPER: u16 = 64;

fn intern(conn: &RustConnection, name: &str) -> u32 {
    conn.intern_atom(false, name.as_bytes())
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.atom)
        .unwrap_or(0)
}

fn find_argb_visual(conn: &RustConnection) -> Option<u32> {
    let scr = &conn.setup().roots[0];
    for depth in &scr.allowed_depths {
        if depth.depth == 32 {
            for v in &depth.visuals {
                if v.class == VisualClass::TRUE_COLOR {
                    return Some(v.visual_id);
                }
            }
        }
    }
    None
}

fn parse_combo(combo: &str) -> (u16, u32) {
    // returns (modifier_bits, keysym)
    let mut mods = 0u16;
    let mut key = "";
    for part in combo.split('+') {
        let p = part.trim().to_lowercase();
        match p.as_str() {
            "super" | "mod4" => mods |= MOD_SUPER,
            "alt" | "mod1" => mods |= MOD_ALT,
            "ctrl" | "control" => mods |= MOD_CTRL,
            "shift" => mods |= MOD_SHIFT,
            _ => key = part.trim(),
        }
    }
    let ks = keysym_of_name(key);
    (mods, ks)
}

fn keysym_of_name(name: &str) -> u32 {
    let n = name.to_lowercase();
    match n.as_str() {
        "enter" | "return" | "kp_enter" => 0xff0d,
        "tab" => 0xff09,
        "space" => 0x20,
        "escape" => 0xff1b,
        "delete" => 0xffff,
        "backspace" => 0xff08,
        "left" | "kp_left" => 0xff51,
        "up" | "kp_up" => 0xff52,
        "right" | "kp_right" => 0xff53,
        "down" | "kp_down" => 0xff54,
        "home" => 0xff50,
        "end" => 0xff57,
        "pageup" | "prior" => 0xff55,
        "pagedown" | "next" => 0xff56,
        "print" => 0xff61,
        "insert" => 0xff63,
        s if s.len() == 1 && s.chars().next().unwrap().is_ascii_digit() => {
            s.chars().next().unwrap() as u32
        }
        s if s.len() == 1 && s.chars().next().unwrap().is_ascii_alphabetic() => {
            s.chars().next().unwrap().to_ascii_lowercase() as u32
        }
        s if s.len() == 1 => s.chars().next().unwrap() as u32,
        s if s.starts_with('f') && s[1..].chars().all(|c| c.is_ascii_digit()) => {
            let num: u32 = s[1..].parse().unwrap_or(1);
            0xffbd + num // F1 = 0xffbe
        }
        _ => 0,
    }
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
        0xff50 => "Home".into(),
        0xff57 => "End".into(),
        0xff55 => "PgUp".into(),
        0xff56 => "PgDn".into(),
        0xff61 => "Print".into(),
        0xff63 => "Insert".into(),
        k if (0xffbe..=0xffd2).contains(&k) => format!("F{}", k - 0xffbd + 1),
        k if (0x61..=0x7a).contains(&k) => (k as u8 as char).to_string(),
        k if (0x41..=0x5a).contains(&k) => (k as u8 as char).to_string(),
        k if k >= 0x20 && k < 0x7f => (k as u8 as char).to_string(),
        _ => format!("Key{:04x}", ks),
    }
}

impl Wm {
    fn connect() -> Result<Wm, String> {
        let (conn, screen) = RustConnection::connect(None).map_err(|e| e.to_string())?;
        let scr = &conn.setup().roots[screen];
        let root = scr.root;
        let (screen_w, screen_h) = (scr.width_in_pixels as i32, scr.height_in_pixels as i32);

        // simpler: re-intern by name
        let mut atoms = HashMap::<&'static str, u32>::new();
        let names = [
            ("_NET_SUPPORTED", "_NET_SUPPORTED"), ("_NET_CLIENT_LIST", "_NET_CLIENT_LIST"),
            ("_NET_CLIENT_LIST_STACKING", "_NET_CLIENT_LIST_STACKING"),
            ("_NET_ACTIVE_WINDOW", "_NET_ACTIVE_WINDOW"),
            ("_NET_CURRENT_DESKTOP", "_NET_CURRENT_DESKTOP"),
            ("_NET_NUMBER_OF_DESKTOPS", "_NET_NUMBER_OF_DESKTOPS"),
            ("_NET_DESKTOP_NAMES", "_NET_DESKTOP_NAMES"),
            ("_NET_WORKAREA", "_NET_WORKAREA"), ("_NET_CLOSE_WINDOW", "_NET_CLOSE_WINDOW"),
            ("_NET_WM_DESKTOP", "_NET_WM_DESKTOP"), ("_NET_WM_STATE", "_NET_WM_STATE"),
            ("_NET_WM_STATE_HIDDEN", "_NET_WM_STATE_HIDDEN"),
            ("_NET_WM_STATE_FULLSCREEN", "_NET_WM_STATE_FULLSCREEN"),
            ("_NET_WM_STATE_MAXIMIZED_VERT", "_NET_WM_STATE_MAXIMIZED_VERT"),
            ("_NET_WM_STATE_MAXIMIZED_HORZ", "_NET_WM_STATE_MAXIMIZED_HORZ"),
            ("_NET_WM_WINDOW_TYPE", "_NET_WM_WINDOW_TYPE"),
            ("_NET_WM_WINDOW_TYPE_DOCK", "_NET_WM_WINDOW_TYPE_DOCK"),
            ("_NET_WM_WINDOW_TYPE_DESKTOP", "_NET_WM_WINDOW_TYPE_DESKTOP"),
            ("_NET_WM_WINDOW_TYPE_NOTIFICATION", "_NET_WM_WINDOW_TYPE_NOTIFICATION"),
            ("_NET_WM_WINDOW_TYPE_SPLASH", "_NET_WM_WINDOW_TYPE_SPLASH"),
            ("_NET_WM_WINDOW_TYPE_COMBO", "_NET_WM_WINDOW_TYPE_COMBO"),
            ("_NET_WM_WINDOW_TYPE_DND", "_NET_WM_WINDOW_TYPE_DND"),
            ("_NET_WM_WINDOW_TYPE_UTILITY", "_NET_WM_WINDOW_TYPE_UTILITY"),
            ("_NET_WM_WINDOW_TYPE_MENU", "_NET_WM_WINDOW_TYPE_MENU"),
            ("_NET_WM_WINDOW_TYPE_TOOLBAR", "_NET_WM_WINDOW_TYPE_TOOLBAR"),
            ("_NET_WM_WINDOW_TYPE_DIALOG", "_NET_WM_WINDOW_TYPE_DIALOG"),
            ("_NET_WM_NAME", "_NET_WM_NAME"), ("_NET_WM_STRUT", "_NET_WM_STRUT"),
            ("_NET_WM_STRUT_PARTIAL", "_NET_WM_STRUT_PARTIAL"),
            ("WM_PROTOCOLS", "WM_PROTOCOLS"), ("WM_DELETE_WINDOW", "WM_DELETE_WINDOW"),
            ("WM_STATE", "WM_STATE"), ("WM_CLASS", "WM_CLASS"),
            ("_BRAVOS_SHELL_CMD", "_BRAVOS_SHELL_CMD"), ("_BRAVOS_CONFIG", "_BRAVOS_CONFIG"),
        ];
        for (k, v) in names {
            atoms.insert(k, intern(&conn, v));
        }
        let utf8_atom = intern(&conn, "UTF8_STRING");

        let gc = conn.generate_id().unwrap();
        conn.create_gc(gc, root, &CreateGCAux::new()).unwrap();

        let min_kc = conn.setup().min_keycode;
        let count = conn.setup().max_keycode - min_kc + 1;
        let (keysyms, kpk) = match conn.get_keyboard_mapping(min_kc, count) {
            Ok(cookie) => match cookie.reply() {
                Ok(r) => {
                    let kpk = r.keysyms.len() / count as usize;
                    (r.keysyms.clone(), kpk)
                }
                Err(_) => (Vec::new(), 0),
            },
            Err(_) => (Vec::new(), 0),
        };
        let mut keysym_index = HashMap::new();
        for (i, ks) in keysyms.iter().enumerate() {
            let kc = min_kc as u32 + (i / kpk.max(1)) as u32;
            if *ks != 0 && !keysym_index.contains_key(ks) {
                keysym_index.insert(*ks, kc as u8);
            }
        }

        let cfg = Config::load();
        let wa = (0, 0, screen_w, screen_h);

        let menu_win = conn.generate_id().unwrap();
        let argb_visual = find_argb_visual(&conn);

        Ok(Wm {
            utf8_atom,
            conn,
            root,
            screen_w,
            screen_h,
            clients: Vec::new(),
            focused: None,
            current_desktop: 0,
            desktops: cfg.workspaces.clamp(1, 9),
            workarea: wa,
            show_desktop_mode: false,
            cfg,
            fonts: Fonts::new(),
            atoms,
            keysym_index,
            keysyms,
            keycode_min: min_kc as u32,
            keysyms_per_keycode: kpk,
            menu_open: false,
            menu_sel: -1,
            menu_items: Vec::new(),
            menu_win,
            menu_cvs: None,
            gc,
            border_glow: true,
            switcher_state: None,
            shell_wins: Vec::new(),
            top_strut: 0,
            bottom_strut: 0,
            argb_visual,
        })
    }

    fn atom(&self, name: &str) -> u32 {
        self.atoms.get(name).copied().unwrap_or(0)
    }

    fn keycode_for(&self, keysym: u32) -> Option<u8> {
        self.keysym_index.get(&keysym).copied()
    }

    // ---------------------------------------------------------------- EWMH

    fn set_u32_prop(&self, win: Window, prop: &str, data: &[u32]) {
        self.conn
            .change_property32(PropMode::REPLACE, win, self.atom(prop), ATOM_CARDINAL, data)
            .ok();
    }

    fn set_atom_prop(&self, win: Window, prop: &str, data: &[u32]) {
        self.conn
            .change_property32(PropMode::REPLACE, win, self.atom(prop), ATOM_ATOM, data)
            .ok();
    }

    fn set_window_prop(&self, win: Window, prop: &str, data: &[u32]) {
        self.conn
            .change_property32(PropMode::REPLACE, win, self.atom(prop), 33u32 /* XA_WINDOW */, data)
            .ok();
    }

    /// Keep panels/docks on top of every client frame. Called after any
    /// frame is mapped or raised, so app windows can never bury the shell.
    fn raise_shell(&self) {
        for &(w, is_desktop) in &self.shell_wins {
            let mode = if is_desktop { StackMode::BELOW } else { StackMode::ABOVE };
            self.conn
                .configure_window(w, &ConfigureWindowAux::new().stack_mode(mode))
                .ok();
        }
        self.conn.flush().ok();
    }

    fn update_client_list(&self) {
        let wins: Vec<u32> = self.clients.iter().map(|c| c.win).collect();
        let stacked: Vec<u32> = self.clients.iter().map(|c| c.win).collect();
        // EWMH: these are window lists - the property type must be XA_WINDOW
        // (writing CARDINAL made xprop/pagers print them as plain integers)
        self.set_window_prop(self.root, "_NET_CLIENT_LIST", &wins);
        self.set_window_prop(self.root, "_NET_CLIENT_LIST_STACKING", &stacked);
        let active = self.focused.unwrap_or(0);
        self.set_window_prop(self.root, "_NET_ACTIVE_WINDOW", &[active]);
    }

    fn update_desktop_props(&self) {
        self.set_u32_prop(self.root, "_NET_NUMBER_OF_DESKTOPS", &[self.desktops]);
        self.set_u32_prop(self.root, "_NET_CURRENT_DESKTOP", &[self.current_desktop]);
        let mut names: Vec<u8> = Vec::new();
        for i in 1..=self.desktops {
            names.extend_from_slice(format!("Workspace {}", i).as_bytes());
            names.push(0);
        }
        self.conn
            .change_property8(PropMode::REPLACE, self.root, self.atom("_NET_DESKTOP_NAMES"), self.utf8_atom, &names)
            .ok();
        for c in &self.clients {
            self.set_u32_prop(c.win, "_NET_WM_DESKTOP", &[c.desktop]);
        }
    }

    fn update_workarea(&mut self) {
        // union of struts: panel/dock reserve space via _NET_WM_STRUT_PARTIAL
        let mut wa = (0, 0, self.screen_w, self.screen_h);
        for c in &self.clients {
            let _ = c;
        }
        // struts are tracked via dedicated scan of unmanaged windows
        wa.1 = self.top_strut;
        wa.3 = self.screen_h - self.top_strut - self.bottom_strut;
        self.set_u32_prop(self.root, "_NET_WORKAREA", &[wa.0 as u32, wa.1 as u32, wa.2 as u32, wa.3 as u32]);
        self.workarea = wa;
    }

    // ------------------------------------------------------------- windows

    fn geom_for_new(&self) -> (i32, i32, i32, i32) {
        let (wx, wy, ww, wh) = self.workarea;
        // centered, 70% of workarea, like a modern default
        let w = ww * 7 / 10;
        let h = wh * 7 / 10;
        (wx + (ww - w) / 2, wy + (wh - h) / 2, w, h)
    }

    /// Position for a new client with a known size: center it on the ACTUAL
    /// size (not the default) and clamp inside the workarea so windows never
    /// hang off the screen or cover the panel/dock bands.
    fn place_for_size(&self, w: i32, h: i32) -> (i32, i32) {
        let (wx, wy, ww, wh) = self.workarea;
        let w = w.min(ww).max(1);
        let h = h.min(wh).max(1);
        let x = (wx + (ww - w) / 2).max(wx);
        let y = (wy + (wh - h) / 2).max(wy);
        (x, y)
    }

    fn window_type_skip(&self, win: Window) -> bool {
        let ty = self.atom("_NET_WM_WINDOW_TYPE");
        let Some(r) = self.conn.get_property(false, win, ty, 0u32, 0, 32).ok().and_then(|c| c.reply().ok()) else {
            return false;
        };
        let vals: Vec<u32> = r.value32().map(|it| it.collect()).unwrap_or_default();
        for skip in [
            "_NET_WM_WINDOW_TYPE_DOCK",
            "_NET_WM_WINDOW_TYPE_DESKTOP",
            "_NET_WM_WINDOW_TYPE_NOTIFICATION",
            "_NET_WM_WINDOW_TYPE_SPLASH",
            "_NET_WM_WINDOW_TYPE_COMBO",
            "_NET_WM_WINDOW_TYPE_DND",
        ] {
            if vals.contains(&self.atom(skip)) {
                return true;
            }
        }
        false
    }

    fn read_title(&self, win: Window) -> String {
        let na = self.atom("_NET_WM_NAME");
        if let Some(r) = self.conn.get_property(false, win, na, 0u32, 0, 1024).ok().and_then(|c| c.reply().ok()) {
            if r.value_len > 0 {
                return String::from_utf8_lossy(&r.value).to_string();
            }
        }
        let wa = self.atom("WM_NAME");
        if let Some(r) = self.conn.get_property(false, win, wa, 0u32, 0, 1024).ok().and_then(|c| c.reply().ok()) {
            if r.value_len > 0 {
                return String::from_utf8_lossy(&r.value).to_string();
            }
        }
        "window".into()
    }

    fn read_class(&self, win: Window) -> String {
        let a = self.atom("WM_CLASS");
        if let Some(r) = self.conn.get_property(false, win, a, 0u32, 0, 1024).ok().and_then(|c| c.reply().ok()) {
            let s = String::from_utf8_lossy(&r.value).to_string();
            // "instance\0Class\0" -> Class
            if let Some(rest) = s.split_once('\0').map(|(_, r)| r) {
                return rest.trim_end_matches('\0').to_string();
            }
            return s.trim_end_matches('\0').to_string();
        }
        String::new()
    }

    fn manage(&mut self, win: Window) {
        if self.clients.iter().any(|c| c.win == win) {
            return;
        }
        let Some(attr) = self
            .conn
            .get_window_attributes(win)
            .ok()
            .and_then(|c| c.reply().ok())
        else {
            return;
        };
        if attr.override_redirect || self.window_type_skip(win) {
            // pass-through window (dock/desktop/etc): map it as-is and track
            // its strut reservation if any
            self.absorb_strut(win);
            self.conn.map_window(win).ok();
            self.shell_wins.push((win, false));
            // desktop-type windows live at the very bottom of the stack
            let ty = self.atom("_NET_WM_WINDOW_TYPE");
            let is_desktop = self
                .conn
                .get_property(false, win, ty, 0u32, 0, 8)
                .ok()
                .and_then(|c| c.reply().ok())
                .and_then(|r| r.value32().map(|it| it.collect::<Vec<u32>>()))
                .map(|v| v.contains(&self.atom("_NET_WM_WINDOW_TYPE_DESKTOP")))
                .unwrap_or(false);
            if let Some(last) = self.shell_wins.last_mut() {
                last.1 = is_desktop;
            }
            if is_desktop {
                self.conn
                    .configure_window(win, &ConfigureWindowAux::new().stack_mode(StackMode::BELOW))
                    .ok();
            } else {
                // dock/panel live above everything except focused app windows
                self.conn
                    .configure_window(win, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE))
                    .ok();
            }
            return;
        }
        let (_dx, _dy, mut w, mut h) = self.geom_for_new();
        // honor the app's requested size if sane
        let Some(geo) = self.conn.get_geometry(win).ok().and_then(|c| c.reply().ok()) else { return };
        if geo.width > 50 && geo.height > 50 {
            w = geo.width as i32;
            h = geo.height as i32;
        }
        // center on the real size, clamped to the workarea (panel + dock
        // reserve their bands; big windows stay inside)
        let (x, y) = self.place_for_size(w, h);
        // position hints from ConfigureWindow by the app are lost pre-manage;
        // keep geometry default.

        let frame = self.conn.generate_id().unwrap();
        self.conn
            .create_window(
                self.conn.setup().roots[0].root_depth,
                frame,
                self.root,
                (x - BORDER) as i16,
                (y - BORDER) as i16,
                (w + 2 * BORDER) as u16,
                (h + 2 * BORDER) as u16,
                0,
                WindowClass::INPUT_OUTPUT,
                self.conn.setup().roots[0].root_visual,
                &CreateWindowAux::new()
                    .event_mask(
                        EventMask::EXPOSURE
                            | EventMask::BUTTON_PRESS
                            | EventMask::STRUCTURE_NOTIFY
                            // children of the frame (the reparented client):
                            // without this we never learn that a client window was
                            // destroyed, and its empty frame would stay mapped on
                            // screen forever (blocking the dock underneath)
                            | EventMask::SUBSTRUCTURE_NOTIFY,
                    ),
            )
            .unwrap();
        let _ = self.conn.change_window_attributes(win, &ChangeWindowAttributesAux::new().event_mask(
            EventMask::PROPERTY_CHANGE | EventMask::FOCUS_CHANGE,
        ));
        self.conn.reparent_window(win, frame, BORDER as i16, BORDER as i16).unwrap();
        self.conn
            .configure_window(win, &ConfigureWindowAux::new().width(w as u32).height(h as u32).x(0).y(0))
            .unwrap();
        self.conn.map_window(win).unwrap();
        self.conn.map_window(frame).unwrap();

        let title = self.read_title(win);
        let class = self.read_class(win);
        let desktop = self.current_desktop;
        self.set_atom_prop(win, "_NET_WM_STATE", &[]);
        self.set_u32_prop(win, "_NET_WM_DESKTOP", &[desktop]);
        self.set_u32_prop(win, "WM_STATE", &[1]); // NormalState

        let c = Client {
            win,
            frame,
            x,
            y,
            w,
            h,
            desktop,
            maximized: false,
            fullscreen: false,
            minimized: false,
            title,
            class,
            pre_max: None,
        };
        self.conn.flush().ok();
        println!("bwm: manage win {} frame {} class [{}]", win, frame, c.class);
        self.clients.push(c);
        self.update_client_list();
        self.update_desktop_props();
        self.redraw_frame(frame);
        self.focus_client(win);
        self.raise_shell();
    }

    fn unmanage(&mut self, win: Window) {
        self.shell_wins.retain(|&(w, _)| w != win);
        if let Some(i) = self.clients.iter().position(|c| c.win == win) {
            let c = self.clients.remove(i);
            println!("bwm: unmanage win {} (frame {})", win, c.frame);
            let _ = self.conn.destroy_window(c.frame);
            if self.focused == Some(win) {
                // focus the next stacked client, or clear focus safely (never
                // leave the focus dangling on a dead window: the server
                // would then discard every keyboard event)
                if let Some(next) = self.clients.last().map(|c| c.win) {
                    self.focus_client(next);
                } else {
                    self.focus_none();
                }
            }
            self.update_client_list();
        }
    }

    fn find(&self, win: Window) -> Option<usize> {
        self.clients.iter().position(|c| c.win == win)
    }

    fn redraw_frame(&mut self, frame: Window) {
        let Some(ci) = self.clients.iter().position(|c| c.frame == frame) else { return };
        let focused = self.focused == Some(self.clients[ci].win);
        let (w, h) = (self.clients[ci].w + 2 * BORDER, self.clients[ci].h + 2 * BORDER);
        let mut cv = Canvas::new(w, h);
        let acc = self.cfg_accent();
        let (fill, border) = if focused {
            ([14, 14, 20, 255], [acc[0], acc[1], acc[2], 255])
        } else {
            ([13, 13, 18, 255], [56, 56, 66, 255])
        };
        cv.fill(fill);
        // outer dark hairline + colored ring for definition
        let b = BORDER;
        cv.rect(0, 0, w, 1, [0, 0, 0, 255]);
        cv.rect(0, h - 1, w, 1, [0, 0, 0, 255]);
        cv.rect(0, 0, 1, h, [0, 0, 0, 255]);
        cv.rect(w - 1, 0, 1, h, [0, 0, 0, 255]);
        cv.rect(1, 1, w - 2, b - 1, border);
        cv.rect(1, h - b, w - 2, b - 1, border);
        cv.rect(1, 1, b - 1, h - 2, border);
        cv.rect(w - b, 1, b - 1, h - 2, border);
        self.conn
            .put_image(ImageFormat::Z_PIXMAP, frame, self.gc, w as u16, h as u16, 0, 0, 0, 24, &cv.to_argb())
            .ok();
        self.conn.flush().ok();
    }

    fn redraw_all_frames(&mut self) {
        let frames: Vec<Window> = self.clients.iter().map(|c| c.frame).collect();
        for f in frames {
            self.redraw_frame(f);
        }
    }

    /// Clear focus SAFELY: an X input focus of None (0) makes the server
    /// discard ALL keyboard events - including GrabKey global shortcuts, so
    /// after the last/focused window disappears the keyboard would silently
    /// die. Focus the root instead (events no window selects, grabs live on).
    fn focus_none(&mut self) {
        self.focused = None;
        self.set_u32_prop(self.root, "_NET_ACTIVE_WINDOW", &[0]);
        self.conn
            .set_input_focus(InputFocus::PARENT, self.root, 0u32)
            .ok();
        self.conn.flush().ok();
    }

    fn focus_client(&mut self, win: Window) {
        if let Some(i) = self.find(win) {
            let c = self.clients[i].clone();
            if c.minimized {
                self.restore(c.win);
            }
            self.focused = Some(win);
            self.conn.set_input_focus(InputFocus::PARENT, win, 0u32).ok();
            self.conn.allow_events(Allow::REPLAY_POINTER, 0u32).ok();
            self.update_client_list();
            self.redraw_all_frames();
        }
    }

    fn frame_geom(&self, c: &Client) -> (i32, i32, i32, i32) {
        if c.fullscreen {
            (0, 0, self.screen_w, self.screen_h)
        } else if c.maximized {
            let (wx, wy, ww, wh) = self.workarea;
            (wx, wy, ww, wh)
        } else {
            (c.x, c.y, c.w, c.h)
        }
    }

    fn apply_geom(&mut self, i: usize) {
        let c = self.clients[i].clone();
        let (fx, fy, fw, fh) = self.frame_geom(&c);
        let bx = if c.fullscreen { 0 } else { BORDER };
        self.conn
            .configure_window(c.frame, &ConfigureWindowAux::new()
                .x((fx - bx) as i32).y((fy - bx) as i32)
                .width((fw + 2 * bx) as u32).height((fh + 2 * bx) as u32))
            .ok();
        self.conn
            .configure_window(c.win, &ConfigureWindowAux::new()
                .x(bx).y(bx).width(fw as u32).height(fh as u32))
            .ok();
        let c = &mut self.clients[i];
        c.x = fx;
        c.y = fy;
        c.w = fw;
        c.h = fh;
        self.conn.flush().ok();
    }

    fn set_wm_state_atoms(&self, i: usize) {
        let c = self.clients[i].clone();
        let mut atoms = Vec::new();
        if c.minimized {
            atoms.push(self.atom("_NET_WM_STATE_HIDDEN"));
        }
        if c.maximized {
            atoms.push(self.atom("_NET_WM_STATE_MAXIMIZED_VERT"));
            atoms.push(self.atom("_NET_WM_STATE_MAXIMIZED_HORZ"));
        }
        if c.fullscreen {
            atoms.push(self.atom("_NET_WM_STATE_FULLSCREEN"));
        }
        self.set_atom_prop(c.win, "_NET_WM_STATE", &atoms);
        let state = if c.minimized { 3 } else { 1 }; // Iconic / Normal
        self.set_u32_prop(c.win, "WM_STATE", &[state]);
    }

    fn minimize(&mut self, win: Window) {
        if let Some(i) = self.find(win) {
            let frame = self.clients[i].frame;
            self.clients[i].minimized = true;
            self.conn.unmap_window(frame).ok();
            self.set_wm_state_atoms(i);
            if self.focused == Some(win) {
                self.focus_none();
            }
        }
    }

    fn restore(&mut self, win: Window) {
        if let Some(i) = self.find(win) {
            if self.clients[i].minimized {
                self.clients[i].minimized = false;
                let frame = self.clients[i].frame;
                self.conn.map_window(frame).ok();
                self.set_wm_state_atoms(i);
                self.show_desktop_mode = false;
            }
        }
    }

    fn close(&mut self, win: Window) {
        if let Some(i) = self.find(win) {
            let c = self.clients[i].clone();
            let del = self.atom("WM_DELETE_WINDOW");
            let prot = self.atom("WM_PROTOCOLS");
            let ev = ClientMessageEvent::new(32, c.win, prot, [del, 0, 0, 0, 0]);
            if self
                .conn
                .send_event(false, c.win, EventMask::NO_EVENT, &ev)
                .is_ok()
            {
                println!("bwm: WM_DELETE sent to {}", c.win);
            } else {
                self.conn.kill_client(c.win).ok();
            }
            self.conn.flush().ok();
        }
    }

    fn set_desktop(&mut self, d: u32) {
        let d = d.clamp(0, self.desktops - 1);
        self.current_desktop = d;
        for c in &self.clients {
            if c.desktop == d && !c.minimized {
                self.conn.map_window(c.frame).ok();
            } else {
                self.conn.unmap_window(c.frame).ok();
            }
        }
        self.update_desktop_props();
        self.conn.flush().ok();
    }

    fn move_to_desktop(&mut self, win: Window, d: u32) {
        if let Some(i) = self.find(win) {
            let d = d.clamp(0, self.desktops - 1);
            self.clients[i].desktop = d;
            if d != self.current_desktop {
                self.conn.unmap_window(self.clients[i].frame).ok();
            }
            self.update_desktop_props();
        }
    }

    fn maximize_toggle(&mut self, win: Window) {
        if let Some(i) = self.find(win) {
            if self.clients[i].maximized {
                if let Some((x, y, w, h)) = self.clients[i].pre_max {
                    self.clients[i].x = x;
                    self.clients[i].y = y;
                    self.clients[i].w = w;
                    self.clients[i].h = h;
                }
                self.clients[i].maximized = false;
            } else {
                self.clients[i].pre_max = Some((self.clients[i].x, self.clients[i].y, self.clients[i].w, self.clients[i].h));
                self.clients[i].maximized = true;
            }
            self.set_wm_state_atoms(i);
            self.apply_geom(i);
            self.redraw_frame(self.clients[i].frame);
        }
    }

    fn fullscreen_toggle(&mut self, win: Window) {
        if let Some(i) = self.find(win) {
            if let Some((x, y, w, h)) = self.clients[i].pre_max {
                self.clients[i].x = x;
                self.clients[i].y = y;
                self.clients[i].w = w;
                self.clients[i].h = h;
            }
            self.clients[i].fullscreen = !self.clients[i].fullscreen;
            if self.clients[i].fullscreen {
                self.clients[i].pre_max = Some((self.clients[i].x, self.clients[i].y, self.clients[i].w, self.clients[i].h));
            }
            self.set_wm_state_atoms(i);
            self.apply_geom(i);
            self.redraw_frame(self.clients[i].frame);
        }
    }

    fn snap(&mut self, win: Window, side: &str) {
        if let Some(i) = self.find(win) {
            let (wx, wy, ww, wh) = self.workarea;
            if !self.clients[i].maximized && self.clients[i].pre_max.is_none() {
                self.clients[i].pre_max = Some((self.clients[i].x, self.clients[i].y, self.clients[i].w, self.clients[i].h));
            }
            let (x, y, w, h) = if side == "left" {
                (wx, wy, ww / 2, wh)
            } else {
                (wx + ww / 2, wy, ww - ww / 2, wh)
            };
            self.clients[i].x = x;
            self.clients[i].y = y;
            self.clients[i].w = w;
            self.clients[i].h = h;
            self.clients[i].maximized = false;
            self.set_wm_state_atoms(i);
            self.apply_geom(i);
            self.redraw_frame(self.clients[i].frame);
        }
    }

    fn show_desktop(&mut self) {
        if self.show_desktop_mode {
            for c in &self.clients {
                if c.desktop == self.current_desktop {
                    self.conn.map_window(c.frame).ok();
                }
            }
        } else {
            for c in &self.clients {
                if c.desktop == self.current_desktop {
                    self.conn.unmap_window(c.frame).ok();
                }
            }
        }
        self.show_desktop_mode = !self.show_desktop_mode;
        self.conn.flush().ok();
    }

    fn cycle_window(&mut self, back: bool) {
        let cur: Vec<usize> = self
            .clients
            .iter()
            .enumerate()
            .filter(|(_, c)| c.desktop == self.current_desktop && !c.minimized)
            .map(|(i, _)| i)
            .collect();
        if cur.is_empty() {
            return;
        }
        let pos = cur
            .iter()
            .position(|&i| Some(self.clients[i].win) == self.focused)
            .unwrap_or(0);
        let next = if back {
            (pos + cur.len() - 1) % cur.len()
        } else {
            (pos + 1) % cur.len()
        };
        let win = self.clients[cur[next]].win;
        self.focus_client(win);
        // raise
        if let Some(i) = self.find(win) {
            let frame = self.clients[i].frame;
            self.conn.configure_window(frame, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE)).ok();
        }
        self.raise_shell();
    }

    // ------------------------------------------------------------ spawning

    fn spawn(&self, argv: &[&str]) {
        let mut cmd = Command::new(argv[0]);
        cmd.args(&argv[1..]);
        // inherit our DISPLAY (we are the session WM); only default when unset
        if std::env::var("DISPLAY").is_err() {
            cmd.env("DISPLAY", ":0");
        }
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        match cmd.spawn() {
            Ok(mut child) => {
                println!("bwm: spawned {:?} (pid {})", argv, child.id());
                // reap on exit so killed children do not linger as zombies
                // (a zombie's /proc entry persists and can keep single-instance
                // locks of other BraveOS apps looking "alive")
                let _ = std::thread::Builder::new()
                    .name("bwm-reap".into())
                    .spawn(move || {
                        let _ = child.wait();
                    });
            }
            Err(e) => eprintln!("bwm: failed to spawn {:?}: {}", argv, e),
        }
    }

    fn action(&mut self, action: &str, arg: &str) {
        match action {
            "terminal" => self.spawn(&["xterm", "-fa", "Monospace", "-fs", "11"]),
            "browser" => self.spawn(&["brave", "--maximized", "--no-first-run"]),
            "files" => self.spawn(&["pcmanfm"]),
            "settings" => {
                if arg.is_empty() {
                    self.spawn(&["bsettings"]);
                } else {
                    self.spawn(&["bsettings", arg]);
                }
            }
            "close" => {
                if let Some(w) = self.focused {
                    self.close(w);
                }
            }
            "workspace_next" => {
                let d = (self.current_desktop + 1).min(self.desktops - 1);
                self.set_desktop(d);
            }
            "workspace_prev" => {
                let d = self.current_desktop.saturating_sub(1);
                self.set_desktop(d);
            }
            "workspace" => {
                let n: u32 = arg.parse().unwrap_or(0);
                if n >= 1 {
                    if n > self.desktops && n <= 9 {
                        self.desktops = n;
                    }
                    self.set_desktop(n - 1);
                }
            }
            "show_desktop" => self.show_desktop(),
            "maximize" => {
                if let Some(w) = self.focused {
                    self.maximize_toggle(w);
                }
            }
            "fullscreen" => {
                if let Some(w) = self.focused {
                    self.fullscreen_toggle(w);
                }
            }
            "snap_left" => {
                if let Some(w) = self.focused {
                    self.snap(w, "left");
                }
            }
            "snap_right" => {
                if let Some(w) = self.focused {
                    self.snap(w, "right");
                }
            }
            "lock" => self.spawn(&["xtrlock"]),
            "screenshot" => self.spawn(&["bshot"]),
            "app_grid" | "power_menu" => self.shell_cmd(if action == "app_grid" { 1 } else { 2 }),
            "switcher" => self.cycle_window(false),
            "switcher_back" => self.cycle_window(true),
            _ => {}
        }
        self.conn.flush().ok();
    }

    fn shell_cmd(&mut self, cmd: u32) {
        let a = self.atom("_BRAVOS_SHELL_CMD");
        let ev = ClientMessageEvent::new(32, self.root, a, [cmd, 0, 0, 0, 0]);
        // broadcast to all shell windows via root propagation
        let _ = self.conn.send_event(true, self.root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, &ev);
        self.conn.flush().ok();
    }

    // ------------------------------------------------------------- keybinds

    fn grab_keys(&mut self) {
        let combos: Vec<String> = self
            .cfg
            .shortcuts
            .values()
            .cloned()
            .chain(vec![
                "super+1".into(), "super+2".into(), "super+3".into(), "super+4".into(),
                "super+5".into(), "super+6".into(), "super+7".into(), "super+8".into(),
            ])
            .collect();
        for combo in combos {
            let (mods, ks) = parse_combo(&combo);
            if ks == 0 {
                continue;
            }
            let Some(kc) = self.keycode_for(ks) else { continue };
            for extra in [0u16, MOD_LOCK, MOD_NUM, 128u16, MOD_LOCK | MOD_NUM, MOD_LOCK | 128, MOD_NUM | 128, MOD_LOCK | MOD_NUM | 128] {
                let m = ModMask::from(mods | extra);
                self.conn
                    .grab_key(false, self.root, m, kc, GrabMode::ASYNC, GrabMode::ASYNC)
                    .ok();
            }
        }
        self.conn.flush().ok();
        println!("bwm: keybinds grabbed");
    }

    fn key_action(&mut self, mods: u16, keysym: u32) {
        // super+digit -> workspace N
        if mods & MOD_SUPER != 0 && (0x31..=0x39).contains(&keysym) {
            let n = keysym - 0x30;
            self.desktops = self.desktops.max(n);
            self.set_desktop(n - 1);
            return;
        }
        // find matching action
        for (act, combo) in self.cfg.shortcuts.clone() {
            let (m, ks) = parse_combo(&combo);
            if ks == keysym && (m == 0 || mods & m == m) {
                self.action(&act, "");
                return;
            }
        }
    }

    // -------------------------------------------------------------- struts

    fn absorb_strut(&mut self, win: Window) {
        let a = self.atom("_NET_WM_STRUT_PARTIAL");
        let mut vals: Option<Vec<u32>> = None;
        if let Some(r) = self.conn.get_property(false, win, a, 0u32, 0, 12).ok().and_then(|c| c.reply().ok()) {
            vals = r.value32().map(|it| it.collect());
        }
        if vals.is_none() {
            let a = self.atom("_NET_WM_STRUT");
            if let Some(r) = self.conn.get_property(false, win, a, 0u32, 0, 12).ok().and_then(|c| c.reply().ok()) {
                vals = r.value32().map(|it| it.collect());
            }
        }
        if let Some(v) = vals {
            if v.len() >= 4 {
                let (l, r_, t, b) = (v[0] as i32, v[1] as i32, v[2] as i32, v[3] as i32);
                if t > self.top_strut {
                    self.top_strut = t;
                }
                if b > self.bottom_strut {
                    self.bottom_strut = b;
                }
                let _ = (l, r_);
                self.update_workarea();
                println!("bwm: strut absorbed from win {}: top={} bottom={}", win, self.top_strut, self.bottom_strut);
            }
        }
    }

    // ---------------------------------------------------------- root menu

    fn open_menu(&mut self, x: i32, y: i32) {
        if self.menu_open {
            return;
        }
        self.menu_items = vec![
            ("settings", ""),
            ("files", ""),
            ("terminal", ""),
            ("browser", ""),
            ("show_desktop", ""),
            ("screenshot", ""),
            ("lock", ""),
            ("power_menu", ""),
        ];
        let mw = 260i32;
        let mh = 44 * self.menu_items.len() as i32 + 12;
        let mx = x.min(self.screen_w - mw - 8).max(8);
        let my = y.min(self.screen_h - mh - 8).max(8);
        self.conn
            .create_window(
                32,
                self.menu_win,
                self.root,
                mx as i16,
                my as i16,
                mw as u16,
                mh as u16,
                0,
                WindowClass::INPUT_OUTPUT,
                self.argb_visual.unwrap_or(self.conn.setup().roots[0].root_visual),
                &CreateWindowAux::new()
                    .event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS | EventMask::POINTER_MOTION | EventMask::LEAVE_WINDOW)
                    .override_redirect(1),
            )
            .ok();
        self.menu_sel = -1;
        self.menu_open = true;
        self.conn.grab_pointer(
            false, self.root,
            EventMask::BUTTON_RELEASE | EventMask::BUTTON_PRESS | EventMask::POINTER_MOTION,
            GrabMode::ASYNC, GrabMode::ASYNC, self.menu_win, 0u32, 0u32,
        ).ok();
        self.conn.map_window(self.menu_win).ok();
        self.shape_menu();
        self.draw_menu();
        self.conn.flush().ok();
    }

    fn close_menu(&mut self) {
        if !self.menu_open {
            return;
        }
        self.menu_open = false;
        self.conn.ungrab_pointer(0u32).ok();
        self.conn.unmap_window(self.menu_win).ok();
        self.conn.destroy_window(self.menu_win).ok();
        self.conn.flush().ok();
    }

    fn cfg_accent(&self) -> [u8; 3] {
        Config::hex_rgb(&self.cfg.accent)
    }

    fn shape_menu(&mut self) {
        use x11rb::protocol::shape::{self, SK, SO};
        let mw = 260i32;
        let mh = 44 * self.menu_items.len() as i32 + 12;
        let r = 14i32;
        let pm = self.conn.generate_id().unwrap();
        self.conn.create_pixmap(1, pm, self.root, mw as u16, mh as u16).unwrap();
        let gcm = self.conn.generate_id().unwrap();
        self.conn.create_gc(gcm, pm, &CreateGCAux::new().foreground(1)).unwrap();
        let mut rects = Vec::new();
        for y in 0..mh {
            let dy = if y < r { r - y } else if y >= mh - r { y - (mh - r - 1) } else { 0 };
            let inr = (((r * r - dy * dy).max(0)) as f64).sqrt() as i32;
            let x0 = if dy > 0 { r - inr } else { 0 };
            let x1 = if dy > 0 { mw - (r - inr) } else { mw };
            if x1 > x0 {
                rects.push(Rectangle { x: x0 as i16, y: y as i16, width: (x1 - x0) as u16, height: 1 });
            }
        }
        self.conn.poly_fill_rectangle(pm, gcm, &rects).unwrap();
        self.conn.flush().unwrap();
        self.conn.shape_mask(SO::SET, SK::BOUNDING, self.menu_win, 0, 0, pm).unwrap();
        self.conn.free_gc(gcm).ok();
        self.conn.free_pixmap(pm).ok();
    }

    fn draw_menu(&mut self) {
        let mw = 260i32;
        let mh = 44 * self.menu_items.len() as i32 + 12;
        let mut cv = Canvas::new(mw, mh);
        let acc = self.cfg_accent();
        cv.fill([0, 0, 0, 0]);
        cv.rounded(0, 0, mw, mh, 14, [14, 14, 20, 244]);
        cv.rounded_outline(0, 0, mw, mh, 14, [255, 255, 255, 30]);
        for (i, (act, _)) in self.menu_items.iter().enumerate() {
            let y = 6 + i as i32 * 44;
            let hot = i as i32 == self.menu_sel;
            if hot {
                cv.rounded(6, y, mw - 12, 40, 10, [acc[0], acc[1], acc[2], 90]);
            }
            let label = match *act {
                "settings" => "Settings",
                "files" => "Files",
                "terminal" => "Terminal",
                "browser" => "Brave Browser",
                "show_desktop" => "Show Desktop",
                "screenshot" => "Take Screenshot",
                "lock" => "Lock Screen",
                "power_menu" => "Power…",
                _ => act,
            };
            self.fonts.draw_centered(&mut cv, Rect::new(0, y, mw, 40), 14, [235, 235, 240, 255], false, label);
        }
        self.menu_cvs = Some(cv);
        self.conn
            .put_image(ImageFormat::Z_PIXMAP, self.menu_win, self.gc, mw as u16, mh as u16, 0, 0, 0, 32,
                &self.menu_cvs.as_ref().unwrap().to_argb())
            .ok();
        self.conn.flush().ok();
    }

    fn menu_click(&mut self, x: i32, y: i32) {
        let idx = ((y - 6) / 44).max(0) as usize;
        if idx < self.menu_items.len() {
            let (act, arg) = self.menu_items[idx];
            self.close_menu();
            self.action(act, arg);
        } else {
            self.close_menu();
        }
    }

    // ------------------------------------------------------- event helpers

    fn event_coords(&self, e: &ButtonPressEvent) -> (i32, i32) {
        // events on root are in root coords already
        (e.root_x as i32, e.root_y as i32)
    }
}


fn main() {
    let mut wm = Wm::connect().expect("bwm: X11 connect");
    wm.border_glow = true;

    // select root events
    wm.conn
        .change_window_attributes(
            wm.root,
            &ChangeWindowAttributesAux::new().event_mask(
                EventMask::SUBSTRUCTURE_REDIRECT
                    | EventMask::SUBSTRUCTURE_NOTIFY
                    | EventMask::BUTTON_PRESS
                    | EventMask::KEY_PRESS
                    | EventMask::KEY_RELEASE
                    | EventMask::PROPERTY_CHANGE,
            ),
        )
        .expect("bwm: another WM running?");

    // EWMH support flags
    let supported = [
        wm.atom("_NET_CLIENT_LIST"), wm.atom("_NET_CLIENT_LIST_STACKING"),
        wm.atom("_NET_ACTIVE_WINDOW"), wm.atom("_NET_CURRENT_DESKTOP"),
        wm.atom("_NET_NUMBER_OF_DESKTOPS"), wm.atom("_NET_DESKTOP_NAMES"),
        wm.atom("_NET_WORKAREA"), wm.atom("_NET_CLOSE_WINDOW"),
        wm.atom("_NET_WM_DESKTOP"), wm.atom("_NET_WM_STATE"),
        wm.atom("_NET_WM_STATE_HIDDEN"), wm.atom("_NET_WM_STATE_FULLSCREEN"),
        wm.atom("_NET_WM_STATE_MAXIMIZED_VERT"), wm.atom("_NET_WM_STATE_MAXIMIZED_HORZ"),
        wm.atom("_NET_WM_NAME"), wm.atom("_NET_WM_STRUT_PARTIAL"),
    ];
    wm.set_atom_prop(wm.root, "_NET_SUPPORTED", &supported);
    wm.update_client_list();
    wm.update_desktop_props();
    wm.update_workarea();

    wm.grab_keys();

    println!("bwm: ready (screen {}x{})", wm.screen_w, wm.screen_h);
    wm.conn.flush().ok();

    // event loop
    let mut sweep: u32 = 0;
    loop {
        // poll_for_event does NOT flush pending writes: flush once per
        // iteration so forwarded ConfigureRequests etc. are actually sent
        wm.conn.flush().ok();
        let Some(ev) = wm.conn.poll_for_event().ok().flatten() else {
            std::thread::sleep(std::time::Duration::from_millis(30));
            // periodic liveness sweep (~every second): if a client died in a
            // way we missed (event race, XID reuse, ...), drop it now instead
            // of leaking its empty frame over the screen
            sweep = sweep.wrapping_add(1);
            if sweep % 32 == 0 {
                let dead: Vec<Window> = wm
                    .clients
                    .iter()
                    .filter(|c| {
                        wm.conn
                            .get_window_attributes(c.win)
                            .ok()
                            .and_then(|r| r.reply().ok())
                            .is_none()
                    })
                    .map(|c| c.win)
                    .collect();
                for win in dead {
                    println!("bwm: sweep unmanage dead client {}", win);
                    wm.unmanage(win);
                }
            }
            continue;
        };
        match ev {
            Event::MapRequest(e) => {
                println!("bwm: map-request {}", e.window);
                wm.manage(e.window);
            }
            Event::UnmapNotify(e) => {
                // e.event == frame: the reparented client withdrew itself
                // (closed to tray / withdrawn state). Verify it is really
                // unmapped right now - the transient unmap our own reparent
                // generates is always followed by us mapping the client
                // again, so by the time we process the event it is viewable.
                if let Some(i) = wm.find(e.window) {
                    let frame = wm.clients[i].frame;
                    let minimized = wm.clients[i].minimized;
                    if e.event == frame && !minimized {
                        let withdrawn = wm
                            .conn
                            .get_window_attributes(e.window)
                            .ok()
                            .and_then(|c| c.reply().ok())
                            .map(|a| a.map_state == MapState::UNMAPPED)
                            .unwrap_or(true);
                        if withdrawn {
                            wm.unmanage(e.window);
                        }
                    } else if e.event == wm.root && !minimized {
                        // pre-reparent unmap: treat as withdraw
                        wm.unmanage(e.window);
                    }
                }
            }
            Event::DestroyNotify(e) => {
                // frame destroyed -> unmanage its client;
                // client destroyed (child of frame, e.event == frame) -> unmanage;
                // plain root child destroyed (shell window) -> bookkeeping only
                if let Some(i) = wm.clients.iter().position(|c| c.frame == e.window) {
                    wm.unmanage(wm.clients[i].win);
                } else if wm.find(e.window).is_some() {
                    wm.unmanage(e.window);
                } else {
                    wm.shell_wins.retain(|&(w, _)| w != e.window);
                }
            }
            Event::ConfigureRequest(e) => {
                // honor app geometry requests for unmanaged windows; for
                // managed ones we own geometry but allow size changes
                if let Some(i) = wm.find(e.window) {
                    let w = e.width as i32;
                    let h = e.height as i32;
                    if w > 50 && h > 50 && !wm.clients[i].maximized && !wm.clients[i].fullscreen {
                        wm.clients[i].w = w;
                        wm.clients[i].h = h;
                        wm.apply_geom(i);
                        wm.redraw_frame(wm.clients[i].frame);
                    }
                } else {
                    let aux = ConfigureWindowAux::new()
                        .x(e.x as i32)
                        .y(e.y as i32)
                        .width(e.width as u32)
                        .height(e.height as u32)
                        .stack_mode(e.stack_mode);
                    wm.conn.configure_window(e.window, &aux).ok();
                    wm.conn.flush().ok();
                }
            }
            Event::Expose(e) => {
                if e.count == 0 {
                    wm.redraw_frame(e.window);
                }
            }
            Event::ButtonPress(e) => {
                let (x, y) = wm.event_coords(&e);
                if e.detail == 3 && e.event == wm.root {
                    wm.open_menu(x, y);
                    continue;
                }
                if e.event == wm.menu_win && wm.menu_open {
                    if e.detail == 1 {
                        wm.menu_click(x, y);
                    } else {
                        wm.close_menu();
                    }
                    continue;
                }
                // click on frame border or client: focus + raise
                if let Some(i) = wm.clients.iter().position(|c| c.frame == e.event || c.win == e.event) {
                    let win = wm.clients[i].win;
                    wm.focus_client(win);
                    if let Some(fi) = wm.find(win) {
                        let frame = wm.clients[fi].frame;
                        wm.conn
                            .configure_window(frame, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE))
                            .ok();
                        // panels and docks stay above app windows, always
                        wm.raise_shell();
                    }
                } else if e.event == wm.root && e.detail == 1 {
                    wm.focus_none();
                    wm.redraw_all_frames();
                }
            }
            Event::MotionNotify(e) => {
                if wm.menu_open && e.event == wm.menu_win {
                    let idx = ((e.event_y as i32 - 6) / 44).max(0) as usize;
                    if idx < wm.menu_items.len() && idx as i32 != wm.menu_sel {
                        wm.menu_sel = idx as i32;
                        wm.draw_menu();
                    }
                }
            }
            Event::ClientMessage(e) => {
                let data = e.data.as_data32();
                if e.type_ == wm.atom("_NET_CLOSE_WINDOW") {
                    wm.close(e.window);
                } else if e.type_ == wm.atom("_NET_ACTIVE_WINDOW_MSG") || e.type_ == wm.atom("_NET_ACTIVE_WINDOW") {
                    wm.focus_client(e.window);
                    if let Some(i) = wm.find(e.window) {
                        let frame = wm.clients[i].frame;
                        wm.conn
                            .configure_window(frame, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE))
                            .ok();
                        wm.raise_shell();
                    }
                } else if e.type_ == wm.atom("_NET_CURRENT_DESKTOP") {
                    wm.set_desktop(data[0]);
                } else if e.type_ == wm.atom("_NET_NUMBER_OF_DESKTOPS") {
                    wm.desktops = data[0].clamp(1, 9);
                    wm.update_desktop_props();
                } else if e.type_ == wm.atom("_BRAVOS_SHELL_CMD") {
                    // shell -> wm messages: cmd 3 = add workspace, 4 = close workspace
                    match data[0] {
                        3 => {
                            wm.desktops = (wm.desktops + 1).clamp(1, 9);
                            wm.update_desktop_props();
                        }
                        4 => {
                            wm.desktops = (wm.desktops - 1).clamp(1, 9);
                            wm.set_desktop(wm.current_desktop.min(wm.desktops - 1));
                        }
                        _ => {}
                    }
                } else if e.type_ == wm.atom("_BRAVOS_CONFIG") {
                    wm.cfg = Config::load();
                    wm.redraw_all_frames();
                }
            }
            Event::PropertyNotify(e) => {
                // struts can be set just after mapping: re-absorb on change
                if e.window != wm.root
                    && (e.atom == wm.atom("_NET_WM_STRUT_PARTIAL") || e.atom == wm.atom("_NET_WM_STRUT"))
                {
                    wm.top_strut = 0;
                    wm.bottom_strut = 0;
                    for cw in &wm.clients {
                        let _ = cw;
                    }
                    // re-scan all unmanaged windows is complex; just re-absorb this one
                    wm.absorb_strut(e.window);
                }
                if e.window != wm.root {
                    // client title changed
                    if let Some(i) = wm.find(e.window) {
                        if e.atom == wm.atom("_NET_WM_NAME") || e.atom == 39 {
                            let t = wm.read_title(e.window);
                            wm.clients[i].title = t;
                        }
                    }
                }
            }
            Event::KeyPress(e) => {
                let ks = wm
                    .keysyms
                    .get(((e.detail as u32 - wm.keycode_min) * wm.keysyms_per_keycode.max(1) as u32) as usize)
                    .copied()
                    .unwrap_or(0);
                let ks = if ks == 0 && wm.keysyms_per_keycode > 1 {
                    wm.keysyms
                        .get(((e.detail as u32 - wm.keycode_min) * wm.keysyms_per_keycode as u32 + 1) as usize)
                        .copied()
                        .unwrap_or(0)
                } else {
                    ks
                };
                wm.key_action(e.state.bits() as u16 & !(MOD_LOCK | MOD_NUM | 128), ks);
            }
            Event::Error(_) => {}
            _ => {}
        }
    }
}
