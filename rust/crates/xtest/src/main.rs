// Color-format probe: creates a 64x64 32-bit ARGB window filled with pure RED,
// and a 24-bit window filled with pure RED, then sleeps.
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::wrapper::ConnectionExt as WrapExt;
use x11rb::rust_connection::RustConnection;

fn main() {
    let (conn, screen) = RustConnection::connect(None).unwrap();
    if std::env::args().nth(1).as_deref() == Some("client") {
        // simple managed test app: name from argv[2], color from argv[3]
        let name = std::env::args().nth(2).unwrap_or_else(|| "TestApp".into());
        let (r, g, b) = match std::env::args().nth(3).as_deref() {
            Some("green") => (40, 160, 90),
            Some("blue") => (60, 90, 200),
            _ => (200, 90, 40),
        };
        let scr = &conn.setup().roots[screen];
        let win = conn.generate_id().unwrap();
        conn.create_window(scr.root_depth, win, scr.root, 200, 200, 500, 360, 0,
            WindowClass::INPUT_OUTPUT, scr.root_visual,
            &CreateWindowAux::new().background_pixel(
                ((r as u32) << 16) | ((g as u32) << 8) | b as u32)
                .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY | EventMask::PROPERTY_CHANGE)).unwrap();
        let nm = format!("{}\0{}", name, name);
        conn.change_property8(PropMode::REPLACE, win, 39u32, 31u32, name.as_bytes()).unwrap();
        conn.change_property8(PropMode::REPLACE, win, 67u32, 31u32, nm.as_bytes()).unwrap();
        conn.map_window(win).unwrap();
        conn.flush().unwrap();
        loop { std::thread::sleep(std::time::Duration::from_secs(60)); }
    }
    if std::env::args().nth(1).as_deref() == Some("click") {
        // click <window-id-hex> <x> <y> [button]
        let win = u32::from_str_radix(std::env::args().nth(2).unwrap_or_default().trim_start_matches("0x"), 16).unwrap();
        let x: i16 = std::env::args().nth(3).unwrap().parse().unwrap();
        let y: i16 = std::env::args().nth(4).unwrap().parse().unwrap();
        let button: u8 = std::env::args().nth(5).and_then(|s| s.parse().ok()).unwrap_or(1);
        let scr = &conn.setup().roots[screen];
        for detail in [BUTTON_PRESS_EVENT, BUTTON_RELEASE_EVENT] {
            let ev = ButtonPressEvent {
                response_type: detail,
                detail: button,
                sequence: 0,
                time: 0,
                root: scr.root,
                event: win,
                child: 0,
                root_x: x, root_y: y,
                event_x: x, event_y: y,
                state: KeyButMask::from(0u16),
                same_screen: true,
            };
            conn.send_event(false, win, EventMask::NO_EVENT, &ev).unwrap();
        }
        conn.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(150));
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("key") {
        // key <keycode> [mods-u16] — sends synthetic KeyPress+Release to root
        let kc: u8 = std::env::args().nth(2).unwrap().parse().unwrap();
        let mods: u16 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let scr = &conn.setup().roots[screen];
        for detail in [KEY_PRESS_EVENT, KEY_RELEASE_EVENT] {
            let ev = KeyPressEvent {
                response_type: detail,
                detail: kc,
                sequence: 0,
                time: 0,
                root: scr.root,
                event: scr.root,
                child: 0,
                root_x: 5, root_y: 5,
                event_x: 5, event_y: 5,
                state: KeyButMask::from(mods),
                same_screen: true,
            };
            conn.send_event(false, scr.root, EventMask::KEY_PRESS, &ev).unwrap();
        }
        conn.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(150));
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("tree") {
        let scr = &conn.setup().roots[screen];
        let tree = conn.query_tree(scr.root).unwrap().reply().unwrap();
        for w in tree.children {
            let geo = conn.get_geometry(w).unwrap().reply().unwrap();
            let attr = conn.get_window_attributes(w).unwrap().reply().unwrap();
            let name = conn.get_property(false, w, 39u32, 0u32, 0, 256).unwrap().reply().unwrap();
            let nm = String::from_utf8_lossy(&name.value).to_string();
            println!("win {:#x} depth={} map={} geo=({},{},{},{}) class_name={:?} ovrd={}",
                w, geo.depth, attr.map_state == x11rb::protocol::xproto::MapState::VIEWABLE,
                geo.x, geo.y, geo.width, geo.height, nm.trim_end_matches(char::from(0)), attr.override_redirect);
        }
        return;
    }
    let scr = &conn.setup().roots[screen];
    let mut argb = None;
    for depth in &scr.allowed_depths {
        if depth.depth == 32 {
            for v in &depth.visuals {
                if v.class == VisualClass::TRUE_COLOR { argb = Some(v.visual_id); break; }
            }
        }
    }
    let av = argb.unwrap();
    // 32-bit window
    let w32 = conn.generate_id().unwrap();
    let cmap = conn.generate_id().unwrap();
    conn.create_colormap(ColormapAlloc::NONE, cmap, scr.root, av).unwrap();
    conn.create_window(32, w32, scr.root, 10, 10, 64, 64, 0,
        WindowClass::INPUT_OUTPUT, av,
        &CreateWindowAux::new().colormap(cmap).override_redirect(1)).unwrap();
    // 24-bit window with solid background pixels (no PutImage)
    let w24 = conn.generate_id().unwrap();
    conn.create_window(scr.root_depth, w24, scr.root, 100, 10, 64, 64, 0,
        WindowClass::INPUT_OUTPUT, scr.root_visual,
        &CreateWindowAux::new().override_redirect(1).background_pixel(0xFF0000)).unwrap();
    // 32-bit window with background pixel
    let w32b = conn.generate_id().unwrap();
    let cmap2 = conn.generate_id().unwrap();
    conn.create_colormap(ColormapAlloc::NONE, cmap2, scr.root, av).unwrap();
    conn.create_window(32, w32b, scr.root, 180, 10, 64, 64, 0,
        WindowClass::INPUT_OUTPUT, av,
        &CreateWindowAux::new().colormap(cmap2).override_redirect(1).background_pixel(0xFF0000)).unwrap();
    let gc = conn.generate_id().unwrap();
    conn.create_gc(gc, scr.root, &CreateGCAux::new()).unwrap();
    // separate GC per depth
    let gc32 = conn.generate_id().unwrap();
    conn.create_gc(gc32, w32, &CreateGCAux::new()).unwrap();
    // RED in B,G,R,A byte order (little-endian ARGB32) and in R,G,B,A order
    let bgra_red: Vec<u8> = vec![0, 0, 255, 255].repeat(64 * 64);
    let rgba_red: Vec<u8> = vec![255, 0, 0, 255].repeat(64 * 64);
    match conn.put_image(ImageFormat::Z_PIXMAP, w32, gc32, 64, 64, 0, 0, 0, 32, &bgra_red) { Ok(c) => { if let Err(e) = c.check() { println!("w32 put_image error: {:?}", e); } else { println!("w32 put_image ok"); } }, Err(e) => println!("w32 req err {:?}", e) }
    match conn.put_image(ImageFormat::Z_PIXMAP, w24, gc, 64, 64, 0, 0, 0, 24, &rgba_red) { Ok(c) => { if let Err(e) = c.check() { println!("w24 put_image error: {:?}", e); } else { println!("w24 put_image ok"); } }, Err(e) => println!("w24 req err {:?}", e) }
    conn.map_window(w32).unwrap();
    conn.map_window(w24).unwrap();
    conn.map_window(w32b).unwrap();
    conn.flush().unwrap();
    // print the visual masks
    for d in &scr.allowed_depths {
        for v in &d.visuals {
            println!("depth {} visual {:#x} masks R={:#x} G={:#x} B={:#x} bpp-units", d.depth, v.visual_id, v.red_mask, v.green_mask, v.blue_mask);
        }
    }
    println!("root depth {}", scr.root_depth);
    println!("probe up");
    std::thread::sleep(std::time::Duration::from_secs(30));
}
