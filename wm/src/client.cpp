// BraveWM - client.cpp: frame creation, decorations (glass titlebar, custom
// buttons), rounded-corner shaping, icon parsing, smooth animations.
#include "client.hpp"
#include "wm.hpp"

namespace bwm {

// geometry helpers ------------------------------------------------------------

const Config &Client::cf() const { return wm->cfg; }

Rect Client::frame_rect() const {
    if (!wants_decor) return geom;
    return Rect(geom.x - cf().border_w, geom.y - cf().titlebar_h,
                geom.w + 2 * cf().border_w,
                geom.h + cf().titlebar_h + cf().border_w);
}

static int button_diameter(const Config &cfg) {
    return std::max(10, cfg.titlebar_h / 2 - 2);
}

Rect Client::button_rect(int which) const {
    const Config &cfg = cf();
    int d = button_diameter(cfg);
    int spacing = 6;
    int fw = frame_rect().w;
    int x;
    // order right -> left: close, max, min
    if (which == BTN_CLOSE)
        x = fw - cfg.border_w - d - 4;
    else if (which == BTN_MAX)
        x = fw - cfg.border_w - d - 4 - d - spacing;
    else
        x = fw - cfg.border_w - d - 4 - 2 * (d + spacing);
    int y = (cfg.titlebar_h - d) / 2;
    return Rect(x, y, d, d);
}

BtnHit Client::hit_button(int fx, int fy) const {
    if (!wants_decor) return BTN_NONE;
    if (fy > titlebar_rect().h) return BTN_NONE;
    for (int b = 1; b <= 3; b++) {
        Rect r = button_rect(b);
        if (fx >= r.x - 3 && fx <= r.x + r.w + 3 && fy >= r.y - 3 &&
            fy <= r.y + r.h + 3)
            return (BtnHit)b;
    }
    return BTN_NONE;
}

Rect Client::titlebar_rect() const {
    return Rect(0, 0, frame_rect().w, cf().titlebar_h);  // frame-relative
}

int Client::hit_edge(int fx, int fy) const {
    Rect fr(0, 0, frame_rect().w, frame_rect().h);
    int e = 0;
    int grab = 6;
    if (fx < fr.x + grab) e |= 1;          // left
    if (fx > fr.x + fr.w - grab) e |= 2;   // right
    if (fy < fr.y + grab) e |= 4;          // top
    if (fy > fr.y + fr.h - grab) e |= 8;   // bottom
    return e;
}

// frame create / destroy ------------------------------------------------------

void client_frame_create(Client *c) {
    Wm *wm = c->wm;
    Display *dpy = wm->dpy;
    Rect fr = c->frame_rect();
    XSetWindowAttributes wa;
    wa.background_pixel = BlackPixel(dpy, wm->screen);
    wa.border_pixel = 0;
    wa.event_mask =
        ExposureMask | ButtonPressMask | ButtonReleaseMask | ButtonMotionMask |
        PointerMotionMask | EnterWindowMask | LeaveWindowMask |
        StructureNotifyMask;
    wa.override_redirect = False;
    c->frame = XCreateWindow(dpy, wm->root, fr.x, fr.y, std::max(1, fr.w),
                             std::max(1, fr.h), 0, CopyFromParent, InputOutput,
                             CopyFromParent,
                             CWBackPixel | CWEventMask | CWOverrideRedirect, &wa);
    XDefineCursor(dpy, c->frame, wm->cursor_normal);
    XReparentWindow(dpy, c->win, c->frame, c->wm->cfg.border_w,
                    c->wm->cfg.titlebar_h);
    XAddToSaveSet(dpy, c->win);  // rescue clients if we crash
    client_apply_shape(c);
    client_sync_frame_extents(c);
}

void client_frame_destroy(Client *c) {
    Wm *wm = c->wm;
    if (c->frame == None) return;
    if (c->win != None) {
        XReparentWindow(wm->dpy, c->win, wm->root, c->geom.x, c->geom.y);
        XRemoveFromSaveSet(wm->dpy, c->win);
    }
    XDestroyWindow(wm->dpy, c->frame);
    c->frame = None;
}

// Place the frame exactly around the current geometry and put the client
// window at its final position inside the frame.
void client_frame_configure(Client *c) {
    Wm *wm = c->wm;
    const Config &cfg = wm->cfg;
    if (!c->wants_decor) {
        if (c->win != None)
            XMoveResizeWindow(wm->dpy, c->win, c->geom.x, c->geom.y,
                              std::max(1, c->geom.w), std::max(1, c->geom.h));
        return;
    }
    Rect fr = c->frame_rect();
    if (c->frame != None)
        XMoveResizeWindow(wm->dpy, c->frame, fr.x, fr.y, std::max(1, fr.w),
                          std::max(1, fr.h));
    // the client sits INSIDE the frame at (border, titlebar), frame-relative
    XMoveResizeWindow(wm->dpy, c->win, cfg.border_w, cfg.titlebar_h,
                      std::max(1, c->geom.w), std::max(1, c->geom.h));
    client_apply_shape(c);
    client_sync_frame_extents(c);
}

void client_sync_frame_extents(Client *c) {
    Wm *wm = c->wm;
    if (c->frame == None) return;
    const Config &cfg = wm->cfg;
    long extents[4] = {0, 0, 0, 0};
    if (c->wants_decor) {
        extents[0] = cfg.border_w;              // left
        extents[1] = cfg.border_w;              // right
        extents[2] = cfg.titlebar_h;            // top
        extents[3] = cfg.border_w;              // bottom
    }
    XChangeProperty(wm->dpy, c->win, wm->atoms.net_frame_extents, XA_CARDINAL,
                    32, PropModeReplace, (unsigned char *)extents, 4);
}

// shaping (rounded top corners) ------------------------------------------------

void client_apply_shape(Client *c) {
    Wm *wm = c->wm;
    if (!wm->have_shape || c->frame == None) return;
    const Config &cfg = wm->cfg;
    int r = c->wants_decor ? cfg.corner_r : 0;
    Rect fr = c->frame_rect();
    if (r <= 0) {
        XShapeCombineMask(wm->dpy, c->frame, ShapeBounding, 0, 0, None, ShapeSet);
        return;
    }
    Pixmap pm = XCreatePixmap(wm->dpy, c->frame, std::max(1, fr.w),
                             std::max(1, fr.h), 1);
    cairo_surface_t *surf = cairo_xlib_surface_create_for_bitmap(
        wm->dpy, pm, DefaultScreenOfDisplay(wm->dpy), std::max(1, fr.w),
         std::max(1, fr.h));
    cairo_t *cr = cairo_create(surf);
    cairo_set_source_rgb(cr, 0, 0, 0);
    cairo_paint(cr);
    cairo_set_source_rgb(cr, 1, 1, 1);
    round_top_path(cr, 0, 0, fr.w, fr.h, r);
    cairo_fill(cr);
    cairo_destroy(cr);
    cairo_surface_destroy(surf);
    XShapeCombineMask(wm->dpy, c->frame, ShapeBounding, 0, 0, pm, ShapeSet);
    XFreePixmap(wm->dpy, pm);
}

// icon parsing -----------------------------------------------------------------

void client_read_icon(Client *c) {
    Wm *wm = c->wm;
    Atom type;
    int fmt;
    unsigned long nitems, bytes_after;
    unsigned char *data = nullptr;
    c->icon_w = c->icon_h = 0;
    c->icon_data.clear();
    if (XGetWindowProperty(wm->dpy, c->win, wm->atoms.net_wm_icon, 0, 1 << 18,
                           False, XA_CARDINAL, &type, &fmt, &nitems,
                           &bytes_after, &data) != Success ||
        !data || nitems < 3) {
        if (data) XFree(data);
        return;
    }
    unsigned long *vals = (unsigned long *)data;
    unsigned long pos = 0;
    long best_d = 1 << 20;
    while (pos + 2 <= nitems) {
        int w = (int)vals[pos], h = (int)vals[pos + 1];
        if (w <= 0 || h <= 0 || w > 256 || h > 256 ||
            pos + 2 + (unsigned long)(w * h) > nitems)
            break;
        long d = std::abs(w - 20);
        if (d < best_d) {
            best_d = d;
            c->icon_w = w;
            c->icon_h = h;
            c->icon_data.assign(vals + pos + 2, vals + pos + 2 + w * h);
        }
        pos += 2 + (unsigned long)(w * h);
    }
    XFree(data);
}

// WM_STATE ----------------------------------------------------------------------

void client_set_wm_state(Client *c, int state) {
    Wm *wm = c->wm;
    unsigned long data[2] = {(unsigned long)state, None};
    XChangeProperty(wm->dpy, c->win, wm->atoms.wm_state, wm->atoms.wm_state, 32,
                    PropModeReplace, (unsigned char *)data, 2);
}

// decorations ---------------------------------------------------------------------

static void draw_lion_glyph(cairo_t *cr, double cx, double cy, double r,
                            Color col) {
    // stylized geometric lion head (mane spikes + face)
    cairo_save(cr);
    cairo_translate(cr, cx, cy);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);
    col.apply(cr);
    for (int i = 0; i < 12; i++) {
        double a0 = (i - 0.5) * (M_PI * 2 / 12);
        double a1 = (i + 0.5) * (M_PI * 2 / 12);
        double am = (a0 + a1) / 2;
        double r0 = r * 0.62;
        double r1 = (i % 2 == 0) ? r : r * 0.82;
        double mid = r * 0.80;
        cairo_move_to(cr, r0 * cos(a0), r0 * sin(a0));
        cairo_line_to(cr, (i % 2 == 0 ? r1 : mid) * cos(am),
                      (i % 2 == 0 ? r1 : mid) * sin(am));
        cairo_line_to(cr, r0 * cos(a1), r0 * sin(a1));
        cairo_close_path(cr);
        cairo_fill(cr);
    }
    cairo_arc(cr, 0, 0, r * 0.48, 0, M_PI * 2);
    cairo_fill(cr);
    cairo_set_source_rgb(cr, 1, 1, 1);
    cairo_arc(cr, -r * 0.17, -r * 0.06, r * 0.07, 0, M_PI * 2);
    cairo_fill(cr);
    cairo_arc(cr, r * 0.17, -r * 0.06, r * 0.07, 0, M_PI * 2);
    cairo_fill(cr);
    cairo_move_to(cr, -r * 0.09, r * 0.13);
    cairo_line_to(cr, r * 0.09, r * 0.13);
    cairo_line_to(cr, 0, r * 0.22);
    cairo_close_path(cr);
    cairo_fill(cr);
    cairo_restore(cr);
}

