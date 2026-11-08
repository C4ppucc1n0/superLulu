"""Design-time helper: writes src/levels/level1.txt .. level3.txt (the ASCII levels the game embeds).

After generation the .txt files are the source of truth - edit them directly if you like.
Coordinates: column c (0 = left), row r (0 = top). Run `cargo test --release` afterwards:
the reach tests check that every level can be finished and every orange can be reached.

Design rules (easy + fun): plain gaps <= 3-4 tiles, steps up <= 2-3 tiles, platforms >= 3 wide,
no ceiling spikes, a checkpoint at the start of every section.
"""
from pathlib import Path

LEGEND = """\
; Each '---' block is a slice of the level; blocks are joined left-to-right and bottom-aligned.
; Lines starting with ';' are comments, lines starting with '@' are settings:
;   @name <text>   @theme meadow|factory|sky   @wind <updraft height in tiles>   @signN <text, '|' = new line>
; #  ground          X  stone block      =  jump-through platform   C  crumbling cookie
; ^  floor spikes    o  orange           *  golden orange (3)       +  heart
; S  shield bubble   P  player start     K  checkpoint              G  goal gate (bottom-left)
; e  jelly (stomp)   h  hedgehog (spiky: throw oranges or avoid)    f  frog (hops at you)
; <  turret firing left   >  turret firing right
; H  horizontal moving platform (run of H = one platform)   V  vertical moving platform
; |  invisible bumper: turns moving platforms and walking enemies around
; T  jelly spring    U  fan (updraft above it)   Q  cookie box (3 oranges)   R  cookie box (heart)
; %  fake ground (secret room: looks solid, you can walk through it)   \\ /  slides (\\ goes down to the right, / down to the left)
; 1-9  signpost (text from @signN)
"""


class Level:
    def __init__(self, w, h, name, theme, wind=8):
        self.w, self.h = w, h
        self.g = [[" "] * w for _ in range(h)]
        self.meta = [f"@name {name}", f"@theme {theme}", f"@wind {wind}"]
        self.name = name

    def put(self, c, r, ch):
        self.g[r][c] = ch

    def rect(self, c0, c1, r0, r1, ch):
        for r in range(r0, r1 + 1):
            for c in range(c0, c1 + 1):
                self.g[r][c] = ch

    def row(self, c0, c1, r, ch):
        self.rect(c0, c1, r, r, ch)

    def ground(self, c0, c1, top, ch="#"):
        self.rect(c0, c1, top, self.h - 1, ch)

    def slide(self, c, r, n, d=1):
        """n slide tiles starting at the top (c, r), going down to the right (d=1) or left (d=-1)."""
        for i in range(n):
            x = c + i * d
            self.put(x, r + i, "\\" if d > 0 else "/")
            self.rect(x, x, r + i + 1, self.h - 1, "#")

    def fans(self, c0, c1, r):
        self.row(c0, c1, r, "U")
        self.rect(c0, c1, r + 1, self.h - 1, "X")

    def sign(self, n, c, r, text):
        self.put(c, r, str(n))
        self.meta.append(f"@sign{n} {text}")

    def write(self, path, block=60):
        lines = [f"; SUPER LULU - {self.name}", LEGEND.rstrip("\n")] + self.meta
        for x0 in range(0, self.w, block):
            x1 = min(self.w, x0 + block)
            lines.append(f"--- columns {x0}-{x1 - 1}")
            for r in range(self.h):
                lines.append("".join(self.g[r][x0:x1]))
        Path(path).write_text("\n".join(lines) + "\n", encoding="utf-8")
        flat = "".join("".join(r) for r in self.g)
        print(f"{path}: {self.w}x{self.h}, {flat.count('o')} oranges, {flat.count('Q')} boxes, "
              f"{flat.count('*')} goldens, {flat.count('K')} checkpoints")


