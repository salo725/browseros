// BraveWM - util.hpp: shared includes, geometry and color helpers.
#pragma once

#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/Xatom.h>
#include <X11/cursorfont.h>
#include <X11/extensions/shape.h>

#include <cairo/cairo-xlib.h>
#include <cairo/cairo.h>

#include <pango/pango.h>
#include <pango/pangocairo.h>

#include <cstdarg>
#include <algorithm>
#include <chrono>
#include <cmath>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <map>
#include <memory>
#include <string>
#include <vector>

namespace bwm {

struct Rect {
    int x = 0, y = 0, w = 0, h = 0;
    Rect() {}
    Rect(int x_, int y_, int w_, int h_) : x(x_), y(y_), w(w_), h(h_) {}
    bool contains(int px, int py) const {
        return px >= x && px < x + w && py >= y && py < y + h;
    }
    bool valid() const { return w > 0 && h > 0; }
};

struct Color {
    double r = 0, g = 0, b = 0, a = 1.0;
    Color() {}
    Color(double r_, double g_, double b_, double a_ = 1.0)
        : r(r_), g(g_), b(b_), a(a_) {}
    static Color from_hex(const std::string &hex, double alpha = 1.0) {
        std::string s = hex;
        if (!s.empty() && s[0] == '#') s = s.substr(1);
        if (s.size() != 6 && s.size() != 8) return Color(0, 0, 0, 1);
        auto two = [](const std::string &t, size_t off) {
            return std::stoul(t.substr(off, 2), nullptr, 16) / 255.0;
        };
        double a = s.size() == 8 ? two(s, 6) : alpha;
        return Color(two(s, 0), two(s, 2), two(s, 4), a);
    }
    Color brighten(double f) const {
        return Color(std::min(1.0, r * f), std::min(1.0, g * f),
                     std::min(1.0, b * f), a);
    }
    Color with_alpha(double alpha) const {
        return Color(r, g, b, alpha);
    }
    void apply(cairo_t *cr) const { cairo_set_source_rgba(cr, r, g, b, a); }
};

inline double now_s() {
    using namespace std::chrono;
    return duration_cast<duration<double>>(steady_clock::now().time_since_epoch())
        .count();
}

inline std::string fmtf(const char *f, ...) {
    char buf[4096];
    va_list ap;
    va_start(ap, f);
    vsnprintf(buf, sizeof(buf), f, ap);
    va_end(ap);
    return std::string(buf);
}

inline void log_msg(const char *tag, const char *f, ...) {
    va_list ap;
    va_start(ap, f);
    fprintf(stderr, "[%s] ", tag);
    vfprintf(stderr, f, ap);
    fprintf(stderr, "\n");
    va_end(ap);
    fflush(stderr);
}

// Rounded rectangle path (all corners).
void round_rect_path(cairo_t *cr, double x, double y, double w, double h,
                     double r);

// Top-only rounded path (titlebar style).
void round_top_path(cairo_t *cr, double x, double y, double w, double h,
                     double r);

// ---- Pango text helpers (proper shaping for Arabic/CJK, font fallback) ------

struct TextMetrics {
    double width = 0, height = 0;  // inked size in px
};

// Measure a UTF-8 string with the given family/size/weight.
TextMetrics text_measure(cairo_t *cr, const std::string &utf8,
                         const std::string &family, double px, bool bold);

// Draw UTF-8 text. (x, y) = top edge of the ink box; horizontal alignment:
//   align -1: x is left edge; 0: x is center; 1: x is right edge.
void text_draw(cairo_t *cr, const std::string &utf8, double x, double y,
              const std::string &family, double px, bool bold,
              const Color &c, int align = -1);

}  // namespace bwm

