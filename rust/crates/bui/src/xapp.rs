//! Minimal X11 application framework: ARGB windows, event dispatch, drawing
//! flush, atoms, keysym lookup. Used by every BraveOS UI binary.

use std::collections::HashMap;

use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::shape::ConnectionExt as ShapeExt;
use x11rb::wrapper::ConnectionExt as WrapExt;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

use crate::canvas::Canvas;
use crate::config::Config;
use crate::font::Fonts;
use crate::icons::IconStore;
use crate::i18n::Tr;

pub type Win = u32;

pub use x11rb::protocol::xproto::EventMask;

pub enum PtrKind {
    Down(u8),   // button number
    Up(u8),
    Move,
    Enter,
    Leave,
}

/// A window's behaviour. Handlers are called with exclusive `&mut App`.
pub trait Handler {
    fn draw(&mut self, app: &mut App, cv: &mut Canvas);
    fn pointer(&mut self, _app: &mut App, _kind: PtrKind, _x: i32, _y: i32) {}
    fn key(&mut self, _app: &mut App, _keysym: u32, _state: u16) -> bool {
        false
    }
    fn tick(&mut self, _app: &mut App) {}
    fn resized(&mut self, _app: &mut App, _w: i32, _h: i32) {}
    fn property(&mut self, _app: &mut App, _atom: u32) {}
}

struct UiWin {
    handler: Box<dyn Handler>,
    canvas: Canvas,
    w: i32,
    h: i32,
    depth: u8,
}

pub struct WinOpts {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub override_redirect: bool,
    pub argb: bool,          // true: 32-bit ARGB visual (needs compositor for alpha)
    pub window_type: &'static str, // "" none, "dock", "desktop", "notification"
    pub class: &'static str,
    pub title: &'static str,
    pub event_mask: EventMask,
}

impl Default for WinOpts {
    fn default() -> Self {
        WinOpts {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
            override_redirect: true,
            argb: true,
            window_type: "dock",
            class: "bravos-shell",
            title: "bravos",
            event_mask: EventMask::EXPOSURE
                | EventMask::BUTTON_PRESS
                | EventMask::BUTTON_RELEASE
                | EventMask::POINTER_MOTION
                | EventMask::KEY_PRESS
                | EventMask::ENTER_WINDOW
                | EventMask::LEAVE_WINDOW
                | EventMask::STRUCTURE_NOTIFY,
        }
    }
}

pub struct App {
    pub conn: RustConnection,
    pub screen: usize,
    pub root: Win,
    pub depth: u8,
    pub visual: u32,
    pub argb_visual: Option<u32>,
    pub screen_w: i32,
    pub screen_h: i32,
    pub composited: bool,
    pub fonts: Fonts,
    pub icons: IconStore,
    pub cfg: Config,
    pub tr: Tr,
    pub accent: [u8; 3],
    wins: HashMap<Win, UiWin>,
    gc: Gcontext,
    keysyms: Vec<u32>, // (min_code - keycode_min)*kpk + col
    keycode_min: u32,
    keysyms_per_keycode: usize,
    atoms: HashMap<&'static str, u32>,
    wm_delete: u32,
    atom_wm_protocols: u32,
    tick_accum: u8,
}

