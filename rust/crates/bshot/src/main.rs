//! bshot — BraveOS screenshot tool: full-screen grab to PNG.
//! Usage: bshot [output.png]

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let dir = format!("{}/Pictures", home);
        let _ = std::fs::create_dir_all(&dir);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{}/bravos-shot-{}.png", dir, ts)
    });

    let (conn, screen) = RustConnection::connect(None).expect("bshot: X11 connect");
    let scr = &conn.setup().roots[screen];
    let (w, h) = (scr.width_in_pixels, scr.height_in_pixels);
    let reply = conn
        .get_image(ImageFormat::Z_PIXMAP, scr.root, 0, 0, w, h, !0u32)
        .expect("get_image")
        .reply()
        .expect("get_image reply");
    let data = reply.data;

    // BGRX (24/32-bit) -> RGBA rows
    let mut rgba = Vec::with_capacity(data.len());
    for px in data.chunks_exact(4) {
        rgba.push(px[2]);
        rgba.push(px[1]);
        rgba.push(px[0]);
        rgba.push(255);
    }

    let file = std::fs::File::create(&out).expect("create output");
    let mut enc = png::Encoder::new(file, w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().expect("png header");
    writer.write_image_data(&rgba).expect("png data");
    println!("bshot: saved {}", out);
}
