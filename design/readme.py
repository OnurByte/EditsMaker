#!/usr/bin/env python3
"""Draws every README picture in assets/readme/, light and dark, from one place.

The rules are the 1976 NASA Graphics Standards Manual's, as in tokens.css: free-standing on the page
(no cards, no rounded boxes), a hairline over each section, a bold label in the left column and
light text in the two right ones, one red accent per view, a folio bottom right.

    python3 design/readme.py      # rewrites assets/readme/*-light.svg and *-dark.svg
"""
import json
import re
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets/readme"
VERSION = re.search(r'^version = "(.+)"', (ROOT / "Cargo.toml").read_text(), re.M)[1]

THEMES = {  # transparent background: the picture sits on GitHub's page like print on paper
    "light": {"INK": "#0B0B0C", "RED": "#E5332A", "GRAY": "#8C877C", "HAIR": "#D8D3C8", "PAPER": "#F4F1EA"},
    "dark": {"INK": "#F4F1EA", "RED": "#E5332A", "GRAY": "#8C877C", "HAIR": "#2A2A2D", "PAPER": "#F4F1EA"},
}
SANS = "Helvetica Neue, Helvetica, Arial, Liberation Sans, sans-serif"
MONO = "ui-monospace, JetBrains Mono, SF Mono, Menlo, monospace"
W, C2, C3 = 960, 320, 640  # page width and the manual's three columns


def t(x, y, s, size=14, w=300, fill="INK", anchor="start", ls=0, font=SANS, raw=False):
    body = s if raw else escape(s)
    return (f'<text x="{x}" y="{y}" font-family="{font}" font-weight="{w}" font-size="{size}" '
            f'fill="%{fill}%" text-anchor="{anchor}" letter-spacing="{ls}">{body}</text>')


def rule(x1, x2, y, c="INK", w=1):
    return f'<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="%{c}%" stroke-width="{w}"/>'


def rect(x, y, w, h, fill="none", stroke=None):
    s = f' stroke="%{stroke}%"' if stroke else ""
    f = f"%{fill}%" if fill != "none" else "none"
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{f}"{s}/>'


def wrap(s, n):
    out, line = [], ""
    for word in s.split():
        if line and len(line) + 1 + len(word) > n:
            out.append(line)
            line = word
        else:
            line = f"{line} {word}".strip()
    return out + [line] if line else out


def head(label, body, per_col=4):
    """Hairline, bold label left, light body flowing from the middle column into the right one."""
    parts = [rule(0, W, 0.5)]
    for i, line in enumerate(label.split("\n")):
        parts.append(t(0, 26 + i * 19, line, 15, 700))
    for i, line in enumerate(wrap(body, 40)):
        col, row = divmod(i, per_col)
        parts.append(t(C2 + col * 320, 26 + row * 19, line))
    return parts


def mark(x, y, s, theme_id):
    e = "M32,18 H74 V32 H32 V41 H74 V55 H32 V64 H74 V78 H32 A14,14 0 0 1 18,64 V32 A14,14 0 0 1 32,18 Z"
    return (f'<g transform="translate({x},{y}) scale({s})"><defs>'
            f'<clipPath id="l{theme_id}"><polygon points="0,0 56,0 44,96 0,96"/></clipPath>'
            f'<clipPath id="r{theme_id}"><polygon points="61,0 96,0 96,96 49,96"/></clipPath></defs>'
            f'<g clip-path="url(#l{theme_id})"><path d="{e}" fill="%INK%"/></g>'
            f'<g clip-path="url(#r{theme_id})"><path d="{e}" fill="%RED%" transform="translate(0,7)"/></g></g>')


# Logos, one colour each (name -> [grid, path]): Simple Icons (CC0), LobeHub icons (MIT) for Codex and
# CapCut, HyperFrames' own favicon. Brands with no free icon (Adobe asked Simple Icons to drop theirs) get
# their initials, set in the type: the manual's way of naming without borrowing artwork.
LOGOS = json.loads((ROOT / "design/logos.json").read_text())
MONOGRAMS = {"Premiere Pro": "Pr", "After Effects": "Ae", "Descript": "D", "auto-editor": "a"}


