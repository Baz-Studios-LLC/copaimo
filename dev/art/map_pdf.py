"""Composes the labelled map at print resolution and lays it onto an A3 PDF page."""
import io
import json
import os

from PIL import Image, ImageDraw, ImageFont

ROOT = r"C:/Users/jsull/Desktop/copaimo"
MAP = os.path.join(ROOT, "dev", "art", "map")

world = Image.open(os.path.join(MAP, "world.png")).convert("RGB")
key = json.load(io.open(os.path.join(MAP, "world.json"), encoding="utf-8"))

# The source is 4 m a pixel; doubling it puts the finished art well above what an
# A3 page at 300 dpi needs, so the downsample into the PDF is what sharpens it.
SCALE = 2
K = SCALE * 2.0  # everything drawn on top scales with the art, or it shrinks away
world = world.resize((world.width * SCALE, world.height * SCALE), Image.LANCZOS)

INK = (14, 17, 22)
PAD_L, PAD_T, PAD_B = int(34 * K), int(92 * K), int(118 * K)
sheet = Image.new("RGB", (world.width + PAD_L * 2, world.height + PAD_T + PAD_B), INK)
sheet.paste(world, (PAD_L, PAD_T))
d = ImageDraw.Draw(sheet)


def font(size, bold=False):
    names = ("seguisb.ttf", "segoeui.ttf") if bold else ("segoeui.ttf",)
    for name in names:
        try:
            return ImageFont.truetype(name, int(size * K))
        except OSError:
            pass
    return ImageFont.load_default()


title, sub = font(38, True), font(17)
land_f, land_s = font(27, True), font(14)
small, tiny = font(14), font(12, True)

d.text((PAD_L, int(22 * K)), "COPAIMO", font=title, fill=(238, 232, 218))
d.text((PAD_L + int(200 * K), int(36 * K)), "THE WARDENS GUILD  ·  WORLD MAP",
       font=sub, fill=(150, 158, 150))
w_m, h_m = key["world"]
d.text((PAD_L, int(64 * K)), f"{w_m/1000:.1f} km x {h_m/1000:.1f} km",
       font=sub, fill=(120, 128, 126))
d.rectangle([PAD_L - 1, PAD_T - 1, PAD_L + world.width, PAD_T + world.height],
            outline=(64, 72, 80), width=max(1, int(K / 2)))


def at(px, pz):
    return PAD_L + px * SCALE, PAD_T + pz * SCALE


for place in key["places"]:
    x, y = at(*place["at"])
    if place["kind"] == "town":
        r = 3 * K
        d.ellipse([x - r, y - r, x + r, y + r], fill=(196, 200, 208), outline=(30, 34, 40))
    else:
        r = (7 if place["kind"] == "ranch" else 6) * K
        colour = (255, 214, 122) if place["kind"] == "ranch" else (236, 236, 240)
        d.rectangle([x - r, y - r, x + r, y + r], fill=colour,
                    outline=(24, 28, 34), width=max(1, int(K)))

for place in key["places"]:
    if place["kind"] == "ranch":
        x, y = at(*place["at"])
        d.text((x + 12 * K, y - 8 * K), "THE RANCH", font=tiny, fill=(255, 214, 122))

def ink_over(box):
    """Light text on dark ground, dark text on light. Fell's name was white on
    snow and its area all but invisible - a label has to read on the country it
    sits on, and that country is different for every continent."""
    x0, y0, x1, y1 = (max(0, int(v)) for v in box)
    patch = sheet.crop((x0, y0, min(sheet.width, x1), min(sheet.height, y1)))
    if patch.width < 1 or patch.height < 1:
        return (246, 242, 232), (12, 14, 18)
    small_patch = patch.resize((8, 8), Image.BILINEAR)
    px = list(small_patch.getdata())
    lum = sum(0.2126 * r + 0.7152 * g + 0.0722 * b for r, g, b in px) / len(px)
    if lum > 150:
        return (24, 26, 30), (250, 250, 252)      # dark ink, pale halo
    return (246, 242, 232), (12, 14, 18)          # pale ink, dark halo


