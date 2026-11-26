#!/usr/bin/env python3
"""Generate BraveOS visual assets: wallpapers, GRUB background, raster icons.

Usage: gen_assets.py [output_dir]   (default: <repo>/build/assets)
Wallpapers are drawn with PIL. SVG icons are rasterized with rsvg-convert
when available (CI), else skipped gracefully.
"""
import os
import shutil
import subprocess
import sys
from PIL import Image, ImageDraw, ImageFilter

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(REPO, "build", "assets")
os.makedirs(OUT, exist_ok=True)

ORANGE = (240, 78, 35)
ORANGE_HI = (255, 122, 69)


def wallpaper(path, dark=True, size=(1920, 1080)):
    """The BraveOS v1.0.2 wallpaper: deep starry midnight-purple sky fading
    into a vibrant sunset horizon on the right, jagged snow-capped mountains,
    pine forests along the shore, a calm mirror lake, and a glowing Brave
    lion emblem in the sky. Anti-aliased vector rendering via cairo."""
    try:
        import cairo
    except ImportError:  # graceful fallback to the legacy PIL wallpaper
        return _wallpaper_legacy(path, dark, size)

    import math
    import random
    w, h = size
    surf = cairo.ImageSurface(cairo.FORMAT_ARGB32, w, h)
    cr = cairo.Context(surf)
    rng = random.Random(42)  # deterministic wallpaper

    horizon = h * 0.62          # mountain base line
    lake_top = h * 0.78         # shore line

    # ---- sky: midnight purple -> deep violet -> sunset horizon ----
    sky = cairo.LinearGradient(0, 0, w * 0.72, h * 0.75)
    sky.add_color_stop_rgb(0.00, 13/255, 6/255, 40/255)      # #0d0628
    sky.add_color_stop_rgb(0.45, 26/255, 13/255, 66/255)     # deep purple
    sky.add_color_stop_rgb(0.72, 42/255, 17/255, 84/255)     # #2a1154 zone
    sky.add_color_stop_rgb(0.88, 120/255, 45/255, 74/255)    # warm blend
    sky.add_color_stop_rgb(1.00, 240/255, 152/255, 25/255)   # #f09819 sun
    cr.set_source(sky)
    cr.rectangle(0, 0, w, lake_top + 4)
    cr.fill()

    # sunset glow on the right horizon: wide flat ellipse hugging the
    # horizon so it does not wash out the midnight-purple sky above
    gcx, gcy = w * 0.80, horizon - h * 0.03
    gr = w * 0.46
    cr.save()
    cr.translate(gcx, gcy)
    cr.scale(1.0, 0.36)                      # wide, flat glow
    glow = cairo.RadialGradient(0, 0, 10, 0, 0, gr)
    glow.add_color_stop_rgba(0.00, 1.00, 0.42, 0.20, 0.95)   # #ff512f core
    glow.add_color_stop_rgba(0.22, 1.00, 0.32, 0.18, 0.72)   # #f04e23
    glow.add_color_stop_rgba(0.50, 1.00, 0.60, 0.10, 0.40)   # #f09819
    glow.add_color_stop_rgba(1.00, 1.00, 0.16, 0.43, 0.00)   # #ff2a6d fade
    cr.set_source(glow)
    cr.paint()
    cr.restore()

    # ---- stars (densest and brightest at the top) ----
    for i in range(520):
        sx = rng.uniform(0, w)
        sy = rng.uniform(0, horizon * 0.9)
        fade = 1.0 - sy / (horizon * 0.95)
        if rng.random() > 0.25 + 0.75 * fade:
            continue
        r = rng.uniform(0.4, 1.4) * (1 + fade)
        a = rng.uniform(0.25, 0.95) * (0.3 + 0.7 * fade)
        cr.set_source_rgba(1, 1, 1, a)
        cr.new_sub_path()
        cr.arc(sx, sy, r, 0, 2 * math.pi)
        cr.fill()
    # a few brighter "sparkle" stars
    for i in range(24):
        sx = rng.uniform(0, w)
        sy = rng.uniform(0, horizon * 0.6)
        r = rng.uniform(1.6, 2.6)
        cr.set_source_rgba(1, 1, 1, rng.uniform(0.8, 1.0))
        cr.new_sub_path()
        cr.arc(sx, sy, r, 0, 2 * math.pi)
        cr.fill()
        cr.set_source_rgba(1, 1, 1, 0.35)
        for dx, dy in ((-3.2 * r, 0), (3.2 * r, 0), (0, -3.2 * r), (0, 3.2 * r)):
            cr.new_sub_path()
            cr.arc(sx + dx, sy + dy, r * 0.35, 0, 2 * math.pi)
            cr.fill()

    # ---- mountains: three jagged snow-capped ranges ----
    def mountain_range(base_y, amp, color, snow_alpha, n, seed, snow_line=0.32):
        r2 = random.Random(seed)
        pts = []
        x = -80
        while x < w + 80:
            peak = r2.uniform(0.35, 1.0)
            pts.append((x, base_y - amp * peak * r2.uniform(0.75, 1.25)))
            x += r2.uniform(w / (n * 2.2), w / n)
        pts.append((w + 80, base_y))
        pts.insert(0, (-80, base_y))
        # body
        cr.new_sub_path()
        cr.move_to(pts[0][0], pts[0][1])
        for px, py in pts[1:]:
            cr.line_to(px, py)
        cr.line_to(w + 80, lake_top + 4)
        cr.line_to(-80, lake_top + 4)
        cr.close_path()
        cr.set_source_rgb(*color)
        cr.fill()
        # snow caps: small white wedges on the highest peaks
        for i in range(1, len(pts) - 1):
            px, py = pts[i]
            if py > base_y - amp * snow_line:
                continue  # too low for snow
            left = pts[i - 1]
            right = pts[i + 1]
            span_l = (px - left[0]) * 0.30
            span_r = (right[0] - px) * 0.30
            cr.new_sub_path()
            cr.move_to(px, py)
            cr.line_to(px - span_l, py + amp * 0.16)
            cr.line_to(px - span_l * 0.4, py + amp * 0.10)
            cr.line_to(px + span_r * 0.4, py + amp * 0.085)
            cr.line_to(px + span_r, py + amp * 0.17)
            cr.close_path()
            cr.set_source_rgba(1, 1, 1, snow_alpha)
            cr.fill()

    mountain_range(horizon + 6, h * 0.26, (24/255, 14/255, 52/255), 0.50, 9, 7)
    mountain_range(horizon + 2, h * 0.20, (16/255, 9/255, 38/255), 0.60, 7, 19)
    mountain_range(horizon - 2, h * 0.135, (10/255, 5/255, 26/255), 0.72, 5, 33)

    # ---- glowing Brave lion emblem in the sky ----
    lx, ly, ls = w * 0.5, h * 0.285, w * 0.115
    aura = cairo.RadialGradient(lx, ly, ls * 0.25, lx, ly, ls * 2.6)
    aura.add_color_stop_rgba(0, 1.0, 0.30, 0.17, 0.50)   # #ff4b2b glow
    aura.add_color_stop_rgba(0.40, 1.0, 0.25, 0.14, 0.22)
    aura.add_color_stop_rgba(0.70, 1.0, 0.22, 0.12, 0.08)
    aura.add_color_stop_rgba(1, 1.0, 0.2, 0.1, 0.0)
    cr.set_source(aura)
    cr.rectangle(lx - ls * 3.4, ly - ls * 3.4, ls * 6.8, ls * 6.8)
    cr.fill()

    grad = cairo.LinearGradient(lx - ls, ly - ls, lx + ls, ly + ls)
    grad.add_color_stop_rgb(0.0, 1.00, 0.42, 0.24)       # #ff6b3d
    grad.add_color_stop_rgb(0.5, 1.00, 0.29, 0.17)       # #ff4b2b
    grad.add_color_stop_rgb(1.0, 0.86, 0.20, 0.09)
    cr.set_source(grad)
    sw, sh = ls * 0.92, ls * 1.02
    x0, y0 = lx - sw / 2, ly - sh / 2
    cr.new_sub_path()
    cr.move_to(lx, y0 + sh * 0.06)
    cr.curve_to(x0 + sw * 0.28, y0 - sh * 0.05, x0 + sw * 0.72, y0 - sh * 0.05,
                x0 + sw, y0 + sh * 0.14)
    cr.curve_to(x0 + sw, y0 + sh * 0.62, lx + sw * 0.18, y0 + sh * 0.9, lx,
                y0 + sh)
    cr.curve_to(lx - sw * 0.18, y0 + sh * 0.9, x0, y0 + sh * 0.62, x0,
                y0 + sh * 0.14)
    cr.curve_to(x0, y0 + sh * 0.14, x0 + sw * 0.06, y0 + sh * 0.02, lx,
                y0 + sh * 0.06)
    cr.close_path()
    cr.fill()
    # lion face (matching the bar/dock mark)
    cr.set_source_rgb(1, 1, 1)
    cr.set_line_width(ls * 0.105)
    cr.set_line_cap(cairo.LINE_CAP_ROUND)
    ey = ly - sh * 0.15
    # fierce angled eyes: outer end high, inner end low (facing forward)
    cr.new_sub_path()
    cr.move_to(lx - sw * 0.34, ey - ls * 0.055)
    cr.line_to(lx - sw * 0.10, ey + ls * 0.05)
    cr.stroke()
    cr.new_sub_path()
    cr.move_to(lx + sw * 0.34, ey - ls * 0.055)
    cr.line_to(lx + sw * 0.10, ey + ls * 0.05)
    cr.stroke()
    # muzzle: small nose + chin notch (no grin)
    cr.set_line_width(ls * 0.085)
    my = ly + sh * 0.14
    cr.new_sub_path()
    cr.move_to(lx - sw * 0.085, my)
    cr.line_to(lx, my + ls * 0.075)
    cr.line_to(lx + sw * 0.085, my)
    cr.stroke()
    cr.new_sub_path()
    cr.move_to(lx - sw * 0.05, my + ls * 0.14)
    cr.curve_to(lx, my + ls * 0.20, lx, my + ls * 0.20, lx + sw * 0.05,
                my + ls * 0.14)
    cr.stroke()
    # mane notches
    cr.set_line_width(ls * 0.07)
    cr.new_sub_path()
    cr.move_to(lx - sw * 0.44, ly + sh * 0.02)
    cr.line_to(lx - sw * 0.24, ly + sh * 0.02)
    cr.stroke()
    cr.new_sub_path()
    cr.move_to(lx + sw * 0.44, ly + sh * 0.02)
    cr.line_to(lx + sw * 0.24, ly + sh * 0.02)
    cr.stroke()

    # ---- lake: mirror of sky + glow, darker ----
    lake = cairo.LinearGradient(0, lake_top, 0, h)
    lake.add_color_stop_rgb(0.00, 58/255, 24/255, 74/255)   # shore purple
    lake.add_color_stop_rgb(0.35, 34/255, 16/255, 56/255)
    lake.add_color_stop_rgb(1.00, 12/255, 7/255, 28/255)
    cr.set_source(lake)
    cr.rectangle(0, lake_top, w, h - lake_top)
    cr.fill()
    # reflected sunset glow on the lake
    lglow = cairo.RadialGradient(w * 0.82, lake_top, 8, w * 0.82, lake_top, w * 0.5)
    lglow.add_color_stop_rgba(0.0, 1.00, 0.50, 0.20, 0.55)
    lglow.add_color_stop_rgba(0.4, 1.00, 0.34, 0.16, 0.30)
    lglow.add_color_stop_rgba(1.0, 1.00, 0.2, 0.4, 0.0)
    cr.set_source(lglow)
    cr.rectangle(0, lake_top, w, h - lake_top)
    cr.fill()
    # faint mountain reflection
    cr.save()
    cr.translate(0, 2 * lake_top)
    cr.scale(1, -1)
    cr.set_source_rgba(24/255, 14/255, 52/255, 0.34)
    cr.new_sub_path()
    cr.move_to(-80, lake_top)
    r3 = random.Random(91)
    x = -80
    refl = [(-80, lake_top)]
    while x < w + 80:
        x += r3.uniform(w / 12, w / 5)
        refl.append((x, lake_top - r3.uniform(4, h * 0.05)))
    refl.append((w + 80, lake_top))
    for px, py in refl:
        cr.line_to(px, py)
    cr.close_path()
    cr.fill()
    cr.restore()
    # emblem reflection: soft warm vertical smear under the lion
    esmear = cairo.LinearGradient(0, lake_top, 0, h)
    esmear.add_color_stop_rgba(0.0, 1.0, 0.35, 0.20, 0.20)
    esmear.add_color_stop_rgba(0.5, 1.0, 0.25, 0.14, 0.07)
    esmear.add_color_stop_rgba(1.0, 1.0, 0.2, 0.1, 0.0)
    cr.set_source(esmear)
    cr.save()
    cr.new_sub_path()
    cr.move_to(lx - ls * 0.55, lake_top + 2)
    cr.curve_to(lx - ls * 0.75, (lake_top + h) / 2, lx - ls * 0.30, h - 30,
                lx - ls * 0.05, h - 2)
    cr.line_to(lx + ls * 0.05, h - 2)
    cr.curve_to(lx + ls * 0.30, h - 30, lx + ls * 0.75, (lake_top + h) / 2,
                lx + ls * 0.55, lake_top + 2)
    cr.close_path()
    cr.fill()
    cr.restore()

    # calm water ripple highlights
    for i in range(46):
        ry = rng.uniform(lake_top + 6, h - 4)
        rx = rng.uniform(w * 0.30, w * 0.98)
        rw = rng.uniform(w * 0.04, w * 0.16) * (1 - (ry - lake_top) / (h - lake_top) + 0.4)
        warm = rx > w * 0.55
        cr.set_source_rgba(*((1.0, 0.75, 0.45) if warm else (0.75, 0.65, 0.95)),
                           rng.uniform(0.10, 0.30))
        cr.set_line_width(rng.uniform(1.0, 2.4))
        cr.new_sub_path()
        cr.move_to(rx - rw, ry)
        cr.curve_to(rx - rw * 0.4, ry - 2.5, rx + rw * 0.4, ry + 2.5, rx + rw, ry)
        cr.stroke()

    # ---- pine forest silhouettes along the shore ----
    def pine(px, base, ht, wid, color):
        cr.set_source_rgb(*color)
        cr.new_sub_path()
        cr.move_to(px, base - ht)
        cr.line_to(px + wid * 0.5, base)
        cr.line_to(px + wid * 0.34, base)
        cr.line_to(px + wid * 0.30, base - ht * 0.30)
        cr.line_to(px + wid * 0.22, base - ht * 0.18)
        cr.line_to(px + wid * 0.16, base - ht * 0.32)
        cr.line_to(px + wid * 0.10, base - ht * 0.22)
        cr.line_to(px + wid * 0.06, base - ht * 0.36)
        cr.line_to(px, base)
        cr.line_to(px - wid * 0.06, base - ht * 0.36)
        cr.line_to(px - wid * 0.10, base - ht * 0.22)
        cr.line_to(px - wid * 0.16, base - ht * 0.32)
        cr.line_to(px - wid * 0.22, base - ht * 0.18)
        cr.line_to(px - wid * 0.30, base - ht * 0.30)
        cr.line_to(px - wid * 0.34, base)
        cr.line_to(px - wid * 0.5, base)
        cr.close_path()
        cr.fill()

    # back row (dark purple) + front row (near-black), tall enough to read
    x = -40
    while x < w + 40:
        ht = rng.uniform(h * 0.055, h * 0.105)
        pine(x, lake_top + 3, ht, ht * 0.64, (16/255, 8/255, 34/255))
        x += rng.uniform(ht * 0.33, ht * 0.55)
    x = -30
    while x < w + 30:
        ht = rng.uniform(h * 0.075, h * 0.135)
        pine(x, lake_top + 9, ht, ht * 0.58, (5/255, 3/255, 14/255))
        x += rng.uniform(ht * 0.38, ht * 0.7)

    # subtle vignette
    vg = cairo.RadialGradient(w / 2, h / 2, h * 0.45, w / 2, h / 2, h * 1.05)
    vg.add_color_stop_rgba(0, 0, 0, 0, 0)
    vg.add_color_stop_rgba(1, 0, 0, 0, 0.28)
    cr.set_source(vg)
    cr.rectangle(0, 0, w, h)
    cr.fill()

    surf.flush()
    # tiny noise dither breaks up gradient banding on 8-bit displays
    import numpy as _np
    arr = _np.frombuffer(surf.get_data(), dtype=_np.uint8)
    if arr.size < w * h * 4:  # stride padding
        arr = _np.frombuffer(bytes(surf.get_data()), dtype=_np.uint8)
    arr = arr.reshape(h, w * 4)
    # cairo ARGB32 on little-endian is BGRA in memory; convert to RGBA
    rgba = arr.reshape(h, w, 4)[:, :, [2, 1, 0, 3]].copy()
    rng_d = _np.random.RandomState(7)
    dith = _np.dstack([rng_d.randint(-2, 3, size=(h, w))] * 3)
    rgba[:, :, :3] = _np.clip(rgba[:, :, :3].astype(_np.int16) + dith,
                              0, 255).astype(_np.uint8)
    from PIL import Image as _Image
    _Image.frombytes("RGBA", (w, h), rgba.tobytes()).save(path, "PNG")
    print(f"wrote {path}")


