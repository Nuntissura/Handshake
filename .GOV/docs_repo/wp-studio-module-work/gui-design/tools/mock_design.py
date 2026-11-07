"""mock-design.html: design domain, proposed Interface Layout Preset. Layout is a designer proposal (D-08)."""
from mock_lib import MAIN_H, cbf, contextbar, dd, foot, group, layer_row, menubar, page, pr, pr2, rail_bottom, section, statusbar, sv, tabstrip, toolrail, write, badge
from mock_icons import ti, ui

INK, MUTE, BG, CARD, GREEN = "#17201b", "#5f6b64", "#f4f6f3", "#ffffff", "#2f7d57"
FONT = 'font-family="Inter,Segoe UI,sans-serif"'


def card(x, y, title, sub, tint):
    return (
        f'<rect x="{x}" y="{y}" width="288" height="96" rx="12" fill="{CARD}" stroke="#e4e8e2"/>'
        f'<rect x="{x+10}" y="{y+10}" width="76" height="76" rx="8" fill="{tint}"/><path d="M{x+14} {y+78}l18-26 12 14 10-10 28 22Z" fill="#fff" opacity=".65"/>'
        f'<text x="{x+98}" y="{y+36}" {FONT} font-size="15" font-weight="600" fill="{INK}">{title}</text>'
        f'<text x="{x+98}" y="{y+58}" {FONT} font-size="12" fill="{MUTE}">{sub}</text>'
    )


def phone_home():
    return (
        f'<rect width="320" height="660" fill="{BG}"/>'
        f'<text x="16" y="52" {FONT} font-size="24" font-weight="700" fill="{INK}">Trailhead</text>'
        f'<rect x="16" y="72" width="288" height="40" rx="10" fill="{CARD}" stroke="#e4e8e2"/><text x="32" y="97" {FONT} font-size="13" fill="{MUTE}">Search nearby trails</text>'
        f'<text x="16" y="146" {FONT} font-size="13" font-weight="600" fill="{MUTE}">NEAR YOU</text>'
        + card(16, 160, "Pier Loop", "3.2 km, easy", "#7fb7a0")
        + card(16, 268, "Cliff Walk", "8.4 km, moderate", "#c59a6a")
        + card(16, 376, "Harbour Ridge", "11.0 km, hard", "#7f96b7")
        + f'<rect x="16" y="492" width="288" height="44" rx="10" fill="{GREEN}"/><text x="160" y="519" text-anchor="middle" {FONT} font-size="14" font-weight="600" fill="#fff">Start a walk</text>'
        + f'<rect y="596" width="320" height="64" fill="{CARD}"/><path d="M0 596h320" stroke="#e4e8e2"/>'
        + "".join(f'<circle cx="{x}" cy="620" r="9" fill="{"%s" % GREEN if x==56 else "#c9d1cb"}"/><text x="{x}" y="645" text-anchor="middle" {FONT} font-size="10" fill="{MUTE}">{t}</text>' for x, t in ((56, "Home"), (160, "Saved"), (264, "Profile")))
    )


def phone_detail():
    return (
        f'<rect width="320" height="660" fill="{BG}"/><rect width="320" height="240" fill="#7fb7a0"/><path d="M0 240l90-110 60 70 50-46 120 86Z" fill="#fff" opacity=".45"/>'
        f'<text x="16" y="282" {FONT} font-size="24" font-weight="700" fill="{INK}">Pier Loop</text>'
        f'<text x="16" y="306" {FONT} font-size="13" fill="{MUTE}">3.2 km, 45 min, easy</text>'
        + "".join(f'<rect x="{16+i*98}" y="326" width="90" height="64" rx="10" fill="{CARD}" stroke="#e4e8e2"/><text x="{61+i*98}" y="354" text-anchor="middle" {FONT} font-size="16" font-weight="700" fill="{INK}">{a}</text><text x="{61+i*98}" y="376" text-anchor="middle" {FONT} font-size="11" fill="{MUTE}">{b}</text>' for i, (a, b) in enumerate((("42 m", "Climb"), ("4.8", "Rating"), ("Open", "Status"))))
        + f'<text x="16" y="424" {FONT} font-size="13" fill="{INK}"><tspan x="16">A flat loop past the old pier and tug</tspan><tspan x="16" dy="20">basin. Best at dusk, with benches at</tspan><tspan x="16" dy="20">the far end.</tspan></text>'
        + f'<rect x="16" y="580" width="288" height="44" rx="10" fill="{GREEN}"/><text x="160" y="607" text-anchor="middle" {FONT} font-size="14" font-weight="600" fill="#fff">Save trail</text>'
    )


