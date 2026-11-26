// xshot - tiny X11 screenshot tool for CI evidence (PNG via cairo).
// usage: xshot <display> <out.png>
#include <X11/Xlib.h>
#include <X11/Xutil.h>

#include <cairo/cairo.h>

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cstdint>

int main(int argc, char **argv) {
    const char *dname = argc > 1 ? argv[1] : ":0";
    const char *out = argc > 2 ? argv[2] : "shot.png";
    Display *dpy = XOpenDisplay(dname);
    if (!dpy) {
        fprintf(stderr, "xshot: cannot open display %s\n", dname);
        return 1;
    }
    Window root = DefaultRootWindow(dpy);
    int w = DisplayWidth(dpy, 0), h = DisplayHeight(dpy, 0);
    Visual *vis = DefaultVisual(dpy, 0);
    unsigned long rm = vis->red_mask, gm = vis->green_mask, bm = vis->blue_mask;
    int rsh = 0, gsh = 0, bsh = 0;
    while (rm && !(rm & 1)) { rm >>= 1; rsh++; }
    while (gm && !(gm & 1)) { gm >>= 1; gsh++; }
    while (bm && !(bm & 1)) { bm >>= 1; bsh++; }

    XImage *img = XGetImage(dpy, root, 0, 0, w, h, AllPlanes, ZPixmap);
    if (!img) {
        fprintf(stderr, "xshot: XGetImage failed\n");
        return 1;
    }
    if (img->bits_per_pixel != 32) {
        fprintf(stderr, "xshot: need 32bpp, got %d\n", img->bits_per_pixel);
        XDestroyImage(img);
        return 1;
    }
    cairo_surface_t *surf = cairo_image_surface_create(CAIRO_FORMAT_RGB24, w, h);
    uint32_t *dst = (uint32_t *)cairo_image_surface_get_data(surf);
    int stride = cairo_image_surface_get_stride(surf);
    for (int y = 0; y < h; y++) {
        uint32_t *src =
            (uint32_t *)(img->data + y * img->bytes_per_line);
        uint32_t *row = (uint32_t *)((unsigned char *)dst + y * stride);
        for (int x = 0; x < w; x++) {
            uint32_t px = src[x];
            uint32_t r = (px >> rsh) & 0xFF;
            uint32_t g = (px >> gsh) & 0xFF;
            uint32_t b = (px >> bsh) & 0xFF;
            row[x] = (r << 16) | (g << 8) | b;  // cairo RGB24: 0x00RRGGBB
        }
    }
    cairo_surface_mark_dirty(surf);
    cairo_status_t st = cairo_surface_write_to_png(surf, out);
    cairo_surface_destroy(surf);
    XDestroyImage(img);
    XCloseDisplay(dpy);
    if (st != CAIRO_STATUS_SUCCESS) {
        fprintf(stderr, "xshot: write png failed: %s\n", cairo_status_to_string(st));
        return 1;
    }
    printf("xshot: %s (%dx%d)\n", out, w, h);
    return 0;
}
