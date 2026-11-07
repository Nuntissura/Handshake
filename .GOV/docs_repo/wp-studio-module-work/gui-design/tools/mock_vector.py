"""mock-vector.html: vector profile, Illustrate Layout Preset (STU-SHL-086). Shows the Shape flyout open."""
from mock_lib import MAIN_H, cbf, contextbar, dd, group, layer_row, menubar, page, pr, pr2, rail_bottom, rail_left, section, statusbar, sv, tabstrip, thumb_svg, toolrail, write, badge
from mock_icons import ti, ui
from mock_raster import ruler_ticks


def poster(w, h):
    return (
        f'<svg width="{w}" height="{h}" viewBox="0 0 640 860" style="display:block">'
        '<rect width="640" height="860" fill="#14233f"/>'
        '<circle cx="320" cy="370" r="170" fill="#f28c38"/>'
        '<circle cx="320" cy="370" r="112" fill="#f7b65a"/>'
        '<path d="M0 470c60-46 120-46 180 0s120 46 180 0 120-46 180 0 70 40 100 22V860H0Z" fill="#1f6f8b"/>'
        '<path d="M0 540c60-46 120-46 180 0s120 46 180 0 120-46 180 0 70 40 100 22V860H0Z" fill="#17506e"/>'
        '<path d="M0 610c60-46 120-46 180 0s120 46 180 0 120-46 180 0 70 40 100 22V860H0Z" fill="#0f3853"/>'
        '<g font-family="Inter,Segoe UI,sans-serif" fill="#f6efe2"><text x="44" y="108" font-size="92" font-weight="800" letter-spacing="2">TIDE</text>'
        '<text x="46" y="146" font-size="24" font-weight="500" letter-spacing="6">FESTIVAL</text>'
        '<text x="44" y="760" font-size="30" font-weight="700">14 to 16 June</text><text x="44" y="798" font-size="20" font-weight="400">Harbour Front, Pier 3</text></g>'
        '<g fill="none" stroke="#f6efe2" stroke-width="3"><path d="M452 748l44-54 44 54Z"/><path d="M496 694v54"/></g>'
        "</svg>"
    )


def overlay(ix, iy):
    # selected ellipse, double stroke with handles; pen path in progress; smart guide; size chip
    cx, cy, r = ix + 320, iy + 370, 170
    x0, y0, x1, y1 = cx - r, cy - r, cx + r, cy + r
    hs = "".join(
        f'<rect x="{x-4}" y="{y-4}" width="8" height="8" fill="#fff" stroke="#0a0a0a"/>'
        for x, y in ((x0, y0), (cx, y0), (x1, y0), (x0, cy), (x1, cy), (x0, y1), (cx, y1), (x1, y1))
    )
    px, py = ix, iy
    pen = (
        f'<path d="M{px+80} {py+420} C{px+150} {py+380} {px+230} {py+450} {px+300} {py+410}" fill="none" stroke="#000" stroke-width="3"/>'
        f'<path d="M{px+80} {py+420} C{px+150} {py+380} {px+230} {py+450} {px+300} {py+410}" fill="none" stroke="#22C55E" stroke-width="1"/>'
        f'<path d="M{px+250} {py+438} L{px+350} {py+382}" stroke="#22C55E" stroke-width="1"/>'
        f'<circle cx="{px+250}" cy="{py+438}" r="3.5" fill="#22C55E" stroke="#000"/><circle cx="{px+350}" cy="{py+382}" r="3.5" fill="#22C55E" stroke="#000"/>'
        f'<rect x="{px+76.5}" y="{py+416.5}" width="7" height="7" fill="#fff" stroke="#0a0a0a"/><rect x="{px+296.5}" y="{py+406.5}" width="7" height="7" fill="#22C55E" stroke="#0a0a0a"/>'
    )
    guide = f'<path d="M{cx} {iy} V{iy+860}" stroke="#F472B6" stroke-width="1"/>'
    box = (
        f'<rect x="{x0-.5}" y="{y0-.5}" width="{2*r+1}" height="{2*r+1}" fill="none" stroke="#000" stroke-width="3"/>'
        f'<rect x="{x0-.5}" y="{y0-.5}" width="{2*r+1}" height="{2*r+1}" fill="none" stroke="#22C55E" stroke-width="1"/>'
    )
    return f'<svg class="abs" style="left:0;top:0" width="1920" height="1080" pointer-events="none">{guide}{box}{hs}{pen}</svg>', (cx - 44, y1 + 12)


def flyout(x, y):
    g = [("rectangle", "Rectangle", ""), ("rounded_rectangle", "Rounded Rectangle", ""), ("ellipse", "Ellipse", "L"), ("polygon", "Polygon", ""), ("star", "Star", ""), ("line", "Line", ""), ("arc", "Arc", ""), ("spiral", "Spiral", ""), ("rectangular_grid", "Rectangular Grid", "")]
    rows = "".join('<div class="mi%s" data-a="studio.tool.shape.%s">%s<span>%s</span><span class="key">%s</span></div>' % (" sel" if i == "ellipse" else "", i, ti(i), n, k) for i, n, k in g)
    return (
        f'<div class="pop" data-a="studio.toolgroup.shape.strip" style="left:{x}px;top:{y}px;width:232px;padding:4px 0">{rows}'
        '<div style="height:1px;background:var(--edge-subtle);margin:4px 0"></div>'
        '<div class="mi"><span style="width:20px;display:flex;justify-content:center">%s</span><span>All 37 shapes</span><span class="key">Family browse</span></div></div>' % ui("presets", 14)
    )