def logo(name, x, y, size, fill="INK"):
    """The brand's logo in a size x size square whose top left is (x, y)."""
    if name == "EditsMaker":
        return mark(x - size * 0.19, y - size * 0.17, size / 96 * 1.33, f"m{x}{y}")
    if name in MONOGRAMS:
        return t(x + size / 2, y + size * 0.8, MONOGRAMS[name], size * 0.78, 700, fill, "middle", -0.5)
    grid, d = LOGOS[name]
    return f'<path transform="translate({x},{y}) scale({size / grid:.4f})" d="{d}" fill="%{fill}%" fill-rule="evenodd"/>'


PAGES = {}


def page(name, h, aria, folio=None):
    def deco(fn):
        PAGES[name] = (h, aria, folio, fn)
        return fn
    return deco


@page("hero", 210, "EditsMaker. Link in. Edits out.")
def hero():
    x = 246  # the lockup (mark + name, about 496 wide) centred on the page
    return [mark(x - 14, 34, 1.05, "h"), t(x + 92, 118, "EditsMaker", 76, 700, ls=-3.5),
            t(W / 2, 176, "Link in. Edits out.", 26, 300, anchor="middle")]


@page("download", 80, "Download: under 4 MB, one file for Linux, macOS and Windows")
def download():
    """The one call to action: white type on NASA red, the only way the manual lets red carry text."""
    out = [rect(0, 0, W, 80, "RED"), t(28, 52, "Download", 32, 700, "PAPER", ls=-1),
           t(C2, 36, "Under 4 MB", 18, 700, "PAPER"), t(C2, 59, "One file for Linux, macOS and Windows", 15, 300, "PAPER")]
    out += [logo(n, 780 + i * 40, 28, 24, "PAPER") for i, n in enumerate(["Apple", "Linux", "Windows"])]
    return out + ['<path d="M904,40 H936 M926,30 L936,40 L926,50" fill="none" stroke="%PAPER%" stroke-width="3"/>']


@page("sites", 130, "Works with links from YouTube, TikTok, Instagram, X, Facebook, Twitch, Kick, "
      "Dailymotion and 1,700+ more sites", "1.1")
def sites():
    names = ["YouTube", "TikTok", "Instagram", "X", "Facebook", "Twitch", "Kick", "Dailymotion"]
    out = [rule(0, W, 0.5), t(0, 26, "Works with", 15, 700), t(0, 45, "links from", 14, 300)]
    for i, n in enumerate(names):
        r, c = divmod(i, 4)
        out += [logo(n, C2 + c * 160, 13 + r * 32, 20), t(C2 + c * 160 + 28, 30 + r * 32, n, 20, 700, ls=-0.4)]
    return out + [t(C2, 94, "+1,700 more sites", 20, 700, "RED", ls=-0.4)]


@page("badges", 56, "MCP server, 7 agent skills, 50 presets, 40 looks, TikTok trends, 1,700+ sites")
def badges():
    cells = ["MCP server", "7 agent skills", "50 presets", "40 looks", "TikTok trends", "1,700+ sites"]
    widths = [len(c) * 7.6 + 30 for c in cells]
    k = W / sum(widths)
    out, x = [rule(0, W, 0.5), rule(0, W, 47.5)], 0.0
    for c, w in zip(cells, widths):
        if x:
            out.append(f'<line x1="{x:.1f}" y1="10" x2="{x:.1f}" y2="38" stroke="%HAIR%"/>')
        out.append(t(round(x + (12 if x else 0)), 29, c, 13, 700, "RED" if "TikTok" in c else "INK"))
        x += w * k
    return out


@page("agents", 340, "Your AI agent edits for you: EditsMaker is an MCP server for Claude Code, Codex, "
      "Cursor, Gemini CLI and any MCP client", "1.2")
