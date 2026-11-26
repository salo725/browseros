// BraveWM - bar.cpp: the BraveOS top status bar implementation.
// Design (BraveOS spec): ~32px translucent dark glass (rgba(15,12,30,0.72))
// with a subtle bottom border, Brave lion + "BraveOS" wordmark, pill-shaped
// workspace switcher (active = #6c3ce9 pill), and right-side system status:
// Wi-Fi, Bluetooth, battery percentage, clock, power button.
#include "bar.hpp"
#include "menu.hpp"
#include "i18n.hpp"

#include <dirent.h>
#include <unistd.h>
#include <fstream>
#include <sstream>

namespace bwm {

// ---- small file helpers ------------------------------------------------------

static bool file_read_trim(const std::string &path, std::string *out) {
    std::ifstream f(path);
    if (!f) return false;
    std::getline(f, *out);
    // trim
    size_t a = out->find_first_not_of(" \t\r\n");
    if (a == std::string::npos) {
        *out = "";
        return true;
    }
    size_t b = out->find_last_not_of(" \t\r\n");
    *out = out->substr(a, b - a + 1);
    return true;
}

static bool dir_exists(const std::string &path) {
    DIR *d = opendir(path.c_str());
    if (!d) return false;
    closedir(d);
    return true;
}

// ---- lifecycle ------------------------------------------------------------------

Bar::Bar(Wm *w) : wm(w) {}

Bar::~Bar() { destroy_win(); }

bool Bar::create() {
    have_argb = argb_visual_available();
    // locale clock format (12h AM/PM default; time_24h=1 in i18n confs)
    time_24h = (g_i18n.tr("time_24h") == "1");
    relayout();
    return win != None;
}

bool Bar::argb_visual_available() {
    XVisualInfo tpl, *vi;
    int n = 0;
    tpl.screen = wm->screen;
    tpl.depth = 32;
    tpl.c_class = TrueColor;
    vi = XGetVisualInfo(wm->dpy, VisualScreenMask | VisualDepthMask |
                                     VisualClassMask, &tpl, &n);
    if (!vi || n == 0) {
        if (vi) XFree(vi);
        return false;
    }
    Visual *v = vi[0].visual;
    unsigned int d = vi[0].depth;
    XFree(vi);
    if (argb_colormap != None) XFreeColormap(wm->dpy, argb_colormap);
    argb_visual = v;
    argb_depth = d;
    argb_colormap = XCreateColormap(wm->dpy, wm->root, v, AllocNone);
    return true;
}

bool Bar::compositor_running() const {
    Atom cm = XInternAtom(wm->dpy, "_NET_WM_CM_S0", False);
    Window owner = XGetSelectionOwner(wm->dpy, cm);
    return owner != None;
}

void Bar::ensure_window_mode() {
    bool compositor = compositor_running();
    want_argb = compositor && have_argb;
    if (win != None && want_argb != argb_mode) {
        // compositor came or went: recreate the window for/without alpha
        int save_hover = hover_id;
        recreate_window(want_argb);
        hover_id = save_hover;
    }
}

void Bar::recreate_window(bool argb) {
    if (win != None) {
        wm->struts.erase(win);
        XDestroyWindow(wm->dpy, win);
        win = None;
    }
    argb_mode = argb;

    Visual *vis = wm->visual;
    Colormap cmap = wm->colormap;
    unsigned int depth = wm->depth;
    if (argb && argb_visual) {
        vis = argb_visual;
        cmap = argb_colormap;
        depth = argb_depth;
    }

    XSetWindowAttributes wa = {};
    wa.override_redirect = True;
    wa.event_mask = ExposureMask | ButtonPressMask | ButtonReleaseMask |
                    PointerMotionMask | LeaveWindowMask | StructureNotifyMask;
    wa.colormap = cmap;
    wa.background_pixel = BlackPixel(wm->dpy, wm->screen);
    wa.border_pixel = 0;
    wa.backing_store = WhenMapped;
    unsigned long mask = CWOverrideRedirect | CWEventMask | CWBackPixel |
                         CWBorderPixel | CWBackingStore;
    if (argb) mask |= CWColormap;
    win = XCreateWindow(wm->dpy, wm->root, x, y, w, h, 0, depth, InputOutput,
                        vis, mask, &wa);
    if (win == None) return;

    // advertise as a dock + panel-ish properties
    Atom dock = wm->atoms.net_wm_window_type_dock;
    XChangeProperty(wm->dpy, win, wm->atoms.net_wm_window_type, XA_ATOM, 32,
                    PropModeReplace, (unsigned char *)&dock, 1);
    long pid = (long)getpid();
    XChangeProperty(wm->dpy, win, wm->atoms.net_wm_pid, XA_CARDINAL, 32,
                    PropModeReplace, (unsigned char *)&pid, 1);

    // strut: reserve the bar band on the primary monitor
    Strut s;
    s.top = h;
    s.top_start = x;
    s.top_end = x + w;
    wm->struts[win] = s;
    wm->update_workarea();

    XMapRaised(wm->dpy, win);
    draw();
}

// ---- geometry / zones ------------------------------------------------------------

int Bar::workspace_windows(int d) const {
    int n = 0;
    for (Client *c : wm->clients) {
        if (c->type == WinType::Desktop || c->type == WinType::Dock) continue;
        if (c->iconified) continue;
        int cd = c->desktop == (int)0xFFFFFFFF ? wm->current_desktop : c->desktop;
        if ((unsigned)cd == (unsigned)d) n++;
    }
    return n;
}

void Bar::relayout() {
    Rect mon = wm->monitors.empty() ? Rect(0, 0, DisplayWidth(wm->dpy, wm->screen),
                                           DisplayHeight(wm->dpy, wm->screen))
                                    : wm->monitors[0].rect;
    // responsive scale: bar height tracks screen height (32px @ 1080p), and
    // never dwarfs tiny screens
    h = (int)lround(mon.h / 33.75);
    if (h < 26) h = 26;
    if (h > 48) h = 48;
    u = h / 32.0;
    x = mon.x;
    y = mon.y;
    w = mon.w;

    if (win == None) {
        want_argb = compositor_running() && have_argb;
        recreate_window(want_argb);
    } else {
        // resize in place
        XMoveResizeWindow(wm->dpy, win, x, y, w, h);
        Strut s;
        s.top = h;
        s.top_start = x;
        s.top_end = x + w;
        wm->struts[win] = s;
        wm->update_workarea();
    }
    layout_zones();
    draw();
}

void Bar::layout_zones() {
    zones.clear();
    cairo_surface_t *tmp = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, 8, 8);
    cairo_t *cr = cairo_create(tmp);

