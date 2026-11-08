"""Build the exe file icon as a COFF object (.rsrc section) that GNU ld can link directly.

The Rust GNU toolchain ships no windres, so this writes the resource object by hand:
RT_ICON entries (PNG-compressed, Vista+) + one RT_GROUP_ICON named 1.

Usage: python -I tools/make_icon.py assets/sprites/lulu_cutout.png mingw-libs/icon.o [preview.ico]
"""
import io
import struct
import sys

from PIL import Image

SIZES = [16, 24, 32, 48, 64, 128, 256]

src = Image.open(sys.argv[1]).convert("RGBA")
# square crop around the head (top ~60% of the character), like the window icon
w, h = src.size
head_h = int(h * 0.60)
side = max(w, head_h)
sq = Image.new("RGBA", (side, side), (0, 0, 0, 0))
sq.paste(src.crop((0, 0, w, head_h)), ((side - w) // 2, side - head_h))

images = []
for s in SIZES:
    buf = io.BytesIO()
    sq.convert("RGBa").resize((s, s), Image.LANCZOS).convert("RGBA").save(buf, "PNG", optimize=True)
    images.append(buf.getvalue())

if len(sys.argv) > 3:  # optional .ico for previewing
    sq.save(sys.argv[3], sizes=[(s, s) for s in SIZES])

# GRPICONDIR + GRPICONDIRENTRY[]
group = struct.pack("<HHH", 0, 1, len(images))
for i, (s, data) in enumerate(zip(SIZES, images)):
    group += struct.pack("<BBBBHHIH", s % 256, s % 256, 0, 0, 1, 32, len(data), i + 1)

# --- resource tree: type -> name/id -> language -> data entry ---
RT_ICON, RT_GROUP_ICON, LANG = 3, 14, 0x0409
leaves = [(RT_ICON, i + 1, d) for i, d in enumerate(images)] + [(RT_GROUP_ICON, 1, group)]
types = sorted({t for t, _, _ in leaves})


def dir_header(n_ids):
    return struct.pack("<IIHHHH", 0, 0, 0, 0, 0, n_ids)


# Layout: root dir, type dirs, name dirs, data entries, then raw data.
root_size = 16 + 8 * len(types)
type_dirs = {t: [l for l in leaves if l[0] == t] for t in types}
off = root_size
type_off = {}
for t in types:
    type_off[t] = off
    off += 16 + 8 * len(type_dirs[t])
name_off = {}
for l in leaves:
    name_off[(l[0], l[1])] = off
    off += 16 + 8
entry_off = {}
for l in leaves:
    entry_off[(l[0], l[1])] = off
    off += 16
data_off = {}
for l in leaves:
    off = (off + 7) & ~7
    data_off[(l[0], l[1])] = off
    off += len(l[2])
total = off

sec = bytearray(total)


def put(o, b):
    sec[o:o + len(b)] = b


SUBDIR = 0x80000000
put(0, dir_header(len(types)))
for i, t in enumerate(types):
    put(16 + 8 * i, struct.pack("<II", t, SUBDIR | type_off[t]))
for t in types:
    o = type_off[t]
    put(o, dir_header(len(type_dirs[t])))
    for i, l in enumerate(sorted(type_dirs[t], key=lambda x: x[1])):
        put(o + 16 + 8 * i, struct.pack("<II", l[1], SUBDIR | name_off[(l[0], l[1])]))
relocs = []
for l in leaves:
    k = (l[0], l[1])
    put(name_off[k], dir_header(1))
    put(name_off[k] + 16, struct.pack("<II", LANG, entry_off[k]))
    # IMAGE_RESOURCE_DATA_ENTRY: OffsetToData is an RVA -> needs an ADDR32NB relocation
    put(entry_off[k], struct.pack("<IIII", data_off[k], len(l[2]), 0, 0))
    relocs.append(entry_off[k])
    put(data_off[k], l[2])

# --- COFF object ---
IMAGE_FILE_MACHINE_AMD64 = 0x8664
IMAGE_REL_AMD64_ADDR32NB = 3
n_sections, n_symbols = 1, 1
hdr_size = 20 + 40
raw_ptr = hdr_size
reloc_ptr = raw_ptr + total
sym_ptr = reloc_ptr + 10 * len(relocs)
coff = struct.pack("<HHIIIHH", IMAGE_FILE_MACHINE_AMD64, n_sections, 0, sym_ptr, n_symbols, 0, 0)
coff += struct.pack("<8sIIIIIIHHI", b".rsrc", 0, 0, total, raw_ptr, reloc_ptr, 0, len(relocs), 0, 0x40000040)
coff += bytes(sec)
for r in relocs:
    coff += struct.pack("<IIH", r, 0, IMAGE_REL_AMD64_ADDR32NB)
# symbol 0: the section symbol .rsrc (relocations are relative to the section start)
coff += struct.pack("<8sIhHBB", b".rsrc", 0, 1, 0, 3, 0)
coff += struct.pack("<I", 4)  # empty string table
open(sys.argv[2], "wb").write(coff)
print(f"wrote {sys.argv[2]}: {len(images)} icon sizes, {total} bytes of resources")
