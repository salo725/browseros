// BraveWM - wm.cpp: window manager core (lifecycle, clients, focus,
// workspaces, EWMH, struts, workareas, menus/launcher spawning).
#include "wm.hpp"
#include "menu.hpp"
#include "switcher.hpp"
#include "bar.hpp"
#include "i18n.hpp"

#include <fcntl.h>
#include <dlfcn.h>
#include <poll.h>
#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

namespace bwm {

static int g_x_error = 0;

static int x_error_handler(Display *dpy, XErrorEvent *ev) {
    char buf[256];
    buf[0] = 0;
    XGetErrorText(dpy, ev->error_code, buf, sizeof(buf));
    fprintf(stderr, "[bravos-wm] X error: %s (request %d, minor %d)\n", buf,
            ev->request_code, ev->minor_code);
    g_x_error = 1;
    return 0;
}

// catch fatal IO errors (X server gone) instead of dying inside libc
static int x_io_error_handler(Display *) {
    fprintf(stderr, "[bravos-wm] X connection lost\n");
    _exit(0);
}

// ---- Xinerama (runtime-loaded, optional) -----------------------------------

typedef struct { int screen_number; short x_org, y_org, width, height; } XiScreenInfo;
typedef XiScreenInfo *(*XiQueryScreensFn)(Display *, int *);
typedef Bool (*XiIsActiveFn)(Display *);
static void *g_xi_lib = nullptr;
static XiQueryScreensFn g_xi_query = nullptr;
static XiIsActiveFn g_xi_active = nullptr;

static void init_xinerama() {
    g_xi_lib = dlopen("libXinerama.so.1", RTLD_NOW | RTLD_GLOBAL);
    if (!g_xi_lib) g_xi_lib = dlopen("libXinerama.so", RTLD_NOW | RTLD_GLOBAL);
    if (!g_xi_lib) return;
    g_xi_query = (XiQueryScreensFn)dlsym(g_xi_lib, "XineramaQueryScreens");
    g_xi_active = (XiIsActiveFn)dlsym(g_xi_lib, "XineramaIsActive");
    if (!g_xi_query || !g_xi_active) {
        g_xi_query = nullptr;
        g_xi_active = nullptr;
    }
}

static bool xinerama_active(Display *dpy) {
    return g_xi_active ? g_xi_active(dpy) : False;
}

static XiScreenInfo *xinerama_query(Display *dpy, int *n) {
    *n = 0;
    if (!g_xi_query) return nullptr;
    return g_xi_query(dpy, n);
}

const char *const KB_SWITCH_CMD = "bravos-kb-switch";

// MWM hints (for honoring client decoration requests, e.g. Brave/Chromium
// custom titlebars and our own CSD apps)
static Atom mwm_hints_atom = None;
struct MwmHints {
    unsigned long flags, functions, decorations, input_mode, status;
};
#define MWM_HINTS_DECOR (1L << 1)
#define MWM_DECOR_ALL (1L << 0)
#define MWM_DECOR_BORDER (1L << 1)
#define MWM_DECOR_TITLE (1L << 3)

// ---- config ------------------------------------------------------------------

// free helpers implemented in wm.cpp, used across TUs
// (declarations live in wm.hpp)

static std::string trim(const std::string &s) {
    size_t a = s.find_first_not_of(" \t\r\n");
    if (a == std::string::npos) return "";
    size_t b = s.find_last_not_of(" \t\r\n");
    return s.substr(a, b - a + 1);
}

void config_load(Config &cfg, const std::string &path) {
    FILE *f = fopen(path.c_str(), "r");
    if (!f) return;
    char line[512];
    while (fgets(line, sizeof(line), f)) {
        std::string s = trim(line);
        if (s.empty() || s[0] == '#' || s[0] == '[') continue;
        size_t eq = s.find('=');
        if (eq == std::string::npos) continue;
        std::string k = trim(s.substr(0, eq));
        std::string v = trim(s.substr(eq + 1));
        if (k == "titlebar_height") cfg.titlebar_h = std::max(18, atoi(v.c_str()));
        else if (k == "corner_radius") cfg.corner_r = std::max(0, atoi(v.c_str()));
        else if (k == "workspaces") cfg.workspaces = std::max(1, atoi(v.c_str()));
        else if (k == "font") cfg.font = v;
        else if (k == "focus_follows_mouse") cfg.focus_follows_mouse = (v == "1" || v == "true");
        else if (k == "animations") cfg.animations = (v == "1" || v == "true");
        else if (k == "color_bg_focused") cfg.c_bg_focused = Color::from_hex(v);
        else if (k == "color_bg_unfocused") cfg.c_bg_unfocused = Color::from_hex(v);
        else if (k == "color_border_focused") cfg.c_border_focused = Color::from_hex(v);
        else if (k == "color_border_unfocused") cfg.c_border_unfocused = Color::from_hex(v);
        else if (k == "color_accent") cfg.c_accent = Color::from_hex(v);
        else if (k == "color_accent2") cfg.c_accent2 = Color::from_hex(v);
        else if (k == "terminal") cfg.term_cmd = v;
        else if (k == "file_manager") cfg.files_cmd = v;
        else if (k == "browser") cfg.browser_cmd = v;
        else if (k == "settings") cfg.settings_cmd = v;
        else if (k == "launcher") cfg.launcher_cmd = v;
        else if (k == "lock") cfg.lock_cmd = v;
        else if (k == "kb_layouts") cfg.kb_layouts = v;
    }
    fclose(f);
}

// ---- lifecycle ------------------------------------------------------------------

bool Wm::init(const std::string &config_path) {
    dpy = XOpenDisplay(nullptr);
    if (!dpy) {
        fprintf(stderr, "[bravos-wm] cannot open display\n");
        return false;
    }
    XSetIOErrorHandler(x_io_error_handler);
    XSetErrorHandler(x_error_handler);
    XSynchronize(dpy, True);  // fail fast during bring-up; disabled below

    screen = DefaultScreen(dpy);
    root = RootWindow(dpy, screen);
    visual = DefaultVisual(dpy, screen);
    colormap = DefaultColormap(dpy, screen);
    depth = DefaultDepth(dpy, screen);

    // we must own the selection/redirect or the session is broken
    XSelectInput(dpy, root,
                 SubstructureRedirectMask | SubstructureNotifyMask |
                     StructureNotifyMask | PropertyChangeMask | ButtonPressMask |
                     FocusChangeMask | EnterWindowMask | KeymapStateMask);

    // extensions
    int err = 0;
    if (XShapeQueryExtension(dpy, &shape_event_base, &err)) have_shape = true;
    init_xinerama();
    have_xinerama = xinerama_active(dpy);

    atoms.init(dpy);
    mwm_hints_atom = XInternAtom(dpy, "_MOTIF_WM_HINTS", False);

    // config: system defaults, then user overrides
    cfg.workspaces = 2;
    config_load(cfg, "/usr/share/bravos/bravos-wm.conf");
    std::string home = getenv("HOME") ? getenv("HOME") : "";
    if (!home.empty())
        config_load(cfg, home + "/.config/bravos/bravos-wm.conf");
    if (!config_path.empty()) config_load(cfg, config_path);
    num_desktops = cfg.workspaces;

    cursor_normal = XCreateFontCursor(dpy, XC_left_ptr);
    cursor_move = XCreateFontCursor(dpy, XC_fleur);
    cursor_nw = XCreateFontCursor(dpy, XC_top_left_corner);
    cursor_ne = XCreateFontCursor(dpy, XC_top_right_corner);
    cursor_sw = XCreateFontCursor(dpy, XC_bottom_left_corner);
    cursor_se = XCreateFontCursor(dpy, XC_bottom_right_corner);
    cursor_h = XCreateFontCursor(dpy, XC_sb_h_double_arrow);
    cursor_v = XCreateFontCursor(dpy, XC_sb_v_double_arrow);

    // _NET_SUPPORTING_WM_CHECK
    XSetWindowAttributes wa = {};
    wa.event_mask = PropertyChangeMask;
    wa.override_redirect = True;
    wm_check_win = XCreateWindow(dpy, root, -100, -100, 1, 1, 0, 0, InputOnly,
                                 CopyFromParent, CWEventMask | CWOverrideRedirect,
                                 &wa);
    XChangeProperty(dpy, wm_check_win, atoms.net_wm_name, atoms.utf8_string, 8,
                    PropModeReplace, (const unsigned char *)"BraveWM", 7);
    XChangeProperty(dpy, root, atoms.net_supporting_wm_check, XA_WINDOW, 32,
                    PropModeReplace, (unsigned char *)&wm_check_win, 1);
    XChangeProperty(dpy, wm_check_win, atoms.net_supporting_wm_check, XA_WINDOW,
                    32, PropModeReplace, (unsigned char *)&wm_check_win, 1);

    // supported atoms
    std::vector<Atom> sup = atoms.supported_list();
    XChangeProperty(dpy, root, atoms.net_supported, XA_ATOM, 32, PropModeReplace,
                    (unsigned char *)sup.data(), (int)sup.size());

    refresh_monitors();
    update_desktop_props();
    update_workarea();

    menu = new Menu(this);
    switcher = new Switcher(this);

    // top status bar (dock window + struts); i18n first for its strings
    g_i18n.init();
    bar = new Bar(this);
    bar->create();

    // snap preview window (override redirect)
    XColor xc;
    xc.red = 0x1400;
    xc.green = 0x1026;
    xc.blue = 0x2600;
    XAllocColor(dpy, colormap, &xc);
    wa = {};
    wa.override_redirect = True;
    wa.background_pixel = BlackPixel(dpy, screen);
    wa.event_mask = ExposureMask;
    snap_preview = XCreateWindow(dpy, root, 0, 0, 10, 10, 0, depth, InputOutput,
                                 visual, CWOverrideRedirect | CWBackPixel |
                                             CWEventMask, &wa);

    scan_existing();
    grab_keys();
    if (bar) bar->refresh();  // workspace activity dots (existing windows)

    XSetErrorHandler(x_error_handler);
    XSynchronize(dpy, False);
    fprintf(stderr, "[bravos-wm] ready (BraveWM %dx%d, %d desktops, shape=%d)\n",
            DisplayWidth(dpy, screen), DisplayHeight(dpy, screen), num_desktops,
            (int)have_shape);
    return true;
}

void Wm::shutdown() {
    if (bar) {
        bar->destroy_win();
        delete bar;
        bar = nullptr;
    }
    // reparent everything back to root so apps survive our exit
    for (Client *c : clients) {
        if (c->frame != None && c->win != None) {
            XUnmapWindow(dpy, c->frame);
            XReparentWindow(dpy, c->win, root, c->geom.x, c->geom.y);
            XRemoveFromSaveSet(dpy, c->win);
        }
        delete c;
    }
    clients.clear();
    stacking.clear();
    focus_mru.clear();
    if (wm_check_win != None) XDestroyWindow(dpy, wm_check_win);
    if (dpy) XCloseDisplay(dpy);
}

// ---- client management -------------------------------------------------------

static bool get_mwm_undecorated(Display *dpy, Window w) {
    Atom type;
    int fmt;
    unsigned long n, after;
    unsigned char *data = nullptr;
    bool undec = false;
    if (XGetWindowProperty(dpy, w, mwm_hints_atom, 0, 5, False, AnyPropertyType,
                           &type, &fmt, &n, &after, &data) == Success &&
        data && n >= 3) {
        unsigned long *h = (unsigned long *)data;
        if ((h[0] & MWM_HINTS_DECOR) &&
            !(h[2] & MWM_DECOR_ALL) &&
            !(h[2] & MWM_DECOR_TITLE) &&
            !(h[2] & MWM_DECOR_BORDER))
            undec = true;
        XFree(data);
    } else if (data) {
        XFree(data);
    }
    return undec;
}

static WinType window_type(Wm *wm, Window w) {
    std::vector<long> types =
        get_cardinals(wm->dpy, w, wm->atoms.net_wm_window_type, 8);
    for (unsigned long t : types) {
        if (t == wm->atoms.net_wm_window_type_desktop) return WinType::Desktop;
        if (t == wm->atoms.net_wm_window_type_dock) return WinType::Dock;
        if (t == wm->atoms.net_wm_window_type_toolbar) return WinType::Toolbar;
        if (t == wm->atoms.net_wm_window_type_menu) return WinType::Menu;
        if (t == wm->atoms.net_wm_window_type_utility) return WinType::Utility;
        if (t == wm->atoms.net_wm_window_type_splash) return WinType::Splash;
        if (t == wm->atoms.net_wm_window_type_dialog) return WinType::Dialog;
        if (t == wm->atoms.net_wm_window_type_dropdown) return WinType::Dropdown;
        if (t == wm->atoms.net_wm_window_type_popup) return WinType::Popup;
        if (t == wm->atoms.net_wm_window_type_tooltip) return WinType::Tooltip;
        if (t == wm->atoms.net_wm_window_type_notification) return WinType::Notification;
        if (t == wm->atoms.net_wm_window_type_combo) return WinType::Combo;
    }
    return WinType::Normal;
}

static bool type_is_layer_bottom(WinType t) {
    return t == WinType::Desktop || t == WinType::Dock;
}
static bool type_dont_decorate(WinType t) {
    return t == WinType::Desktop || t == WinType::Dock || t == WinType::Splash ||
           t == WinType::Toolbar || t == WinType::Utility ||
           t == WinType::Dropdown || t == WinType::Popup ||
           t == WinType::Tooltip || t == WinType::Notification ||
           t == WinType::Combo;
}
static bool type_dont_focus(WinType t) {
    return t == WinType::Desktop || t == WinType::Dock ||
           t == WinType::Dropdown || t == WinType::Popup ||
           t == WinType::Tooltip || t == WinType::Notification ||
           t == WinType::Combo;
}

void Wm::manage(Window w) {
    if (find_client(w)) return;
    XWindowAttributes wa;
    if (!XGetWindowAttributes(dpy, w, &wa)) return;
    if (wa.override_redirect) return;
    if (w == root || w == wm_check_win || w == snap_preview) return;

    Client *c = new Client();
    c->wm = this;
    c->win = w;
    c->type = window_type(this, w);
    c->wants_decor = !type_dont_decorate(c->type) && !get_mwm_undecorated(dpy, w);
    c->never_focus = type_dont_focus(c->type);
    c->desktop = current_desktop;
    if (type_is_layer_bottom(c->type)) c->desktop = 0xFFFFFFFF;  // all desktops

    XWMHints *hints = XGetWMHints(dpy, w);
    if (hints) {
        if (hints->flags & InputHint && hints->input == False) c->never_focus = true;
        if (hints->flags & XUrgencyHint) c->urgent = true;
        if (hints->flags & WindowGroupHint && hints->window_group != None) {
            // group leaders: nothing special for now
        }
        XFree(hints);
    }
    XGetTransientForHint(dpy, w, &c->transient_for);
    if (c->transient_for == w) c->transient_for = None;
    if (c->transient_for != None && c->type == WinType::Normal)
        c->type = WinType::Dialog;

    XClassHint ch;
    if (XGetClassHint(dpy, w, &ch)) {
        c->res_name = ch.res_name ? ch.res_name : "";
        c->res_class = ch.res_class ? ch.res_class : "";
        if (ch.res_name) XFree(ch.res_name);
        if (ch.res_class) XFree(ch.res_class);
    }
    if (!get_text(dpy, w, atoms.net_wm_name, &c->title))
        if (!get_text(dpy, w, atoms.wm_name, &c->title)) c->title = "";
    client_read_icon(c);

    // initial geometry
    long usr_pos = 0;
    XSizeHints sh;
    long supplied = 0;
    if (XGetWMNormalHints(dpy, w, &sh, &supplied)) {
        if (supplied & USPosition) usr_pos = 1;
        if (supplied & PPosition) usr_pos = 2;
    }
    Rect mon = monitors.empty() ? Rect(0, 0, DisplayWidth(dpy, screen),
                                       DisplayHeight(dpy, screen))
                                : monitors[0].rect;
    if (wa.width > 4 && wa.height > 4) {
        c->geom = Rect(wa.x, wa.y, wa.width, wa.height);
    } else {
        c->geom = Rect(mon.x + 60, mon.y + 80, std::min(900, mon.w - 120),
                       std::min(600, mon.h - 160));
    }
    // position sanity: place user-positioned/transient windows as requested,
    // everything else via cascade; center transients on their parent
    Client *parent =
        c->transient_for != None ? find_client(c->transient_for) : nullptr;
    if (parent) {
        c->geom.x = parent->geom.x + std::max(0, (parent->geom.w - c->geom.w) / 2);
        c->geom.y = parent->geom.y + std::max(0, (parent->geom.h - c->geom.h) / 2);
    } else if (!usr_pos || c->geom.x < mon.x - 40 || c->geom.y < mon.y - 40 ||
               c->geom.x + c->geom.w > mon.x + mon.w + 40) {
        int n = (int)(clients.size() % 8);
        c->geom.x = mon.x + 40 + 26 * n;
        c->geom.y = mon.y + 60 + 26 * n;
        if (c->geom.x + c->geom.w > mon.x + mon.w)
            c->geom.x = mon.x + std::max(8, (mon.w - c->geom.w) / 3);
        if (c->geom.y + c->geom.h > mon.y + mon.h)
            c->geom.y = mon.y + std::max(8, (mon.h - c->geom.h) / 3);
    }
    // clamp absurd sizes
    int rw = DisplayWidth(dpy, screen), rh = DisplayHeight(dpy, screen);
    if (c->geom.w > rw) c->geom.w = rw - 20;
    if (c->geom.h > rh) c->geom.h = rh - 20;

    clients.push_back(c);
    stacking.push_back(c);
    read_struts(c);

    // select for client-side events (title/icon changes, clicks for focus).
    // Multiple clients may select the same events on one window; the app
    // keeps its own copies, so nothing is stolen from it.
    XSelectInput(dpy, c->win, PropertyChangeMask | ButtonPressMask);

    if (c->wants_decor) {
        client_frame_create(c);
    }
    client_set_wm_state(c, NormalState);
    // _NET_WM_STATE / allowed actions / desktop
    set_net_wm_state(this, c);
    set_net_allowed_actions(this, c);
    if (c->desktop != (int)0xFFFFFFFF)
        XChangeProperty(dpy, c->win, atoms.net_wm_desktop, XA_CARDINAL, 32,
                        PropModeReplace, (unsigned char *)&c->desktop, 1);
    else {
        unsigned long all = 0xFFFFFFFF;
        XChangeProperty(dpy, c->win, atoms.net_wm_desktop, XA_CARDINAL, 32,
                        PropModeReplace, (unsigned char *)&all, 1);
    }

    // desktop visibility
    bool show = (c->desktop == (int)0xFFFFFFFF || c->desktop == current_desktop);
    c->visible = show;
    if (show) {
        XMapWindow(dpy, c->win);
        if (c->frame != None) {
            XMapWindow(dpy, c->frame);
            if (cfg.animations && c->type == WinType::Normal) {
                Rect fr = c->frame_rect();
                Rect small = Rect(fr.x + fr.w / 2 - fr.w / 8,
                                  fr.y + fr.h / 2 - fr.h / 8, fr.w / 4, fr.h / 4);
                XMoveResizeWindow(dpy, c->frame, small.x, small.y, small.w, small.h);
                client_anim_start(c, fr, false);
            } else {
                client_frame_configure(c);
            }
        }
        if (!c->never_focus && !showing_desktop) focus(c);
    } else {
        // hidden desktop: keep it withdrawn until its desktop is shown
    }

    // layering: desktop windows at the very back; dock windows float above
    // normal windows (panels/docks get the above layer)
    if (c->type == WinType::Desktop) {
        send_to_back(c);
    } else if (c->type == WinType::Dock) {
        if (!c->above) {
            c->above = true;
            set_net_wm_state(this, c);
        }
        bring_to_front(c);
    } else {
        bring_to_front(c);
    }

    sync_net_client_list();
    update_workarea();
    if (bar) bar->refresh();
}

void Wm::unmanage(Client *c, bool client_gone) {
    if (!c) return;
    auto it = std::find(clients.begin(), clients.end(), c);
    if (it == clients.end()) return;
    clients.erase(it);
    auto st = std::find(stacking.begin(), stacking.end(), c);
    if (st != stacking.end()) stacking.erase(st);
    auto mr = std::find(focus_mru.begin(), focus_mru.end(), c);
    if (mr != focus_mru.end()) focus_mru.erase(mr);

    if (c->frame != None) {
        if (!client_gone) {
            // app withdrew the window: give it back to root, keep it alive
            XUnmapWindow(dpy, c->frame);
            XReparentWindow(dpy, c->win, root, c->geom.x, c->geom.y);
            XRemoveFromSaveSet(dpy, c->win);
        }
        XDestroyWindow(dpy, c->frame);
        c->frame = None;
    }
    struts.erase(c->win);

    if (focused_client == c) update_focus_after(c);
    if (interact_c == c) end_interactive();

    delete c;
    sync_net_client_list();
    update_workarea();
}

Client *Wm::find_client(Window w) const {
    for (Client *c : clients)
        if (c->win == w) return c;
    return nullptr;
}

Client *Wm::find_client_by_frame(Window f) const {
    for (Client *c : clients)
        if (c->frame == f) return c;
    return nullptr;
}

void Wm::close(Client *c) {
    if (!c) return;
    if (has_protocol(dpy, c->win, atoms.wm_delete_window)) {
        XEvent e = {};
        e.type = ClientMessage;
        e.xclient.window = c->win;
        e.xclient.message_type = atoms.wm_protocols;
        e.xclient.format = 32;
        e.xclient.data.l[0] = (long)atoms.wm_delete_window;
        e.xclient.data.l[1] = (long)last_event_time;
        XSendEvent(dpy, c->win, False, NoEventMask, &e);
    } else {
        XKillClient(dpy, c->win);
    }
}

void Wm::kill(Client *c) {
    if (c) XKillClient(dpy, c->win);
}

void Wm::iconify(Client *c) {
    if (!c || c->iconified || !c->visible) return;
    Rect fr = c->frame_rect();
    // shrink toward bottom-center dock, then unmap (anim step completes it)
    Rect to(fr.x + fr.w / 2 - 40, DisplayHeight(dpy, screen) - 20, 80, 60);
    if (!c->wants_decor) to = fr;
    if (cfg.animations && c->frame != None)
        client_anim_start(c, to, true);
    else {
        c->iconic_by_wm = true;
        XUnmapWindow(dpy, c->win);
        if (c->frame != None) XUnmapWindow(dpy, c->frame);
        c->iconic_by_wm = false;
        c->visible = false;
        c->iconified = true;
        client_set_wm_state(c, IconicState);
        update_focus_after(c);
        sync_net_client_list();
    }
}

void Wm::uniconify(Client *c) {
    if (!c) return;
    bool show = (c->desktop == (int)0xFFFFFFFF || c->desktop == current_desktop);
    if (!show) switch_desktop(c->desktop);
    if (!c->iconified && c->visible) return;
    c->iconified = false;
    c->visible = true;
    client_set_wm_state(c, NormalState);
    client_frame_configure(c);
    if (c->frame != None) XMapWindow(dpy, c->frame);
    XMapWindow(dpy, c->win);
    if (cfg.animations && c->frame != None) {
        Rect fr = c->frame_rect();
        Rect small(fr.x + fr.w / 2 - fr.w / 8, fr.y + fr.h / 2 - fr.h / 8,
                   fr.w / 4, fr.h / 4);
        XMoveResizeWindow(dpy, c->frame, small.x, small.y, small.w, small.h);
        client_anim_start(c, fr, false);
    }
    bring_to_front(c);
    focus(c);
    sync_net_client_list();
}

void Wm::toggle_maximize(Client *c) {
    if (!c || c->fullscreen) return;
    Rect wa = workarea_for(c);
    if (c->maximized) {
        c->maximized = false;
        c->geom = c->restore;
        client_frame_configure(c);
    } else {
        c->restore = c->geom;
        c->maximized = true;
        // client area sits inside the frame decorations
        c->geom = Rect(wa.x + cfg.border_w, wa.y + cfg.titlebar_h,
                       wa.w - 2 * cfg.border_w,
                       wa.h - cfg.titlebar_h - cfg.border_w);
        client_frame_configure(c);
        if (cfg.animations && c->frame != None)
            client_anim_start(c, c->frame_rect(), false);
    }
    set_net_wm_state(this, c);
    draw_client(c);
}

void Wm::set_fullscreen(Client *c, bool on) {
    if (!c || on == c->fullscreen) return;
    Rect mon = monitor_for_window(c);
    if (on) {
        c->restore = c->geom;
        c->fullscreen = true;
        if (c->frame != None) client_frame_destroy(c);
        c->wants_decor = false;  // remembered only for fullscreen lifetime
        c->geom = mon;
        XMoveResizeWindow(dpy, c->win, mon.x, mon.y, mon.w, mon.h);
        bring_to_front(c);
        focus(c);
    } else {
        c->fullscreen = false;
        // restore decoration unless the client asked to be undecorated itself
        c->wants_decor =
            !type_dont_decorate(c->type) && !get_mwm_undecorated(dpy, c->win);
        c->geom = c->restore;
        if (c->wants_decor) client_frame_create(c);
        client_frame_configure(c);
        bring_to_front(c);
    }
    set_net_wm_state(this, c);
    sync_net_client_list();
}

void Wm::toggle_above(Client *c) {
    if (!c) return;
    c->above = !c->above;
    if (c->above) bring_to_front(c);
    else send_to_back(c);
    set_net_wm_state(this, c);
}

void Wm::move_to_desktop(Client *c, int d) {
    if (!c || d < 0 || d >= num_desktops) return;
    int old = c->desktop;
    c->desktop = d;
    unsigned long dl = (unsigned long)d;
    XChangeProperty(dpy, c->win, atoms.net_wm_desktop, XA_CARDINAL, 32,
                    PropModeReplace, (unsigned char *)&dl, 1);
    bool show = (d == current_desktop);
    if (show && !c->visible && !c->iconified) {
        c->visible = true;
        if (c->frame != None) XMapWindow(dpy, c->frame);
        XMapWindow(dpy, c->win);
        client_frame_configure(c);
        focus(c);
    } else if (!show && c->visible && old != current_desktop) {
        if (c->frame != None) XUnmapWindow(dpy, c->frame);
        XUnmapWindow(dpy, c->win);
        c->visible = false;
        if (focused_client == c) update_focus_after(c);
    }
    sync_net_client_list();
    if (bar) bar->refresh();
}

void Wm::move_window(Client *c, int x, int y) {
    if (!c) return;
    c->geom.x = x;
    c->geom.y = y;
    client_frame_configure(c);
}

void Wm::resize_window(Client *c, int x, int y, int w, int h) {
    if (!c) return;
    w = std::max(50, w);
    h = std::max(50, h);
    c->geom = Rect(x, y, w, h);
    if (c->maximized) {
        c->maximized = false;
        set_net_wm_state(this, c);
    }
    client_frame_configure(c);
    draw_client(c);
}

void Wm::send_to_back(Client *c) {
    if (!c) return;
    auto it = std::find(stacking.begin(), stacking.end(), c);
    if (it != stacking.end()) stacking.erase(it);
    // insert after all desktop windows (desktop stays lowest)
    size_t insert_at = 0;
    while (insert_at < stacking.size() &&
           stacking[insert_at]->type == WinType::Desktop)
        insert_at++;
    stacking.insert(stacking.begin() + insert_at, c);
    restack_all();
}

void Wm::bring_to_front(Client *c) {
    if (!c) return;
    auto it = std::find(stacking.begin(), stacking.end(), c);
    if (it != stacking.end()) stacking.erase(it);
    stacking.push_back(c);
    restack_all();
}

// ---- focus ---------------------------------------------------------------------

void Wm::focus(Client *c, bool raise_win) {
    if (c && (c->never_focus || c->iconified || !c->visible)) return;
    if (c && c->type == WinType::Desktop) return;
    Client *prev = focused_client;
    focused_client = c;
    if (prev && prev != c) draw_client(prev);
    if (!c) {
        XSetInputFocus(dpy, PointerRoot, RevertToPointerRoot, last_event_time);
        set_active_window(nullptr);
        return;
    }
    if (raise_win) bring_to_front(c);
    // WM_TAKE_FOCUS + input focus
    if (has_protocol(dpy, c->win, atoms.wm_take_focus)) {
        XEvent e = {};
        e.type = ClientMessage;
        e.xclient.window = c->win;
        e.xclient.message_type = atoms.wm_protocols;
        e.xclient.format = 32;
        e.xclient.data.l[0] = (long)atoms.wm_take_focus;
        e.xclient.data.l[1] = (long)last_event_time;
        XSendEvent(dpy, c->win, False, NoEventMask, &e);
    }
    // only focus viewable windows (XSetInputFocus -> BadMatch otherwise)
    XWindowAttributes fwa;
    if (XGetWindowAttributes(dpy, c->win, &fwa) && fwa.map_state == IsViewable)
        XSetInputFocus(dpy, c->win, RevertToPointerRoot, last_event_time);
    c->urgent = false;
    draw_client(c);
    set_active_window(c);
    // MRU order
    auto it = std::find(focus_mru.begin(), focus_mru.end(), c);
    if (it != focus_mru.end()) focus_mru.erase(it);
    focus_mru.insert(focus_mru.begin(), c);
}

void Wm::unfocus() {
    Client *prev = focused_client;
    focused_client = nullptr;
    if (prev) draw_client(prev);
    set_active_window(nullptr);
}

void Wm::update_focus_after(Client *gone) {
    if (focused_client != gone) return;
    focused_client = nullptr;
    for (Client *c : focus_mru) {
        if (c != gone && c->visible && !c->iconified && !c->never_focus) {
            focus(c, false);
            return;
        }
    }
    set_active_window(nullptr);
}

void Wm::switch_desktop(int d) {
    if (d < 0 || d >= num_desktops || d == current_desktop) return;
    current_desktop = d;
    apply_desktop_visibility(false);
    update_desktop_props();
    if (bar) bar->refresh();
}

void Wm::set_desktop_count(int n) {
    n = std::max(1, std::min(9, n));
    if (n == num_desktops) return;
    if (n < num_desktops) {
        for (Client *c : clients) {
            if (c->desktop != (int)0xFFFFFFFF && c->desktop >= n) c->desktop = n - 1;
        }
        if (current_desktop >= n) current_desktop = n - 1;
    }
    num_desktops = n;
    apply_desktop_visibility(false);
    update_desktop_props();
    update_workarea();
    if (bar) bar->refresh();
}

void Wm::set_show_desktop(bool on) {
    if (showing_desktop == on) return;
    showing_desktop = on;
    unsigned long v = on ? 1ul : 0ul;
    XChangeProperty(dpy, root, atoms.net_showing_desktop, XA_CARDINAL, 32,
                    PropModeReplace, (unsigned char *)&v, 1);
    for (Client *c : clients) {
        bool showable = (c->desktop == (int)0xFFFFFFFF || c->desktop == current_desktop) &&
                        !type_is_layer_bottom(c->type);
        if (!showable) continue;
        if (on && c->visible && !c->iconified) {
            if (c->frame != None) XUnmapWindow(dpy, c->frame);
            XUnmapWindow(dpy, c->win);
            c->visible = false;
            c->iconified = true;
            c->hidden_by_show_desktop = true;
            client_set_wm_state(c, IconicState);
        }
    }
    if (on) {
        unfocus();
    } else {
        // un-iconify windows hidden by show-desktop only
        for (Client *c : clients) {
            if (c->hidden_by_show_desktop) {
                c->hidden_by_show_desktop = false;
                uniconify(c);
            }
        }
    }
    sync_net_client_list();
}

void Wm::toggle_show_desktop() { set_show_desktop(!showing_desktop); }

void Wm::apply_desktop_visibility(bool animate) {
    (void)animate;
    for (Client *c : clients) {
        bool show = (c->desktop == (int)0xFFFFFFFF || c->desktop == current_desktop);
        if (show && !c->visible && !c->iconified) {
            c->visible = true;
            if (c->frame != None) XMapWindow(dpy, c->frame);
            XMapWindow(dpy, c->win);
            client_frame_configure(c);
        } else if (!show && c->visible) {
            if (c->frame != None) XUnmapWindow(dpy, c->frame);
            XUnmapWindow(dpy, c->win);
            c->visible = false;
            if (focused_client == c) focused_client = nullptr;
        }
    }
    // refocus top window of this desktop
    Client *pick = nullptr;
    for (Client *c : focus_mru) {
        if (c->visible && !c->iconified && !c->never_focus) {
            pick = c;
            break;
        }
    }
    if (pick) focus(pick, false);
    else {
        unfocus();
    }
    sync_net_client_list();
}

// ---- EWMH sync --------------------------------------------------------------------

void Wm::sync_net_client_list() {
    std::vector<Window> list;
    for (Client *c : clients)
        if (!c->iconified || true) list.push_back(c->win);  // EWMH: all managed
    XChangeProperty(dpy, root, atoms.net_client_list, XA_WINDOW, 32,
                    PropModeReplace, (unsigned char *)list.data(),
                    (int)list.size());
    std::vector<Window> stack;
    for (Client *c : stacking) stack.push_back(c->win);
    XChangeProperty(dpy, root, atoms.net_client_list_stacking, XA_WINDOW, 32,
                    PropModeReplace, (unsigned char *)stack.data(),
                    (int)stack.size());
}

void Wm::set_active_window(Client *c) {
    Window w = c ? c->win : None;
    XChangeProperty(dpy, root, atoms.net_active_window, XA_WINDOW, 32,
                    PropModeReplace, (unsigned char *)&w, 1);
}

void Wm::update_desktop_props() {
    unsigned long n = (unsigned long)num_desktops;
    XChangeProperty(dpy, root, atoms.net_number_of_desktops, XA_CARDINAL, 32,
                    PropModeReplace, (unsigned char *)&n, 1);
    unsigned long cur = (unsigned long)current_desktop;
    XChangeProperty(dpy, root, atoms.net_current_desktop, XA_CARDINAL, 32,
                    PropModeReplace, (unsigned char *)&cur, 1);
    // names
    std::string names;
    for (int i = 0; i < num_desktops; i++)
        names += fmtf("Workspace %d", i + 1) + std::string(1, '\0');
    XChangeProperty(dpy, root, atoms.net_desktop_names, atoms.utf8_string, 8,
                    PropModeReplace, (const unsigned char *)names.c_str(),
                    (int)names.size());
    // viewport
    unsigned long vp[2] = {0, 0};
    XChangeProperty(dpy, root, atoms.net_desktop_viewport, XA_CARDINAL, 32,
                    PropModeReplace, (unsigned char *)vp, 2);
    unsigned long dg[2] = {(unsigned long)DisplayWidth(dpy, screen),
                           (unsigned long)DisplayHeight(dpy, screen)};
    XChangeProperty(dpy, root, atoms.net_desktop_geometry, XA_CARDINAL, 32,
                    PropModeReplace, (unsigned char *)dg, 2);
}

void Wm::update_workarea() {
    update_desktop_props();
    Rect wa = workarea_for_monitor(
        monitors.empty() ? Rect(0, 0, DisplayWidth(dpy, screen),
                               DisplayHeight(dpy, screen))
                         : monitors[0].rect);
    std::vector<unsigned long> areas;
    for (int i = 0; i < num_desktops; i++) {
        areas.push_back((unsigned long)wa.x);
        areas.push_back((unsigned long)wa.y);
        areas.push_back((unsigned long)wa.w);
        areas.push_back((unsigned long)wa.h);
    }
    XChangeProperty(dpy, root, atoms.net_workarea, XA_CARDINAL, 32,
                    PropModeReplace, (unsigned char *)areas.data(),
                    (int)areas.size());
}

void Wm::read_struts(Client *c) {
    std::vector<long> v = get_cardinals(dpy, c->win, atoms.net_wm_strut_partial, 12);
    Strut s;
    if (v.size() >= 12) {
        s.left = v[0];
        s.right = v[1];
        s.top = v[2];
        s.bottom = v[3];
        s.left_start = v[4];
        s.left_end = v[5];
        s.right_start = v[6];
        s.right_end = v[7];
        s.top_start = v[8];
        s.top_end = v[9];
        s.bottom_start = v[10];
        s.bottom_end = v[11];
        struts[c->win] = s;
        update_workarea();
    } else {
        std::vector<long> s4 = get_cardinals(dpy, c->win, atoms.net_wm_strut, 4);
        if (s4.size() >= 4) {
            s.left = s4[0];
            s.right = s4[1];
            s.top = s4[2];
            s.bottom = s4[3];
            struts[c->win] = s;
            update_workarea();
        }
    }
}

Rect Wm::workarea_for_monitor(const Rect &mon) const {
    int rw = DisplayWidth(dpy, screen), rh = DisplayHeight(dpy, screen);
    int L = 0, R = 0, T = 0, B = 0;
    for (auto &kv : struts) {
        const Strut &s = kv.second;
        // apply struts to monitors they touch (simplified: global edges)
        if (s.left > 0 && mon.x <= 0) L = std::max(L, s.left);
        if (s.right > 0 && mon.x + mon.w >= rw) R = std::max(R, s.right);
        if (s.top > 0 && mon.y <= 0) T = std::max(T, s.top);
        if (s.bottom > 0 && mon.y + mon.h >= rh) B = std::max(B, s.bottom);
    }
    Rect wa(mon.x + L, mon.y + T, mon.w - L - R, mon.h - T - B);
    if (wa.w < 100) wa.w = mon.w;
    if (wa.h < 100) wa.h = mon.h;
    return wa;
}

Rect Wm::workarea_for(Client *c) const {
    return workarea_for_monitor(monitor_for_window(c));
}

Rect Wm::monitor_for_point(int x, int y) const {
    for (auto &m : monitors)
        if (m.rect.contains(x, y)) return m.rect;
    if (!monitors.empty()) return monitors[0].rect;
    return Rect(0, 0, DisplayWidth(dpy, screen), DisplayHeight(dpy, screen));
}

Rect Wm::monitor_for_window(Client *c) const {
    if (!c) {
        int px = 0, py = 0, rx = 0, ry = 0;
        Window rr, cr;
        unsigned int mk;
        if (XQueryPointer((Display *)dpy, root, &rr, &cr, &rx, &ry, &px, &py, &mk))
            return monitor_for_point(rx, ry);
        return monitors.empty() ? Rect(0, 0, DisplayWidth(dpy, screen),
                                       DisplayHeight(dpy, screen))
                                : monitors[0].rect;
    }
    int cx = c->geom.x + c->geom.w / 2;
    int cy = c->geom.y + c->geom.h / 2;
    return monitor_for_point(cx, cy);
}

void Wm::refresh_monitors() {
    monitors.clear();
    if (have_xinerama) {
        int n = 0;
        XiScreenInfo *si = xinerama_query(dpy, &n);
        if (si) {
            for (int i = 0; i < n; i++)
                monitors.push_back(
                    {Rect(si[i].x_org, si[i].y_org, si[i].width, si[i].height),
                     (int)si[i].screen_number, i == 0});
            XFree(si);
        }
    }
    if (monitors.empty()) {
        monitors.push_back(
            {Rect(0, 0, DisplayWidth(dpy, screen), DisplayHeight(dpy, screen)), 0,
             true});
    }
}

// ---- spawning / menus ------------------------------------------------------------

void Wm::spawn(const std::string &cmd) const {
    pid_t pid = fork();
    if (pid < 0) return;
    if (pid == 0) {
        // detach from our stdio; child gets its own session
        int devnull = open("/dev/null", O_RDWR);
        if (devnull >= 0) {
            dup2(devnull, 0);
            dup2(devnull, 1);
            dup2(devnull, 2);
            if (devnull > 2) ::close(devnull);
        }
        setsid();
        // run via sh so users can use arguments in config
        char *argv[] = {(char *)"/bin/sh", (char *)"-c", (char *)cmd.c_str(),
                        nullptr};
        execv("/bin/sh", argv);
        _exit(127);
    }
    // reap later children so we do not accumulate zombies
    signal(SIGCHLD, SIG_IGN);
}

void Wm::open_root_menu(int x, int y) {
    std::vector<MenuEntry> items;
    items.push_back({g_i18n.tr("menu_browser"), "",
                     [this]() { spawn(cfg.browser_cmd); }, true});
    items.push_back({g_i18n.tr("menu_files"), "",
                     [this]() { spawn(cfg.files_cmd); }, true});
    items.push_back({g_i18n.tr("menu_terminal"), "",
                     [this]() { spawn(cfg.term_cmd); }, true});
    items.push_back({"", "", nullptr, false});
    items.push_back({g_i18n.tr("menu_settings"), "",
                     [this]() { spawn(cfg.settings_cmd); }, true});
    items.push_back({g_i18n.tr("menu_appgrid"), "",
                     [this]() { spawn(cfg.launcher_cmd); }, true});
    items.push_back({g_i18n.tr("menu_shortcuts"), "",
                     [this]() { spawn(cfg.settings_cmd + " --page shortcuts"); },
                     true});
    items.push_back({"", "", nullptr, false});
    items.push_back({g_i18n.tr("menu_lock"), "",
                     [this]() { spawn(cfg.lock_cmd); }, true});
    items.push_back({g_i18n.tr("power_logout"), "",
                     [this]() { if (bar) bar->logout(); }, true});
    items.push_back({g_i18n.tr("menu_restart_wm"), "",
                     [this]() { spawn("pkill -USR1 -x bravos-wm"); }, true});
    menu->show(items, x, y, false);
}

void Wm::open_window_menu(Client *c, int x, int y) {
    if (!c) return;
    std::vector<MenuEntry> items;
    items.push_back({c->maximized ? g_i18n.tr("menu_unmaximize")
                                  : g_i18n.tr("menu_maximize"),
                     "", [this, c]() { toggle_maximize(c); }, true});
    items.push_back({g_i18n.tr("menu_minimize"), "",
                     [this, c]() { iconify(c); }, true});
    items.push_back({c->fullscreen ? g_i18n.tr("menu_exit_fullscreen")
                                   : g_i18n.tr("menu_fullscreen"),
                     "", [this, c]() { set_fullscreen(c, !c->fullscreen); }, true});
    items.push_back({c->above ? g_i18n.tr("menu_unpin")
                              : g_i18n.tr("menu_always_top"),
                     "", [this, c]() { toggle_above(c); }, true});
    items.push_back({"", "", nullptr, false});
    if (num_desktops > 1) {
        for (int d = 0; d < num_desktops; d++) {
            int dd = d;
            items.push_back({fmtf(g_i18n.tr("menu_move_ws").c_str(), d + 1), "",
                             [this, c, dd]() { move_to_desktop(c, dd); }, true});
        }
        items.push_back({"", "", nullptr, false});
    }
    items.push_back({g_i18n.tr("menu_close"), "", [this, c]() { close(c); }, true});
    menu->show(items, x, y, true);
}

void Wm::open_power_menu(int x, int y) {
    if (bar) {
        bar->open_power_menu(x, y);
        return;
    }
    std::vector<MenuEntry> items;
    items.push_back({g_i18n.tr("power_suspend"), "",
                     [this]() { spawn("bravos-power suspend"); }, true});
    items.push_back({g_i18n.tr("power_restart"), "",
                     [this]() { spawn("bravos-power reboot"); }, true});
    items.push_back({g_i18n.tr("power_shutdown"), "",
                     [this]() { spawn("bravos-power poweroff"); }, true});
    menu->show(items, x, y, false);
}

void Wm::draw_client(Client *c) {
    if (!c || c->frame == None) return;
    client_draw_decorations(c);
}

void Wm::tick_animations() {
    double now = now_s();
    for (Client *c : clients)
        if (c->animating) client_anim_step(c, now);
}

void Wm::cycle_keyboard_layout() { spawn(KB_SWITCH_CMD); }

void Wm::session_bar_tick() {
    if (bar) bar->tick();
}

// helpers used from client.cpp ----------------------------------------------------

void set_net_wm_state(Wm *wm, Client *c) {
    std::vector<Atom> states;
    if (c->fullscreen) states.push_back(wm->atoms.net_wm_state_fullscreen);
    if (c->maximized) {
        states.push_back(wm->atoms.net_wm_state_maximized_vert);
        states.push_back(wm->atoms.net_wm_state_maximized_horz);
    }
    if (c->above) states.push_back(wm->atoms.net_wm_state_above);
    XChangeProperty(wm->dpy, c->win, wm->atoms.net_wm_state, XA_ATOM, 32,
                    PropModeReplace, (unsigned char *)states.data(),
                    (int)states.size());
}

void set_net_allowed_actions(Wm *wm, Client *c) {
    std::vector<Atom> acts;
    acts.push_back(wm->atoms.net_wm_action_move);
    acts.push_back(wm->atoms.net_wm_action_resize);
    acts.push_back(wm->atoms.net_wm_action_minimize);
    acts.push_back(wm->atoms.net_wm_action_maximize_horz);
    acts.push_back(wm->atoms.net_wm_action_maximize_vert);
    acts.push_back(wm->atoms.net_wm_action_fullscreen);
    acts.push_back(wm->atoms.net_wm_action_close);
    acts.push_back(wm->atoms.net_wm_action_change_desktop);
    acts.push_back(wm->atoms.net_wm_action_stick);
    XChangeProperty(wm->dpy, c->win, wm->atoms.net_wm_allowed_actions, XA_ATOM,
                    32, PropModeReplace, (unsigned char *)acts.data(),
                    (int)acts.size());
}

}  // namespace bwm