def properties():
    return (
        '<div class="pad">'
        '<div class="flex" style="height:24px;margin-bottom:4px;gap:6px"><span style="color:var(--text-tertiary)">%s</span><span class="strong grow ell">Sun, outer</span><span class="dim">Ellipse</span></div>' % ui("vector", 14)
        + section("Transform", True, '<span class="dim">%s</span>' % ui("link", 12))
        + pr2("X", "150", "Y", "200")
        + pr2("W", "340", "H", "340")
        + pr("Rotation", sv("0.0", "°", meter=0))
        + section("Appearance")
        + '<div class="pr"><span class="l">Fill</span><div class="swbtn" style="width:72px"><span class="sw" style="background:#f28c38"></span><span class="num">F28C38</span></div></div>'
        + '<div class="pr"><span class="l">Stroke</span><div class="swbtn" style="width:72px"><span class="sw" style="background:linear-gradient(to top right,transparent 46%,#EF4444 46%,#EF4444 54%,transparent 54%),#fff"></span><span class="dim">None</span></div></div>'
        + pr("Opacity", sv("100", "%", meter=100))
        + section("Stroke", False)
        + section("Effects", False)
        + "</div>"
    )


def swatches():
    cols = ["#14233f", "#17506e", "#1f6f8b", "#0f3853", "#f28c38", "#f7b65a", "#f6efe2", "#ffffff", "#000000", "#d6573f", "#2f855a", "#805ad5", "#e53e3e", "#dd6b20", "#d69e2e", "#38a169", "#319795", "#3182ce", "#5a67d8", "#b83280", "#718096", "#a0aec0", "#cbd5e0", "#edf2f7"]
    cells = "".join('<div style="width:20px;height:20px;background:%s;box-shadow:%s"></div>' % (c, "0 0 0 2px var(--surface-panel),0 0 0 4px var(--accent-base)" if c == "#f28c38" else "inset 0 0 0 1px rgba(128,128,128,.35)") for c in cols)
    return (
        '<div class="pad" style="padding-top:8px"><div class="flex" style="gap:6px;margin-bottom:8px">'
        + dd("Festival palette", 180)
        + '<div class="ghost ib">%s</div></div><div style="display:grid;grid-template-columns:repeat(10,20px);gap:6px">%s</div>' % (ui("plus", 14), cells)
        + "</div>"
    )


def layers():
    orange = '<svg width="24" height="24" style="display:block"><rect width="24" height="24" fill="#14233f"/><circle cx="12" cy="12" r="8" fill="#f28c38"/></svg>'
    wave = thumb_svg("#14233f", "#1f6f8b")
    txt = '<div style="width:24px;height:24px;display:flex;align-items:center;justify-content:center;color:var(--text-secondary);background:var(--surface-field)">%s</div>' % ui("text", 14)
    rows = (
        layer_row("Poster A2", thumb=thumb_svg("#14233f", "#0f3853"), badges=badge("pages"))
        + layer_row("Title", thumb=txt, indent=1, badges=badge("text"), tag="var(--tag-yellow)")
        + layer_row("Sun, outer", thumb=orange, indent=1, sel=True)
        + layer_row("Sun, inner", thumb=orange, indent=1)
        + layer_row("Waves", thumb=wave, indent=1, badges=badge("group"), tag="var(--tag-cyan)")
        + layer_row("Background", thumb=thumb_svg("#14233f", "#14233f"), indent=1, locked=True)
        + layer_row("Flyer A5", thumb=thumb_svg("#f6efe2", "#f7b65a"), badges=badge("pages"))
    )
    return (
        '<div class="flex" style="flex-direction:column;height:100%;align-items:stretch"><div class="grow" style="overflow:hidden;padding-top:2px">' + rows + "</div>"
        '<div class="actbar">' + "".join('<div class="ghost ib" title="%s">%s</div>' % (t, ui(i, 14)) for t, i in (("New artboard", "pages"), ("Group", "folder"), ("New layer", "plus"), ("Delete", "trash"))) + "</div></div>"
    )