def agents():
    out = head("Your AI agent\nedits for you", "EditsMaker is an MCP server. Ask in plain words; the "
               "agent picks the preset, the song and the lines.")
    for x, label in ((0, "You"), (C2, "MCP tools"), (C3, "Works with")):
        out += [rule(x, x + 296, 100.5), t(x, 122, label, 12, 400, "GRAY")]
    for i, s in enumerate(["“Make 3 sad edits from", "@channel with a trending", "song and words from", "his own videos.”"]):
        out.append(t(0, 152 + i * 24, s, 18, 700, ls=-0.3))
    tools = ["edits_make", "edits_check", "edits_preset_get", "edits_preset_save", "edits_trends", "edits_looks", "edits_doctor"]
    for i, s in enumerate(tools):
        out.append(t(C2, 150 + i * 22, s, 14, 400, "RED" if i == 0 else "INK", font=MONO))
    for i, s in enumerate(["Claude Code", "Codex", "Cursor", "Gemini CLI", "any MCP client"]):
        y = 154 + i * 34
        if s in LOGOS:
            out.append(logo(s, C3, y - 19, 24))
        out.append(t(C3 + 36 if s in LOGOS else C3, y, s, 18, 700 if i < 4 else 300, ls=-0.3))
    return out


# (name, MB, how the size was got, free, open source, built for agents, TikTok trends): 2 yes, 1 partly, 0 no.
# Sources are linked in the README under the picture.
RIVALS = [
    ("EditsMaker", 3.7, "download", 2, 2, 2, 2),
    ("auto-editor", 46, "download", 2, 2, 0, 0),
    ("HyperFrames", 360, "npm + Chrome", 2, 2, 2, 0),
    ("CapCut", 510, "~ download", 1, 0, 0, 1),
    ("Descript", 654, "~ download", 1, 0, 0, 0),
    ("Filmora", 810, "~ download", 1, 0, 0, 0),
    ("DaVinci Resolve", 1700, "~ download", 2, 0, 0, 0),
    ("Premiere Pro", 8000, "to install", 0, 0, 0, 0),
    ("After Effects", 8000, "to install", 0, 0, 0, 0),
]


def dot(cx, cy, v, c):
    """Yes, partly, no: a full, half and empty circle, like the manual's binder holes."""
    r = 6
    if v == 2:
        return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="%{c}%"/>'
    ring = f'<circle cx="{cx}" cy="{cy}" r="{r - 0.5}" fill="none" stroke="%{"GRAY" if v == 0 else c}%"/>'
    return ring + (f'<path d="M{cx},{cy - r} A{r},{r} 0 0 0 {cx},{cy + r} Z" fill="%{c}%"/>' if v == 1 else "")


@page("size", 500, "EditsMaker against other editors. Download: EditsMaker 3.7 MB, auto-editor 46 MB, HyperFrames "
      "about 360 MB with its Chrome, CapCut about 510 MB, Descript about 654 MB, Filmora about 810 MB, DaVinci "
      "Resolve about 1.7 GB, Premiere Pro and After Effects 8 GB each to install. EditsMaker is the only one that "
      "is free, open source, built for AI agents and reads TikTok's trending songs.", "1.3")
def size():
    out = head("Size, and what\nyou get for it", "The whole editor, with its presets, looks, MCP server and skills, "
               "is one file. No Python, no Node, no Electron, no cloud.", 4)
    out += [t(C3, 84, "3.7", 64, 700, "RED", ls=-3), t(C3 + 100, 84, "MB", 22, 700, "RED")]
    top, bar0, span = 132, 190, 300
    cols = [("Free", 680), ("Open source", 760), ("For agents", 840), ("TikTok trends", 920)]
    out += [t(0, top - 12, "Size", 12, 400, "GRAY"), rule(0, W, top - 4.5, "HAIR")]
    out += [t(x, top - 12, h, 12, 400, "GRAY", "middle") for h, x in cols]
    for i, (name, mb, how, *marks) in enumerate(RIVALS):
        y, us = top + 8 + i * 34, i == 0
        c = "RED" if us else "INK"
        w = max(2, round(mb / RIVALS[-1][1] * span))
        val = f"{mb / 1000:g} GB" if mb >= 1000 else f"{mb:g} MB"
        out += [logo(name, 0, y, 20, c), t(30, y + 15, name, 14, 700, c),
                rect(bar0, y + 3, w, 14, "RED" if us else "GRAY"),
                t(bar0 + w + 8, y + 15, val, 13, 700, c), t(bar0 + w + 12 + len(val) * 7.6, y + 15, how, 11, 400, "GRAY")]
        out += [dot(x, y + 10, v, c) for (_, x), v in zip(cols, marks)]
        out.append(rule(0, W, y + 26.5, "HAIR"))
    out += [dot(5, 478, 2, "INK"), t(16, 482, "yes", 11, 400, "GRAY"), dot(49, 478, 1, "INK"),
            t(60, 482, "partly", 11, 400, "GRAY"), dot(112, 478, 0, "INK"),
            t(123, 482, "no", 11, 400, "GRAY")]
    return out


