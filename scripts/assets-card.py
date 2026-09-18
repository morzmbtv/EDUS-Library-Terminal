"""Rebuild the supplied EDUS card as editable SVG paths.

This is an asset build tool, not a runtime dependency. It uses Pillow and NumPy.
The original colored wordmark is never modified. Its silhouette is traced for
the white card-only variant. The ornament is a documented reconstruction.
"""
from pathlib import Path
import hashlib
import json
import math
import struct
import xml.etree.ElementTree as ET

import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "public" / "assets"


def distance(point, start, end):
    dx, dy = end[0] - start[0], end[1] - start[1]
    if dx == dy == 0:
        return math.hypot(point[0] - start[0], point[1] - start[1])
    t = max(0, min(1, ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / (dx * dx + dy * dy)))
    return math.hypot(point[0] - start[0] - t * dx, point[1] - start[1] - t * dy)


def simplify(points, epsilon):
    if len(points) < 3:
        return points
    i, d = max(enumerate(distance(p, points[0], points[-1]) for p in points), key=lambda v: v[1])
    if d <= epsilon:
        return [points[0], points[-1]]
    return simplify(points[:i + 1], epsilon)[:-1] + simplify(points[i:], epsilon)


def trace(alpha, epsilon=0.23):
    """Subpixel marching-squares contours, including internal white cutouts."""
    values = np.pad(alpha, 1)
    edges = {}
    points = {}

    def add(a, b):
        ka, kb = tuple(round(v, 6) for v in a), tuple(round(v, 6) for v in b)
        points[ka], points[kb] = a, b
        edges.setdefault(ka, []).append(kb)
        edges.setdefault(kb, []).append(ka)

    for y in range(values.shape[0] - 1):
        for x in range(values.shape[1] - 1):
            v = (values[y, x], values[y, x + 1], values[y + 1, x + 1], values[y + 1, x])
            coords = ((x - .5, y - .5), (x + .5, y - .5), (x + .5, y + .5), (x - .5, y + .5))
            crossings = []
            for i in range(4):
                j = (i + 1) % 4
                if (v[i] > .5) != (v[j] > .5):
                    t = (.5 - v[i]) / (v[j] - v[i])
                    crossings.append(tuple(coords[i][k] + t * (coords[j][k] - coords[i][k]) for k in range(2)))
            if len(crossings) == 2:
                add(*crossings)
            elif len(crossings) == 4:
                add(crossings[0], crossings[1])
                add(crossings[2], crossings[3])

    paths = []
    while edges:
        first = next(iter(edges))
        current = first
        loop = [points[first]]
        while True:
            nxt = edges[current][0]
            edges[current].remove(nxt)
            if not edges[current]:
                del edges[current]
            edges[nxt].remove(current)
            if not edges[nxt]:
                del edges[nxt]
            current = nxt
            loop.append(points[current])
            if current == first:
                break
        area = abs(sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(loop, loop[1:])) / 2)
        if area < 1:
            continue
        loop = simplify(loop, epsilon)
        paths.append("M" + " L".join(f"{x:.2f} {y:.2f}" for x, y in loop[:-1]) + " Z")
    return " ".join(paths)


logo = Image.open(ASSETS / "edus-logo.png").convert("RGB")
logo_alpha = np.max(255 - np.asarray(logo).astype(float), axis=2) / 255
logo_path = trace(logo_alpha)

# The URL is also outlined, so the card has no system-font dependency.
font = ImageFont.truetype(str(ASSETS / "fonts" / "NotoSans-Variable.ttf"), 104)
bbox = font.getbbox("www.edus.kz")
url_img = Image.new("L", (bbox[2] - bbox[0] + 4, bbox[3] - bbox[1] + 4))
ImageDraw.Draw(url_img).text((2 - bbox[0], 2 - bbox[1]), "www.edus.kz", font=font, fill=255)
url_path = trace(np.asarray(url_img).astype(float) / 255, .25)
url_scale = 177 / url_img.width
url_height = url_img.height * url_scale