    // ---- left: lion + wordmark + divider + workspace pills ----
    double cx = x + 12 * u;
    double lion_s = h - 16 * u;
    brand_x = cx;  // lion icon position
    zones.push_back({Rect((int)cx, (int)(y + (h - lion_s) / 2), (int)lion_s,
                          (int)lion_s), Z_NONE});  // brand (no action)
    cx += lion_s + 8 * u;
    TextMetrics tm = text_measure(cr, "BraveOS", wm->cfg.font, 13.5 * u, true);
    cx += tm.width + 10 * u;
    divider_x = cx;
    cx += 10 * u;

    // pills: 1..N, then "+"
    double pill_h = 20 * u;
    double pill_y = y + (h - pill_h) / 2;
    for (int i = 0; i < wm->num_desktops; i++) {
        std::string label = std::to_string(i + 1);
        TextMetrics pt = text_measure(cr, label, wm->cfg.font, 12 * u,
                                      i == wm->current_desktop);
        double pill_w = std::max(30 * u, pt.width + 18 * u);
        zones.push_back({Rect((int)cx, (int)pill_y, (int)pill_w, (int)pill_h),
                         Z_PILL0 + i});
        cx += pill_w + 6 * u;
    }
    zones.push_back({Rect((int)cx, (int)pill_y, (int)pill_h, (int)pill_h),
                     Z_ADD});
    cx += pill_h + 6 * u;

    // ---- right side: power, clock, battery, wifi, bluetooth ----
    double rx = x + w - 10 * u;  // right edge cursor (exclusive)

    // power button: 26u box
    double pw = 26 * u;
    rx -= pw;
    zones.push_back({Rect((int)rx, (int)(y + (h - pw) / 2), (int)pw, (int)pw),
                     Z_POWER});
    rx -= 8 * u;

    // clock (layout: "10:24 PM" + optional layout code)
    std::string kbl = kb_layout.empty() ? "" : (" " + kb_layout);
    std::string ct = clock_text();
    tm = text_measure(cr, ct + kbl + "  ", wm->cfg.font, 12.5 * u, false);
    double cw = std::max(58 * u, tm.width + 12 * u);
    rx -= cw;
    zones.push_back({Rect((int)rx, (int)y, (int)cw, h), Z_CLOCK});
    rx -= 4 * u;

    // battery: icon + "NN%"
    if (status.battery_present) {
        tm = text_measure(cr, fmtf("%d%%", status.battery_pct), wm->cfg.font,
                          11.5 * u, false);
        double bw = 20 * u + tm.width + 14 * u;
        rx -= bw;
        zones.push_back({Rect((int)rx, (int)y, (int)bw, h), Z_BATTERY});
        rx -= 8 * u;
    }

