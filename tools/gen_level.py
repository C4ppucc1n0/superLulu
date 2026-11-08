"""Design-time helper: writes src/levels/level1.txt (the ASCII level the game embeds).

After generation the .txt file is the source of truth - edit it directly if you like.
Coordinates: column c (0 = left), row r (0 = top, 19 = bottom). Normal ground top is row 17.

Legend (also documented at the top of the generated file):
  #  ground          X  stone block      =  jump-through platform   C  crumbling block
  ^  floor spikes    v  ceiling spikes   o  orange (coin)           *  golden orange (3)
  S  shield bubble   P  player start     K  checkpoint              G  goal gate (bottom-left)
  e  jelly (stomp)   h  hedgehog (spiky, do NOT stomp)              f  frog (hops at you)
  <  turret firing left   >  turret firing right
  H  horizontal moving platform (run of H = one platform)   V  vertical moving platform
  |  invisible bumper: turns moving platforms and walking enemies around
"""
import sys
from pathlib import Path

W, H = 290, 20
g = [[" "] * W for _ in range(H)]


def put(c, r, ch):
    g[r][c] = ch


def rect(c0, c1, r0, r1, ch):
    for r in range(r0, r1 + 1):
        for c in range(c0, c1 + 1):
            g[r][c] = ch


def ground(c0, c1, top):
    rect(c0, c1, top, H - 1, "#")


def row(c0, c1, r, ch):
    rect(c0, c1, r, r, ch)


# ------------------------------------------------------------------ 1: Orange Meadow (warm-up)
ground(0, 11, 17)
put(3, 16, "P")
row(6, 8, 14, "o")
# pit 12-14
row(12, 14, 15, "o")
ground(15, 24, 17)
row(18, 20, 14, "X")
row(18, 20, 13, "o")
put(22, 16, "e")
ground(25, 28, 15)
# pit 29-32
row(29, 32, 12, "o")
ground(33, 44, 17)
row(36, 40, 14, "=")
row(36, 39, 13, "o")
put(40, 13, "S")
put(41, 16, "f")
# spike pit 45-49 with a single stone pillar in the middle
ground(45, 49, 19)
row(45, 49, 18, "^")
rect(47, 47, 14, 18, "X")
put(47, 10, "*")  # golden #1: full jump straight up from the pillar
put(46, 13, "o")
put(48, 13, "o")
ground(50, 58, 17)
put(53, 16, "e")
put(57, 16, "e")
ground(59, 59, 16)
ground(60, 60, 15)
ground(61, 61, 14)
ground(62, 64, 13)
row(65, 68, 11, "o")  # 4-wide drop into segment 2

# ------------------------------------------------------------------ 2: Machinery
ground(69, 76, 17)
# spike pit 77-90 with jump-through stepping platforms
ground(77, 90, 19)
row(77, 90, 18, "^")
row(79, 80, 15, "=")
row(83, 84, 13, "=")
row(87, 88, 15, "=")
row(82, 85, 9, "v")  # no full jumps from the middle platform!
put(79, 14, "o"); put(80, 14, "o")
put(83, 12, "o"); put(84, 12, "o")
put(87, 14, "o"); put(88, 14, "o")
ground(91, 95, 17)
# crumbling bridge over the void, two 1-tile holes, a jelly patrolling between them
for c in range(96, 108):
    if c not in (100, 104):
        put(c, 17, "C")
put(102, 16, "e")
row(97, 99, 15, "o")
row(105, 107, 15, "o")
ground(108, 112, 17)
# moving platforms over the void
put(113, 15, "|")
row(115, 117, 15, "H")
put(123, 15, "|")
put(123, 13, "|")
row(124, 126, 13, "H")
put(131, 13, "|")
row(124, 130, 9, "v")  # don't jump high on the second platform
row(118, 120, 13, "o")
ground(131, 135, 17)
# elevator up the wall
row(133, 134, 16, "V")
row(133, 134, 9, "|")
row(133, 134, 8, "o")
ground(136, 143, 10)