impl App {
    pub fn connect() -> Result<App, String> {
        let (conn, screen) = RustConnection::connect(None).map_err(|e| e.to_string())?;
        let (root, screen_w, screen_h, root_depth, root_visual, argb_visual) = {
            let scr = &conn.setup().roots[screen];
            let mut argb_visual = None;
            // Find a 32-bit TrueColor visual (ARGB)
            for depth in &scr.allowed_depths {
                if depth.depth == 32 {
                    for v in &depth.visuals {
                        if v.class == VisualClass::TRUE_COLOR {
                            argb_visual = Some(v.visual_id);
                            break;
                        }
                    }
                    if argb_visual.is_some() {
                        break;
                    }
                }
            }
            (scr.root, scr.width_in_pixels as i32, scr.height_in_pixels as i32,
             scr.root_depth, scr.root_visual, argb_visual)
        };

        // Compositor detection: _NET_WM_CM_S<n> selection owner
        let cm_atom = conn
            .intern_atom(false, format!("_NET_WM_CM_S{}", conn.setup().roots[screen].root).as_bytes())
            .ok()
            .and_then(|r| r.reply().ok())
            .map(|r| r.atom);
        let composited = match cm_atom {
            Some(a) => conn
                .get_selection_owner(a)
                .ok()
                .and_then(|r| r.reply().ok())
                .map(|r| r.owner != 0u32)
                .unwrap_or(false),
            None => false,
        };

        let gc = conn.generate_id().unwrap();
        conn.create_gc(gc, root, &CreateGCAux::new()).unwrap();

        // Keyboard mapping for keysym lookup
        let min_kc = conn.setup().min_keycode as u32;
        let max_kc = conn.setup().max_keycode as u32;
        let kpk = if max_kc >= min_kc { (max_kc - min_kc + 1) as usize } else { 0 };
        let mut keysyms = Vec::new();
        if kpk > 0 {
            if let Ok(r) = conn.get_keyboard_mapping(min_kc as u8, (max_kc - min_kc + 1) as u8) {
                if let Ok(reply) = r.reply() {
                    keysyms = reply.keysyms.clone();
                }
            }
        }
        let keysyms_len = keysyms.len();

        let cfg = Config::load();
        let tr = Tr::load(&cfg.language);
        let accent = Config::hex_rgb(&cfg.accent);

        Ok(App {
            conn,
            screen,
            root,
            depth: root_depth,
            visual: root_visual,
            argb_visual,
            screen_w,
            screen_h,
            composited,
            fonts: Fonts::new(),
            icons: IconStore::new(),
            cfg,
            tr,
            accent,
            wins: HashMap::new(),
            gc,
            keysyms,
            keycode_min: min_kc,
            keysyms_per_keycode: if kpk > 0 { keysyms_len / kpk } else { 0 },
            atoms: HashMap::new(),
            wm_delete: 0,
            atom_wm_protocols: 0,
            tick_accum: 0,
        })
    }

    pub fn atom(&mut self, name: &'static str) -> u32 {
        if let Some(&a) = self.atoms.get(name) {
            return a;
        }
        let a = self
            .conn
            .intern_atom(false, name.as_bytes())
            .ok()
            .and_then(|r| r.reply().ok())
            .map(|r| r.atom)
            .unwrap_or(0);
        // (intern_atom takes &'static name; we cache below)
        self.atoms.insert(name, a);
        a
    }

    pub fn keysym_for(&self, keycode: u8) -> u32 {
        if keycode < self.keycode_min as u8 || self.keysyms_per_keycode == 0 {
            return 0;
        }
        let idx = (keycode as usize - self.keycode_min as usize) * self.keysyms_per_keycode;
        // column 0 (unshifted); fall back to column 1
        if idx < self.keysyms.len() && self.keysyms[idx] != 0 {
            return self.keysyms[idx];
        }
        if idx + 1 < self.keysyms.len() {
            self.keysyms[idx + 1]
        } else {
            0
        }
    }

    /// Create a shell window and register its handler.
    pub fn add_win(&mut self, opts: WinOpts, handler: Box<dyn Handler>) -> Win {
        let id = self.conn.generate_id().unwrap();
        // 32-bit ARGB windows render correctly only under a compositor; without
        // one we fall back to opaque 24-bit windows (apps draw the fallback style).
        let use_argb = opts.argb && self.argb_visual.is_some() && self.composited;
        let (depth, visual, aux) = if use_argb {
            let cmap = self.conn.generate_id().unwrap();
            self.conn
                .create_colormap(ColormapAlloc::NONE, cmap, self.root, self.argb_visual.unwrap())
                .unwrap();
            let mut a = CreateWindowAux::new().colormap(cmap).event_mask(opts.event_mask);
            if opts.override_redirect {
                a = a.override_redirect(1);
            }
            (32u8, self.argb_visual.unwrap(), a)
        } else {
            let mut a = CreateWindowAux::new().event_mask(opts.event_mask);
            if opts.override_redirect {
                a = a.override_redirect(1);
            }
            (self.depth, self.visual, a)
        };
        self.conn
            .create_window(depth, id, self.root, opts.x as i16, opts.y as i16, opts.w as u16, opts.h as u16, 0,
                WindowClass::INPUT_OUTPUT, visual, &aux)
            .unwrap();

        if !opts.window_type.is_empty() {
            let type_name = format!("_NET_WM_WINDOW_TYPE_{}", opts.window_type.to_uppercase());
            let t = self.atom("_NET_WM_WINDOW_TYPE");
            let v = match self.conn.intern_atom(false, type_name.as_bytes()).ok()
                .and_then(|r| r.reply().ok()) { Some(r) => r.atom, None => 0 };
            self.conn
                .change_property32(PropMode::REPLACE, id, t, 4u32, &[v])
                .unwrap();
        }
        // WM_CLASS so tools can find us
        let cls = format!("{}\0{}", opts.class, opts.class);
        let clsa = self.atom("WM_CLASS");
        self.conn
            .change_property8(PropMode::REPLACE, id, clsa, 31u32, cls.as_bytes())
            .unwrap();
        if !opts.override_redirect {
            // normal managed window: set name + delete protocol
            let na = self.atom("_NET_WM_NAME");
            let utf8 = self.atom("UTF8_STRING");
            self.conn
                .change_property8(PropMode::REPLACE, id, na, utf8, opts.title.as_bytes())
                .unwrap();
            self.atom_wm_protocols = self.atom("WM_PROTOCOLS");
            self.wm_delete = self.atom("WM_DELETE_WINDOW");
            self.conn
                .change_property32(PropMode::REPLACE, id, self.atom_wm_protocols, 4u32, &[self.wm_delete])
                .unwrap();
        }

        self.wins.insert(id, UiWin {
            handler,
            canvas: Canvas::new(opts.w, opts.h),
            w: opts.w,
            h: opts.h,
            depth,
        });
        self.conn.map_window(id).unwrap();
        self.redraw(id);
        id
    }

