"""Render the Glassboard logo set. Requires rsvg-convert and Python Pillow."""

from io import BytesIO
from pathlib import Path
import shutil
import subprocess
from xml.etree import ElementTree as ET

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]
BRAND = ROOT / "assets" / "brand"
MINT = "#A3E9D1"
INK = "#143D33"
SVG_NS = "{http://www.w3.org/2000/svg}"
source = ET.parse(BRAND / "mark.svg").getroot()
paths = "\n".join(
    '<path ' + " ".join(f'{key}="{value}"' for key, value in node.attrib.items()) + "/>"
    for node in source.findall(f"{SVG_NS}path")
)


def svg(content, viewbox="0 0 24 24"):
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{viewbox}" fill="none">\n'
        f"<title>Glassboard</title>\n{content}\n</svg>\n"
    )


def render(document, size):
    output = subprocess.run(
        ["rsvg-convert", "--width", str(size), "--height", str(size)],
        input=document.encode(), capture_output=True, check=True,
    ).stdout
    return Image.open(BytesIO(output)).convert("RGBA")


def save_variant(name, color):
    document = svg(paths.replace("currentColor", color))
    (BRAND / f"{name}.svg").write_text(document)
    return document


dark = save_variant("mark-dark", INK)
white = save_variant("mark-white", "#FFFFFF")
mint = save_variant("mark-mint", MINT)
black = svg(paths.replace("currentColor", "#000000"))

for size in (16, 22, 32, 44):
    render(black, size).save(BRAND / f"menu-bar-{size}.png")

app = svg(
    f'<rect x="64" y="64" width="896" height="896" rx="200" fill="{MINT}"/>\n'
    f'<g transform="translate(160 160) scale(29.3333333333)">\n'
    + paths.replace("currentColor", INK) + "\n</g>",
    "0 0 1024 1024",
)
avatar = svg(
    f'<path fill="{MINT}" d="M0 0H1024V1024H0Z"/>\n'
    f'<g transform="translate(160 160) scale(29.3333333333)">\n'
    + paths.replace("currentColor", INK) + "\n</g>",
    "0 0 1024 1024",
)
for name, document in (("app-icon", app), ("github-avatar", avatar)):
    (BRAND / f"{name}.svg").write_text(document)
    render(document, 1024).save(BRAND / f"{name}.png")

app_image = render(app, 1024)
app_image.save(BRAND / "app-icon.icns", format="ICNS")
app_image.save(BRAND / "app-icon.ico", format="ICO", sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])

favicon = svg(
    '<style>path{stroke:#143D33}@media(prefers-color-scheme:dark){path{stroke:#A3E9D1}}</style>\n'
    + paths
)
(BRAND / "favicon.svg").write_text(favicon)
for size in (16, 32):
    render(avatar, size).save(BRAND / f"favicon-{size}.png")
render(avatar, 256).save(BRAND / "favicon.ico", format="ICO", sizes=[(16, 16), (32, 32), (48, 48)])

# Ship the same generated artwork in native bundles and the browser preview.
native_icons = ROOT / "apps" / "desktop" / "src-tauri" / "icons"
public = ROOT / "apps" / "desktop" / "public"
native_icons.mkdir(parents=True, exist_ok=True)
public.mkdir(parents=True, exist_ok=True)
for extension in ("png", "icns", "ico"):
    shutil.copyfile(BRAND / f"app-icon.{extension}", native_icons / f"icon.{extension}")
# Tauri's macOS tray implementation displays images at 18 logical points.
render(black, 36).save(native_icons / "tray-template.png")
render(app, 32).save(native_icons / "tray.png")
for name in ("favicon.svg", "favicon.ico"):
    shutil.copyfile(BRAND / name, public / name)

# A review sheet made from the same source assets, with small icons at actual size.
sheet = Image.new("RGB", (1200, 720), "#F6F8F7")
draw = ImageDraw.Draw(sheet)
font_path = "/System/Library/Fonts/Helvetica.ttc"
try:
    heading = ImageFont.truetype(font_path, 30)
    label = ImageFont.truetype(font_path, 19)
    wordmark = ImageFont.truetype(font_path, 54)
except OSError:
    heading = ImageFont.load_default(size=30)
    label = ImageFont.load_default(size=19)
    wordmark = ImageFont.load_default(size=54)


def place(document, size, position):
    im = render(document, size)
    sheet.paste(im, position, im)


draw.text((52, 40), "Glassboard / Marked pane", font=heading, fill=INK)
draw.text((52, 108), "App icon", font=label, fill=INK)
place(app, 248, (48, 150))
draw.text((416, 108), "Website", font=label, fill=INK)
place(dark, 80, (410, 185))
draw.text((506, 191), "Glassboard", font=wordmark, fill=INK)
draw.rounded_rectangle((398, 294, 1148, 415), 20, fill=INK)
place(mint, 80, (410, 315))
draw.text((506, 321), "Glassboard", font=wordmark, fill="#FFFFFF")
draw.text((52, 471), "GitHub avatar", font=label, fill=INK)
place(avatar, 136, (60, 516))
draw.text((416, 471), "Menu bar / actual pixel sizes", font=label, fill=INK)
draw.rounded_rectangle((398, 518, 1148, 578), 12, fill="#E4EAE7")
draw.rounded_rectangle((398, 592, 1148, 652), 12, fill=INK)
for x, size in ((432, 16), (624, 22), (820, 32)):
    place(dark, size, (x, 548 - size // 2))
    place(white, size, (x, 622 - size // 2))
    draw.text((x + 48, 536), f"{size} px", font=label, fill=INK)
    draw.text((x + 48, 610), f"{size} px", font=label, fill="#FFFFFF")
sheet.save(BRAND / "preview.png")
print(f"Generated logo assets in {BRAND}")