    // wifi
    if (status.wifi_present) {
        double ww = 22 * u;
        rx -= ww;
        zones.push_back({Rect((int)rx, (int)y, (int)ww, h), Z_WIFI});
        rx -= 8 * u;
    }

    // bluetooth
    if (status.bt_present) {
        double bw = 20 * u;
        rx -= bw;
        zones.push_back({Rect((int)rx, (int)y, (int)bw, h), Z_BT});
        rx -= 8 * u;
    }

    cairo_destroy(cr);
    cairo_surface_destroy(tmp);

    // export zone geometry for automated UI tests (harmless debug aid)
    std::string zoneprop;
    for (auto &z : zones) {
        zoneprop += fmtf("%d:%d,%d,%d,%d;", z.id, z.r.x, z.r.y, z.r.w, z.r.h);
    }
    Atom zatom = XInternAtom(wm->dpy, "_BRAVEOS_BAR_ZONES", False);
    XChangeProperty(wm->dpy, wm->root, zatom, XA_STRING, 8, PropModeReplace,
                    (const unsigned char *)zoneprop.c_str(),
                    (int)zoneprop.size());
}

// ---- status polling ---------------------------------------------------------------

void Bar::poll_status() {
    BarStatus s;

    // battery
    DIR *d = opendir("/sys/class/power_supply");
    if (d) {
        struct dirent *e;
        while ((e = readdir(d))) {
            if (e->d_name[0] == '.') continue;
            std::string base = std::string("/sys/class/power_supply/") + e->d_name;
            std::string type;
            if (!file_read_trim(base + "/type", &type) || type != "Battery")
                continue;
            s.battery_present = true;
            std::string cap, st;
            if (file_read_trim(base + "/capacity", &cap))
                s.battery_pct = atoi(cap.c_str());
            if (file_read_trim(base + "/status", &st) && st == "Charging")
                s.charging = true;
            break;
        }
        closedir(d);
    }

    // wifi: adapter presence, rfkill state, link
    DIR *nd = opendir("/sys/class/net");
    if (nd) {
        struct dirent *e;
        while ((e = readdir(nd))) {
            std::string n = e->d_name;
            if (n == "." || n == "..") continue;
            std::string base = "/sys/class/net/" + n;
            std::string type;
            file_read_trim(base + "/type", &type);
            if (type == "1" && (n.rfind("wlan", 0) == 0 || n.rfind("wlp", 0) == 0 ||
                                 n.rfind("wls", 0) == 0))
                s.wifi_present = true;
        }
        closedir(nd);
    }
    if (s.wifi_present) {
        // operstate of any wireless iface
        DIR *nd2 = opendir("/sys/class/net");
        if (nd2) {
            struct dirent *e;
            while ((e = readdir(nd2))) {
                std::string n = e->d_name;
                if (n.rfind("wl", 0) != 0) continue;
                std::string st;
                if (file_read_trim("/sys/class/net/" + n + "/operstate", &st) &&
                    (st == "up" || st == "dormant")) {
                    s.wifi_on = true;
                    s.wifi_up = true;
                }
            }
            closedir(nd2);
        }
        // quality from /proc/net/wireless
        std::ifstream wf("/proc/net/wireless");
        std::string line;
        std::getline(wf, line);
        std::getline(wf, line);
        while (std::getline(wf, line)) {
            std::istringstream ss(line);
            std::string iface;
            int face = 0, quality = 0, level = 0, noise = 0;
            ss >> iface >> face >> quality >> level >> noise;
            int q = quality * 3 / 70;
            if (q < 1) q = 1;
            if (q > 3) q = 3;
            s.wifi_quality = q;
        }
        // rfkill soft-blocked?
        if (dir_exists("/sys/class/rfkill")) {
            DIR *rd = opendir("/sys/class/rfkill");
            if (rd) {
                struct dirent *e2;
                while ((e2 = readdir(rd))) {
                    std::string b = std::string("/sys/class/rfkill/") + e2->d_name;
                    std::string t, soft;
                    if (file_read_trim(b + "/type", &t) && t == "wlan" &&
                        file_read_trim(b + "/soft", &soft) && soft == "1") {
                        s.wifi_on = false;
                        s.wifi_up = false;
                        s.wifi_quality = 0;
                    }
                }
                closedir(rd);
            }
        }
    }

    // bluetooth
    if (dir_exists("/sys/class/bluetooth")) {
        s.bt_present = true;
        s.bt_on = true;
        DIR *rd = opendir("/sys/class/rfkill");
        if (rd) {
            struct dirent *e;
            while ((e = readdir(rd))) {
                std::string b = std::string("/sys/class/rfkill/") + e->d_name;
                std::string t, soft;
                if (file_read_trim(b + "/type", &t) && t == "bluetooth" &&
                    file_read_trim(b + "/soft", &soft) && soft == "1")
                    s.bt_on = false;
            }
            closedir(rd);
        }
    }

    // relayout if the visible element set changed
    std::string sig = fmtf("%d %d %d %d", (int)s.battery_present,
                           (int)s.wifi_present, (int)s.bt_present,
                           (int)(s.battery_pct / 10));
    if (sig != last_status_sig) {
        status = s;
        last_status_sig = sig;
        layout_zones();
        draw();
    } else {
        status = s;
    }
}

