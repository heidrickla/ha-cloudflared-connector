"""Derive the add-on's icon.png (128x128) and logo.png (250x100) from
icon-source-dark.png. Scales and composes only; does not redraw the artwork.
icon-family-reference.png is the design reference for the wider icon family.

    python brand/generate.py
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
ADDON = ROOT / "cloudflared_connector"
SOURCE = ROOT / "brand" / "icon-source-dark.png"
S = 4                       # supersampling for the composed logo

ORANGE = (246, 130, 31)
SLATE = (31, 41, 51)


def icon(size):
    return Image.open(SOURCE).convert("RGBA").resize((size, size), Image.LANCZOS)


def logo(width, height):
    """Tile left, wordmark right, transparent background. Name in slate: the
    store draws logos on a light card."""
    W, H = width * S, height * S
    im = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    tile = icon(H)
    im.paste(tile, (0, 0), tile)
    d = ImageDraw.Draw(im)
    x = H + W * 0.04
    avail = W - x - W * 0.03

    def fitted(text, frac):
        size = int(H * frac)
        while size > 8:
            f = ImageFont.load_default(size=size)
            if d.textlength(text, font=f) <= avail:
                return f
            size -= 2
        return ImageFont.load_default(size=8)

    d.text((x, H * 0.17), "Cloudflared", font=fitted("Cloudflared", 0.34), fill=SLATE)
    d.text((x, H * 0.58), "Connector", font=fitted("Connector", 0.22), fill=ORANGE)
    return im.resize((width, height), Image.LANCZOS)


if __name__ == "__main__":
    ADDON.mkdir(parents=True, exist_ok=True)
    icon(128).save(ADDON / "icon.png", optimize=True)
    logo(250, 100).save(ADDON / "logo.png", optimize=True)
    for p in (ADDON / "icon.png", ADDON / "logo.png"):
        w, h = Image.open(p).size
        print("  %-9s %dx%d  %d bytes" % (p.name, w, h, p.stat().st_size))
