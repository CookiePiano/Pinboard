"""Draws installer/assets/{sidebar,header}.{svg,bmp} for setup and both uninstallers.

Run from source/ with a Python that has Pillow, e.g. Inkscape's bundled one:
    python -I scripts/installer-art.py
Optional arguments: <icon_png> <out_dir> (defaults: src-tauri/icons/128x128@2x.png, installer/assets).
The sidebar is 2x MUI's 164x314 welcome bitmap and the header 2x its 150x57 image, so they
stay sharp at 200% scaling. The art carries branding only (wordmark, note colors); every
sentence stays a native, translatable label.
"""
import base64
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

INKSCAPE = r"C:\Program Files\Inkscape\bin\inkscape.com"
HARBOR = "#1C2B36"  # must equal PB_HEADER_BG in installer/branding.nsh
HARBOR_DEEP = "#22394A"
MEMO = {"amber": "#F2DEB6", "sage": "#CFE2D6", "lilac": "#DCD4EF", "rose": "#F1D2D7", "frost": "#E9EEF3"}

source = Path(__file__).resolve().parent.parent
dog_png = Path(sys.argv[1]) if len(sys.argv) > 1 else source / "src-tauri" / "icons" / "128x128@2x.png"
out = Path(sys.argv[2]) if len(sys.argv) > 2 else source / "installer" / "assets"
out.mkdir(parents=True, exist_ok=True)
dog = "data:image/png;base64," + base64.b64encode(dog_png.read_bytes()).decode()


def card(color: str, angle: float, cx: float, cy: float, size: float) -> str:
    x, y = cx - size / 2, cy - size / 2
    return (f'<g transform="rotate({angle} {cx} {cy})" filter="url(#shadow)">'
            f'<rect x="{x}" y="{y}" width="{size}" height="{size}" rx="9" fill="{color}" fill-opacity="0.94"/>'
            f'<rect x="{x}" y="{y}" width="{size}" height="18" rx="9" fill="#FFFFFF" fill-opacity="0.35"/></g>')


DEFS = f"""<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{HARBOR}"/><stop offset="1" stop-color="{HARBOR_DEEP}"/>
  </linearGradient>
  <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
    <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.16"/><stop offset="1" stop-color="#FFFFFF" stop-opacity="0"/>
  </radialGradient>
  <filter id="shadow" x="-30%" y="-30%" width="160%" height="160%">
    <feGaussianBlur in="SourceAlpha" stdDeviation="7"/><feOffset dy="6"/>
    <feComponentTransfer><feFuncA type="linear" slope="0.38"/></feComponentTransfer>
    <feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge>
  </filter>
</defs>"""

dots = "".join(f'<circle cx="{44 + i * 18}" cy="584" r="5" fill="{MEMO[name]}"/>'
               for i, name in enumerate(["frost", "sage", "lilac", "amber", "rose"]))
sidebar = f"""<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="328" height="628" viewBox="0 0 328 628">
{DEFS}
<rect width="328" height="628" fill="url(#bg)"/>
<circle cx="164" cy="262" r="190" fill="url(#glow)"/>
{card(MEMO['lilac'], -13, 150, 250, 158)}
{card(MEMO['sage'], 9, 182, 244, 158)}
{card(MEMO['amber'], -3, 164, 268, 158)}
<image x="62" y="150" width="204" height="204" xlink:href="{dog}" filter="url(#shadow)"/>
<text x="40" y="552" font-family="Segoe UI Semibold, Segoe UI" font-weight="600" font-size="34" fill="#FFFFFF" letter-spacing="0.3">Pinboard</text>
{dots}
</svg>"""

header = f"""<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="300" height="114" viewBox="0 0 300 114">
{DEFS}
<rect width="300" height="114" fill="{HARBOR}"/>
<circle cx="236" cy="57" r="70" fill="url(#glow)"/>
<g transform="rotate(-8 238 64)"><rect x="207" y="36" width="62" height="62" rx="6" fill="{MEMO['amber']}" fill-opacity="0.85"/></g>
<image x="196" y="16" width="84" height="84" xlink:href="{dog}"/>
</svg>"""

with tempfile.TemporaryDirectory() as work:
    for name, svg in (("sidebar", sidebar), ("header", header)):
        svg_path = out / f"{name}.svg"
        png_path = Path(work) / f"{name}.png"
        svg_path.write_text(svg, encoding="utf-8", newline="\n")
        subprocess.run([INKSCAPE, str(svg_path), "--export-type=png", f"--export-filename={png_path}", "--export-area-page"], check=True, capture_output=True)
        image = Image.open(png_path).convert("RGBA")
        flat = Image.new("RGB", image.size, HARBOR)  # NSIS needs an opaque 24-bit BMP
        flat.paste(image, mask=image.split()[3])
        flat.save(out / f"{name}.bmp", format="BMP")
        print(name, flat.size)