# Symmetric ram-horn ornament, recreated from the card photograph. All geometry
# is editable; no portion of the photographed plastic is embedded.
ornament = '''
      <g id="ornament-half" fill="none" stroke="currentColor" stroke-width="6.5" stroke-linejoin="round" stroke-linecap="round">
        <path d="M-4 6 C-13-9-43-13-51 4 C-60 25-48 44-28 44 C-10 44-8 22-19 17 C-30 12-39 20-34 28 C-31 32-26 30-25 26"/>
        <path d="M-5 54 C-20 44-47 48-53 65 C-59 83-43 95-25 91 C-7 86-9 66-20 65 C-27 64-34 70-29 76"/>
        <path d="M-51 76 C-55 100-48 116-49 133 C-41 121-32 112-22 104"/>
        <path d="M-7 86 C-3 112-18 141-37 173 C-54 202-67 229-66 253 C-65 276-45 290-26 282 C-10 276-11 254-25 251 C-35 249-43 256-40 264 C-37 271-28 270-27 263"/>
        <path d="M-4 179 C-5 192-8 201-22 201 C-27 200-31 195-34 195 C-33 206-25 211-13 210 C-15 222-18 229-27 231"/>
      </g>
      <g id="ornament-unit">
        <use href="#ornament-half"/>
        <use href="#ornament-half" transform="scale(-1 1)"/>
        <path d="M0 171 L-7 189 L0 203 L7 189 Z" fill="currentColor"/>
      </g>'''

svg = f'''<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="600" height="960" viewBox="0 0 600 960" role="img" aria-labelledby="title description">
  <title id="title">Карта EDUS</title>
  <desc id="description">Чистая векторная реконструкция карты по фотографии. Синий фон #046BC8, тёмно-синий орнамент, золотой знак #E39300, белый логотип EDUS и www.edus.kz. Орнамент восстановлен, это не исходный типографский макет.</desc>
  <defs>
    <clipPath id="card-corners"><rect width="600" height="960" rx="36"/></clipPath>
    {ornament}
  </defs>
  <g id="card" clip-path="url(#card-corners)">
    <rect id="brand-blue-background" width="600" height="960" fill="#046BC8"/>
    <g id="reconstructed-kazakh-ornament" color="#0B1A4D" opacity=".40" transform="translate(310 -8)">
      <use href="#ornament-unit"/>
      <use href="#ornament-unit" transform="translate(0 300)"/>
      <use href="#ornament-unit" transform="translate(0 600)"/>
      <use href="#ornament-unit" transform="translate(0 900)"/>
    </g>
    <g id="edus-card-symbol">
      <rect x="208" y="272" width="204" height="204" rx="35" fill="#E39300"/>
      <path d="M245 309 H310 V329 H265 V404 H310 V424 H245 Z M314 330 H375 V444 H314 V424 H355 V350 H314 Z" fill="#FFFFFF"/>
    </g>
    <g id="white-edus-wordmark" transform="translate(180 515) scale(.55675)" fill="#FFFFFF" fill-rule="evenodd">
      <path d="{logo_path}"/>
    </g>
    <g id="website-address" transform="translate(221.5 {682 - url_height:.3f}) scale({url_scale:.7f})" fill="#FFFFFF" fill-rule="evenodd">
      <path d="{url_path}"/>
    </g>
  </g>
</svg>
'''
(ASSETS / "edus-card.svg").write_text(svg, encoding="utf-8")

tree = ET.fromstring(svg)
assert not tree.findall(".//{http://www.w3.org/2000/svg}image")
assert "base64" not in svg
print(json.dumps({"svg": str(ASSETS / "edus-card.svg"), "dimensions": [600, 960], "path_count": len(tree.findall('.//{http://www.w3.org/2000/svg}path')), "embedded_raster": False, "logo_sha256": hashlib.sha256((ASSETS / 'edus-logo.png').read_bytes()).hexdigest()}, ensure_ascii=False))
