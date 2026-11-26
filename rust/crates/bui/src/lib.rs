//! bui — BraveOS shared UI toolkit (pure Rust, X11 + freetype/fontconfig).
//!
//! Provides: PNG decoding, an RGBA canvas with glass-style drawing helpers,
//! text shaping/rasterization with per-character font fallback (all scripts),
//! i18n loading, config storage, icon loading, and a small X11 event-loop
//! framework used by the window manager, shell (panel + dock), settings app,
//! wallpaper and screenshot tools.

pub mod canvas;
pub mod config;
pub mod font;
pub mod geom;
pub mod i18n;
pub mod icons;
pub mod pngutil;
pub mod xapp;

pub use canvas::Canvas;
pub use config::Config;
pub use geom::Rect;
pub use i18n::Tr;
pub use xapp::App;