@page("sounds", 340, "Any sound, any trend: paste a TikTok, Reel, Short, YouTube or SoundCloud link and EditsMaker "
      "takes its sound, or ask for TikTok's trending songs in any country with --music trend:3", "1.4")
def sounds():
    out = head("Any sound,\nany trend", "Paste a TikTok, Reel, Short or SoundCloud link and edits takes its sound. "
               "Or ask for trend:3, the 3rd song on TikTok's chart in your country this week, found on YouTube "
               "with the part people replay most.", 3)
    plats = ["TikTok", "Instagram", "YouTube Shorts", "YouTube", "YouTube Music", "SoundCloud", "X", "Twitch"]
    out.append(rule(0, W, 100.5))
    for i, n in enumerate(plats):
        x = i * 120
        out += [logo(n, x, 120, 30, "RED" if i == 0 else "INK"),
                t(x, 172, "Reels" if n == "Instagram" else n, 13, 700 if i == 0 else 400, "RED" if i == 0 else "INK")]
    cmds = [("--music trend:3", "TikTok's 3rd trending song, here"), ("--music trend:us:1", "number one in the US"),
            ("--music https://www.tiktok.com/…", "the sound of that TikTok"), ("edits trends us", "the chart itself")]
    out.append(rule(0, W, 196.5, "HAIR"))
    for i, (cmd, what) in enumerate(cmds):
        y = 222 + i * 28
        out += [t(0, y, cmd, 13, 400, "RED" if "trend:" in cmd else "INK", font=MONO), t(C2, y, what, 14, 300)]
    return out


@page("loop", 220, "How an agent makes an edit good: idea, footage, lines, render, check, fix", "2.1")
def loop():
    out = head("Render, look, fix", "A first render is a draft. The agent measures it and looks at it "
               "before you do.")
    steps = [("idea", "one feeling, one line", 0), ("footage", "captions, frames", 0), ("lines", "words on screen", 0),
             ("render", "edits_make", 1), ("check", "edits_check", 1), ("fix", "preset, lines, song", 0)]
    out.append(rule(0, W, 90.5))
    for i, (name, sub, mono) in enumerate(steps):
        x = i * 160
        out += [t(x, 116, str(i + 1), 12, 400, "GRAY"), t(x, 142, name, 20, 700, ls=-0.4),
                t(x, 164, sub, 13, 400 if mono else 300, font=MONO if mono else SANS)]
    out.append('<path d="M802,178 V194 H482 V182" fill="none" stroke="%RED%" stroke-width="1.5"/>'
               '<path d="M477,184 L482,176 L487,184 Z" fill="%RED%"/>')
    return out + [t(642, 210, "until it is good", 12, 400, "RED", anchor="middle")]


@page("steps", 130, "Link or topic, pick a preset, pick a song, your edit", "2.2")
def steps():
    out = [rule(0, W, 0.5)]
    for i, s in enumerate(["Link or topic", "Pick a preset", "Pick a song", "Your edit"]):
        c = "RED" if i == 3 else "INK"
        out += [t(i * 240, 70, str(i + 1), 60, 700, c, ls=-3), t(i * 240, 104, s, 18, 700, c, ls=-0.3)]
    return out


@page("features", 230, "Presets, trends, looks, agents, reads captions first, every frame size", "3.1")
def features():
    cells = [("Presets", "50 real edits, as JSON", "an agent can change"), ("Trends", "TikTok's chart,", "one flag: trend:3"),
             ("Looks", "40 looks", "and your own LUTs"), ("Agents", "MCP server", "and 7 skills"),
             ("Reads first", "captions before video:", "downloads only the best"), ("Every frame", "9:16 to 21:9", "or any WxH")]
    out = []
    for i, (h, a, b) in enumerate(cells):
        r, c = divmod(i, 3)
        x, y = c * 320, r * 110
        out += [rule(x, x + 296, y + 0.5), t(x, y + 30, h, 18, 700, ls=-0.3), t(x, y + 54, a), t(x, y + 73, b)]
    return out


