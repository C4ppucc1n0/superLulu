"""Cut the Lulu character out of the phone screenshot and export game-ready sprites.

Usage: python -I tools/cutout.py <source.jpeg> <out_dir>

Outputs (in out_dir):
  lulu_cutout.png   full-resolution transparent cutout (reference / future edits)
  lulu.png          in-game sprite, 160 px tall, with a thin dark outline
  lulu_title.png    title-screen sprite, 420 px tall, with outline
  icon16/32/64.rgba raw RGBA window icons (head crop)
  preview.png       contact sheet on light + dark backgrounds for checking
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage as ndi

src, out = Path(sys.argv[1]), Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)

rgb = np.asarray(Image.open(src).convert("RGB")).astype(np.float32)
H, W, _ = rgb.shape

# --- 1. Remove the "内容由AI生成" watermark (grey text overlapping the head edge) ---
zone = np.zeros((H, W), bool)
zone[1150:1410, 25:95] = True
mx, mn = rgb.max(2), rgb.min(2)
sat = (mx - mn) / np.maximum(mx, 1)
text = zone & (sat < 0.22) & (mx > 40)
text = ndi.binary_dilation(text, iterations=3) & zone
# fill every text pixel with the colour of the nearest non-text pixel
_, (iy, ix) = ndi.distance_transform_edt(text, return_indices=True)
rgb = rgb[iy, ix]

# --- 2. Background = dark pixels connected to the image border ---
bg_col = np.median(np.concatenate([rgb[:, :4].reshape(-1, 3), rgb[:, -4:].reshape(-1, 3)]), 0)
dist = np.abs(rgb - bg_col).max(2)
T = 22.0
cand = dist < T
lab, n = ndi.label(cand)
border = np.unique(np.concatenate([lab[0], lab[-1], lab[:, 0], lab[:, -1]]))
bg = np.isin(lab, border[border > 0])
fg = ~bg

# --- 3. Keep only the character (largest foreground island; drops status bar, X button, home bar) ---
lab, n = ndi.label(fg)
sizes = ndi.sum(np.ones_like(lab), lab, range(1, n + 1))
char = lab == (1 + int(np.argmax(sizes)))
char = ndi.binary_fill_holes(char)

# --- 4. Soft edge: partial alpha for the anti-aliased ring just outside the solid mask ---
ring = ndi.binary_dilation(char, iterations=2) & ~char
alpha = char.astype(np.float32)
alpha[ring] = np.clip(dist[ring] / (T * 3), 0, 1)
# slight erosion-smoothing of the hard edge
alpha = np.maximum(alpha * (ndi.uniform_filter(char.astype(np.float32), 3) > 0.34), alpha * ring)
a = np.clip(alpha, 0, 1)
# un-mix the dark background from edge pixels
safe = np.maximum(a, 1e-3)[..., None]
col = np.clip((rgb - (1 - a[..., None]) * bg_col) / safe, 0, 255)

# --- 5. Rebuild the head's left edge where the watermark glyphs overlapped it ---
# Fit the edge curve x = f(y) from clean rows above/below the glyphs, then redraw the
# alpha from that curve and copy the edge colour profile from the clean rows.
PATCH0, PATCH1 = 1168, 1292            # rows touched by 内/容/由
REF0, REF1 = 1160, 1294                # clean reference rows
DEPTH = 30                             # how far into the head we repaint


def left_edge(y):
    row = char[y, :260]
    return int(np.argmax(row)) if row.any() else None


fit_rows = [y for y in list(range(1060, 1166, 2)) + list(range(1296, 1336, 2)) if left_edge(y) is not None]
coef = np.polyfit(fit_rows, [left_edge(y) for y in fit_rows], 3)
e_ref0, e_ref1 = left_edge(REF0), left_edge(REF1)
for y in range(PATCH0, PATCH1):
    e = float(np.polyval(coef, y))
    t = (y - REF0) / (REF1 - REF0)
    for x in range(0, int(e) + DEPTH):
        a[y, x] = np.clip(x - e + 0.5, 0, 1)
        k = x - int(e)
        c0 = col[REF0, min(max(e_ref0 + k, 0), W - 1)]
        c1 = col[REF1, min(max(e_ref1 + k, 0), W - 1)]
        if k >= 0:
            col[y, x] = c0 * (1 - t) + c1 * t
        else:
            col[y, x] = col[REF0, e_ref0] * (1 - t) + col[REF1, e_ref1] * t
    # feather the seam between repainted and original pixels
    for k in range(4):
        x = int(e) + DEPTH + k
        w = (k + 1) / 5
        col[y, x] = col[y, x] * w + col[y, x - 1] * (1 - w)

ys, xs = np.nonzero(a > 0.02)
y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
rgba = np.dstack([col, a * 255])[y0:y1, x0:x1].round().astype(np.uint8)
full = Image.fromarray(rgba, "RGBA")
full.save(out / "lulu_cutout.png")
print(f"cutout bbox x={x0}..{x1} y={y0}..{y1}  size={full.size}")


def resize_h(img: Image.Image, h: int) -> Image.Image:
    w = round(img.width * h / img.height)
    return img.convert("RGBa").resize((w, h), Image.LANCZOS).convert("RGBA")


def outlined(img: Image.Image, px: int, color=(58, 34, 22)) -> Image.Image:
    """Add a dark cartoon outline of `px` pixels so the render reads well in-game."""
    pad = px + 1
    arr = np.asarray(img).astype(np.float32)
    arr = np.pad(arr, ((pad, pad), (pad, pad), (0, 0)))
    al = arr[..., 3] / 255
    # grow the alpha by `px` with a soft disk
    yy, xx = np.mgrid[-px:px + 1, -px:px + 1]
    disk = (xx * xx + yy * yy) <= px * px + 0.5
    grown = ndi.maximum_filter(al, footprint=disk)
    grown = ndi.uniform_filter(grown, 2)
    o = np.zeros_like(arr)
    o[..., :3] = color
    o[..., 3] = grown * 255
    # composite sprite over outline
    fa = al[..., None]
    outc = arr[..., :3] * fa + o[..., :3] * (1 - fa)
    outa = np.maximum(al, grown)
    res = np.dstack([outc, outa * 255]).clip(0, 255).round().astype(np.uint8)
    return Image.fromarray(res, "RGBA")


game = outlined(resize_h(full, 156), 2)
game.save(out / "lulu.png")
title = outlined(resize_h(full, 410), 4)
title.save(out / "lulu_title.png")
print("game sprite", game.size, "title sprite", title.size)

# --- icons: square crop around the head (top ~62% of the character) ---
fw, fh = full.size
head_h = int(fh * 0.60)
side = max(fw, head_h)
sq = Image.new("RGBA", (side, side), (0, 0, 0, 0))
sq.paste(full.crop((0, 0, fw, head_h)), ((side - fw) // 2, side - head_h))
for s in (16, 32, 64):
    ic = sq.convert("RGBa").resize((s, s), Image.LANCZOS).convert("RGBA")
    (out / f"icon{s}.rgba").write_bytes(ic.tobytes())
    ic.save(out / f"icon{s}.png")

# --- preview sheet ---
sheet = Image.new("RGBA", (game.width * 4 + title.width + 60, max(title.height, game.height) + 20), (0, 0, 0, 0))
x = 10
for bgc in [(135, 206, 250, 255), (40, 40, 48, 255)]:
    tile = Image.new("RGBA", (game.width + 10, game.height + 10), bgc)
    tile.alpha_composite(game, (5, 5))
    sheet.alpha_composite(tile, (x, 10))
    x += tile.width + 5
tile = Image.new("RGBA", (title.width + 10, title.height + 10), (150, 210, 120, 255))
tile.alpha_composite(title, (5, 5))
sheet.alpha_composite(tile, (x, 10))
sheet.save(out / "preview.png")