def phone_profile():
    return (
        f'<rect width="320" height="660" fill="{BG}"/><circle cx="160" cy="96" r="44" fill="#c59a6a"/>'
        f'<text x="160" y="170" text-anchor="middle" {FONT} font-size="20" font-weight="700" fill="{INK}">Sam Rivera</text><text x="160" y="192" text-anchor="middle" {FONT} font-size="13" fill="{MUTE}">212 km this year</text>'
        + "".join(f'<rect x="16" y="{220+i*56}" width="288" height="48" rx="10" fill="{CARD}" stroke="#e4e8e2"/><text x="32" y="{249+i*56}" {FONT} font-size="14" fill="{INK}">{t}</text>' for i, t in enumerate(("Saved trails", "Walk history", "Downloads", "Settings")))
    )


def frame(x, y, label, content, w=320, h=660):
    return (
        f'<div class="abs cap dim nowrap" style="left:{x}px;top:{y-18}px">{label}</div>'
        f'<svg class="abs" style="left:{x}px;top:{y}px;box-shadow:0 0 0 1px var(--edge-seam)" width="{w}" height="{h}" viewBox="0 0 {w} {h}">{content}</svg>'
    )


def pages():
    row = lambda ic, nm, ind=0, sel=False, r="": f'<div class="tree{" sel" if sel else ""}" style="padding-left:{8+ind*16}px">{ui(ic, 14)}<span class="grow ell">{nm}</span><span class="dim cap">{r}</span></div>'
    return (
        '<div class="sh" style="margin:0;padding:0 8px;box-shadow:none">%s<span class="grow">Pages</span><span class="dim">%s</span></div>' % (ui("chev_down", 12), ui("plus", 12))
        + row("pages", "Flows", 1, True) + row("pages", "Components", 1) + row("pages", "Archive", 1)
        + '<div class="sh" style="margin:6px 0 0;padding:0 8px">%s<span class="grow">Artboards on Flows</span><span class="num dim cap">6</span></div>' % ui("chev_down", 12)
        + row("layout", "Home", 1, False, "320 x 660") + row("layout", "Trail detail", 1, False, "320 x 660") + row("layout", "Profile", 1, False, "320 x 660") + row("layout", "Search results", 1, False, "320 x 660") + row("layout", "Empty state", 1, False, "320 x 660") + row("layout", "Tablet home", 1, False, "768 x 1024")
        + '<div class="sh" style="margin:6px 0 0;padding:0 8px">%s<span class="grow">Local components</span><span class="num dim cap">14</span></div>' % ui("chev_down", 12)
        + row("symbols", "Trail card", 1) + row("symbols", "Primary button", 1) + row("symbols", "Search field", 1) + row("symbols", "Tab bar", 1) + row("symbols", "Stat tile", 1)
    )


