"""Derive the add-on's icon.png and logo.png from Lewis's artwork.

The mark is brand/icon-source-dark.png (512x512): an orange cloud with a
tunnel cut from its base and a blue plug set into the opening, on a navy tile.
Home Assistant add-ons take icon.png (128x128) and logo.png (about 250x100)
beside config.yaml; the icon is the tile scaled down, the logo pairs the tile
with a two-line wordmark. The artwork itself is not redrawn here.

brand/icon-family-reference.png is Lewis's sheet for the wider icon family and
is kept as the design reference. Run from the repo root:
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
    """Tile at left, wordmark at right, transparent background. Slate for the
    name so it reads on the light card the store draws logos on."""
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