static void draw_glyph(cairo_t *cr, int which, double cx, double cy, double r,
                       double lw) {
    // white glyph on the circular button
    cairo_set_source_rgb(cr, 1, 1, 1);
    cairo_set_line_width(cr, lw);
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    double s = r * 0.5;
    if (which == BTN_CLOSE) {  // x
        cairo_move_to(cr, cx - s, cy - s);
        cairo_line_to(cr, cx + s, cy + s);
        cairo_move_to(cr, cx + s, cy - s);
        cairo_line_to(cr, cx - s, cy + s);
        cairo_stroke(cr);
    } else if (which == BTN_MAX) {  // square outline
        cairo_rectangle(cr, cx - s, cy - s, 2 * s, 2 * s);
        cairo_stroke(cr);
    } else {  // minimize: dash
        cairo_move_to(cr, cx - s, cy + s * 0.7);
        cairo_line_to(cr, cx + s, cy + s * 0.7);
        cairo_stroke(cr);
    }
}

void client_draw_decorations(Client *c) {
    Wm *wm = c->wm;
    if (c->frame == None || !c->wants_decor) return;
    const Config &cfg = wm->cfg;
    Rect fr = c->frame_rect();
    int W = std::max(1, fr.w), H = std::max(1, fr.h);

    cairo_surface_t *surf =
        cairo_xlib_surface_create(wm->dpy, c->frame, wm->visual, W, H);
    cairo_t *cr = cairo_create(surf);

    bool focused = (wm->focused_client == c);

    // ---- titlebar: dark cosmic glass ----
    cairo_set_operator(cr, CAIRO_OPERATOR_SOURCE);
    (focused ? cfg.c_bg_focused : cfg.c_bg_unfocused).apply(cr);
    cairo_paint(cr);
    cairo_set_operator(cr, CAIRO_OPERATOR_OVER);

    // subtle top highlight
    cairo_set_source_rgba(cr, 1, 1, 1, focused ? 0.10 : 0.05);
    cairo_move_to(cr, cfg.border_w + cfg.corner_r, cfg.border_w + 1.5);
    cairo_line_to(cr, W - cfg.border_w - cfg.corner_r, cfg.border_w + 1.5);
    cairo_set_line_width(cr, 1);
    cairo_stroke(cr);

    // accent underline when focused
    if (focused) {
        cairo_set_source_rgba(cr, cfg.c_accent2.r, cfg.c_accent2.g,
                              cfg.c_accent2.b, 0.55);
        cairo_move_to(cr, cfg.border_w + cfg.corner_r, cfg.titlebar_h - 1);
        cairo_line_to(cr, W - cfg.border_w - cfg.corner_r, cfg.titlebar_h - 1);
        cairo_set_line_width(cr, 1.4);
        cairo_stroke(cr);
    }
    // bottom hairline
    cairo_set_source_rgba(cr, 1, 1, 1, 0.08);
    cairo_move_to(cr, cfg.border_w, H - cfg.border_w - 0.5);
    cairo_line_to(cr, W - cfg.border_w, H - cfg.border_w - 0.5);
    cairo_stroke(cr);

    // ---- title + icon ----
    double font_px = std::max(10.0, cfg.titlebar_h * 0.38);
    double tx = cfg.border_w + 8;
    double ty = cfg.titlebar_h / 2.0;
    double icon_r = cfg.titlebar_h * 0.22;
    if (c->icon_w > 0 && c->icon_data.size() >= (size_t)(c->icon_w * c->icon_h)) {
        double scale = (2.0 * icon_r) / c->icon_w;
        cairo_surface_t *isurf = cairo_image_surface_create_for_data(
            (unsigned char *)c->icon_data.data(), CAIRO_FORMAT_ARGB32,
            c->icon_w, c->icon_h, c->icon_w * 4);
        cairo_save(cr);
        cairo_translate(cr, tx + icon_r, ty);
        cairo_scale(cr, scale, scale);
        cairo_set_source_surface(cr, isurf, -c->icon_w / 2.0, -c->icon_h / 2.0);
        cairo_paint(cr);
        cairo_restore(cr);
        cairo_surface_destroy(isurf);
        tx += 2 * icon_r + 7;
    } else {
        draw_lion_glyph(cr, tx + icon_r, ty, icon_r * 0.95,
                        focused ? cfg.c_accent : cfg.c_text_unfocused);
        tx += 2 * icon_r + 7;
    }
    // ellipsized title (Pango: proper shaping for any language)
    std::string title = c->title.empty() ? "Untitled" : c->title;
    Rect b1 = c->button_rect(BTN_MIN);
    double avail = b1.x - tx - 8;
    std::string shown = title;
    TextMetrics tm = text_measure(cr, shown, cfg.font, font_px, true);
    if (avail < 24) {
        shown = "";
    } else if (tm.width > avail) {
        size_t lo = 0, hi = shown.size();
        while (lo < hi) {
            size_t mid = (lo + hi) / 2;
            std::string t = shown.substr(0, mid) + "...";
            TextMetrics tm2 = text_measure(cr, t, cfg.font, font_px, true);
            if (tm2.width > avail) hi = mid;
            else lo = mid + 1;
        }
        shown = shown.substr(0, std::max((size_t)0, hi)) + "...";
    }
    tm = text_measure(cr, shown, cfg.font, font_px, true);
    text_draw(cr, shown, tx, ty - tm.height / 2.0, cfg.font, font_px, true,
              focused ? cfg.c_text_focused : cfg.c_text_unfocused);

    // ---- window buttons: circular glass ----
    struct BtnDef { int id; Color base; };
    BtnDef btns[3] = {
        {BTN_CLOSE, cfg.c_accent},
        {BTN_MAX, cfg.c_accent2},
        {BTN_MIN, cfg.c_btn_min},
    };
    for (auto &bd : btns) {
        Rect r = c->button_rect(bd.id);
        double cx = r.x + r.w / 2.0, cy = r.y + r.h / 2.0, rad = r.w / 2.0;
        bool hov = (c->hover_btn == bd.id);
        Color col = hov ? bd.base.brighten(1.35) : bd.base;
        cairo_set_line_width(cr, 1);
        if (hov) {
            cairo_set_source_rgba(cr, bd.base.r, bd.base.g, bd.base.b, 0.4);
            cairo_arc(cr, cx, cy, rad + 3.0, 0, M_PI * 2);
            cairo_stroke(cr);
        }
        col.apply(cr);
        cairo_arc(cr, cx, cy, rad, 0, M_PI * 2);
        cairo_fill(cr);
        draw_glyph(cr, bd.id, cx, cy, rad, 1.6);
    }

    cairo_destroy(cr);
    cairo_surface_destroy(surf);
}