def props():
    swatch = lambda c, t: f'<div class="swbtn" style="width:112px"><span class="sw" style="background:{c}"></span><span class="num">{t}</span></div>'
    return (
        '<div class="pad">'
        '<div class="flex" style="height:24px;margin-bottom:4px;gap:6px"><span style="color:var(--text-tertiary)">%s</span><span class="strong grow ell">Trail list</span><span class="dim">Frame, auto layout</span></div>' % ui("row", 14)
        + section("Position and Size")
        + pr2("X", "16", "Y", "160")
        + pr2("W", "288", "H", "312")
        + '<div class="pr"><span class="l">Resizing</span>' + dd("Fill width", 72) + "</div>"
        + section("Auto Layout", True, '<span class="dim" title="Remove">%s</span>' % ui("minus", 12))
        + '<div class="pr"><span class="l">Direction</span><div class="seg"><span title="Vertical" class="on">%s</span><span title="Horizontal">%s</span><span title="Wrap">%s</span></div></div>' % (ui("col", 14), ui("row", 14), ui("presets", 14))
        + pr("Gap", sv("12", "px", meter=12))
        + pr2("L", "0", "T", "0")
        + pr2("R", "0", "B", "0")
        + '<div class="pr"><span class="l">Align</span>' + dd("Top left", 72) + "</div>"
        + section("Appearance")
        + pr("Opacity", sv("100", "%", meter=100))
        + pr("Corner", sv("0", "px", meter=0))
        + section("Fill", True, '<span class="dim">%s</span>' % ui("plus", 12))
        + '<div class="pr">' + swatch("transparent", "None") + "</div>"
        + section("Stroke", True, '<span class="dim">%s</span>' % ui("plus", 12))
        + section("Effects", False)
        + "</div>"
    )


def text_styles():
    rows = [("Display", "24 / 30, 700"), ("Title", "15 / 20, 600"), ("Body", "13 / 20, 400"), ("Caption", "11 / 14, 400")]
    return '<div style="padding-top:4px">' + "".join(f'<div class="tree" style="padding-left:12px"><span class="sec" style="width:20px;display:flex">{ui("text", 12)}</span><span class="grow">{a}</span><span class="dim cap num">{b}</span></div>' for a, b in rows) + "</div>"


def layers():
    ic = lambda i, c="var(--text-secondary)": f'<div style="width:24px;height:24px;display:flex;align-items:center;justify-content:center;color:{c};background:var(--surface-field)">{ui(i, 14)}</div>'
    return (
        layer_row("Home", thumb=ic("layout"))
        + layer_row("Trail list", thumb=ic("row"), indent=1, sel=True)
        + layer_row("Pier Loop", thumb=ic("symbols", "var(--text-primary)"), indent=2)
        + layer_row("Cliff Walk", thumb=ic("symbols", "var(--text-primary)"), indent=2)
        + layer_row("Harbour Ridge", thumb=ic("symbols", "var(--text-primary)"), indent=2)
        + layer_row("Search field", thumb=ic("symbols", "var(--text-primary)"), indent=1)
    )