    pub fn redraw(&mut self, id: Win) {
        let Some(mut w) = self.wins.remove(&id) else { return };
        if w.canvas.w != w.w || w.canvas.h != w.h {
            w.canvas = Canvas::new(w.w, w.h);
        } else {
            w.canvas = Canvas::new(w.w, w.h); // always start from a clean buffer
        }
        w.handler.draw(self, &mut w.canvas);
        if std::env::var("BRAVOS_DEBUG_CANVAS").is_ok() {
            let path = format!("/tmp/canvas-{:#x}.png", id);
            if let Ok(file) = std::fs::File::create(&path) {
                let mut enc = png::Encoder::new(file, w.canvas.w as u32, w.canvas.h as u32);
                enc.set_color(png::ColorType::Rgba);
                enc.set_depth(png::BitDepth::Eight);
                if let Ok(mut wr) = enc.write_header() {
                    let _ = wr.write_image_data(&w.canvas.data);
                }
            }
        }
        if let Ok(cookie) = self.conn.put_image(
            ImageFormat::Z_PIXMAP, id, self.gc, w.w as u16, w.h as u16, 0, 0, 0, w.depth,
            &w.canvas.to_argb(),
        ) {
            if let Err(e) = cookie.check() {
                eprintln!("bui: put_image error on win {} depth {}: {:?}", id, w.depth, e);
            }
        }
        self.wins.insert(id, w);
        self.conn.flush().ok();
    }

    pub fn set_geom(&mut self, id: Win, x: i32, y: i32, w: i32, h: i32) {
        self.conn
            .configure_window(id, &ConfigureWindowAux::new()
                .x(x as i32).y(y as i32).width(w as u32).height(h as u32))
            .ok();
        if let Some(win) = self.wins.get_mut(&id) {
            win.w = w;
            win.h = h;
        }
        self.redraw(id);
    }

    /// True when the delete-window protocol wants the window closed.
    pub fn is_wm_delete(&self, ev: &ClientMessageEvent) -> bool {
        ev.type_ == self.atom_wm_protocols
            && ev.data.as_data32()[0] == self.wm_delete
    }

    /// Apply a bounding shape (spans of (y, x_start, width)) — used for the
    /// dock dome and rounded popups when no compositor is running.
    pub fn set_shape_spans(&mut self, id: Win, spans: &[(i32, i32, i32)]) {
        use x11rb::protocol::shape::{self, SK, SO};
        let (w, h) = match self.wins.get(&id) {
            Some(win) => (win.w, win.h),
            None => return,
        };
        if w <= 0 || h <= 0 || spans.is_empty() {
            return;
        }
        let pm = self.conn.generate_id().unwrap();
        self.conn.create_pixmap(1, pm, self.root, w as u16, h as u16).unwrap();
        let gc = self.conn.generate_id().unwrap();
        self.conn.create_gc(gc, pm, &CreateGCAux::new().foreground(1)).unwrap();
        let rects: Vec<Rectangle> = spans
            .iter()
            .map(|(y, x, ww)| Rectangle { x: *x as i16, y: *y as i16, width: *ww as u16, height: 1 })
            .collect();
        self.conn.poly_fill_rectangle(pm, gc, &rects).unwrap();
        self.conn.flush().unwrap();
        self.conn
            .shape_mask(SO::SET, SK::BOUNDING, id, 0, 0, pm)
            .unwrap();
        self.conn.free_gc(gc).ok();
        self.conn.free_pixmap(pm).ok();
        self.conn.flush().ok();
    }

    /// Ids of all registered windows.
    pub fn win_ids(&self) -> Vec<Win> {
        self.wins.keys().copied().collect()
    }

