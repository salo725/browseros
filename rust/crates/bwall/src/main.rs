//! bwall — BraveOS desktop wallpaper (fullscreen desktop window).

use std::cell::RefCell;
use std::rc::Rc;

use bui::canvas::Canvas;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use bui::pngutil::Image;
use bui::xapp::{App, Handler, WinOpts};

#[derive(Clone)]
struct Wall {
    img: Option<Image>,
    stamp: u64,
    wp: String,
    win: u32,
}

fn config_stamp() -> u64 {
    std::fs::metadata("/tmp/.bravos-config-stamp")
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

struct WallH(Rc<RefCell<Wall>>);
impl Handler for WallH {
    fn draw(&mut self, _app: &mut App, cv: &mut Canvas) {
        let w = self.0.borrow();
        cv.fill([8, 8, 12, 255]);
        if let Some(img) = &w.img {
            cv.image(img, 0, 0);
        } else {
            // fallback gradient
            for y in 0..cv.h {
                let t = ((y * 255) / cv.h.max(1)) as u32;
                let c = bui::geom::lerp_rgb([24, 20, 48], [140, 40, 90], t);
                cv.rect(0, y, cv.w, 1, [c[0], c[1], c[2], 255]);
            }
        }
    }

    fn tick(&mut self, app: &mut App) {
        let s = config_stamp();
        let mut w = self.0.borrow_mut();
        if s != w.stamp {
            w.stamp = s;
            app.cfg = bui::config::Config::load();
            if app.cfg.wallpaper != w.wp {
                w.wp = app.cfg.wallpaper.clone();
                w.img = load_wall(app, &w.wp);
                // our own window is out of the map while our tick runs: this
                // queues the redraw and the event loop flushes it right after
                let id = w.win;
                drop(w);
                app.redraw(id);
            }
        }
    }
}

fn load_wall(app: &App, name: &str) -> Option<Image> {
    let base = std::env::var("BRAVOS_WALLPAPER_DIR")
        .unwrap_or_else(|_| "/usr/share/bravos/wallpapers".into());
    let img = Image::decode(&format!("{}/{}", base, name))
        .or_else(|| Image::decode(&format!("{}/braveos.png", base)))?;
    Some(img.cover(app.screen_w, app.screen_h))
}

fn main() {
    let mut app = App::connect().expect("X11 connect");
    let wp_name = app.cfg.wallpaper.clone();
    let img = load_wall(&app, &wp_name);
    let wall = Rc::new(RefCell::new(Wall {
        img,
        stamp: config_stamp(),
        wp: wp_name,
        win: 0,
    }));
    let win = app.add_win(
        WinOpts {
            x: 0,
            y: 0,
            w: app.screen_w,
            h: app.screen_h,
            override_redirect: true,
            argb: false,
            window_type: "desktop",
            class: "bwall",
            title: "bravos-wall",
            event_mask: bui::xapp::EventMask::EXPOSURE,
        },
        Box::new(WallH(wall.clone())),
    );
    wall.borrow_mut().win = win;
    // drop to the bottom of the root's sibling stack so the shell sits above
    app.conn
        .configure_window(win,
            &ConfigureWindowAux::new().stack_mode(StackMode::BELOW))
        .ok();
    app.conn.flush().ok();
    app.run_until(&|_app| false);
}