// animations ---------------------------------------------------------------------

// Animate (or instantly apply) the frame to frame-rect `to`.
// `closing` = minimize-style shrink toward the dock, then unmap.
void client_anim_start(Client *c, const Rect &to, bool closing) {
    Wm *wm = c->wm;
    if (c->frame == None || !wm->cfg.animations) {
        c->animating = false;
        return;
    }
    c->anim_from = c->frame_rect();
    c->anim_to = to;
    c->anim_t0 = now_s();
    c->animating = true;
    c->anim_phase = closing ? 1 : 0;
}

void client_anim_step(Client *c, double now) {
    if (!c->animating) return;
    const Config &cfg = c->wm->cfg;
    double total = (cfg.anim_steps * cfg.anim_step_ms) / 1000.0;
    double t = total <= 0 ? 1.0 : (now - c->anim_t0) / total;
    if (t >= 1.0) t = 1.0;
    double e = 1.0 - pow(1.0 - t, 3.0);  // ease-out cubic
    Rect a = c->anim_from, b = c->anim_to;
    Rect cur((int)(a.x + (b.x - a.x) * e), (int)(a.y + (b.y - a.y) * e),
             (int)(a.w + (b.w - a.w) * e), (int)(a.h + (b.h - a.h) * e));
    if (c->frame != None)
        XMoveResizeWindow(c->wm->dpy, c->frame, cur.x, cur.y, std::max(1, cur.w),
                          std::max(1, cur.h));
    if (t >= 1.0) {
        c->animating = false;
        Wm *wm = c->wm;
        if (c->anim_phase == 1) {
            // minimize finished: unmap for real
            c->iconic_by_wm = true;
            XUnmapWindow(wm->dpy, c->win);
            XUnmapWindow(wm->dpy, c->frame);
            c->iconic_by_wm = false;
            c->visible = false;
            c->iconified = true;
            client_set_wm_state(c, IconicState);
            wm->update_focus_after(c);
            wm->sync_net_client_list();
        } else {
            // open finished: snap everything to final geometry
            client_frame_configure(c);
        }
    }
}

}  // namespace bwm