for mass in key["landmasses"]:
    x, y = at(*mass["at"])
    name = mass["name"].upper()
    box = d.textbbox((0, 0), name, font=land_f)
    w = box[2] - box[0]
    h = box[3] - box[1]
    note = f"{mass['km2']:.1f} km²"
    nb = d.textbbox((0, 0), note, font=land_s)
    ink, halo_ink = ink_over(
        (x - w / 2, y - 18 * K, x + w / 2, y + 18 * K + (nb[3] - nb[1]))
    )
    halo = max(2, int(K))
    for ox in (-halo, 0, halo):
        for oy in (-halo, 0, halo):
            if ox or oy:
                d.text((x - w / 2 + ox, y - 18 * K + oy), name, font=land_f, fill=halo_ink)
    d.text((x - w / 2, y - 18 * K), name, font=land_f, fill=ink)
    nx = x - (nb[2] - nb[0]) / 2
    for ox in (-1, 0, 1):
        for oy in (-1, 0, 1):
            if ox or oy:
                d.text((nx + ox * K / 2, y + 12 * K + oy * K / 2), note,
                       font=land_s, fill=halo_ink)
    d.text((nx, y + 12 * K), note, font=land_s, fill=ink)

LY = PAD_T + world.height + int(20 * K)
swatches = [((96, 132, 72), "Grassland"), ((58, 96, 56), "Forest"),
            ((201, 176, 118), "Desert"), ((232, 236, 240), "Snow"),
            ((130, 126, 118), "Rock"), ((206, 194, 156), "Shore"),
            ((150, 140, 116), "Settled"), ((38, 74, 108), "Sea")]
x = PAD_L
for colour, label in swatches:
    s_ = 15 * K
    d.rectangle([x, LY, x + s_, LY + s_], fill=colour, outline=(60, 66, 74))
    d.text((x + 21 * K, LY + 1), label, font=small, fill=(178, 184, 190))
    x += 21 * K + d.textbbox((0, 0), label, font=small)[2] + 22 * K

LY2 = LY + int(26 * K)
x = PAD_L
d.rectangle([x, LY2 + 2, x + 11 * K, LY2 + 13 * K], fill=(236, 236, 240), outline=(24, 28, 34))
d.text((x + 19 * K, LY2), "City (Warden Exam)", font=small, fill=(178, 184, 190))
x += 175 * K
d.ellipse([x + 2, LY2 + 4, x + 11 * K, LY2 + 13 * K], fill=(196, 200, 208), outline=(30, 34, 40))
d.text((x + 19 * K, LY2), "Town", font=small, fill=(178, 184, 190))
x += 95 * K
d.rectangle([x, LY2 + 2, x + 11 * K, LY2 + 13 * K], fill=(255, 214, 122), outline=(24, 28, 34))
d.text((x + 19 * K, LY2), "The ranch — where you begin", font=small, fill=(178, 184, 190))

cities = sum(1 for p in key["places"] if p["kind"] == "city")
towns = sum(1 for p in key["places"] if p["kind"] == "town")
tally = f"{cities} cities · {towns} towns · {len(key['landmasses'])} landmasses"
tb = d.textbbox((0, 0), tally, font=small)
d.text((sheet.width - PAD_L - (tb[2] - tb[0]), LY2), tally, font=small, fill=(140, 148, 154))

mpp = key["metres_per_pixel"] / SCALE
bar_px = 2000 / mpp
bx = sheet.width - PAD_L - bar_px
by = PAD_T + world.height - int(26 * K)
d.rectangle([bx, by, bx + bar_px, by + 6 * K], fill=(238, 236, 230), outline=(20, 22, 26))
d.rectangle([bx, by, bx + bar_px / 2, by + 6 * K], fill=(40, 44, 50), outline=(20, 22, 26))
d.text((bx, by - 17 * K), "0", font=tiny, fill=(238, 236, 230))
d.text((bx + bar_px - 16 * K, by - 17 * K), "2 km", font=tiny, fill=(238, 236, 230))

png = os.path.join(MAP, "copaimo-world-map.png")
sheet.save(png)
print("art", sheet.size, "->", png)

# ---------------------------------------------------------------- the page
DPI = 300
PAGE = (int(16.54 * DPI), int(11.69 * DPI))  # A3 landscape
page = Image.new("RGB", PAGE, INK)
fit = min((PAGE[0] - int(0.30 * DPI)) / sheet.width, (PAGE[1] - int(0.30 * DPI)) / sheet.height)
art = sheet.resize((max(1, int(sheet.width * fit)), max(1, int(sheet.height * fit))), Image.LANCZOS)
page.paste(art, ((PAGE[0] - art.width) // 2, (PAGE[1] - art.height) // 2))

pdf = os.path.join(MAP, "copaimo-world-map.pdf")
page.save(pdf, "PDF", resolution=DPI, title="Copaimo — World Map")
print("page", page.size, f"A3 landscape at {DPI} dpi ->", pdf,
      f"{os.path.getsize(pdf)/1e6:.1f} MB")
