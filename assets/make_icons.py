#!/usr/bin/env python3
"""Portkeeper's mark: a colon, as in localhost:3000. Two dots on a dark tile,
the lower one is the app's green "running" dot.

Writes assets/icon.png (1024px app icon on the macOS grid), site/favicon.png,
site/img/icon-256.png and src-tauri/icons/tray.png (macOS template icon).
Requires Pillow. `npm run icons` runs this and lets Tauri derive every size."""
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
TILE = (14, 14, 16, 255)
WHITE = (255, 255, 255, 255)
GREEN = (48, 209, 88, 255)
SS = 4  # supersampling for smooth edges


def mark(size, body, radius, dot, gap, tile=TILE, top=WHITE, bottom=GREEN):
    """Tile of side `body` centred in `size`, two dots of diameter `dot`, `gap` between them."""
    S = size * SS
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    o = (size - body) / 2 * SS
    if tile:
        # faint edge so the tile still reads on dark docks and taskbars
        d.rounded_rectangle([o, o, S - o - 1, S - o - 1], radius=radius * SS, fill=tile,
                            outline=(255, 255, 255, 30), width=max(SS, int(body * SS / 280)))
    cx, r = S / 2, dot / 2 * SS
    span = (2 * dot + gap) * SS
    y0 = (S - span) / 2
    for i, col in enumerate((top, bottom)):
        cy = y0 + r + i * (dot + gap) * SS
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=col)
    return img.resize((size, size), Image.LANCZOS)


def app_icon():
    # macOS grid: 824px body in a 1024 canvas, ~185px corner radius.
    return mark(1024, 824, 185, dot=150, gap=96)


def favicon(size):
    # Full-bleed tile: favicons have no room for margins.
    return mark(size, size, size * 0.225, dot=size * 0.19, gap=size * 0.12)


def tray_icon(size=44):
    # Template image (black + alpha): outlined tile with the colon inside.
    S = size * SS
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    m, w = 5 * SS, 3.2 * SS
    d.rounded_rectangle([m, m, S - m - 1, S - m - 1], radius=9 * SS, outline=(0, 0, 0, 255), width=int(w))
    dot, gap = 7 * SS, 4 * SS
    cx, y0 = S / 2, (S - (2 * dot + gap)) / 2
    for i in range(2):
        cy = y0 + dot / 2 + i * (dot + gap)
        d.ellipse([cx - dot / 2, cy - dot / 2, cx + dot / 2, cy + dot / 2], fill=(0, 0, 0, 255))
    return img.resize((size, size), Image.LANCZOS)


if __name__ == "__main__":
    app_icon().save(ROOT / "assets" / "icon.png")
    favicon(64).save(ROOT / "site" / "favicon.png")
    favicon(256).save(ROOT / "site" / "img" / "icon-256.png")
    tray_icon().save(ROOT / "src-tauri" / "icons" / "tray.png")
    print("wrote assets/icon.png, site/favicon.png, site/img/icon-256.png, src-tauri/icons/tray.png")
