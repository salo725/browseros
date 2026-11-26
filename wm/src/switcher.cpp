// BraveWM - switcher.cpp: Alt+Tab application switcher.
#include "switcher.hpp"
#include "menu.hpp"

namespace bwm {

static const int SW_ICON = 40;
static const int SW_TILE = 96;
static const int SW_MAX_TILES = 10;

Switcher::Switcher(Wm *w) : wm(w) {}
Switcher::~Switcher() { cancel(); }

void Switcher::layout() {
    // list: focusable, visible-or-iconified windows on this desktop, MRU first
    list.clear();
    for (Client *c : wm->focus_mru) {
        if (c->never_focus) continue;
        if (c->desktop != (int)0xFFFFFFFF && c->desktop != wm->current_desktop) continue;
        list.push_back(c);
    }
    // append windows not in MRU (e.g. never focused)
    for (Client *c : wm->clients) {
        if (c->never_focus) continue;
        if (c->desktop != (int)0xFFFFFFFF && c->desktop != wm->current_desktop) continue;
        if (std::find(list.begin(), list.end(), c) == list.end()) list.push_back(c);
    }
    if (list.size() > SW_MAX_TILES) list.resize(SW_MAX_TILES);
}

void Switcher::begin(bool backward) {
    layout();
    if (list.empty()) return;
    if (list.size() == 1) {
        wm->uniconify(list[0]);
        wm->focus(list[0]);
        return;
    }
    cancel();

    int tiles = (int)list.size();
    w = tiles * SW_TILE + 2 * 18;
    h = SW_ICON + 58;
    int sw = DisplayWidth(wm->dpy, wm->screen);
    int sh = DisplayHeight(wm->dpy, wm->screen);
    x = (sw - w) / 2;
    y = (int)(sh * 0.38) - h / 2;

    XSetWindowAttributes wa = {};
    wa.override_redirect = True;
    wa.background_pixel = BlackPixel(wm->dpy, wm->screen);
    wa.event_mask = ExposureMask;
    win = XCreateWindow(wm->dpy, wm->root, x, y, w, h, 0, CopyFromParent,
                        InputOutput, CopyFromParent,
                        CWOverrideRedirect | CWBackPixel | CWEventMask, &wa);
    if (wm->have_shape) {
        Pixmap pm = XCreatePixmap(wm->dpy, win, w, h, 1);
        cairo_surface_t *s = cairo_xlib_surface_create_for_bitmap(
            wm->dpy, pm, DefaultScreenOfDisplay(wm->dpy), w, h);
        cairo_t *c = cairo_create(s);
        cairo_set_source_rgb(c, 0, 0, 0);
        cairo_paint(c);
        cairo_set_source_rgb(c, 1, 1, 1);
        round_rect_path(c, 0, 0, w, h, 16);
        cairo_fill(c);
        cairo_destroy(c);
        cairo_surface_destroy(s);
        XShapeCombineMask(wm->dpy, win, ShapeBounding, 0, 0, pm, ShapeSet);
        XFreePixmap(wm->dpy, pm);
    }
    selected = backward ? 1 : 1;  // first Tab press skips to the next window
    started = now_s();
    XMapRaised(wm->dpy, win);
    // grab the keyboard so we can see the Alt release
    XGrabKeyboard(wm->dpy, wm->root, True, GrabModeAsync, GrabModeAsync,
                  CurrentTime);
    draw();
}

void Switcher::step(bool backward) {
    if (!active() || list.empty()) return;
    int n = (int)list.size();
    int dir = backward ? -1 : 1;
    for (int i = 1; i <= n; i++) {
        int idx = (selected + dir * i + 2 * n) % n;
        selected = idx;
        break;
    }
    draw();
}

void Switcher::commit() {
    if (!active()) return;
    Client *pick = list.empty() ? nullptr : list[selected];
    Window w = win;
    win = None;
    XUngrabKeyboard(wm->dpy, CurrentTime);
    XDestroyWindow(wm->dpy, w);
    if (pick) {
        wm->uniconify(pick);
        wm->focus(pick);
    }
}

void Switcher::cancel() {
    if (!active()) return;
    Window w = win;
    win = None;
    XUngrabKeyboard(wm->dpy, CurrentTime);
    XDestroyWindow(wm->dpy, w);
}

void Switcher::handle_key(XKeyEvent &e) {
    if (e.type != KeyPress) return;
    KeySym ks = XKeycodeToKeysym(wm->dpy, e.keycode, 0);
    if (ks == XK_Tab || ks == XK_ISO_Left_Tab) step(e.state & ShiftMask);
    else if (ks == XK_Escape) cancel();
}

void Switcher::draw() {
    if (win == None) return;
    cairo_surface_t *surf =
        cairo_xlib_surface_create(wm->dpy, win, wm->visual, w, h);
    cairo_t *cr = cairo_create(surf);

    cairo_set_operator(cr, CAIRO_OPERATOR_SOURCE);
    cairo_set_source_rgba(cr, 0.075, 0.059, 0.164, 0.94);
    cairo_paint(cr);
    cairo_set_operator(cr, CAIRO_OPERATOR_OVER);
    cairo_set_source_rgba(cr, 1, 1, 1, 0.10);
    cairo_set_line_width(cr, 1.2);
    round_rect_path(cr, 0.5, 0.5, w - 1, h - 1, 16);
    cairo_stroke(cr);

    int tiles = (int)list.size();
    for (int i = 0; i < tiles; i++) {
        Client *c = list[i];
        double cx = 18 + i * SW_TILE + SW_TILE / 2.0;
        double cy = 26 + SW_ICON / 2.0;
        if (i == selected) {
            // orange glow ring
            cairo_set_source_rgba(cr, 0.94, 0.31, 0.14, 0.9);  // #f04e23
            cairo_set_line_width(cr, 2);
            cairo_arc(cr, cx, cy, SW_ICON / 2.0 + 8, 0, M_PI * 2);
            cairo_stroke(cr);
            cairo_set_line_width(cr, 1);
        }
        // tile background
        cairo_set_source_rgba(cr, 1, 1, 1, i == selected ? 0.14 : 0.06);
        cairo_arc(cr, cx, cy, SW_ICON / 2.0 + 3, 0, M_PI * 2);
        cairo_fill(cr);
        // icon
        if (c->icon_w > 0 && c->icon_data.size() >= (size_t)(c->icon_w * c->icon_h)) {
            double sc = SW_ICON / (double)c->icon_w;
            cairo_surface_t *isurf = cairo_image_surface_create_for_data(
                (unsigned char *)c->icon_data.data(), CAIRO_FORMAT_ARGB32,
                c->icon_w, c->icon_h, c->icon_w * 4);
            cairo_save(cr);
            cairo_translate(cr, cx, cy);
            cairo_scale(cr, sc, sc);
            cairo_set_source_surface(cr, isurf, -c->icon_w / 2.0,
                                     -c->icon_h / 2.0);
            cairo_paint(cr);
            cairo_restore(cr);
            cairo_surface_destroy(isurf);
        } else {
            cairo_set_source_rgb(cr, 0.94, 0.31, 0.14);
            cairo_arc(cr, cx, cy, SW_ICON / 2.5, 0, M_PI * 2);
            cairo_fill(cr);
        }
        // title (2 lines, clipped)
        if (i == selected) {
            std::string t = c->title.empty() ? "Window" : c->title;
            if (t.size() > 26) t = t.substr(0, 25) + "...";
            TextMetrics tm = text_measure(cr, t, wm->cfg.font, 12.5, true);
            text_draw(cr, t, w / 2.0, h - 20 - tm.height, wm->cfg.font, 12.5,
                      true, Color(1, 1, 1, 1), 0);
        }
    }
    cairo_destroy(cr);
    cairo_surface_destroy(surf);
}

}  // namespace bwm
