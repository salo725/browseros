// BraveWM - menu.cpp: custom popup menu implementation.
#include "menu.hpp"

namespace bwm {

static const int MENU_PAD = 10;
static const int MENU_ITEM_H = 30;
static const int MENU_SEP_H = 10;
static const int MENU_MAX_W = 280;

Menu::Menu(Wm *w) : wm(w) {}
Menu::~Menu() { hide(); }

int Menu::item_at(int py) const {
    // returns index whose rect contains frame-relative y
    for (size_t i = 0; i < items.size(); i++) {
        Rect r = item_rect((int)i);
        if (!items[i].callback) {  // separator
            continue;
        }
        if (py >= r.y && py < r.y + r.h) return (int)i;
    }
    return -1;
}

Rect Menu::item_rect(int i) const {
    int y = MENU_PAD;
    for (int k = 0; k < i; k++) {
        y += items[k].callback ? MENU_ITEM_H : MENU_SEP_H;
    }
    return Rect(0, y, w, items[i].callback ? MENU_ITEM_H : MENU_SEP_H);
}

void Menu::show(std::vector<MenuEntry> its, int px, int py, bool near_pointer) {
    if (win != None) hide();
    items = std::move(its);
    if (items.empty()) return;

    // measure
    cairo_surface_t *tmp = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, 8, 8);
    cairo_t *cr = cairo_create(tmp);
    int max_w = 120;
    for (auto &it : items) {
        if (!it.callback) continue;
        TextMetrics tm = text_measure(cr, it.label, wm->cfg.font, 13.5, false);
        max_w = std::max(max_w, (int)tm.width + 4);
    }
    cairo_destroy(cr);
    cairo_surface_destroy(tmp);
    w = std::min(MENU_MAX_W, max_w + 2 * MENU_PAD + 26);
    h = MENU_PAD;
    for (auto &it : items) h += it.callback ? MENU_ITEM_H : MENU_SEP_H;
    h += MENU_PAD;

    // position (keep on screen, near pointer for window menus)
    int sw = DisplayWidth(wm->dpy, wm->screen);
    int sh = DisplayHeight(wm->dpy, wm->screen);
    int rx = px, ry = py;
    if (!near_pointer) {
        // centered horizontally on the click point
        rx = px - w / 2;
        ry = py + 4;
    }
    if (rx + w > sw - 8) rx = sw - w - 8;
    if (ry + h > sh - 8) ry = sh - h - 8;
    if (rx < 8) rx = 8;
    if (ry < 8) ry = 8;
    x = rx;
    y = ry;

    XSetWindowAttributes wa = {};
    wa.override_redirect = True;
    wa.background_pixel = BlackPixel(wm->dpy, wm->screen);
    wa.event_mask = ExposureMask | ButtonPressMask | ButtonReleaseMask |
                    PointerMotionMask | LeaveWindowMask | KeyPressMask |
                    StructureNotifyMask;
    win = XCreateWindow(wm->dpy, wm->root, x, y, w, h, 0, CopyFromParent,
                        InputOutput, CopyFromParent,
                        CWOverrideRedirect | CWBackPixel | CWEventMask, &wa);
    // rounded shape
    if (wm->have_shape) {
        Pixmap pm = XCreatePixmap(wm->dpy, win, w, h, 1);
        cairo_surface_t *s = cairo_xlib_surface_create_for_bitmap(
            wm->dpy, pm, DefaultScreenOfDisplay(wm->dpy), w, h);
        cairo_t *c = cairo_create(s);
        cairo_set_source_rgb(c, 0, 0, 0);
        cairo_paint(c);
        cairo_set_source_rgb(c, 1, 1, 1);
        round_rect_path(c, 0, 0, w, h, 12);
        cairo_fill(c);
        cairo_destroy(c);
        cairo_surface_destroy(s);
        XShapeCombineMask(wm->dpy, win, ShapeBounding, 0, 0, pm, ShapeSet);
        XFreePixmap(wm->dpy, pm);
    }
    hover = -1;
    XMapRaised(wm->dpy, win);
    // grab pointer + keyboard so the menu behaves like a real menu
    XGrabPointer(wm->dpy, wm->root, True,
                 ButtonPressMask | ButtonReleaseMask | PointerMotionMask,
                 GrabModeAsync, GrabModeAsync, wm->root, wm->cursor_normal,
                 CurrentTime);
    XGrabKeyboard(wm->dpy, wm->root, True, GrabModeAsync, GrabModeAsync,
                  CurrentTime);
    XFlush(wm->dpy);
}