def level1():
    L = Level(178, 20, "Orange Meadow", "meadow")
    # ---------------------------------------------------------------- A: learn the moves
    L.ground(0, 14, 17)
    L.put(3, 16, "P")
    L.sign(1, 5, 16, "Hold jump to jump higher.|Hold X to run.")
    L.row(7, 9, 14, "o")
    L.sign(2, 10, 16, "Bump cookie boxes from below!|A hole only costs one heart.")
    L.put(12, 13, "Q")
    L.row(15, 16, 14, "o")  # pit 15-16
    L.ground(17, 24, 17)
    L.put(21, 16, "e")
    L.ground(25, 28, 15)
    L.row(25, 28, 13, "o")
    L.row(29, 31, 12, "o")  # pit 29-31
    L.ground(32, 45, 17)
    L.sign(3, 34, 16, "Press C to throw an orange.|It knocks out any critter,|even a spiky hedgehog!")
    L.put(35, 16, "|")
    L.put(38, 16, "h")
    L.put(41, 16, "|")
    L.row(40, 42, 12, "=")  # perch next to the spring
    L.put(41, 11, "*")  # golden 1
    L.put(44, 17, "T")  # spring up the cliff
    L.ground(46, 51, 12)
    L.row(47, 50, 10, "o")
    L.slide(52, 12, 5)
    L.row(53, 56, 11, "o")
    L.ground(57, 59, 17)
    for c, r in [(60, 14), (61, 13), (62, 13), (63, 14)]:  # pit 60-63
        L.put(c, r, "o")
    # ---------------------------------------------------------------- B: glide + secret room
    L.ground(64, 76, 17)
    L.put(66, 16, "K")
    L.sign(4, 69, 16, "Jump, then press jump AGAIN|and HOLD it to glide!")
    for c, r in [(77, 13), (78, 12), (79, 11), (80, 11), (81, 11), (82, 12), (83, 13), (84, 14)]:  # pit 77-84
        L.put(c, r, "o")
    L.ground(85, 110, 17)
    L.row(86, 88, 15, "=")
    L.row(86, 88, 13, "o")
    L.ground(90, 99, 13)  # hill with a secret room inside
    L.rect(90, 97, 14, 16, "%")  # the whole room looks like ground until you walk in
    L.row(91, 95, 16, "o")
    L.put(96, 16, "*")  # golden 2
    L.row(88, 89, 16, "o")  # breadcrumbs into the wall
    L.row(91, 98, 11, "o")
    L.put(104, 16, "f")
    L.put(110, 15, "|")
    L.row(111, 113, 15, "H")  # pit 111-118
    L.put(119, 15, "|")
    L.row(112, 117, 12, "o")
    # ---------------------------------------------------------------- C: cookie bridge + spring + glide
    L.ground(119, 131, 17)
    L.put(121, 16, "K")
    L.put(125, 13, "R")
    L.put(129, 16, "e")
    L.row(132, 141, 17, "C")  # crumbling bridge over a pit
    L.row(133, 140, 15, "o")
    L.ground(142, 177, 17)
    L.put(143, 16, "|")
    L.put(145, 16, "e")
    L.put(148, 16, "|")
    L.put(151, 17, "T")
    L.row(148, 154, 12, "=")
    L.row(149, 153, 10, "o")
    for c, r in [(156, 10), (158, 10), (160, 11)]:
        L.put(c, r, "o")
    L.row(162, 165, 12, "X")
    L.put(164, 11, "*")  # golden 3
    L.row(158, 160, 16, "o")
    L.put(169, 16, "G")
    return L


def level2():
    L = Level(190, 20, "Cookie Factory", "factory", wind=9)
    # ---------------------------------------------------------------- A: seed shooter, chute, cookie bridge
    L.ground(0, 12, 17)
    L.put(2, 16, "P")
    L.sign(1, 4, 16, "Welcome to the Cookie Factory!|Jump over the seeds - or stomp them.")
    L.row(6, 9, 14, "o")
    L.put(8, 12, "Q")
    L.row(13, 15, 14, "o")  # pit 13-15
    L.ground(16, 29, 17)
    L.put(20, 16, "e")
    L.row(23, 25, 14, "=")
    L.row(23, 25, 12, "o")
    L.rect(28, 29, 13, 16, "X")  # tower with a seed shooter at its foot
    L.put(28, 16, "<")
    L.slide(30, 13, 4)
    L.row(31, 33, 12, "o")
    L.ground(34, 44, 17)
    L.put(35, 16, "|")
    L.put(39, 16, "h")
    L.put(43, 16, "|")
    L.put(41, 13, "R")
    L.row(45, 52, 17, "C")
    L.row(46, 51, 15, "o")
    # ---------------------------------------------------------------- B: updrafts, trapdoor, fan hopping
    L.ground(53, 64, 17)
    L.put(55, 16, "K")
    L.sign(2, 59, 16, "Fans blow wind upward.|Glide into the wind to ride it up!")
    L.fans(65, 67, 17)
    L.put(66, 13, "o")
    L.put(66, 10, "o")
    L.ground(68, 80, 10)  # high walkway
    L.rect(69, 76, 12, 14, "%")  # secret room under a cookie trapdoor
    L.rect(71, 72, 10, 11, "%")
    L.put(72, 15, "T")
    L.row(69, 70, 14, "o")
    L.row(74, 75, 14, "o")
    L.put(76, 14, "+")
    L.row(76, 79, 8, "o")
    L.slide(81, 10, 7)
    L.row(82, 87, 8, "o")
    L.ground(88, 92, 17)
    L.fans(95, 97, 17)  # fan hopping over a pit 93-114
    L.fans(102, 104, 17)
    L.fans(109, 111, 17)
    L.put(103, 7, "*")  # golden 1: top of the middle updraft
    L.row(99, 100, 13, "o")
    L.row(106, 107, 13, "o")
    L.row(112, 113, 13, "o")
    # ---------------------------------------------------------------- C: moving platform, spikes, hedgehog ledge
    L.ground(115, 125, 17)
    L.put(118, 16, "K")
    L.put(125, 15, "|")
    L.row(126, 128, 15, "H")  # pit 126-137
    L.put(138, 15, "|")
    L.row(128, 135, 12, "o")
    L.put(131, 10, "*")  # golden 2: jump from the moving platform
    L.ground(138, 150, 17)
    L.put(143, 16, "f")
    L.put(146, 13, "Q")
    L.row(151, 153, 14, "o")  # pit 151-153
    L.ground(154, 189, 17)
    L.row(157, 158, 16, "^")
    L.row(162, 163, 16, "^")
    L.row(156, 160, 13, "=")
    L.row(157, 159, 11, "o")
    L.row(161, 164, 14, "o")
    L.put(168, 17, "T")
    L.row(165, 171, 12, "=")
    L.row(172, 181, 12, "X")  # ledge with a hedgehog guarding golden 3
    L.put(176, 11, "h")
    L.put(180, 11, "*")  # golden 3
    L.row(166, 170, 10, "o")
    L.put(184, 16, "G")
    return L


