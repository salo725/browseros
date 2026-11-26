// BraveWM - util.cpp: cairo path helpers.
#include "util.hpp"

namespace bwm {

void round_rect_path(cairo_t *cr, double x, double y, double w, double h,
                     double r) {
    if (r > w / 2.0) r = w / 2.0;
    if (r > h / 2.0) r = h / 2.0;
    cairo_new_sub_path(cr);
    cairo_arc(cr, x + w - r, y + r, r, -M_PI / 2, 0);
    cairo_arc(cr, x + w - r, y + h - r, r, 0, M_PI / 2);
    cairo_arc(cr, x + r, y + h - r, r, M_PI / 2, M_PI);
    cairo_arc(cr, x + r, y + r, r, M_PI, 3 * M_PI / 2);
    cairo_close_path(cr);
}

void round_top_path(cairo_t *cr, double x, double y, double w, double h,
                    double r) {
    if (r > w / 2.0) r = w / 2.0;
    cairo_new_sub_path(cr);
    cairo_move_to(cr, x, y + h);
    cairo_line_to(cr, x, y + r);
    cairo_arc(cr, x + r, y + r, r, M_PI, 3 * M_PI / 2);
    cairo_arc(cr, x + w - r, y + r, r, -M_PI / 2, 0);
    cairo_line_to(cr, x + w, y + h);
    cairo_close_path(cr);
}

// ---- Pango text helpers -------------------------------------------------------

static PangoLayout *make_layout(cairo_t *cr, const std::string &utf8,
                                const std::string &family, double px,
                                bool bold) {
    PangoLayout *pl = pango_cairo_create_layout(cr);
    PangoFontDescription *fd = pango_font_description_new();
    std::string fam = family.empty() ? "Sans" : family;
    pango_font_description_set_family(fd, fam.c_str());
    pango_font_description_set_absolute_size(fd, px * PANGO_SCALE);
    pango_font_description_set_weight(fd, bold ? PANGO_WEIGHT_SEMIBOLD
                                               : PANGO_WEIGHT_NORMAL);
    pango_layout_set_font_description(pl, fd);
    pango_font_description_free(fd);
    pango_layout_set_text(pl, utf8.c_str(), (int)utf8.size());
    return pl;
}

TextMetrics text_measure(cairo_t *cr, const std::string &utf8,
                         const std::string &family, double px, bool bold) {
    TextMetrics tm;
    PangoLayout *pl = make_layout(cr, utf8, family, px, bold);
    PangoRectangle ink, logical;
    pango_layout_get_pixel_extents(pl, &ink, &logical);
    tm.width = logical.width == 0 ? 0 : logical.width;
    tm.height = logical.height;
    g_object_unref(pl);
    return tm;
}

void text_draw(cairo_t *cr, const std::string &utf8, double x, double y,
               const std::string &family, double px, bool bold, const Color &c,
               int align) {
    if (utf8.empty()) return;
    PangoLayout *pl = make_layout(cr, utf8, family, px, bold);
    int lw = 0, lh = 0;
    pango_layout_get_pixel_size(pl, &lw, &lh);
    if (align == 0) x -= lw / 2.0;
    else if (align > 0) x -= lw;
    cairo_move_to(cr, x, y);
    c.apply(cr);
    pango_cairo_show_layout(cr, pl);
    g_object_unref(pl);
}

}  // namespace bwm