@page("fx", 290, "Everything an edit needs: speed ramps, freeze, face tracking, hits, captions, looks, LUTs, "
      "music ducking, stabilize, ProRes", "3.2")
def fx():
    out = head("Everything an edit needs", "From the editors you know, in one command.")
    cols = [("Motion", ["velocity ramps", "speed 0.1x-8x", "freeze frame", "zoom punch", "face tracking", "reframe 9:16"]),
            ("Hits", ["shake", "flash", "RGB split", "glitch", "negative", "blur hit"]),
            ("Text", ["11 text styles", "word highlight", "hook title", "words on screen", "SRT"]),
            ("Colour and sound", ["40 looks", "your LUTs", "music ducking", "whoosh / boom", "denoise", "loudness"]),
            ("Finish", ["stabilize", "smooth slow-mo", "H.265 / ProRes / AV1", "any frame size", "FCPXML / EDL"])]
    for i, (h, items) in enumerate(cols):
        x = i * 192
        out += [rule(x, x + 176, 80.5), t(x, 104, h, 15, 700, "RED" if i == 1 else "INK")]
        out += [t(x, 132 + j * 21, s) for j, s in enumerate(items)]
    return out


@page("basics", 580, "The basics, done for you: jump cuts, word-by-word captions, a hook on top, music ducked "
      "under the voice, cuts on the beat, faces followed into 9:16", "3.3")
def basics():
    out = head("The basics, done for you", "Every talking clip gets these before you ask for anything.")
    rows = [("Jump cuts", "Pauses cut out, every other cut punches in."),
            ("Captions", "Word by word, 11 styles, burned in."),
            ("Hook", "A title on top for the first seconds."),
            ("Music bed", "The song under the voice, ducked while they talk."),
            ("Beat cuts", "Shots land on the song's beats."),
            ("Face reframe", "Faces followed into 9:16, from any frame size.")]
    for i, (h, d) in enumerate(rows):
        y = 84 + i * 80
        x, y0 = C3, y + 16
        out += [rule(0, W, y + 0.5, "HAIR"), t(0, y + 30, h, 16, 700)]
        out += [t(C2, y + 30 + j * 19, s) for j, s in enumerate(wrap(d, 40))]
        if i == 0:  # speech with pauses, then tight
            for a, b in ((0, 60), (100, 170), (200, 260)):
                out.append(rect(x + a, y0 + 4, b - a, 10, "INK"))
            for a, b in ((60, 100), (170, 200)):
                out.append(rect(x + a + 0.5, y0 + 4.5, b - a - 1, 9, stroke="HAIR"))
            out += [rect(x, y0 + 28, 60, 10, "INK"), rect(x + 62, y0 + 28, 70, 10, "RED"), rect(x + 134, y0 + 28, 60, 10, "INK")]
        elif i == 1:
            out.append(t(x, y0 + 30, 'NOBODY <tspan fill="%RED%">SAW</tspan> THIS', 26, 700, ls=-0.5, raw=True))
        elif i == 2:
            out += [rect(x + 0.5, y0 - 4.5, 27, 48, stroke="INK"), rect(x + 4, y0 + 1, 20, 5, "RED"),
                    t(x + 44, y0 + 26, "wait for it", 20, 700, ls=-0.3)]
        elif i == 3:
            for a, b in ((0, 70), (110, 200), (240, 300)):
                out.append(rect(x + a, y0, b - a, 10, "INK"))
            out.append(f'<path d="M{x},{y0 + 40} H{x + 0} L{x + 6},{y0 + 30} H{x + 66} L{x + 76},{y0 + 40} H{x + 106} '
                       f'L{x + 112},{y0 + 30} H{x + 196} L{x + 206},{y0 + 40} H{x + 236} L{x + 242},{y0 + 30} H{x + 300}" '
                       f'fill="none" stroke="%RED%" stroke-width="2"/>')
        elif i == 4:
            for k in range(10):
                out.append(f'<line x1="{x + 4 + k * 32}" y1="{y0}" x2="{x + 4 + k * 32}" y2="{y0 + 10}" stroke="%GRAY%"/>')
                if k % 2 == 0:
                    out.append(f'<line x1="{x + 4 + k * 32}" y1="{y0 + 18}" x2="{x + 4 + k * 32}" y2="{y0 + 44}" '
                               f'stroke="%RED%" stroke-width="2"/>')
        else:
            out += [rect(x + 0.5, y0 - 3.5, 85, 48, stroke="INK"), f'<circle cx="{x + 52}" cy="{y0 + 14}" r="7" fill="%INK%"/>',
                    rect(x + 39, y0 - 3.5, 26, 47, stroke="RED"), t(x + 110, y0 + 26, "16:9 → 9:16", 20, 300)]
    return out


