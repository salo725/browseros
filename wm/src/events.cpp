// BraveWM - events.cpp: X event dispatch, keybindings, mouse interactions
// (move/resize/snap), root/window menus, client messages.
#include "wm.hpp"
#include "menu.hpp"
#include "switcher.hpp"
#include "bar.hpp"

#include <poll.h>

namespace bwm {

// ---- keybinding table ----------------------------------------------------------

struct KeyBinding {
    const char *keysym_name;
    unsigned int modifiers;  // Mod1Mask (Alt), Mod4Mask (Super), ControlMask, ShiftMask
};

static const unsigned int A = Mod1Mask;     // Alt
static const unsigned int S = Mod4Mask;     // Super
static const unsigned int C = ControlMask;

// ---- scanning / run loop --------------------------------------------------------

void Wm::scan_existing() {
    Window rroot, parent;
    Window *children = nullptr;
    unsigned int n = 0;
    if (XQueryTree(dpy, root, &rroot, &parent, &children, &n) && children) {
        for (unsigned int i = 0; i < n; i++) {
            XWindowAttributes wa;
            if (children[i] == wm_check_win || children[i] == snap_preview)
                continue;
            if (XGetWindowAttributes(dpy, children[i], &wa) && !wa.override_redirect)
                manage(children[i]);
        }
        XFree(children);
    }
}

void Wm::run() {
    while (running) {
        if (g_sigterm_flag || g_sigrestart_flag) break;  // signal-driven exit
        // status bar clock/poll at ~1 Hz (throttled internally)
        session_bar_tick();
        // tick animations at ~60fps when anything moves
        bool any_anim = false;
        for (Client *c : clients)
            if (c->animating) any_anim = true;

        if (XPending(dpy)) {
            XEvent e;
            XNextEvent(dpy, &e);
            last_event_time = CurrentTime;
            switch (e.type) {
                case MapRequest: on_map_request(e.xmaprequest); break;
                case ConfigureRequest: on_configure_request(e.xconfigurerequest); break;
                case DestroyNotify: on_destroy(e.xdestroywindow); break;
                case UnmapNotify: on_unmap(e.xunmap); break;
                case ReparentNotify: on_reparent(e.xreparent); break;
                case PropertyNotify: on_property(e.xproperty); break;
                case ClientMessage: on_client_message(e.xclient); break;
                case Expose: on_expose(e.xexpose); break;
                case ButtonPress:
                    last_event_time = e.xbutton.time;
                    on_button_press(e.xbutton);
                    break;
                case ButtonRelease:
                    last_event_time = e.xbutton.time;
                    on_button_release(e.xbutton);
                    break;
                case MotionNotify: on_motion(e.xmotion); break;
                case EnterNotify:
                case LeaveNotify: on_enter(e.xcrossing); break;
                case KeyPress:
                    last_event_time = e.xkey.time;
                    on_key_press(e.xkey);
                    break;
                case KeyRelease: {
                    KeySym ks = XKeycodeToKeysym(dpy, e.xkey.keycode, 0);
                    if (switcher && switcher->active() &&
                        (ks == XK_Alt_L || ks == XK_Alt_R))
                        switcher->commit();
                    break;
                }
                case MapNotify:
                case CirculateNotify:
                default: break;
            }
            if (e.type == MapRequest) on_map_request(e.xmaprequest);
            else if (e.type == ConfigureNotify && e.xconfigure.window == root) {
                // root resized: recompute monitors, workarea, bar geometry
                refresh_monitors();
                update_workarea();
                if (bar) bar->relayout();
            }
            continue;
        }
        if (any_anim) {
            tick_animations();
            XFlush(dpy);
            poll(nullptr, 0, 12);
        } else {
            poll(nullptr, 0, 40);
        }
    }
}

// ---- keybindings ------------------------------------------------------------------

void Wm::grab_keys() {
    struct Grab {
        const char *sym;
        unsigned int mod;
    };
    Grab grabs[] = {
        {"Tab", A}, {"Tab", A | ShiftMask},
        {"F4", A},
        {"Up", S}, {"Down", S}, {"Left", S}, {"Right", S},
        {"D", S},
        {"E", S}, {"S", S}, {"B", S},
        {"F1", S},
        {"Escape", S},
        {"space", S},
        {"T", C | A},
        {"F2", A},
        {"L", C | A},
        {"Left", C | A}, {"Right", C | A},
        {"1", S}, {"2", S}, {"3", S}, {"4", S}, {"5", S},
        {"6", S}, {"7", S}, {"8", S}, {"9", S},
        {"Print", 0},
        {"XF86AudioRaiseVolume", 0},
        {"XF86AudioLowerVolume", 0},
        {"XF86AudioMute", 0},
        {"Shift_L", C},
        {"space", A},
    };
    unsigned int lock_variants[] = {0, LockMask, Mod2Mask, LockMask | Mod2Mask};
    for (auto &g : grabs) {
        KeySym ks = XStringToKeysym(g.sym);
        if (ks == NoSymbol) continue;
        KeyCode kc = XKeysymToKeycode(dpy, ks);
        if (kc == 0) continue;
        for (unsigned int lv : lock_variants) {
            XGrabKey(dpy, kc, g.mod | lv, root, True, GrabModeAsync, GrabModeAsync);
        }
    }
}

static std::string current_iso_date() {
    char buf[64];
    time_t t = time(nullptr);
    struct tm tmv;
    localtime_r(&t, &tmv);
    strftime(buf, sizeof(buf), "%Y-%m-%d-%H%M%S", &tmv);
    return buf;
}

void Wm::on_key_press(XKeyEvent &e) {
    if (menu && menu->active()) {
        menu->handle_key(e);
        return;
    }
    if (switcher && switcher->active()) {
        switcher->handle_key(e);
        return;
    }
    unsigned int state = e.state & ~(LockMask | Mod2Mask);
    KeySym ks = XKeycodeToKeysym(dpy, e.keycode, 0);

    Client *c = focused_client;

    // Alt+Tab / Alt+Shift+Tab
    if (ks == XK_Tab && (state & A)) {
        switcher->begin(state & ShiftMask);
        return;
    }
    // Alt+F4 close
    if (ks == XK_F4 && state == A) {
        if (c) close(c);
        return;
    }
    // Super+arrows: snapping
    if (state == S) {
        Rect wa = c ? workarea_for(c) : workarea_for_monitor(monitor_for_point(e.x_root, e.y_root));
        if (ks == XK_Up && c) { toggle_maximize(c); return; }
        if (ks == XK_Down && c) {
            if (c->maximized) { toggle_maximize(c); return; }
            // snap to bottom half
            c->restore = c->geom;
            c->maximized = false;
            resize_window(c, wa.x, wa.y + wa.h / 2, wa.w, wa.h / 2);
            return;
        }
        if (ks == XK_Left && c) {
            c->restore = c->geom;
            c->maximized = false;
            resize_window(c, wa.x, wa.y, wa.w / 2, wa.h);
            return;
        }
        if (ks == XK_Right && c) {
            c->restore = c->geom;
            c->maximized = false;
            resize_window(c, wa.x + wa.w / 2, wa.y, wa.w / 2, wa.h);
            return;
        }
        if (ks == XK_d || ks == XK_D) { toggle_show_desktop(); return; }
        if (ks >= XK_1 && ks <= XK_9) {
            switch_desktop(ks - XK_1);
            return;
        }
        // Super+E/S/B: files / settings / browser
        if (ks == XK_e || ks == XK_E) { spawn(cfg.files_cmd); return; }
        if (ks == XK_s || ks == XK_S) { spawn(cfg.settings_cmd); return; }
        if (ks == XK_b || ks == XK_B) { spawn(cfg.browser_cmd); return; }
        // Super+F1: keyboard shortcut cheatsheet
        if (ks == XK_F1) {
            spawn(cfg.settings_cmd + " --page shortcuts");
            return;
        }
        // Super+Escape: power menu at pointer
        if (ks == XK_Escape) {
            int rx = 0, ry = 0, wx = 0, wy = 0;
            Window rr, cr;
            unsigned int mk;
            if (XQueryPointer(dpy, root, &rr, &cr, &rx, &ry, &wx, &wy, &mk))
                open_power_menu(rx, ry);
            return;
        }
        // Super+Space: root menu at pointer
        if (ks == XK_space) {
            int rx = 0, ry = 0, wx = 0, wy = 0;
            Window rr, cr;
            unsigned int mk;
            if (XQueryPointer(dpy, root, &rr, &cr, &rx, &ry, &wx, &wy, &mk))
                open_root_menu(rx, ry);
            return;
        }
    }
    // Ctrl+Alt+T terminal
    if (ks == XK_t || ks == XK_T) {
        if (state == (C | A)) { spawn(cfg.term_cmd); return; }
    }
    // Alt+F2 launcher
    if (ks == XK_F2 && state == A) { spawn(cfg.launcher_cmd); return; }
    // Ctrl+Alt+L lock
    if ((ks == XK_l || ks == XK_L) && state == (C | A)) { spawn(cfg.lock_cmd); return; }
    // Ctrl+Alt+Left/Right workspace nav
    if (state == (C | A)) {
        if (ks == XK_Left) { switch_desktop((current_desktop + num_desktops - 1) % num_desktops); return; }
        if (ks == XK_Right) { switch_desktop((current_desktop + 1) % num_desktops); return; }
    }
    // Print: screenshot
    if (ks == XK_Print) {
        spawn("scrot -z \"$(xdg-user-dir PICTURES)/Screenshot-" +
              current_iso_date() + ".png\"");
        return;
    }
    // Media keys
    if (ks == XStringToKeysym("XF86AudioRaiseVolume")) {
        spawn("pactl set-sink-volume @DEFAULT_SINK@ +5%; bravos-osd volume up");
        return;
    }
    if (ks == XStringToKeysym("XF86AudioLowerVolume")) {
        spawn("pactl set-sink-volume @DEFAULT_SINK@ -5%; bravos-osd volume down");
        return;
    }
    if (ks == XStringToKeysym("XF86AudioMute")) {
        spawn("pactl set-sink-mute @DEFAULT_SINK@ toggle; bravos-osd volume mute");
        return;
    }
    // Ctrl+Shift: keyboard layout toggle (delegated to bravos-kb-switch)
    if ((ks == XK_Shift_L || ks == XK_Shift_R) && state == C) {
        cycle_keyboard_layout();
        return;
    }
    // Alt+Space: window menu for the focused window
    if ((ks == XK_space) && state == A) {
        if (c) {
            int px = c->geom.x + c->geom.w / 2;
            int py = c->geom.y + c->geom.h / 2;
            open_window_menu(c, px, py);
        }
        return;
    }
}

// ---- mouse ----------------------------------------------------------------------

void Wm::on_button_press(XButtonEvent &e) {
    if (menu && menu->active()) {
        menu->handle_button(e);
        return;
    }
    if (switcher && switcher->active()) switcher->commit();

    // status bar interactions (workspace pills, status icons, power)
    if (bar && bar->owns(e.window)) {
        bar->handle_button(e);
        return;
    }

    // root clicks
    if (e.window == root) {
        if (e.button == Button3) open_root_menu(e.x_root, e.y_root);
        else if (e.button == Button1) unfocus();
        else if (e.button == Button4) switch_desktop((current_desktop + num_desktops - 1) % num_desktops);
        else if (e.button == Button5) switch_desktop((current_desktop + 1) % num_desktops);
        return;
    }

    // client-window clicks (Alt+drag anywhere in a window)
    Client *c = find_client(e.window);
    if (c && !c->never_focus && !c->iconified && c->visible) focus(c, false);
    if (c && (e.state & Mod1Mask)) {
        if (e.button == Button1) { start_interactive_move(c, e.x_root, e.y_root); return; }
        if (e.button == Button3) { start_interactive_resize(c, e.x_root, e.y_root, 5); return; }
    }

    // frame clicks
    Client *fc = find_client_by_frame(e.window);
    if (!fc) return;
    if (!fc->visible) return;
    focus(fc, false);

    if (e.button == Button3) {
        open_window_menu(fc, e.x_root, e.y_root);
        return;
    }
    if (e.button == Button1) {
        // titlebar?
        int tb = fc->titlebar_rect().h;
        if (e.y <= tb) {
            BtnHit hit = fc->hit_button(e.x, e.y);
            if (hit == BTN_CLOSE) { close(fc); return; }
            if (hit == BTN_MAX) { toggle_maximize(fc); return; }
            if (hit == BTN_MIN) { iconify(fc); return; }
            // double-click = maximize
            long now_ms = e.time;
            if (now_ms - fc->last_click_ms < 400) {
                fc->last_click_ms = 0;
                toggle_maximize(fc);
                return;
            }
            fc->last_click_ms = now_ms;
        }
        // resize from edges?
        int edge = fc->hit_edge(e.x, e.y);
        if (edge) { start_interactive_resize(fc, e.x_root, e.y_root, edge); return; }
        start_interactive_move(fc, e.x_root, e.y_root);
        return;
    }
}

void Wm::start_interactive_move(Client *c, int px, int py) {
    if (!c || c->fullscreen) return;
    interact = Interact::Move;
    interact_c = c;
    interact_edge = 0;
    start_px = px;
    start_py = py;
    start_geom = c->geom;
    snap_zone = 0;
    XGrabPointer(dpy, root, False,
                 ButtonPressMask | ButtonReleaseMask | PointerMotionMask,
                 GrabModeAsync, GrabModeAsync, root, cursor_move, CurrentTime);
    XDefineCursor(dpy, root, cursor_move);
}

void Wm::start_interactive_resize(Client *c, int px, int py, int edge) {
    if (!c || c->fullscreen || !edge) return;
    interact = Interact::Resize;
    interact_c = c;
    interact_edge = edge;
    start_px = px;
    start_py = py;
    start_geom = c->geom;
    XGrabPointer(dpy, root, False,
                 ButtonPressMask | ButtonReleaseMask | PointerMotionMask,
                 GrabModeAsync, GrabModeAsync, root, cursor_move, CurrentTime);
    if (edge == 1) XDefineCursor(dpy, root, cursor_h);
    else if (edge == 2) XDefineCursor(dpy, root, cursor_h);
    else if (edge == 4) XDefineCursor(dpy, root, cursor_v);
    else if (edge == 8) XDefineCursor(dpy, root, cursor_v);
    else if (edge == (1 | 4)) XDefineCursor(dpy, root, cursor_nw);
    else if (edge == (2 | 4)) XDefineCursor(dpy, root, cursor_ne);
    else if (edge == (1 | 8)) XDefineCursor(dpy, root, cursor_sw);
    else XDefineCursor(dpy, root, cursor_se);
}

void Wm::update_snap_zone(int px, int py) {
    Rect mon = monitor_for_point(px, py);
    Rect wa = workarea_for_monitor(mon);
    int m = 28;
    bool near_left = px <= wa.x + m;
    bool near_right = px >= wa.x + wa.w - m;
    bool near_top = py <= wa.y + m;
    bool near_bottom = py >= wa.y + wa.h - m;
    int z = 0;
    if (near_top && near_left) z = 4;       // top-left quarter
    else if (near_top && near_right) z = 5;  // top-right quarter
    else if (near_bottom && near_left) z = 6;
    else if (near_bottom && near_right) z = 7;
    else if (near_top) z = 3;              // maximize
    else if (near_left) z = 1;             // left half
    else if (near_right) z = 2;            // right half
    if (z != snap_zone) {
        snap_zone = z;
        if (!z) hide_snap_preview();
        else if (z != 3) show_snap_preview(snap_target_rect(z, wa));
        else show_snap_preview(wa);
    }
}

Rect Wm::snap_target_rect(int zone, const Rect &wa) const {
    switch (zone) {
        case 1: return Rect(wa.x, wa.y, wa.w / 2, wa.h);
        case 2: return Rect(wa.x + wa.w / 2, wa.y, wa.w / 2, wa.h);
        case 3: return wa;
        case 4: return Rect(wa.x, wa.y, wa.w / 2, wa.h / 2);
        case 5: return Rect(wa.x + wa.w / 2, wa.y, wa.w / 2, wa.h / 2);
        case 6: return Rect(wa.x, wa.y + wa.h / 2, wa.w / 2, wa.h / 2);
        case 7: return Rect(wa.x + wa.w / 2, wa.y + wa.h / 2, wa.w / 2, wa.h / 2);
        default: return wa;
    }
}

void Wm::show_snap_preview(Rect r) {
    XMoveResizeWindow(dpy, snap_preview, r.x + 4, r.y + 4, std::max(1, r.w - 8),
                       std::max(1, r.h - 8));
    XMapRaised(dpy, snap_preview);
    draw_snap_preview();
}

void Wm::hide_snap_preview() { XUnmapWindow(dpy, snap_preview); }

void Wm::draw_snap_preview() {
    // translucent dark glass tile with accent border
    cairo_surface_t *surf = cairo_xlib_surface_create(
        dpy, snap_preview, visual, 
        std::max(1, snap_preview_w()), std::max(1, snap_preview_h()));
    cairo_t *cr = cairo_create(surf);
    cairo_set_source_rgba(cr, 0.42, 0.24, 0.91, 0.28);  // #6c3ce9
    cairo_paint(cr);
    cairo_set_source_rgba(cr, 0.94, 0.31, 0.14, 0.9);  // #f04e23
    cairo_set_line_width(cr, 2);
    cairo_rectangle(cr, 1, 1, snap_preview_w() - 2, snap_preview_h() - 2);
    cairo_stroke(cr);
    cairo_destroy(cr);
    cairo_surface_destroy(surf);
}

int Wm::snap_preview_w() const {
    Window r; int x, y; unsigned int w, h, bw, d;
    if (XGetGeometry(dpy, snap_preview, &r, &x, &y, &w, &h, &bw, &d)) return (int)w;
    return 10;
}
int Wm::snap_preview_h() const {
    Window r; int x, y; unsigned int w, h, bw, d;
    if (XGetGeometry(dpy, snap_preview, &r, &x, &y, &w, &h, &bw, &d)) return (int)h;
    return 10;
}

void Wm::end_interactive() {
    if (interact == Interact::Idle) return;
    XUngrabPointer(dpy, CurrentTime);
    XDefineCursor(dpy, root, cursor_normal);
    interact = Interact::Idle;
    interact_c = nullptr;
    interact_edge = 0;
    hide_snap_preview();
    snap_zone = 0;
}

void Wm::on_button_release(XButtonEvent &e) {
    if (bar && bar->owns(e.window)) {
        bar->handle_button(e);
        return;
    }
    (void)e;
    if (interact == Interact::Move && interact_c) {
        Client *c = interact_c;
        if (snap_zone) {
            Rect wa = workarea_for(c);
            Rect target = snap_target_rect(snap_zone, wa);
            c->restore = c->geom;
            c->maximized = false;
            if (snap_zone == 3) {
                toggle_maximize(c);
            } else {
                Rect inner(target.x + cfg.border_w, target.y + cfg.titlebar_h,
                           target.w - 2 * cfg.border_w,
                           target.h - cfg.titlebar_h - cfg.border_w);
                c->geom = inner;
                client_frame_configure(c);
                if (cfg.animations && c->frame != None)
                    client_anim_start(c, c->frame_rect(), false);
            }
            set_net_wm_state(this, c);
            draw_client(c);
        }
    }
    end_interactive();
}

void Wm::on_motion(XMotionEvent &e) {
    if (menu && menu->active()) {
        menu->handle_motion(e);
        return;
    }
    if (bar && bar->owns(e.window)) {
        bar->handle_motion(e);
        return;
    }
    if (interact == Interact::Move && interact_c) {
        Client *c = interact_c;
        int dx = e.x_root - start_px;
        int dy = e.y_root - start_py;
        Rect g = start_geom;
        g.x = start_geom.x + dx;
        g.y = start_geom.y + dy;
        c->geom.x = g.x;
        c->geom.y = g.y;
        if (c->frame != None) {
            Rect fr = c->frame_rect();
            XMoveWindow(dpy, c->frame, fr.x, fr.y);
        } else {
            XMoveWindow(dpy, c->win, g.x, g.y);
        }
        c->maximized = false;
        update_snap_zone(e.x_root, e.y_root);
        return;
    }
    if (interact == Interact::Resize && interact_c) {
        Client *c = interact_c;
        int dx = e.x_root - start_px;
        int dy = e.y_root - start_py;
        Rect g = start_geom;
        if (interact_edge & 1) { g.x = start_geom.x + dx; g.w = start_geom.w - dx; }
        if (interact_edge & 2) { g.w = start_geom.w + dx; }
        if (interact_edge & 4) { g.y = start_geom.y + dy; g.h = start_geom.h - dy; }
        if (interact_edge & 8) { g.h = start_geom.h + dy; }
        if (g.w < 80) g.w = 80;
        if (g.h < 60) g.h = 60;
        c->geom = g;
        c->maximized = false;
        client_frame_configure(c);
        draw_client(c);
        return;
    }
    // hover effects on titlebars
    Client *fc = find_client_by_frame(e.window);
    if (fc && fc->wants_decor) {
        BtnHit hit = fc->hit_button(e.x, e.y);
        if (hit != fc->hover_btn) {
            fc->hover_btn = hit;
            draw_client(fc);
        }
    }
}

void Wm::on_enter(XCrossingEvent &e) {
    if (e.mode != NotifyNormal && e.mode != NotifyUngrab) return;
    if (e.type == LeaveNotify && bar && bar->owns(e.window)) {
        bar->handle_leave(e);
        return;
    }
    if (cfg.focus_follows_mouse) {
        Client *fc = find_client_by_frame(e.window);
        if (!fc) fc = find_client(e.window);
        if (fc && fc->visible && !fc->iconified && !fc->never_focus) focus(fc, false);
    }
}

// ---- X event handlers ---------------------------------------------------------

void Wm::on_map_request(XMapRequestEvent &e) {
    Client *c = find_client(e.window);
    if (c) {
        if (c->iconified || !c->visible) uniconify(c);
        else {
            XMapWindow(dpy, c->win);
            if (c->frame != None) XMapWindow(dpy, c->frame);
            c->visible = true;
        }
        return;
    }
    manage(e.window);
}

void Wm::on_configure_request(XConfigureRequestEvent &e) {
    Client *c = find_client(e.window);
    if (!c) {
        // honor directly for unmanaged windows
        XWindowChanges wc;
        wc.x = e.x;
        wc.y = e.y;
        wc.width = e.width;
        wc.height = e.height;
        wc.border_width = e.border_width;
        wc.sibling = e.above;
        wc.stack_mode = e.detail;
        XConfigureWindow(dpy, e.window, e.value_mask, &wc);
        return;
    }
    if (e.value_mask & CWX) c->geom.x = e.x;
    if (e.value_mask & CWY) c->geom.y = e.y;
    if (e.value_mask & CWWidth) c->geom.w = std::max(80, e.width);
    if (e.value_mask & CWHeight) c->geom.h = std::max(60, e.height);
    if (e.value_mask & CWStackMode) {
        if (e.detail == Above) bring_to_front(c);
        else if (e.detail == Below) send_to_back(c);
    }
    client_frame_configure(c);
    draw_client(c);
}

void Wm::on_destroy(XDestroyWindowEvent &e) {
    Client *c = find_client(e.window);
    if (c) unmanage(c, true);  // client window is gone; frame only
}

void Wm::on_unmap(XUnmapEvent &e) {
    // We receive UnmapNotify either via SubstructureNotify on root (app
    // unmapped its top-level) or via StructureNotify on our own frames
    // (then e.window is a frame id, and find_client simply misses).
    if (e.window == root) return;
    Client *c = find_client(e.window);
    if (!c) return;
    if (c->iconic_by_wm) {  // we initiated this unmap (iconify)
        c->iconic_by_wm = false;
        return;
    }
    if (!c->visible) return;  // already hidden (desktop switch, show-desktop)
    // app withdrew/hid its window: treat as iconify (stays in taskbar)
    if (c->frame != None) XUnmapWindow(dpy, c->frame);
    c->visible = false;
    c->iconified = true;
    client_set_wm_state(c, IconicState);
    update_focus_after(c);
    sync_net_client_list();
}

void Wm::on_reparent(XReparentEvent &e) {
    Client *c = find_client(e.window);
    if (!c) return;
    if (e.parent != c->frame && e.parent != root) {
        // the client reparented itself away (shutdown or embedding): let go
        unmanage(c);
    } else if (e.parent == root && c->frame != None) {
        // got the client back at root: unmanage cleanly (it will re-map)
        unmanage(c);
    }
}

void Wm::on_property(XPropertyEvent &e) {
    if (e.window == root) {
        if (e.atom == atoms.net_number_of_desktops && e.state == PropertyNewValue) {
            long v = 0;
            if (get_cardinal(dpy, root, atoms.net_number_of_desktops, &v) &&
                v != num_desktops && v >= 1 && v <= 9)
                set_desktop_count((int)v);
            return;
        }
        if (e.atom == atoms.net_current_desktop && e.state == PropertyNewValue) {
            long v = 0;
            if (get_cardinal(dpy, root, atoms.net_current_desktop, &v) &&
                v != current_desktop)
                switch_desktop((int)v);
            return;
        }
        if (e.atom == atoms.net_showing_desktop && e.state == PropertyNewValue) {
            long v = 0;
            if (get_cardinal(dpy, root, atoms.net_showing_desktop, &v))
                set_show_desktop(v != 0);
            return;
        }
        if (e.atom == atoms.net_active_window && e.state == PropertyNewValue) {
            // external activation request (panel/dock click)
            Window w = None;
            Atom type;
            int fmt;
            unsigned long n, after;
            unsigned char *data = nullptr;
            if (XGetWindowProperty(dpy, root, atoms.net_active_window, 0, 1, False,
                                   XA_WINDOW, &type, &fmt, &n, &after,
                                   &data) == Success &&
                data && n == 1) {
                w = *(Window *)data;
                XFree(data);
            }
            Client *c = find_client(w);
            if (c && c != focused_client) {
                if (c->desktop != (int)0xFFFFFFFF && c->desktop != current_desktop)
                    switch_desktop(c->desktop);
                uniconify(c);
                focus(c);
            }
            return;
        }
        return;
    }
    Client *c = find_client(e.window);
    if (!c) return;
    if (e.atom == atoms.net_wm_name || e.atom == atoms.wm_name) {
        std::string t;
        if (!get_text(dpy, c->win, atoms.net_wm_name, &t))
            if (!get_text(dpy, c->win, atoms.wm_name, &t)) t = "";
        if (t != c->title) {
            c->title = t;
            draw_client(c);
            if (switcher && switcher->active()) switcher->draw();
        }
        return;
    }
    if (e.atom == atoms.net_wm_icon) {
        client_read_icon(c);
        draw_client(c);
        return;
    }
    if (e.atom == atoms.net_wm_strut_partial || e.atom == atoms.net_wm_strut) {
        read_struts(c);
        update_workarea();
        return;
    }
    if (e.atom == atoms.wm_normal_hints) {
        return;  // sizes re-clamped on next resize
    }
    if (e.atom == XA_WM_HINTS) {
        XWMHints *h = XGetWMHints(dpy, c->win);
        if (h) {
            if (h->flags & XUrgencyHint) {
                c->urgent = true;
                draw_client(c);
            }
            XFree(h);
        }
        return;
    }
    if (e.atom == atoms.net_wm_state) {
        std::vector<long> v = get_cardinals(dpy, c->win, atoms.net_wm_state, 16);
        bool fs = false, maxv = false, maxh = false, ab = false;
        for (long s : v) {
            if (s == (long)atoms.net_wm_state_fullscreen) fs = true;
            if (s == (long)atoms.net_wm_state_maximized_vert) maxv = true;
            if (s == (long)atoms.net_wm_state_maximized_horz) maxh = true;
            if (s == (long)atoms.net_wm_state_above) ab = true;
        }
        if (fs != c->fullscreen) set_fullscreen(c, fs);
        c->maximized = maxv && maxh;
        if (ab != c->above) {
            c->above = ab;
            restack_all();
        }
        return;
    }
}

void Wm::on_client_message(XClientMessageEvent &e) {
    Client *c = find_client(e.window);
    if (e.message_type == atoms.net_wm_state && c) {
        long action = e.data.l[0];
        for (int i = 1; i <= 2; i++) {
            long a = e.data.l[i];
            if (!a) continue;
            if (a == (long)atoms.net_wm_state_fullscreen)
                set_fullscreen(c, action == 0 ? !c->fullscreen : action == 1);
            else if (a == (long)atoms.net_wm_state_maximized_vert ||
                     a == (long)atoms.net_wm_state_maximized_horz) {
                if (action == 0) { if (c->maximized) toggle_maximize(c); }
                else if (action == 1) { if (!c->maximized) toggle_maximize(c); }
                else toggle_maximize(c);
            } else if (a == (long)atoms.net_wm_state_above)
                c->above = action == 2 ? !c->above : action == 1;
            else if (a == (long)atoms.net_wm_state_below)
                c->above = false;
            else if (a == (long)atoms.net_wm_state_demands_attention) {
                c->urgent = true;
                draw_client(c);
            }
        }
        set_net_wm_state(this, c);
        restack_all();
        return;
    }
    if (e.message_type == atoms.net_active_window) {
        Client *a = c;
        if (!a && e.data.l[0] != 0) {
            // activated by window id in data
        }
        if (a) {
            if (a->desktop != (int)0xFFFFFFFF && a->desktop != current_desktop)
                switch_desktop(a->desktop);
            uniconify(a);
            focus(a);
        }
        return;
    }
    if (e.message_type == atoms.net_close_window && c) { close(c); return; }
    if (e.message_type == atoms.net_wm_desktop && c) {
        long d = e.data.l[0];
        if (d == 0xFFFFFFFF) {
            c->desktop = 0xFFFFFFFF;
            unsigned long all = 0xFFFFFFFF;
            XChangeProperty(dpy, c->win, atoms.net_wm_desktop, XA_CARDINAL, 32,
                            PropModeReplace, (unsigned char *)&all, 1);
        } else {
            move_to_desktop(c, (int)d);
        }
        return;
    }
    if (e.message_type == atoms.net_current_desktop) {
        switch_desktop((int)e.data.l[0]);
        return;
    }
    if (e.message_type == atoms.wm_change_state && c) {
        if (e.data.l[0] == IconicState) iconify(c);
        else if (e.data.l[0] == NormalState) uniconify(c);
        return;
    }
    if (e.message_type == atoms.net_wm_moveresize && c) {
        long dir = e.data.l[4];
        int px = (int)e.data.l[2], py = (int)e.data.l[3];
        if (dir == 8) start_interactive_move(c, px, py);
        else if (dir >= 0 && dir <= 7) {
            int edge = 0;
            if (dir == 0 || dir == 3 || dir == 6) edge |= 1;
            if (dir == 2 || dir == 5 || dir == 8) edge |= 2;
            if (dir == 0 || dir == 1 || dir == 2) edge |= 4;
            if (dir == 6 || dir == 7) edge |= 8;
            start_interactive_resize(c, px, py, edge);
        }
        return;
    }
    if (e.message_type == atoms.net_restack_window && c) {
        bring_to_front(c);
        return;
    }
}

void Wm::on_expose(XExposeEvent &e) {
    if (e.count > 0) return;
    if (menu && menu->active() && e.window == menu->win) { menu->handle_expose(e); return; }
    if (switcher && switcher->active() && e.window == switcher->win) { switcher->draw(); return; }
    if (bar && bar->owns(e.window)) { bar->handle_expose(e); return; }
    Client *c = find_client_by_frame(e.window);
    if (c) draw_client(c);
}

// ---- restacking ------------------------------------------------------------------

void Wm::restack_all() {
    // stacking passes, lowest to highest: desktop, normal, dock + above-flagged
    for (Client *c : stacking) {
        if (c->type != WinType::Desktop) continue;
        if (c->visible) XRaiseWindow(dpy, c->frame != None ? c->frame : c->win);
    }
    for (Client *c : stacking) {
        if (c->type == WinType::Desktop || c->type == WinType::Dock ||
            c->above)
            continue;
        if (c->visible) XRaiseWindow(dpy, c->frame != None ? c->frame : c->win);
    }
    for (Client *c : stacking) {
        if (c->type != WinType::Dock && !c->above) continue;
        if (c->visible) XRaiseWindow(dpy, c->frame != None ? c->frame : c->win);
    }
}

void Wm::snap_to_zone(Client *c) {
    if (!c || !snap_zone) return;
    Rect wa = workarea_for(c);
    Rect target = snap_target_rect(snap_zone, wa);
    if (snap_zone == 3) { toggle_maximize(c); return; }
    c->restore = c->geom;
    c->maximized = false;
    c->geom = Rect(target.x + cfg.border_w, target.y + cfg.titlebar_h,
                   target.w - 2 * cfg.border_w,
                   target.h - cfg.titlebar_h - cfg.border_w);
    client_frame_configure(c);
    set_net_wm_state(this, c);
    draw_client(c);
}

}  // namespace bwm
