#!/usr/bin/env python3
"""Generate BraveOS v2 assets: wallpaper (from the approved BraveOS.png design),
light + aurora variants, GRUB background, and the full 2026-style icon set
(SVG sources rasterized with rsvg-convert).

Usage: gen_assets.py [output_dir]   (default: <repo>/build/assets)
"""
import os
import shutil
import subprocess
import sys
from PIL import Image, ImageDraw, ImageEnhance, ImageFilter

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(REPO, "build", "assets")
UPLOAD = os.environ.get("BRAVOS_DESIGN_PNG", "/home/z/my-project/upload/BraveOS.png")
os.makedirs(OUT, exist_ok=True)

ORANGE = (240, 78, 35)


# ------------------------------------------------------------------ wallpaper
def wallpapers():
    if os.path.exists(UPLOAD):
        src = Image.open(UPLOAD).convert("RGB")
    else:  # fallback: procedural sunset in the same palette
        src = Image.new("RGB", (1920, 1080), (24, 16, 44))
        d = ImageDraw.Draw(src)
        for y in range(1080):
            t = y / 1080
            d.line([(0, y), (1920, y)], fill=(int(24 + 90 * t), int(16 + 30 * t), int(44 + 60 * t)))
    big = src.resize((1920, 1080), Image.LANCZOS)
    big.save(os.path.join(OUT, "wallpaper-dark.png"), "PNG")
    print("wrote wallpaper-dark.png")

    # Light variant: airy pastel version of the same scene
    light = big.filter(ImageFilter.GaussianBlur(2))
    light = ImageEnhance.Brightness(light).enhance(1.65)
    light = ImageEnhance.Color(light).enhance(1.15)
    white = Image.new("RGB", big.size, (246, 244, 248))
    light = Image.blend(light, white, 0.55)
    light.save(os.path.join(OUT, "wallpaper-light.png"), "PNG")
    print("wrote wallpaper-light.png")

    # Aurora variant: procedural teal/violet curtains + stars
    w, h = 1920, 1080
    img = Image.new("RGB", (w, h), (10, 10, 24))
    d = ImageDraw.Draw(img)
    for y in range(h):
        t = y / h
        d.line([(0, y), (w, y)],
               fill=(int(10 + 20 * t), int(10 + 40 * t), int(24 + 70 * (1 - t))))
    import random
    random.seed(7)
    ov = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    od = ImageDraw.Draw(ov)
    for _ in range(240):
        x, y = random.randint(0, w), random.randint(0, int(h * 0.7))
        b = random.randint(120, 255)
        od.point((x, y), fill=(b, b, min(255, b + 40), 255))
    for i, (x0, col) in enumerate([(500, (40, 200, 170, 60)), (1100, (130, 90, 240, 60))]):
        for k in range(14):
            xoff = x0 + k * 26
            od.line([(xoff, 0), (xoff + 130, h)], fill=col, width=44)
    ov = ov.filter(ImageFilter.GaussianBlur(34))
    img = Image.alpha_composite(img.convert("RGBA"), ov).convert("RGB")
    img.save(os.path.join(OUT, "wallpaper-aurora.png"), "PNG")
    print("wrote wallpaper-aurora.png")


def grub_bg(path, size=(1280, 720)):
    base = os.path.join(OUT, "wallpaper-dark.png")
    if os.path.exists(base):
        Image.open(base).resize(size, Image.LANCZOS).save(path, "PNG")
    print(f"wrote {path}")