def build():
    top, h = 94, MAIN_H - 28
    foot_h = 28
    avail = h - foot_h
    gs = [round(avail * 0.52), round(avail * 0.18)]
    gs.append(avail - sum(gs))
    right = (
        f'<div class="abs dock flex" style="left:1620px;top:{top}px;width:300px;height:{h}px;flex-direction:column;align-items:stretch;box-shadow:inset 1px 0 0 var(--edge-seam)">'
        + group(["properties", "interactions", "appearance", "anchoring"], "properties", 300, gs[0], props(), thumb=(4, 300))
        + group(["colour", "swatches", "text-styles", "object-styles"], "text-styles", 300, gs[1], text_styles(), focused=False)
        + group(["layers", "states-comps", "history", "info"], "layers", 300, gs[2], layers(), focused=False)
        + foot(["css-designer", "export", "align-distribute"])
        + "</div>"
    )
    left = (
        f'<div class="abs dock" style="left:44px;top:{top}px;width:260px;height:{h}px;box-shadow:inset -1px 0 0 var(--edge-seam)">'
        + group(["pages-artboards", "symbols", "library", "preset-library"], "pages-artboards", 259, h, pages(), focused=False)
        + "</div>"
    )
    cx, cw = 304, 1316
    fy = top + 150
    fx = [cx + 90, cx + 490, cx + 890]
    # selection overlay on Trail list of the first frame, with auto layout spacing bands
    sx, sy, sw, sh = fx[0] + 16, fy + 160, 288, 312
    band = lambda y: f'<rect x="{sx}" y="{y}" width="{sw}" height="12" fill="#F472B6" opacity=".35"/>'
    overlay = (
        '<svg class="abs" style="left:0;top:0" width="1920" height="1080">'
        + band(sy + 96) + band(sy + 204)
        + f'<rect x="{sx-.5}" y="{sy-.5}" width="{sw+1}" height="{sh+1}" fill="none" stroke="#000" stroke-width="3"/><rect x="{sx-.5}" y="{sy-.5}" width="{sw+1}" height="{sh+1}" fill="none" stroke="#22C55E"/>'
        + "".join(f'<rect x="{x-4}" y="{y-4}" width="8" height="8" fill="#fff" stroke="#0a0a0a"/>' for x, y in ((sx, sy), (sx + sw, sy), (sx, sy + sh), (sx + sw, sy + sh)))
        # prototype connector from the Start a walk button to the next frame
        + f'<path d="M{fx[0]+304} {fy+514} C{fx[0]+350} {fy+514} {fx[1]-60} {fy+120} {fx[1]-6} {fy+120}" fill="none" stroke="#000" stroke-width="3"/><path d="M{fx[0]+304} {fy+514} C{fx[0]+350} {fy+514} {fx[1]-60} {fy+120} {fx[1]-6} {fy+120}" fill="none" stroke="#22C55E" stroke-width="1.5"/><path d="M{fx[1]-10} {fy+114}l8 6-8 6Z" fill="#22C55E"/><circle cx="{fx[0]+304}" cy="{fy+514}" r="4" fill="#22C55E" stroke="#000"/>'
        + "</svg>"
    )
    canvas = (
        f'<div class="vp" data-a="studio.viewer.canvas" style="left:{cx}px;top:{top}px;width:{cw}px;height:{h}px;background:var(--canvas-surround)"></div>'
        + frame(fx[0], fy, "Home  320 x 660", phone_home()) + frame(fx[1], fy, "Trail detail  320 x 660", phone_detail()) + frame(fx[2], fy, "Profile  320 x 660", phone_profile())
        + overlay
        + f'<div class="ovchip" style="left:{sx+sw//2-40}px;top:{sy+sh+2}px">288 x 312, Fill</div>'
        + f'<div class="ovchip" style="left:{sx+sw+6}px;top:{sy+98}px">12</div>'
        + f'<div class="ovchip" style="left:{fx[0]+336}px;top:{fy+290}px">On tap</div>'
    )
    tz = ['<div class="ghost on" style="padding:0 8px">Auto Select</div>', '<div class="ghost" style="padding:0 8px">Deep</div>', '<div class="ghost" style="padding:0 8px">Scale</div>']
    sz = [
        '<span class="dim nowrap">Frame</span>',
        '<div class="pair">' + dd("Fill width", 96) + "</div>",
        '<div class="flex" style="gap:2px">' + "".join('<div class="ghost ib" title="%s">%s</div>' % (t, ui(i, 14)) for t, i in (("Align left", "align_l"), ("Align centre", "align_c"), ("Align right", "align_r"), ("Align top", "align_t"), ("Align middle", "align_m"), ("Align bottom", "align_b"), ("Distribute", "dist_h"))) + "</div>",
        '<div class="zdiv"></div>',
        '<div class="btn">%s Component</div>' % ui("symbols", 14),
        '<div class="btn">%s Play</div>' % ui("play", 12),
    ]
    body = (
        menubar("trailhead-app: Studio", ["M"])
        + tabstrip([{"name": "trailhead-app", "on": True}, {"name": "tide-festival-poster"}], "Interface")
        + contextbar("select", "Select", tz, sz, overflow=True)
        + toolrail(["select", "frame", "shape", "pen", "type", "fill", "move", "crop", "measure", "navigate"], "select", fg="#2F7D57", bg="#FFFFFF")
        + left + canvas + right
        + rail_bottom(["find-replace", "preflight", "jobs", "problems"], 44, top + h, 1876)
        + statusbar(
            [("", "Select: click to pick, drag to move"), ("Shift", "add to selection"), ("Alt", "measure to neighbours"), ("Ctrl", "pick nested")],
            ["Flows", '<span class="num">1 selected</span>', '<span class="num">100%</span>', '%s<span>Model lane: naming layers</span>' % ui("jobs", 12), '%s<span>Vulkan ready</span>' % ui("cpu", 12)],
        )
    )
    write("mock-design.html", page("Studio mock: design, proposed Interface Layout Preset", body))