def build():
    top, h = 94, MAIN_H - 28
    gs = [round(h * x) for x in (0.32, 0.24)]
    gs.append(h - sum(gs))
    right = (
        f'<div class="abs dock" style="left:1620px;top:{top}px;width:300px;height:{h}px;box-shadow:inset 1px 0 0 var(--edge-seam)">'
        + group(["properties", "appearance", "stroke", "gradient", "transform"], "properties", 300, gs[0], properties(), thumb=(4, 190))
        + group(["colour", "swatches", "align-distribute", "pathfinder", "navigator", "info"], "swatches", 300, gs[1], swatches(), focused=False)
        + group(["layers", "pages-artboards", "object-styles", "history"], "layers", 300, gs[2], layers(), focused=False)
        + "</div>"
    )
    cx, cy, cw, ch = 76, top, 1544, h
    ax, ay = cx + 20 + 240, cy + 20 + 27
    second = f'<div class="abs" style="left:{ax+700}px;top:{ay+200}px;width:420px;height:594px;background:#f6efe2;box-shadow:0 0 0 1px var(--edge-seam)"><svg width="420" height="594"><circle cx="210" cy="220" r="120" fill="#f7b65a"/><path d="M0 330c50-30 100-30 150 0s100 30 150 0 90-30 120-6v270H0Z" fill="#1f6f8b"/><text x="28" y="66" font-family="Inter,Segoe UI,sans-serif" font-size="48" font-weight="800" fill="#14233f">TIDE</text></svg></div>'
    ov, (chipx, chipy) = overlay(ax, ay)
    canvas = (
        f'<div class="vp" data-a="studio.viewer.canvas" style="left:{cx}px;top:{cy}px;width:{cw}px;height:{ch}px;background:var(--canvas-surround)"></div>'
        f'<div class="ruler" style="left:{cx+20}px;top:{cy}px;width:{cw-20}px;height:20px;box-shadow:inset 0 -1px 0 var(--edge-seam)">{ruler_ticks(cw-20, 20)}</div>'
        f'<div class="ruler" style="left:{cx}px;top:{cy+20}px;width:20px;height:{ch-20}px;box-shadow:inset -1px 0 0 var(--edge-seam)">{ruler_ticks(ch-20, 20, True)}</div>'
        f'<div class="abs" style="left:{cx}px;top:{cy}px;width:20px;height:20px;background:var(--surface-strip);box-shadow:inset -1px -1px 0 var(--edge-seam)"></div>'
        f'<div class="abs cap dim" style="left:{ax}px;top:{ay-18}px">Poster A2</div><div class="abs cap dim" style="left:{ax+700}px;top:{ay+182}px">Flyer A5</div>'
        f'<div class="abs" style="left:{ax}px;top:{ay}px;box-shadow:0 0 0 1px var(--edge-seam)">{poster(640, 860)}</div>{second}{ov}'
        f'<div class="ovchip" style="left:{chipx}px;top:{chipy}px">340 x 340 mm</div>'
    )
    tz = ['<div class="ghost on" style="padding:0 8px">Rubber Band</div>', '<div class="ghost">Auto Add Delete</div>', cbf("Corner", "0", "mm", meter=0)]
    sz = [
        '<span class="dim nowrap">Ellipse</span>',
        '<div class="pair"><span class="cl">Fill</span><span class="sw" style="background:#f28c38"></span></div>',
        '<div class="pair"><span class="cl">Stroke</span><span class="sw" style="background:linear-gradient(to top right,transparent 46%,#EF4444 46%,#EF4444 54%,transparent 54%),#fff"></span></div>',
        cbf("Weight", "0", "pt", meter=0),
        cbf("Opacity", "100", "%", meter=100),
        '<div class="zdiv"></div>',
        '<div class="flex" style="gap:2px">' + "".join('<div class="ghost ib" title="%s">%s</div>' % (t, ui(i, 14)) for t, i in (("Align left", "align_l"), ("Align centre", "align_c"), ("Align right", "align_r"), ("Align top", "align_t"), ("Align middle", "align_m"), ("Align bottom", "align_b"))) + "</div>",
    ]
    slots = ["select", "pen", {"id": "shape", "state": "hov"}, "paint", "erase", "type", "fill", "move", "warp", "crop", "lasso", "symbol", "graph", "measure", "navigate"]
    body = (
        menubar("tide-festival-poster: Studio")
        + tabstrip([{"name": "tide-festival-poster", "on": True, "dirty": True}, {"name": "harbour-dusk-retouch"}], "Illustrate")
        + contextbar("pen", "Pen", tz, sz)
        + toolrail(slots, "pen", fg="#f28c38", bg="#FFFFFF")
        + rail_left(["symbols", "brushes", "library", "shape-presets", "gradient-presets", "pattern-presets", "preset-library", "actions-macros"], top, h)
        + canvas
        + right
        + rail_bottom(["find-replace", "preflight", "jobs", "problems"], 44, top + h, 1876)
        + flyout(52, top + 2 + 2 * 32)
        + statusbar(
            [("", "Pen: click for a corner, drag for a curve"), ("Shift", "constrain 45 degrees"), ("Alt", "convert point"), ("Ctrl", "direct select")],
            ['<span class="num">594 x 841 mm</span>', "RGB 8", "Snap to Objects", '<span class="num">27.0%</span>', '%s<span>Vulkan ready</span>' % ui("cpu", 12)],
        )
    )
    write("mock-vector.html", page("Studio mock: vector, Illustrate Layout Preset", body))