    /// Run the event loop until `stop` returns true.
    pub fn run_until(&mut self, stop: &dyn Fn(&App) -> bool) {
        loop {
            if stop(self) {
                return;
            }
            // Dispatch all pending events, then tick
            let mut handled_any = false;
            while let Some(ev) = self
                .conn
                .poll_for_event()
                .ok()
                .flatten()
            {
                self.dispatch(ev);
                handled_any = true;
                if stop(self) {
                    return;
                }
            }
            if !handled_any {
                // 250 ms sleep between idle passes; also drive 1s ticks
                std::thread::sleep(std::time::Duration::from_millis(40));
                self.tick_accum += 1;
                if self.tick_accum >= 25 {
                    self.tick_accum = 0;
                    let ids: Vec<Win> = self.wins.keys().copied().collect();
                    for id in ids {
                        let Some(mut w) = self.wins.remove(&id) else { continue };
                        w.handler.tick(self);
                        self.wins.insert(id, w);
                    }
                }
            }
        }
    }

    fn dispatch(&mut self, ev: Event) {
        match ev {
            Event::Expose(e) => {
                if e.count == 0 {
                    self.redraw(e.window);
                }
            }
            Event::ConfigureNotify(e) => {
                let id = e.window;
                if let Some(w) = self.wins.get(&id) {
                    if e.width as i32 != w.w || e.height as i32 != w.h {
                        let (nw, nh) = (e.width as i32, e.height as i32);
                        if let Some(w) = self.wins.get_mut(&id) {
                            w.w = nw;
                            w.h = nh;
                        }
                        let mut w = self.wins.remove(&id).unwrap();
                        w.handler.resized(self, nw, nh);
                        self.wins.insert(id, w);
                        self.redraw(id);
                    }
                }
            }
            Event::ButtonPress(e) => {
                let (id, x, y) = (e.event, e.event_x as i32, e.event_y as i32);
                let mut w = match self.wins.remove(&id) { Some(w) => w, None => return };
                w.handler.pointer(self, PtrKind::Down(e.detail), x, y);
                self.wins.insert(id, w);
            }
            Event::ButtonRelease(e) => {
                let (id, x, y) = (e.event, e.event_x as i32, e.event_y as i32);
                let mut w = match self.wins.remove(&id) { Some(w) => w, None => return };
                w.handler.pointer(self, PtrKind::Up(e.detail), x, y);
                self.wins.insert(id, w);
            }
            Event::MotionNotify(e) => {
                let (id, x, y) = (e.event, e.event_x as i32, e.event_y as i32);
                let mut w = match self.wins.remove(&id) { Some(w) => w, None => return };
                w.handler.pointer(self, PtrKind::Move, x, y);
                self.wins.insert(id, w);
            }
            Event::KeyPress(e) => {
                let id = e.event;
                let kc = e.detail;
                let state = e.state;
                let ks = self.keysym_for(kc);
                let mut w = match self.wins.remove(&id) { Some(w) => w, None => return };
                w.handler.key(self, ks, state.bits() as u16);
                self.wins.insert(id, w);
            }
            Event::ClientMessage(e) => {
                if self.is_wm_delete(&e) {
                    // treat as stop signal for normal windows
                    self.wins.remove(&e.window);
                } else if e.type_ == self.atom("_BRAVOS_CONFIG") {
                    // config reloaded: refresh shared state, redraw everything
                    self.cfg = Config::load();
                    self.tr = Tr::load(&self.cfg.language);
                    self.accent = Config::hex_rgb(&self.cfg.accent);
                    let ids: Vec<Win> = self.wins.keys().copied().collect();
                    for id in ids {
                        self.redraw(id);
                    }
                }
            }
            Event::PropertyNotify(e) => {
                let id = e.window;
                let mut w = match self.wins.remove(&id) { Some(w) => w, None => return };
                w.handler.property(self, e.atom);
                self.wins.insert(id, w);
            }
            _ => {}
        }
    }

    /// Poll keyboard mapping after layout changes.
    pub fn refresh_keyboard(&mut self) {
        let setup = self.conn.setup();
        let min_kc = setup.min_keycode as u8;
        let count = (setup.max_keycode - setup.min_keycode + 1) as u8;
        if count > 0 {
            if let Ok(cookie) = self.conn.get_keyboard_mapping(min_kc, count) {
                if let Ok(r) = cookie.reply() {
                    let kpk = r.keysyms.len() / count as usize;
                    self.keysyms = r.keysyms.clone();
                    self.keysyms_per_keycode = kpk;
                }
            }
        }
    }
}