void Menu::hide() {
    if (win == None) return;
    XUngrabPointer(wm->dpy, CurrentTime);
    XUngrabKeyboard(wm->dpy, CurrentTime);
    XDestroyWindow(wm->dpy, win);
    win = None;
    items.clear();
    hover = -1;
}

void Menu::handle_button(XButtonEvent &e) {
    if (win == None) return;
    if (e.type == ButtonPress) {
        int lx = e.x, ly = e.y;
        if (lx < 0 || lx > w || ly < 0 || ly > h) {
            hide();
            return;
        }
        int idx = item_at(ly);
        if (idx < 0 || !items[idx].callback || !items[idx].enabled) {
            hide();
            return;
        }
        auto cb = items[idx].callback;
        hide();
        cb();
    }
}

void Menu::handle_motion(XMotionEvent &e) {
    if (win == None) return;
    int idx = (e.x >= 0 && e.x <= w && e.y >= 0 && e.y <= h) ? item_at(e.y) : -1;
    if (idx != hover) {
        hover = idx;
        draw();
    }
}

void Menu::handle_key(XKeyEvent &e) {
    if (win == None) return;
    KeySym ks = XKeycodeToKeysym(wm->dpy, e.keycode, 0);
    if (ks == XK_Escape) {
        hide();
        return;
    }
    if (ks == XK_Up || ks == XK_Down) {
        int dir = (ks == XK_Down) ? 1 : -1;
        int start = hover < 0 ? (dir > 0 ? -1 : (int)items.size()) : hover;
        for (int i = 1; i <= (int)items.size(); i++) {
            int idx = (start + dir * i + (int)items.size() * 2) % (int)items.size();
            if (items[idx].callback) {
                hover = idx;
                draw();
                break;
            }
        }
        return;
    }
    if (ks == XK_Return || ks == XK_KP_Enter) {
        if (hover >= 0 && items[hover].callback) {
            auto cb = items[hover].callback;
            hide();
            cb();
        }
        return;
    }
    hide();
}

void Menu::handle_expose(XExposeEvent &) { draw(); }

void Menu::draw() {
    if (win == None) return;
    cairo_surface_t *surf =
        cairo_xlib_surface_create(wm->dpy, win, wm->visual, w, h);
    cairo_t *cr = cairo_create(surf);

    // glass background
    cairo_set_operator(cr, CAIRO_OPERATOR_SOURCE);
    cairo_set_source_rgba(cr, 0.075, 0.059, 0.164, 0.96);  // #14102a
    cairo_paint(cr);
    cairo_set_operator(cr, CAIRO_OPERATOR_OVER);
    // border
    cairo_set_source_rgba(cr, 1, 1, 1, 0.10);
    cairo_set_line_width(cr, 1.2);
    round_rect_path(cr, 0.5, 0.5, w - 1, h - 1, 12);
    cairo_stroke(cr);

    for (size_t i = 0; i < items.size(); i++) {
        Rect r = item_rect((int)i);
        if (!items[i].callback) {  // separator
            cairo_set_source_rgba(cr, 1, 1, 1, 0.08);
            cairo_move_to(cr, MENU_PAD, r.y + MENU_SEP_H / 2.0);
            cairo_line_to(cr, w - MENU_PAD, r.y + MENU_SEP_H / 2.0);
            cairo_stroke(cr);
            continue;
        }
        bool sel = ((int)i == hover) && items[i].enabled;
        if (sel) {  // pill highlight
            cairo_set_source_rgba(cr, 0.42, 0.24, 0.91, 0.85);  // #6c3ce9
            round_rect_path(cr, MENU_PAD - 4, r.y + 2, w - 2 * (MENU_PAD - 4),
                            r.h - 4, 7);
            cairo_fill(cr);
        }
        cairo_set_source_rgb(cr, 1, 1, 1);
        if (!items[i].enabled) cairo_set_source_rgb(cr, 0.6, 0.58, 0.72);
        // Pango-drawn label (Arabic/CJK shaping); vertically centered
        TextMetrics tm = text_measure(cr, items[i].label, wm->cfg.font, 13.5, false);
        text_draw(cr, items[i].label, MENU_PAD + 2,
                  r.y + r.h / 2.0 - tm.height / 2.0, wm->cfg.font, 13.5, false,
                  items[i].enabled ? Color(1, 1, 1, 1) : Color(0.6, 0.58, 0.72, 1));
    }

    cairo_destroy(cr);
    cairo_surface_destroy(surf);
}

}  // namespace bwm