void Bar::poll_keyboard_layout() {
    std::string v;
    if (file_read_trim("/tmp/bravos-kb-layout", &v) && !v.empty()) {
        if (v != kb_layout) {
            kb_layout = v;
            layout_zones();
            draw();
        }
    } else if (!kb_layout.empty()) {
        kb_layout = "";
        layout_zones();
        draw();
    }
}

std::string Bar::clock_text() {
    time_t t = time(nullptr);
    struct tm tmv;
    localtime_r(&t, &tmv);
    char buf[64];
    if (time_24h) {
        strftime(buf, sizeof(buf), "%H:%M", &tmv);
    } else {
        strftime(buf, sizeof(buf), "%I:%M", &tmv);
        // strip leading zero
        if (buf[0] == '0') memmove(buf, buf + 1, strlen(buf));
        std::string am = tmv.tm_hour < 12 ? g_i18n.tr("am") : g_i18n.tr("pm");
        return std::string(buf) + " " + am;
    }
    return buf;
}

void Bar::tick() {
    time_t now = time(nullptr);
    if (now != clock_sec) {
        clock_sec = now;
        struct tm tmv;
        localtime_r(&now, &tmv);
        if (tmv.tm_min != clock_shown_min) {
            clock_shown_min = tmv.tm_min;
            draw();
        }
        poll_keyboard_layout();
        if (now % 2 == 0) poll_status();
        ensure_window_mode();
    }
}

// ---- events -------------------------------------------------------------------------

int Bar::zone_at(int px, int py) const {
    for (auto &z : zones)
        if (z.r.contains(px, py)) return z.id;
    return Z_NONE;
}

bool Bar::handle_expose(XExposeEvent &) {
    if (win == None) return false;
    draw();
    return true;
}

bool Bar::handle_motion(XMotionEvent &e) {
    if (win == None || e.window != win) return false;
    int id = zone_at(e.x, e.y);
    if (id != hover_id) {
        hover_id = id;
        draw();
    }
    return true;
}

bool Bar::handle_leave(XCrossingEvent &e) {
    if (win == None || e.window != win) return false;
    if (hover_id != Z_NONE) {
        hover_id = Z_NONE;
        draw();
    }
    return true;
}

bool Bar::handle_button(XButtonEvent &e) {
    if (win == None || e.window != win) return false;
    if (e.type == ButtonPress) {
        int id = zone_at(e.x, e.y);
        press_id = id;
        if (e.button == Button1) {
            if (id >= Z_PILL0 && id < Z_PILL0 + 100) {
                int d = id - Z_PILL0;
                if (d != wm->current_desktop) {
                    wm->switch_desktop(d);
                    press_id = Z_NONE;
                    return true;
                }
            } else if (id == Z_ADD) {
                if (wm->num_desktops < 9) wm->set_desktop_count(wm->num_desktops + 1);
                press_id = Z_NONE;
                return true;
            } else if (id == Z_WIFI || id == Z_BT) {
                wm->spawn(wm->cfg.settings_cmd + " --page network");
                press_id = Z_NONE;
                return true;
            } else if (id == Z_BATTERY) {
                wm->spawn(wm->cfg.settings_cmd + " --page power");
                press_id = Z_NONE;
                return true;
            } else if (id == Z_CLOCK) {
                wm->spawn(wm->cfg.settings_cmd + " --page datetime");
                press_id = Z_NONE;
                return true;
            } else if (id == Z_POWER) {
                open_power_menu(e.x_root, e.y_root);
                press_id = Z_NONE;
                return true;
            }
        } else if (e.button == Button4) {
            wm->switch_desktop((wm->current_desktop + wm->num_desktops - 1) %
                               wm->num_desktops);
        } else if (e.button == Button5) {
            wm->switch_desktop((wm->current_desktop + 1) % wm->num_desktops);
        }
        press_id = Z_NONE;
        return true;
    }
    // ButtonRelease: press feedback ends
    if (press_id != Z_NONE) {
        press_id = Z_NONE;
        draw();
    }
    return true;
}