def level3():
    L = Level(192, 22, "Sky World", "sky", wind=10)
    # ---------------------------------------------------------------- A: glide gaps, spring tower
    L.ground(0, 12, 19)
    L.put(2, 18, "P")
    L.sign(1, 4, 18, "Welcome to Sky World!|Glide from cloud to cloud.")
    L.row(6, 9, 16, "o")
    L.row(13, 15, 16, "o")  # gap 13-15
    L.ground(16, 25, 18)
    L.put(21, 17, "e")
    L.put(19, 14, "Q")
    for c, r in [(26, 15), (27, 14), (28, 13), (29, 13), (30, 13), (31, 13), (32, 14), (33, 15), (34, 16)]:  # gap 26-34
        L.put(c, r, "o")
    L.ground(35, 46, 19)
    L.sign(2, 37, 18, "Jelly pads bounce you up.|Bounce your way to the top!")
    L.put(44, 19, "T")
    L.row(42, 48, 14, "=")
    L.put(49, 14, "T")
    L.row(47, 53, 9, "=")
    L.row(43, 46, 12, "o")
    L.put(53, 6, "*")  # golden 1
    for c, r in [(56, 9), (58, 10), (60, 11), (62, 12)]:  # big gap 47-63: glide down
        L.put(c, r, "o")
    # ---------------------------------------------------------------- B: updraft chain
    L.ground(64, 74, 18)
    L.put(66, 17, "K")
    L.sign(3, 68, 17, "Glide INTO the wind to rise,|then glide on to the next fan.")
    L.fans(78, 80, 19)
    L.fans(88, 90, 17)
    L.fans(98, 100, 15)
    L.fans(108, 110, 17)
    for c, r in [(79, 15), (79, 12), (89, 13), (89, 10), (99, 11), (99, 8), (109, 13), (109, 10)]:
        L.put(c, r, "o")
    L.put(104, 6, "*")  # golden 2: glide off the top of the tallest updraft
    L.row(114, 116, 10, "o")
    # ---------------------------------------------------------------- C: moving clouds, secret cloud, lift
    L.ground(119, 130, 18)
    L.put(121, 17, "K")
    L.put(125, 15, "S")
    L.put(130, 16, "|")
    L.row(131, 133, 16, "H")  # gap 131-145
    L.put(146, 16, "|")
    L.row(133, 143, 13, "o")
    L.ground(146, 156, 18)
    L.put(152, 17, "f")  # walk under the cloud (2 tiles of headroom)
    L.rect(149, 155, 12, 15, "#")  # floating cloud with a secret room
    L.rect(149, 154, 13, 14, "%")
    L.row(150, 153, 14, "o")
    L.put(154, 14, "+")
    L.row(147, 148, 13, "o")
    L.row(157, 164, 18, "C")
    L.row(158, 163, 16, "o")
    L.ground(165, 175, 17)
    L.put(169, 16, "e")
    L.row(176, 178, 7, "|")  # lift to golden 3
    L.row(176, 178, 16, "V")
    L.row(176, 178, 18, "|")
    L.row(179, 184, 8, "=")
    L.put(183, 7, "*")  # golden 3
    L.row(177, 177, 10, "o")
    L.row(177, 177, 13, "o")
    L.ground(179, 191, 18)
    L.put(185, 17, "G")
    return L


if __name__ == "__main__":
    out = Path(__file__).resolve().parent.parent / "src" / "levels"
    for i, make in enumerate([level1, level2, level3], start=1):
        make().write(out / f"level{i}.txt")
