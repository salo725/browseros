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