@page("presets", 300, "Real edits as data: 50 Alight Motion, CapCut and hand-made edits, 40 looks and 178 "
      "Premiere cut moves, every shot and keyframe as editable JSON", "3.4")
def presets():
    out = head("Real edits, as data", "50 edits, 40 looks, 178 cut moves. Every shot, ramp, keyframe and layer "
               "is JSON you can change.")
    out.append(rect(C2 + 0.5, 100.5, 639, 132, stroke="INK"))
    x, cuts = C2 + 10, [80, 50, 120, 60, 90, 70, 100, 50]
    for j, name in enumerate(["shots", "zoom", "shake", "speed"]):
        out.append(t(C2 - 16, 126 + j * 30, name, 12, 400, "GRAY", anchor="end"))
    zoom, shake, speed = [], [], []
    for k, w in enumerate(cuts):
        out.append(rect(x, 112, w - 4, 20, "RED" if k == 2 else "GRAY"))
        zoom += [f"{x},{168}", f"{x + w * 0.6:.0f},{148}", f"{x + w - 4},{150}"]
        shake += [f"{x + s},{(184 if s % 8 else 194) if k in (2, 5) else 189}" for s in range(0, w - 4, 4)]
        speed += [f"{x},{226 if k % 3 else 210}", f"{x + w - 4},{226 if k % 3 else 210}"]
        x += w
    out += [f'<polyline points="{" ".join(p)}" fill="none" stroke="%{c}%" stroke-width="1.5"/>'
            for p, c in ((zoom, "RED"), (shake, "INK"), (speed, "INK"))]
    js = ('{"start": 3.9, "end": 4.73, "speed": 1.02, <tspan fill="%RED%">"zoom"</tspan>: '
          '[[0, 1.09, "linear"], [0.51, 1.9, "out 2.9"]]}')
    return out + [t(0, 268, js, 12, 400, font=MONO, raw=True)]


@page("why", 150, "Why EditsMaker: one command for a link from 1,700+ sites, trend-aware, reads first, "
      "under 4 MB: one file on your computer", "3.5")
def why():
    cols = [("One command", "A link from 1,700+ sites, a channel or a topic in; finished edits out."),
            ("Trend-aware", "Reads TikTok's chart and cuts to the song people are using this week."),
            ("Reads first", "Skims every video's captions and only downloads the good ones."),
            ("Under 4 MB", "One file, no installer, runs on your own computer.")]
    out = []
    for i, (h, d) in enumerate(cols):
        x = i * 240
        out += [rule(x, x + 220, 0.5), t(x, 30, h, 18, 700, "RED" if i == 3 else "INK", ls=-0.3)]
        out += [t(x, 56 + j * 19, s) for j, s in enumerate(wrap(d, 28))]
    return out


def main():
    for name, (h, aria, folio, fn) in PAGES.items():
        body = fn()
        if folio:
            body.append(t(W, h - 4, folio, 11, 400, "GRAY", anchor="end"))
        svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{h}" viewBox="0 0 {W} {h}" '
               f'role="img" aria-label="{escape(aria, {chr(34): "&quot;"})}">\n' + "\n".join(body) + "\n</svg>\n")
        for theme, colors in THEMES.items():
            out = svg
            for k, v in colors.items():
                out = out.replace(f"%{k}%", v)
            assert "%" not in re.sub(r">[^<]*<", "><", out), f"unreplaced token in {name}"
            (OUT / f"{name}-{theme}.svg").write_text(out)
    print(f"{len(PAGES) * 2} pictures written to {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