def _wallpaper_legacy(path, dark=True, size=(1920, 1080)):
    w, h = size
    base = (18, 18, 20) if dark else (245, 244, 242)
    img = Image.new("RGB", size, base)
    d = ImageDraw.Draw(img)

    # vertical gradient
    top = base
    bottom = (10, 10, 12) if dark else (228, 226, 222)
    for y in range(h):
        t = y / h
        r = int(top[0] + (bottom[0] - top[0]) * t)
        g = int(top[1] + (bottom[1] - top[1]) * t)
        b = int(top[2] + (bottom[2] - top[2]) * t)
        d.line([(0, y), (w, y)], fill=(r, g, b))

    # big soft orange glow bottom-right
    glow = Image.new("RGB", size, (0, 0, 0))
    gd = ImageDraw.Draw(glow)
    cx, cy, rad = int(w * 0.82), int(h * 0.78), int(min(w, h) * 0.55)
    gd.ellipse([cx - rad, cy - rad, cx + rad, cy + rad], fill=ORANGE)
    glow = glow.filter(ImageFilter.GaussianBlur(160))
    img = Image.blend(img, Image.composite(
        Image.new("RGB", size, ORANGE), img,
        glow.convert("L").point(lambda p: p // 6 if dark else p // 10)), 0.9)

    d = ImageDraw.Draw(img)

    # concentric arc rings (brand motif), off-center right
    ring_c = (255, 255, 255, 26) if dark else (30, 30, 30, 30)
    ov = Image.new("RGBA", size, (0, 0, 0, 0))
    od = ImageDraw.Draw(ov)
    for i in range(6):
        rr = 220 + i * 130
        od.arc([w - 420 - rr, h - 320 - rr, w - 420 + rr, h - 320 + rr],
               start=190, end=320, fill=ring_c, width=3)
    img = Image.alpha_composite(img.convert("RGBA"), ov).convert("RGB")
    d = ImageDraw.Draw(img)

    # lion shield mark
    mx, my = w - 480, int(h * 0.42)
    s = 5
    pts_shield = [(mx, my - 70 * s // 2), (mx + 80, my - 40), (mx + 80, my + 40),
                  (mx + 30, my + 90), (mx - 30, my + 90), (mx - 80, my + 40),
                  (mx - 80, my - 40)]
    d.polygon(pts_shield, fill=ORANGE)
    d.polygon([(mx, my - 45), (mx + 32, my - 28), (mx + 22, my - 10),
               (mx, my - 22), (mx - 22, my - 10), (mx - 32, my - 28)],
              fill=(122, 36, 16))
    d.polygon([(mx - 45, my), (mx, my + 40), (mx + 45, my), (mx + 33, my + 60),
               (mx, my + 75), (mx - 33, my + 60)], fill=(255, 225, 207))
    d.ellipse([mx - 18, my + 8, mx - 10, my + 16], fill=(122, 36, 16))
    d.ellipse([mx + 10, my + 8, mx + 18, my + 16], fill=(122, 36, 16))

    # wordmark
    try:
        from PIL import ImageFont
        font = ImageFont.truetype(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 44)
        small = ImageFont.truetype(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 20)
    except OSError:
        font = small = None
    d.text((mx - 100, my + 110), "BraveOS", fill=(255, 243, 236), font=font)
    d.text((mx - 100, my + 165),
           "The Brave Browser Distribution  -  unofficial",
           fill=(230, 140, 105), font=small)

    img.save(path, "PNG")
    print(f"wrote {path}")


def grub_bg(path, size=(1280, 720)):
    img = Image.new("RGB", size, (12, 12, 14))
    d = ImageDraw.Draw(img)
    for y in range(size[1]):
        t = y / size[1]
        d.line([(0, y), (size[0], y)], fill=(int(12 + 16 * t), int(12 + 14 * t), int(14 + 18 * t)))
    cx, cy = size[0] // 2, size[1] // 2
    d.ellipse([cx - 260, cy - 260, cx + 260, cy + 260], outline=ORANGE, width=3)
    d.ellipse([cx - 200, cy - 200, cx + 200, cy + 200], outline=(90, 90, 95), width=1)
    d.polygon([(cx, cy - 90), (cx + 64, cy - 58), (cx + 64, cy + 10),
               (cx + 24, cy + 72), (cx - 24, cy + 72), (cx - 64, cy + 10),
               (cx - 64, cy - 58)], fill=ORANGE)
    d.polygon([(cx, cy - 58), (cx + 26, cy - 38), (cx, cy - 24),
               (cx - 26, cy - 38)], fill=(122, 36, 16))
    d.polygon([(cx - 42, cy - 4), (cx, cy + 34), (cx + 42, cy - 4),
               (cx + 30, cy + 44), (cx, cy + 58), (cx - 30, cy + 44)],
              fill=(255, 243, 236))
    try:
        from PIL import ImageFont
        f_big = ImageFont.truetype(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 56)
        f_sm = ImageFont.truetype(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 22)
    except OSError:
        f_big = f_sm = None
    d.text((cx, cy + 110), "BraveOS", fill=(255, 243, 236),
           font=f_big, anchor="mm")
    d.text((cx, cy + 160), "unofficial Brave browser distribution",
           fill=(200, 120, 90), font=f_sm, anchor="mm")
    img.save(path, "PNG")
    print(f"wrote {path}")


def rasterize_icons():
    rsvg = shutil.which("rsvg-convert")
    src = os.path.join(REPO, "assets", "icons")
    if not os.path.isdir(src):
        print("no icon sources found")
        return
    for fn in sorted(os.listdir(src)):
        if not fn.endswith(".svg"):
            continue
        name = "icon-" + os.path.splitext(fn)[0] + ".png"
        dst = os.path.join(OUT, name)
        if rsvg:
            subprocess.run(["rsvg-convert", "-w", "48", "-h", "48",
                            os.path.join(src, fn), "-o", dst], check=True)
        else:
            print(f"rsvg-convert missing; skipped {name}")
            continue
        print(f"wrote {dst}")


if __name__ == "__main__":
    wallpaper(os.path.join(OUT, "wallpaper-dark.png"), dark=True)
    wallpaper(os.path.join(OUT, "wallpaper-light.png"), dark=False)
    grub_bg(os.path.join(OUT, "grub-bg.png"))
    rasterize_icons()
    print("assets done")