// ---- power menu -------------------------------------------------------------------

void Bar::open_power_menu(int px, int py) {
    std::vector<MenuEntry> items;
    items.push_back({g_i18n.tr("power_lock"), "",
                     [this]() { wm->spawn(wm->cfg.lock_cmd); }, true});
    items.push_back({g_i18n.tr("power_logout"), "", [this]() { logout(); }, true});
    items.push_back({"", "", nullptr, false});
    items.push_back({g_i18n.tr("power_suspend"), "",
                     [this]() { wm->spawn("bravos-power suspend"); }, true});
    items.push_back({g_i18n.tr("power_restart"), "",
                     [this]() { wm->spawn("bravos-power reboot"); }, true});
    items.push_back({g_i18n.tr("power_shutdown"), "",
                     [this]() { wm->spawn("bravos-power poweroff"); }, true});
    wm->menu->show(items, px, py, false);
}

void Bar::logout() {
    // clean end of the WM -> the session supervisor exits and tears down X
    wm->running = false;
}

void Bar::destroy_win() {
    if (win == None) return;
    wm->struts.erase(win);
    XDestroyWindow(wm->dpy, win);
    win = None;
}

// ---- drawing -----------------------------------------------------------------------

// Stylized Brave lion mark: orange shield + white lion features.
static void draw_lion(cairo_t *cr, double cx, double cy, double s) {
    double r = s / 2.0;
    // shield: rounded diamond, orange gradient
    cairo_pattern_t *pat = cairo_pattern_create_linear(cx - r, cy - r, cx + r, cy + r);
    cairo_pattern_add_color_stop_rgb(pat, 0.0, 0.984, 0.329, 0.169);   // #fb542b
    cairo_pattern_add_color_stop_rgb(pat, 0.5, 0.941, 0.306, 0.137);   // #f04e23
    cairo_pattern_add_color_stop_rgb(pat, 1.0, 0.812, 0.208, 0.086);   // #cf3516
    cairo_set_source(cr, pat);

    double w = s * 0.86, hh = s * 0.94;
    double x0 = cx - w / 2, y0 = cy - hh / 2;
    // shield path: top arc, sides tapering to a bottom point
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx, y0 + hh * 0.06);
    cairo_curve_to(cr, x0 + w * 0.28, y0 - hh * 0.05, x0 + w * 0.72, y0 - hh * 0.05,
                   x0 + w, y0 + hh * 0.14);
    cairo_curve_to(cr, x0 + w, y0 + hh * 0.62, cx + w * 0.18, y0 + hh * 0.9, cx,
                   y0 + hh);
    cairo_curve_to(cr, cx - w * 0.18, y0 + hh * 0.9, x0, y0 + hh * 0.62, x0,
                   y0 + hh * 0.14);
    cairo_curve_to(cr, x0, y0 + hh * 0.14, x0 + w * 0.06, y0 + hh * 0.02, cx,
                   y0 + hh * 0.06);
    cairo_close_path(cr);
    cairo_fill(cr);
    cairo_pattern_destroy(pat);

    // lion face: eyes + muzzle in white
    cairo_set_source_rgb(cr, 1, 1, 1);
    // eyes (two angled slits)
    cairo_set_line_width(cr, s * 0.085);
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    double ey = cy - hh * 0.13;
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx - w * 0.30, ey - s * 0.02);
    cairo_line_to(cr, cx - w * 0.11, ey + s * 0.045);
    cairo_stroke(cr);
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx + w * 0.30, ey - s * 0.02);
    cairo_line_to(cr, cx + w * 0.11, ey + s * 0.045);
    cairo_stroke(cr);
    // muzzle: rounded W shape
    cairo_set_line_width(cr, s * 0.07);
    double my = cy + hh * 0.16;
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx - w * 0.20, my);
    cairo_curve_to(cr, cx - w * 0.10, my + s * 0.08, cx - w * 0.03, my + s * 0.01,
                   cx, my + s * 0.05);
    cairo_curve_to(cr, cx + w * 0.03, my + s * 0.01, cx + w * 0.10, my + s * 0.08,
                   cx + w * 0.20, my);
    cairo_stroke(cr);
    // mane notches (side cuts)
    cairo_set_line_width(cr, s * 0.05);
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx - w * 0.42, cy + hh * 0.02);
    cairo_line_to(cr, cx - w * 0.22, cy + hh * 0.02);
    cairo_stroke(cr);
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx + w * 0.42, cy + hh * 0.02);
    cairo_line_to(cr, cx + w * 0.22, cy + hh * 0.02);
    cairo_stroke(cr);
}

