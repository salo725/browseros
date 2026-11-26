//! bwall — BraveOS desktop wallpaper (fullscreen desktop window).

use bui::canvas::Canvas;
use bui::pngutil::Image;
use bui::xapp::{App, Handler, WinOpts};

struct Wall {
    img: Option<Image>,
    stamp: u64,
    wp: String,
}

fn config_stamp() -> u64 {
    std::fs::metadata("/tmp/.bravos-config-stamp")
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

impl Handler for Wall {
    fn draw(&mut self, app: &mut App, cv: &mut Canvas) {
        cv.fill([8, 8, 12, 255]);
        if let Some(img) = &self.img {
            cv.image(img, 0, 0);
        } else {
            // fallback gradient
            for y in 0..cv.h {
                let t = ((y * 255) / cv.h.max(1)) as u32;
                let c = bui::geom::lerp_rgb([24, 20, 48], [140, 40, 90], t);
                cv.rect(0, y, cv.w, 1, [c[0], c[1], c[2], 255]);
            }
        }
        let _ = app;
    }

    fn tick(&mut self, app: &mut App) {
        let s = config_stamp();
        if s != self.stamp {
            self.stamp = s;
            app.cfg = bui::config::Config::load();
            if app.cfg.wallpaper != self.wp {
                self.wp = app.cfg.wallpaper.clone();
                self.img = load_wall(app, &self.wp);
                let id = app.win_ids()[0];
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
    let wall = Wall { img, stamp: config_stamp(), wp: wp_name };
    app.add_win(
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
        Box::new(wall),
    );
    app.run_until(&|_app| false);
}