# ------------------------------------------------------------------ icons
ICONS = {
    "brave": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<g fill="none">
<path d="M48 8 62 16 78 12 86 30 80 44 84 58 74 78 60 88 48 84 36 88 22 78 12 58 16 44 10 30 18 12 34 16Z" fill="#e85a2a"/>
<path d="M48 8 62 16 78 12 86 30 80 44 84 58 74 78 60 88 48 84 36 88 22 78 12 58 16 44 10 30 18 12 34 16Z" stroke="#ff8a50" stroke-width="2" fill-opacity="0.95"/>
<path d="M30 34 40 40 44 34Z" fill="#7a2410"/>
<path d="M66 34 56 40 52 34Z" fill="#7a2410"/>
<path d="M34 44c5-3 12-3 14 2-6 4-11 3-14-2Z" fill="#fff3ec"/>
<path d="M62 44c-5-3-12-3-14 2 6 4 11 3 14-2Z" fill="#fff3ec"/>
<path d="M44 54h8l-4 8Z" fill="#7a2410"/>
<path d="M40 66c4 6 12 6 16 0 4 6 2 12-8 14-10-2-12-8-8-14Z" fill="#fff3ec"/>
</g></svg>''',
    "folder": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M12 24c0-4 3-7 7-7h18l8 9h32c4 0 7 3 7 7v40c0 4-3 7-7 7H19c-4 0-7-3-7-7Z" fill="#4f8cf7"/>
<path d="M12 36h72v37c0 4-3 7-7 7H19c-4 0-7-3-7-7Z" fill="#6ca6ff"/>
</svg>''',
    "settings": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<g>
<circle cx="48" cy="48" r="17" fill="none" stroke="#e6e6ee" stroke-width="7"/>
<circle cx="48" cy="48" r="6" fill="#e6e6ee"/>
<g fill="#e6e6ee">
<rect x="43" y="8" width="10" height="14" rx="4"/>
<rect x="43" y="74" width="10" height="14" rx="4"/>
<rect x="8" y="43" width="14" height="10" rx="4"/>
<rect x="74" y="43" width="14" height="10" rx="4"/>
<rect x="43" y="8" width="10" height="14" rx="4" transform="rotate(45 48 48)"/>
<rect x="43" y="74" width="10" height="14" rx="4" transform="rotate(45 48 48)"/>
<rect x="8" y="43" width="14" height="10" rx="4" transform="rotate(45 48 48)"/>
<rect x="74" y="43" width="14" height="10" rx="4" transform="rotate(45 48 48)"/>
</g>
</g></svg>''',
    "terminal": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<rect x="10" y="14" width="76" height="68" rx="10" fill="#14141e"/>
<path d="M26 36l12 12-12 12" stroke="#5ad0a0" stroke-width="7" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M48 60h22" stroke="#e6e6ee" stroke-width="7" stroke-linecap="round"/>
</svg>''',
    "grid": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<g fill="#e6e6ee">
<circle cx="26" cy="26" r="8"/><circle cx="48" cy="26" r="8"/><circle cx="70" cy="26" r="8"/>
<circle cx="26" cy="48" r="8"/><circle cx="48" cy="48" r="8"/><circle cx="70" cy="48" r="8"/>
<circle cx="26" cy="70" r="8"/><circle cx="48" cy="70" r="8"/><circle cx="70" cy="70" r="8"/>
</g></svg>''',
    "moon": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M60 14a34 34 0 1 0 22 52A38 38 0 0 1 60 14Z" fill="#c9c9e2"/>
</svg>''',
    "wifi": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<g fill="none" stroke="#e6e6ee" stroke-width="7" stroke-linecap="round">
<path d="M14 36a52 52 0 0 1 68 0"/>
<path d="M26 52a34 34 0 0 1 44 0"/>
<path d="M38 67a17 17 0 0 1 20 0"/>
</g>
<circle cx="48" cy="79" r="6" fill="#e6e6ee"/>
</svg>''',
    "bluetooth": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M48 10v76M48 10l20 17-40 42M48 86l20-17-40-42" fill="none" stroke="#e6e6ee" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/>
</svg>''',
    "battery": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<rect x="14" y="30" width="60" height="36" rx="8" fill="none" stroke="#e6e6ee" stroke-width="6"/>
<rect x="20" y="36" width="34" height="24" rx="4" fill="#7fd88f"/>
<rect x="78" y="42" width="7" height="12" rx="3" fill="#e6e6ee"/>
</svg>''',
    "power": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M48 14v30" stroke="#e6e6ee" stroke-width="8" stroke-linecap="round"/>
<path d="M30 26a30 30 0 1 0 36 0" fill="none" stroke="#e6e6ee" stroke-width="8" stroke-linecap="round"/>
</svg>''',
    "lock": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<rect x="22" y="42" width="52" height="38" rx="9" fill="#e6e6ee"/>
<path d="M34 42v-10a14 14 0 0 1 28 0v10" fill="none" stroke="#e6e6ee" stroke-width="7"/>
<circle cx="48" cy="58" r="6" fill="#14141e"/>
</svg>''',
    "restart": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M74 40a30 30 0 1 0 2 18" fill="none" stroke="#e6e6ee" stroke-width="7" stroke-linecap="round"/>
<path d="M78 14v24h-24" fill="none" stroke="#e6e6ee" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/>
</svg>''',
    "app": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<circle cx="48" cy="48" r="34" fill="#e85a2a"/>
<path d="M34 62l14-30 14 30M39 52h18" fill="none" stroke="#fff" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/>
</svg>''',
    "about": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<circle cx="48" cy="48" r="34" fill="none" stroke="#e6e6ee" stroke-width="6"/>
<circle cx="48" cy="33" r="5" fill="#e6e6ee"/>
<path d="M48 44v22" stroke="#e6e6ee" stroke-width="7" stroke-linecap="round"/>
</svg>''',
    "appearance": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M48 14a34 34 0 1 0 0 68c6 0 8-4 6-9-2-6 1-11 8-11h8c7 0 12-6 12-14C82 30 67 14 48 14Z" fill="none" stroke="#e6e6ee" stroke-width="6"/>
<circle cx="34" cy="38" r="6" fill="#e85a2a"/><circle cx="52" cy="32" r="6" fill="#4f8cf7"/><circle cx="36" cy="56" r="6" fill="#7fd88f"/>
</svg>''',
    "display": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<rect x="12" y="18" width="72" height="46" rx="8" fill="none" stroke="#e6e6ee" stroke-width="6"/>
<path d="M36 78h24M48 64v14" stroke="#e6e6ee" stroke-width="6" stroke-linecap="round"/>
</svg>''',
    "network": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<circle cx="48" cy="48" r="34" fill="none" stroke="#e6e6ee" stroke-width="6"/>
<path d="M14 48h68M48 14c12 12 12 56 0 68M48 14c-12 12-12 56 0 68" fill="none" stroke="#e6e6ee" stroke-width="5"/>
</svg>''',
    "sound": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M20 38h12l16-14v48L32 58H20Z" fill="#e6e6ee"/>
<path d="M60 34a20 20 0 0 1 0 28M68 26a30 30 0 0 1 0 44" fill="none" stroke="#e6e6ee" stroke-width="6" stroke-linecap="round"/>
</svg>''',
    "keyboard": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<rect x="10" y="26" width="76" height="44" rx="9" fill="none" stroke="#e6e6ee" stroke-width="6"/>
<g fill="#e6e6ee">
<rect x="20" y="36" width="8" height="8" rx="2"/><rect x="34" y="36" width="8" height="8" rx="2"/>
<rect x="48" y="36" width="8" height="8" rx="2"/><rect x="62" y="36" width="8" height="8" rx="2"/>
<rect x="26" y="52" width="44" height="8" rx="2"/>
</g></svg>''',
    "language": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M14 24c0-6 4-10 10-10h48c6 0 10 4 10 10v30c0 6-4 10-10 10H38L22 82V64h-2c-4 0-6-4-6-8Z" fill="none" stroke="#e6e6ee" stroke-width="6"/>
<path d="M34 36h14M41 36c0 12-6 18-12 22M44 40c4 8 10 14 18 16" fill="none" stroke="#e85a2a" stroke-width="5" stroke-linecap="round"/>
</svg>''',
    "mouse": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<rect x="30" y="14" width="36" height="66" rx="18" fill="none" stroke="#e6e6ee" stroke-width="6"/>
<path d="M48 14v22" stroke="#e6e6ee" stroke-width="6"/>
</svg>''',
    "shortcuts": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<g fill="none" stroke="#e6e6ee" stroke-width="6">
<circle cx="34" cy="34" r="9"/><circle cx="62" cy="34" r="9"/><circle cx="34" cy="62" r="9"/><circle cx="62" cy="62" r="9"/>
</g></svg>''',
    "time": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<circle cx="48" cy="48" r="34" fill="none" stroke="#e6e6ee" stroke-width="6"/>
<path d="M48 30v20l14 10" fill="none" stroke="#e6e6ee" stroke-width="6" stroke-linecap="round"/>
</svg>''',
    "welcome": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M48 10 84 40 48 86 12 40Z" fill="#e85a2a"/>
<path d="M48 26 68 44 48 72 28 44Z" fill="#fff3ec"/>
</svg>''',
    "installer": '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96">
<path d="M48 14v40" stroke="#e6e6ee" stroke-width="8" stroke-linecap="round"/>
<path d="M34 42l14 14 14-14" fill="none" stroke="#e6e6ee" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>
<rect x="18" y="66" width="60" height="12" rx="6" fill="#e6e6ee"/>
</svg>''',
}


def rasterize_icon_bytes(svg: str, size: int = 96) -> bytes:
    """Rasterize an SVG string to PNG bytes: rsvg-convert if present, else cairosvg."""
    rsvg = shutil.which("rsvg-convert")
    if rsvg:
        p = subprocess.run([rsvg, "-w", str(size), "-h", str(size)],
                           input=svg.encode(), capture_output=True, check=True)
        return p.stdout
    import io
    import cairosvg
    return cairosvg.svg2png(bytestring=svg.encode(), output_width=size, output_height=size)


def rasterize_icons():
    src = os.path.join(REPO, "assets", "icons")
    os.makedirs(src, exist_ok=True)
    n = 0
    for name, svg in ICONS.items():
        svg_path = os.path.join(src, f"{name}.svg")
        with open(svg_path, "w") as f:
            f.write(svg)
        dst = os.path.join(OUT, f"icon-{name}.png")
        with open(dst, "wb") as f:
            f.write(rasterize_icon_bytes(svg))
        n += 1
    # keep classic aliases used by .desktop files
    for old in ["bravos", "welcome"]:
        old_svg = os.path.join(src, f"{old}.svg")
        if os.path.exists(old_svg):
            with open(old_svg) as f:
                svg = f.read()
            with open(os.path.join(OUT, f"icon-{old}.png"), "wb") as f:
                f.write(rasterize_icon_bytes(svg))
            n += 1
    print(f"rasterized {n} icons")


if __name__ == "__main__":
    wallpapers()
    grub_bg(os.path.join(OUT, "grub-bg.png"))
    rasterize_icons()
    print("assets done")