static void draw_wifi_icon(cairo_t *cr, double cx, double cy, double s, int q,
                           bool enabled) {
    // q: 0 off, 1..3 strength
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_width(cr, s * 0.11);
    if (q <= 0) {
        // crossed-out dot
        cairo_set_line_width(cr, s * 0.10);
        cairo_new_sub_path(cr);
        cairo_arc(cr, cx, cy + s * 0.12, s * 0.10, 0, 2 * M_PI);
        cairo_fill(cr);
        cairo_new_sub_path(cr);
        cairo_move_to(cr, cx - s * 0.32, cy + s * 0.30);
        cairo_line_to(cr, cx + s * 0.32, cy - s * 0.34);
        cairo_stroke(cr);
        return;
    }
    cairo_new_sub_path(cr);
    cairo_arc(cr, cx, cy + s * 0.34, s * 0.09, 0, 2 * M_PI);
    cairo_fill(cr);
    double radii[3] = {s * 0.22, s * 0.40, s * 0.58};
    for (int i = 0; i < 3; i++) {
        double rr = radii[i] * 0.9;
        if (i >= q) {
            cairo_set_source_rgba(cr, 1, 1, 1, 0.22);
        } else if (!enabled) {
            cairo_set_source_rgba(cr, 1, 1, 1, 0.35);
        } else {
            cairo_set_source_rgb(cr, 1, 1, 1);
        }
        cairo_new_sub_path(cr);
        cairo_arc(cr, cx, cy + s * 0.34, rr, -M_PI * 0.78, -M_PI * 0.22);
        cairo_stroke(cr);
    }
}

static void draw_bt_icon(cairo_t *cr, double cx, double cy, double s, bool on) {
    // Bluetooth rune: two stacked triangles + vertical stem
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);
    cairo_set_line_width(cr, s * 0.11);
    if (!on) cairo_set_source_rgba(cr, 1, 1, 1, 0.38);
    double hh = s * 0.42;
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx, cy - hh);
    cairo_line_to(cr, cx, cy + hh);
    cairo_stroke(cr);
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx - hh * 0.62, cy - hh * 0.28);
    cairo_line_to(cr, cx, cy + hh * 0.34);
    cairo_line_to(cr, cx + hh * 0.62, cy - hh * 0.28);
    cairo_stroke(cr);
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx - hh * 0.62, cy + hh * 0.28);
    cairo_line_to(cr, cx, cy - hh * 0.34);
    cairo_line_to(cr, cx + hh * 0.62, cy + hh * 0.28);
    cairo_stroke(cr);
}

static void draw_battery_icon(cairo_t *cr, double x, double cy, double s,
                              int pct, bool charging) {
    // horizontal battery, height s*0.5, width s
    double bw = s, bh = s * 0.52;
    double bx = x, by = cy - bh / 2;
    cairo_set_line_width(cr, s * 0.075);
    // body outline
    cairo_new_sub_path(cr);
    round_rect_path(cr, bx + s * 0.03, by, bw - s * 0.10, bh, s * 0.10);
    cairo_set_source_rgb(cr, 1, 1, 1);
    cairo_stroke(cr);
    // tip
    cairo_new_sub_path(cr);
    cairo_move_to(cr, bx + bw - s * 0.05, cy - bh * 0.18);
    cairo_line_to(cr, bx + bw - s * 0.05, cy + bh * 0.18);
    cairo_set_line_width(cr, s * 0.07);
    cairo_stroke(cr);
    // fill level
    double fw = (bw - s * 0.26) * std::max(2, std::min(100, pct)) / 100.0;
    double fh = bh - s * 0.18;
    cairo_new_sub_path(cr);
    round_rect_path(cr, bx + s * 0.11, cy - fh / 2, fw, fh, s * 0.055);
    if (pct <= 15 && !charging) {
        cairo_set_source_rgb(cr, 0.96, 0.31, 0.25);
    } else {
        cairo_set_source_rgb(cr, 1, 1, 1);
    }
    cairo_fill(cr);
    // charging bolt
    if (charging) {
        cairo_set_source_rgb(cr, 0.25, 0.92, 0.55);
        double cxx = bx + bw / 2 - s * 0.015, cyy = cy;
        cairo_new_sub_path(cr);
        cairo_move_to(cr, cxx + s * 0.06, cyy - s * 0.16);
        cairo_line_to(cr, cxx - s * 0.06, cyy + s * 0.02);
        cairo_line_to(cr, cxx + s * 0.03, cyy + s * 0.02);
        cairo_line_to(cr, cxx - s * 0.05, cyy + s * 0.18);
        cairo_line_to(cr, cxx + s * 0.09, cyy - s * 0.02);
        cairo_line_to(cr, cxx - s * 0.01, cyy - s * 0.02);
        cairo_close_path(cr);
        cairo_fill(cr);
    }
}