# ------------------------------------------------------------------ 3: Critter Gauntlet
put(140, 9, "K")
put(142, 9, "S")
ground(144, 152, 17)
put(150, 16, "h")
row(145, 147, 14, "o")
# narrow pillars over the void
ground(155, 156, 14)
ground(159, 160, 13)
ground(163, 164, 14)
ground(167, 168, 12)
put(163, 13, "f")
put(155, 12, "o"); put(159, 11, "o"); put(163, 11, "o"); put(167, 10, "o")
# turret alley
ground(171, 186, 17)
ground(187, 189, 12)
put(187, 16, "<")
put(187, 13, "<")
row(183, 184, 14, "=")
row(174, 180, 15, "o")
put(176, 16, "e")
# hedgehog yard with a safe(r) upper route
ground(190, 199, 17)
put(193, 16, "h")
put(197, 16, "h")
row(192, 196, 14, "=")
row(192, 196, 13, "o")
# 5-wide pit: needs a running jump
put(202, 12, "X")
put(202, 11, "*")  # golden #2: long running jump from the upper route
row(200, 204, 14, "o")
ground(205, 211, 17)
put(209, 16, "f")

# ------------------------------------------------------------------ 4: The Climb & the Sky Road
ground(212, 223, 17)
rect(212, 212, 3, 14, "X")   # left shaft wall (entrance at rows 15-16)
rect(223, 223, 5, 16, "X")   # right shaft wall
put(216, 16, "S")
row(219, 221, 14, "=")        # P1
row(214, 216, 12, "=")        # P2
row(218, 219, 9, "C")         # P3 (crumbles)
row(213, 215, 7, "=")         # P4
row(217, 218, 5, "C")         # P5 (crumbles)
row(220, 226, 4, "X")         # exit ledge
row(221, 222, 5, "v")
put(223, 13, "<")
put(223, 10, "<")
put(223, 6, "<")
put(213, 4, "*")              # golden #3
put(220, 13, "o"); put(215, 11, "o"); put(218, 8, "o"); put(214, 6, "o"); put(217, 4, "o")
# sky road
put(227, 6, "|")
row(228, 229, 6, "H")
put(237, 6, "|")
for c in (239, 242, 245):
    put(c, 5, "C")
    put(c, 4, "o")
row(238, 246, 1, "v")  # ceiling spikes: only short running hops between the crumbles
put(246, 7, "|")
row(247, 248, 7, "H")
put(253, 7, "|")
rect(254, 258, 8, 19, "X")
put(256, 7, "h")
# final run
ground(259, 289, 17)
row(263, 267, 16, "^")
row(262, 268, 14, "=")
put(265, 13, "e")
row(263, 267, 12, "o")
put(272, 16, "f")
put(276, 16, "h")
row(279, 280, 16, "^")
put(283, 16, "G")
row(270, 275, 13, "o")

# ------------------------------------------------------------------ output
SEGMENTS = [(0, 70, "1 Orange Meadow"), (70, 140, "2 Machinery"), (140, 212, "3 Critter Gauntlet"),
            (212, 290, "4 The Climb & the Sky Road")]
out = Path(sys.argv[1])
lines = [line.rstrip("\n") for line in __doc__.splitlines()[6:16]]
text = ["; SUPER LULU - level 1", "; Each '---' block is a slice of the level; blocks are joined left-to-right and",
        "; bottom-aligned, so a block may have fewer rows than the level height (20).",
        "; Lines starting with ';' are comments."] + ["; " + l.strip() for l in lines if l.strip()]
for c0, c1, name in SEGMENTS:
    text.append(f"--- segment {name} (columns {c0}-{c1 - 1})")
    rows = ["".join(g[r][c0:c1]) for r in range(H)]
    text += [r.rstrip().ljust(c1 - c0) for r in rows]
out.write_text("\n".join(text) + "\n", encoding="utf-8")
print("wrote", out, "items:", sum(r.count("o") for r in g), "oranges,", sum(r.count("*") for r in g), "golden")