static void draw_power_icon(cairo_t *cr, double cx, double cy, double s) {
    // power symbol: broken circle + stem
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_width(cr, s * 0.11);
    double r = s * 0.34;
    cairo_new_sub_path(cr);
    cairo_arc(cr, cx, cy, r, -M_PI * 0.32, M_PI * 1.32);
    cairo_stroke(cr);
    cairo_new_sub_path(cr);
    cairo_move_to(cr, cx, cy - r - s * 0.16);
    cairo_line_to(cr, cx, cy + s * 0.10);
    cairo_stroke(cr);
}

void Bar::refresh() {
    layout_zones();
    draw();
}

void Bar::draw() {
    if (win == None) return;
    cairo_surface_t *surf =
        cairo_xlib_surface_create(wm->dpy, win,
                                  argb_mode ? argb_visual : wm->visual, w, h);
    cairo_t *cr = cairo_create(surf);

    // ---- glass background ----
    cairo_set_operator(cr, CAIRO_OPERATOR_SOURCE);
    if (argb_mode) {
        cairo_set_source_rgba(cr, 15 / 255.0, 12 / 255.0, 30 / 255.0, 0.72);
    } else {
        // no compositor: near-solid approximation of the design
        cairo_set_source_rgb(cr, 15 / 255.0, 12 / 255.0, 30 / 255.0);
    }
    cairo_paint(cr);
    cairo_set_operator(cr, CAIRO_OPERATOR_OVER);

    // subtle inner top highlight
    cairo_pattern_t *sheen = cairo_pattern_create_linear(0, 0, 0, h);
    cairo_pattern_add_color_stop_rgba(sheen, 0.0, 1, 1, 1, 0.05);
    cairo_pattern_add_color_stop_rgba(sheen, 1.0, 1, 1, 1, 0.0);
    cairo_set_source(cr, sheen);
    cairo_rectangle(cr, 0, 0, w, h * 0.5);
    cairo_fill(cr);
    cairo_pattern_destroy(sheen);

    // bottom border
    cairo_set_source_rgba(cr, 1, 1, 1, 0.08);
    cairo_set_line_width(cr, 1.0);
    cairo_move_to(cr, 0, h - 0.5);
    cairo_line_to(cr, w, h - 0.5);
    cairo_stroke(cr);

    // ---- left: brand ----
    double lion_s = h - 16 * u;
    for (auto &z : zones) {
        if (z.id == Z_NONE && z.r.w == (int)lion_s) {  // brand zone
            draw_lion(cr, z.r.x + lion_s / 2, y + h / 2, lion_s);
            break;
        }
    }
    Color white(1, 1, 1, 1);
    text_draw(cr, "BraveOS", x + 12 * u + lion_s + 8 * u, y + h / 2 - 9 * u,
              wm->cfg.font, 13.5 * u, true, white);

    // divider
    cairo_set_source_rgba(cr, 1, 1, 1, 0.15);
    cairo_set_line_width(cr, 1);
    cairo_move_to(cr, divider_x + 0.5, y + 8 * u);
    cairo_line_to(cr, divider_x + 0.5, y + h - 8 * u);
    cairo_stroke(cr);

    // ---- workspace pills ----
    double pill_h = 20 * u;
    double pill_y = y + (h - pill_h) / 2;
    for (auto &z : zones) {
        if (z.id >= Z_PILL0 && z.id < Z_PILL0 + 100) {
            int i = z.id - Z_PILL0;
            bool active = (i == wm->current_desktop);
            bool hover = (z.id == hover_id);
            double pw = z.r.w;
            if (active) {
                // accent pill #6c3ce9
                cairo_set_source_rgba(cr, 0.424, 0.235, 0.914, 0.95);
                round_rect_path(cr, z.r.x, z.r.y, pw, pill_h, pill_h / 2);
                cairo_fill(cr);
                // pill glow
                cairo_set_source_rgba(cr, 0.424, 0.235, 0.914, 0.25);
                round_rect_path(cr, z.r.x - 1, z.r.y - 1, pw + 2, pill_h + 2,
                                (pill_h + 2) / 2);
                cairo_set_line_width(cr, 1.2);
                cairo_stroke(cr);
            } else if (hover) {
                cairo_set_source_rgba(cr, 1, 1, 1, 0.10);
                round_rect_path(cr, z.r.x, z.r.y, pw, pill_h, pill_h / 2);
                cairo_fill(cr);
            }
            Color pill_text = active ? white : Color(0.79, 0.77, 0.87, 1);
            text_draw(cr, std::to_string(i + 1), z.r.x + pw / 2,
                      pill_y + pill_h / 2 - 6.5 * u, wm->cfg.font, 12 * u, active,
                      pill_text, 0);
            // activity dot: workspaces that hold windows
            int nw = workspace_windows(i);
            if (nw > 0 && !active) {
                cairo_set_source_rgba(cr, 0.941, 0.306, 0.137, 0.9);
                cairo_new_sub_path(cr);
                cairo_arc(cr, z.r.x + pw / 2, z.r.y + pill_h + 3.5 * u,
                          std::max(1.6, 1.6 * u), 0, 2 * M_PI);
                cairo_fill(cr);
            }
        } else if (z.id == Z_ADD) {
            bool hover = (z.id == hover_id);
            if (hover) {
                cairo_set_source_rgba(cr, 1, 1, 1, 0.12);
                round_rect_path(cr, z.r.x, z.r.y, pill_h, pill_h, pill_h / 2);
                cairo_fill(cr);
            }
            // plus glyph
            double cxx = z.r.x + pill_h / 2, cyy = z.r.y + pill_h / 2;
            double a = pill_h * 0.22;
            cairo_set_source_rgba(cr, 1, 1, 1, hover ? 0.95 : 0.55);
            cairo_set_line_width(cr, 1.8 * u);
            cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
            cairo_new_sub_path(cr);
            cairo_move_to(cr, cxx - a, cyy);
            cairo_line_to(cr, cxx + a, cyy);
            cairo_move_to(cr, cxx, cyy - a);
            cairo_line_to(cr, cxx, cyy + a);
            cairo_stroke(cr);
        }
    }

    // ---- right side ----
    for (auto &z : zones) {
        bool hover = (z.id == hover_id);
        bool press = (z.id == press_id);
        // hover feedback pill for icon zones
        if ((z.id == Z_WIFI || z.id == Z_BT || z.id == Z_BATTERY ||
             z.id == Z_CLOCK || z.id == Z_POWER) &&
            (hover || press)) {
            cairo_set_source_rgba(cr, 1, 1, 1, press ? 0.20 : 0.10);
            round_rect_path(cr, z.r.x + 1, z.r.y + 3, z.r.w - 2, h - 6, 7 * u);
            cairo_fill(cr);
        }
        double cyy = y + h / 2;
        if (z.id == Z_POWER) {
            double s = 16 * u;
            draw_power_icon(cr, z.r.x + z.r.w / 2, cyy, s);
        } else if (z.id == Z_CLOCK) {
            std::string ct = clock_text();
            TextMetrics tm = text_measure(cr, ct, wm->cfg.font, 12.5 * u, false);
            double tx = z.r.x + z.r.w - 2 * u;
            text_draw(cr, ct, tx, cyy - 7 * u, wm->cfg.font, 12.5 * u, false,
                      Color(0.98, 0.98, 1.0, 1), 1);
            // keyboard layout code (when a non-default layout is active)
            if (!kb_layout.empty()) {
                text_draw(cr, kb_layout, tx - tm.width - 10 * u, cyy - 6 * u,
                          wm->cfg.font, 10 * u, false, Color(0.95, 0.60, 0.45, 1),
                          1);
            }
        } else if (z.id == Z_BATTERY && status.battery_present) {
            double is = 18 * u;
            draw_battery_icon(cr, z.r.x + 3 * u, cyy, is, status.battery_pct,
                              status.charging);
            text_draw(cr, fmtf("%d%%", status.battery_pct), z.r.x + 3 * u + is + 6 * u,
                      cyy - 6 * u, wm->cfg.font, 11.5 * u, false,
                      Color(0.95, 0.94, 0.98, 1));
        } else if (z.id == Z_WIFI && status.wifi_present) {
            double s = 17 * u;
            draw_wifi_icon(cr, z.r.x + z.r.w / 2, cyy, s,
                           status.wifi_up ? status.wifi_quality
                                          : (status.wifi_on ? 1 : 0),
                           status.wifi_on);
        } else if (z.id == Z_BT && status.bt_present) {
            double s = 16 * u;
            draw_bt_icon(cr, z.r.x + z.r.w / 2, cyy, s, status.bt_on);
        }
    }

    cairo_destroy(cr);
    cairo_surface_destroy(surf);
    XFlush(wm->dpy);
}

}  // namespace bwm
