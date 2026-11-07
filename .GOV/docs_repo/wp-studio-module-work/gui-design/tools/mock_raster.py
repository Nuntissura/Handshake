"""mock-raster.html: raster profile, Photo Layout Preset (STU-SHL-085)."""
from mock_lib import *  # noqa: F401,F403
from mock_lib import cbf, collapsed_group, contextbar, dd, foot, group, layer_row, menubar, MAIN_H, page, pr, pr2, rail_bottom, rail_left, section, statusbar, sv, tabstrip, thumb_svg, toolrail, write, badge
from mock_icons import ti, ui


def harbour(w, h):
    return (
        f'<svg width="{w}" height="{h}" viewBox="0 0 1200 800" style="display:block">'
        '<defs><linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2a3556"/><stop offset=".38" stop-color="#7d5a78"/><stop offset=".62" stop-color="#e08a5a"/><stop offset="1" stop-color="#f6c47a"/></linearGradient>'
        '<linearGradient id="sea" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#e49a68"/><stop offset=".35" stop-color="#7f6078"/><stop offset="1" stop-color="#1b2740"/></linearGradient>'
        '<radialGradient id="sun"><stop offset="0" stop-color="#fff3c4"/><stop offset=".3" stop-color="#ffd27f" stop-opacity=".9"/><stop offset="1" stop-color="#ffb060" stop-opacity="0"/></radialGradient>'
        '<filter id="grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency=".9" numOctaves="2" seed="4"/><feColorMatrix values="0 0 0 0 .5 0 0 0 0 .5 0 0 0 0 .5 0 0 0 .22 0"/></filter>'
        '<filter id="cloud" x="-20%" y="-50%" width="140%" height="200%"><feTurbulence type="fractalNoise" baseFrequency=".008 .04" numOctaves="4" seed="7"/><feColorMatrix values="0 0 0 0 .95 0 0 0 0 .72 0 0 0 0 .62 0 0 0 1.5 -.62"/></filter>'
        '<filter id="soft"><feGaussianBlur stdDeviation="1.1"/></filter>'
        '</defs><rect width="1200" height="470" fill="url(#sky)"/><rect width="1200" height="430" fill="#fff" filter="url(#cloud)" opacity=".75"/>'
        '<circle cx="790" cy="420" r="170" fill="url(#sun)"/>'
        '<path d="M0 410 C120 360 220 380 330 350 S520 395 640 402 S900 372 1010 392 1160 370 1200 384V480H0Z" fill="#40324f"/>'
        '<path d="M0 440C150 410 250 440 420 425s280 15 430 8 240-18 350-10V490H0Z" fill="#2b2438"/>'
        '<rect y="468" width="1200" height="332" fill="url(#sea)"/>'
        '<g fill="#ffd9a0" opacity=".32" filter="url(#soft)">' + "".join(f'<rect x="{790 - 18 - i*7 + (i*53)%40}" y="{474 + i*7}" width="{26 + (i*67)%90}" height="{2 + i//10}" rx="1"/>' for i in range(44)) + '</g>'
        '<g fill="#fff" opacity=".07">' + "".join(f'<rect x="{(i*137)%1100}" y="{500 + (i*23)%290}" width="{60 + (i*31)%120}" height="2"/>' for i in range(46)) + '</g>'
        '<g filter="url(#soft)"><path d="M180 560l150-2-22 34H210Z" fill="#14161f"/><path d="M252 556V438" stroke="#14161f" stroke-width="4"/><path d="M256 446l58 100h-58Z" fill="#d9c6ae"/><path d="M248 462l-44 84h44Z" fill="#b8a48f"/></g>'
        '<g filter="url(#soft)"><path d="M520 524c40 12 120 12 174 0l-20 28H544Z" fill="#23202b"/><rect x="566" y="492" width="58" height="26" fill="#d6573f"/><rect x="580" y="474" width="26" height="20" fill="#e9dcc8"/><path d="M592 474V440" stroke="#23202b" stroke-width="3"/><rect x="574" y="498" width="10" height="9" fill="#ffe1a0"/><rect x="592" y="498" width="10" height="9" fill="#ffe1a0"/></g>'
        '<g filter="url(#soft)"><path d="M920 610c60 16 140 12 200-4l-26 36H950Z" fill="#101219"/><path d="M1004 604V486" stroke="#101219" stroke-width="4"/><path d="M1010 494l64 100h-64Z" fill="#c8b4a0"/></g>'
        '<path d="M0 800V690c80-10 180 2 300-8s190 20 300 14l90 104Z" fill="#0d0f16"/>'
        '<g stroke="#0d0f16" stroke-width="9">' + "".join(f'<path d="M{60+i*104} 800V{660+((i*17)%18)}"/>' for i in range(6)) + '</g>'
        '<rect width="1200" height="800" filter="url(#grain)" opacity=".5"/></svg>'
    )


def ruler_ticks(length, step, vertical=False):
    out = ""
    for i in range(0, length, step):
        major = (i // step) % 5 == 0
        if vertical:
            out += f'<div class="abs" style="top:{i}px;right:0;width:{10 if major else 5}px;height:1px;background:var(--text-disabled)"></div>'
            if major and i < length - 40:
                out += f'<div class="abs num" style="top:{i+1}px;left:2px;writing-mode:vertical-rl;transform:rotate(180deg)">{i*5}</div>'
        else:
            out += f'<div class="abs" style="left:{i}px;bottom:0;height:{10 if major else 5}px;width:1px;background:var(--text-disabled)"></div>'
            if major and i < length - 40:
                out += f'<div class="abs num" style="left:{i+3}px;top:1px">{i*5}</div>'
    return out


def properties(w):
    return (
        '<div class="pad">'
        '<div class="flex" style="height:24px;margin-bottom:4px;gap:6px"><span style="color:var(--text-tertiary)">%s</span><span class="strong grow ell">Sky retouch</span><span class="dim">Pixel layer</span></div>' % ui("image", 14)
        + section("Transform", True, '<span class="dim">%s</span>' % ui("link", 12))
        + pr2("W", "6,000", "H", "4,000")
        + pr2("X", "0", "Y", "0")
        + pr("Rotation", sv("0.0", "°", meter=0))
        + section("Appearance")
        + pr("Opacity", sv("100", "%", meter=100))
        + pr("Fill", sv("100", "%", meter=100))
        + pr("Blend", dd("Normal", 104))
        + section("Mask")
        + pr("Density", sv("100", "%", meter=100))
        + pr("Feather", sv("12.0", "px", meter=12))
        + "</div>"
    )


def colour_body():
    return (
        '<div class="pad" style="padding-top:8px">'
        '<div style="height:72px;border-radius:2px;background:linear-gradient(to top,#000,rgba(0,0,0,0)),linear-gradient(to right,#fff,hsl(28,100%,50%));position:relative;margin-bottom:8px">'
        '<div class="abs" style="left:176px;top:22px;width:12px;height:12px;border-radius:6px;box-shadow:0 0 0 2px #fff,0 0 0 3px #000"></div></div>'
        '<div class="rail" style="height:8px;background:linear-gradient(to right,#f00,#ff0,#0f0,#0ff,#00f,#f0f,#f00);margin-bottom:10px"><b style="left:20px;top:-2px"></b></div>'
        + pr2("R", "216", "G", "162", "")
        + pr2("B", "94", "A", "100", "")
        + "</div>"
    )


def layers_body():
    sky = thumb_svg("#7d5a78", "#e49a68")
    boat = thumb_svg("#6a5a78", "#23202b")
    full = thumb_svg("#d08a68", "#2b3350")
    mask = '<div class="lth" style="margin:0 0 0 2px;background:#fff"><svg width="24" height="24"><path d="M0 24 24 8v16Z" fill="#111"/></svg></div>'
    adj = '<svg width="24" height="24" style="display:block"><rect width="24" height="24" fill="#333"/><path d="M12 5a7 7 0 0 1 0 14Z" fill="#ddd"/><path d="M12 5a7 7 0 0 0 0 14" fill="none" stroke="#ddd"/></svg>'
    rows = (
        layer_row("Grade: warm dusk", thumb=adj, badges=badge("adjust"), tag="var(--tag-orange)")
        + layer_row("Sky retouch", thumb=sky, sel=True, badges=mask + badge("fx"))
        + layer_row("Boats", thumb=boat, indent=0, badges=badge("group"), tag="var(--tag-blue)")
        + layer_row("Red tug clean-up", thumb=boat, indent=1)
        + layer_row("Sail healing", thumb=boat, indent=1)
        + layer_row("Sensor dust", thumb=full, hidden=True)
        + layer_row("harbour-0412.raw", thumb=full, locked=True)
    )
    return (
        '<div class="flex" style="flex-direction:column;height:100%;align-items:stretch">'
        '<div class="flex" style="height:32px;padding:0 22px 0 12px;gap:8px;flex:none">' + dd("Normal", 104) + '<span class="cl" style="margin-left:auto">Opacity</span>' + sv("100", "%", w=72, meter=100) + "</div>"
        '<div class="flex" style="height:28px;padding:0 22px 0 12px;gap:2px;flex:none;box-shadow:inset 0 -1px 0 var(--edge-subtle)"><span class="cl" style="margin-right:6px">Lock</span>'
        + "".join('<div class="ghost ib">%s</div>' % ui(i, 14) for i in ("image", "vector", "move_", "lock") if i != "move_")
        + '<div class="grow"></div><div class="ghost ib" title="Filter layers">%s</div></div>' % ui("search", 14)
        + '<div class="grow" style="overflow:hidden">' + rows + "</div>"
        + '<div class="actbar">' + "".join('<div class="ghost ib" title="%s">%s</div>' % (t, ui(i, 14)) for t, i in (("Add effect", "fx"), ("Add mask", "mask"), ("Add adjustment", "adjust"), ("Group", "folder"), ("New layer", "plus"), ("Delete", "trash"))) + "</div></div>"
    )


def build(theme="dark", fname="mock-raster.html"):
    rw, top, h = 300, 94, MAIN_H - 28
    gs = [round(h * x) for x in (0.30, 0.24, 0.46)]
    gs[2] = h - gs[0] - gs[1]
    right = (
        f'<div class="abs dock" style="left:1620px;top:{top}px;width:300px;height:{h}px;box-shadow:inset 1px 0 0 var(--edge-seam)">'
        + group(["properties", "adjustments", "effect-controls", "masks"], "properties", 300, gs[0], properties(300), thumb=(4, 150))
        + group(["colour", "swatches", "scopes", "info", "navigator"], "colour", 300, gs[1], colour_body(), focused=False)
        + group(["layers", "channels", "paths", "history"], "layers", 300, gs[2], layers_body(), focused=False)
        + "</div>"
    )
    cx, cy, cw, ch = 76, 94, 1544, h
    ix, iy = cx + 20 + (cw - 20 - 1200) // 2, cy + 20 + (ch - 20 - 800) // 2
    canvas = (
        f'<div class="vp" data-a="studio.viewer.canvas" style="left:{cx}px;top:{cy}px;width:{cw}px;height:{ch}px;background:var(--canvas-surround)"></div>'
        f'<div class="ruler" style="left:{cx+20}px;top:{cy}px;width:{cw-20}px;height:20px;box-shadow:inset 0 -1px 0 var(--edge-seam)">{ruler_ticks(cw-20, 20)}</div>'
        f'<div class="ruler" style="left:{cx}px;top:{cy+20}px;width:20px;height:{ch-20}px;box-shadow:inset -1px 0 0 var(--edge-seam)">{ruler_ticks(ch-20, 20, True)}</div>'
        f'<div class="abs" style="left:{cx}px;top:{cy}px;width:20px;height:20px;background:var(--surface-strip);box-shadow:inset -1px -1px 0 var(--edge-seam)"></div>'
        f'<div class="abs" style="left:{ix}px;top:{iy}px;box-shadow:0 0 0 1px var(--edge-seam)">{harbour(1200, 800)}</div>'
        # selection: marching ants drawn as black and white dashes
        f'<svg class="abs" style="left:{ix}px;top:{iy}px" width="1200" height="800"><path id="sel" d="M58 120 C240 60 480 150 690 96 S1020 70 1140 138 L1132 322 C900 300 640 380 420 340 S120 360 66 310 Z" fill="none" stroke="#000" stroke-width="1"/><path d="M58 120 C240 60 480 150 690 96 S1020 70 1140 138 L1132 322 C900 300 640 380 420 340 S120 360 66 310 Z" fill="none" stroke="#fff" stroke-width="1" stroke-dasharray="4 4"/>'
        # brush cursor, double ring
        '<circle cx="470" cy="210" r="32" fill="none" stroke="#000" stroke-width="1"/><circle cx="470" cy="210" r="31" fill="none" stroke="#fff" stroke-width="1"/></svg>'
    )
    # L1 tooltip beside the dimmed pen slot: reason plus one-click remedy (STU-SHL-232)
    pen_y = 94 + 2 + 13 * 32
    tip = (
        f'<div class="pop" data-a="studio.tooltip" style="left:52px;top:{pen_y-6}px;width:300px;padding:8px 10px">'
        '<div class="flex" style="gap:8px;margin-bottom:4px"><span class="strong">Node</span><span class="kbd" style="margin-left:auto">A</span></div>'
        '<div style="color:var(--text-secondary);margin-bottom:6px">Not available on a pixel layer. Select a vector layer, or:</div>'
        '<div class="flex" style="gap:6px;color:var(--accent-text)"><span>Convert this pixel layer to curves</span></div>'
        '<div class="dim cap" style="margin-top:6px">F1 opens this tool in the manual</div></div>'
    )
    tz = [cbf("Size", "64", "px", meter=22), cbf("Hardness", "80", "%", meter=80), cbf("Opacity", "100", "%", meter=100), cbf("Flow", "72", "%", meter=72), '<div class="pair"><span class="cl">Mode</span>' + dd("Normal", 96) + "</div>", cbf("Smoothing", "10", "%", meter=10)]
    sz = ['<span class="dim nowrap">Pixel layer</span>', '<div class="pair"><span class="cl">Blend</span>' + dd("Normal", 96) + "</div>", cbf("Opacity", "100", "%", meter=100), '<div class="btn">%s Add Mask</div>' % ui("mask", 14)]
    slots = ["select", "smart_select", "marquee", "lasso", "wand", "move", "crop", "paint", "erase", "retouch", "tone", "mask", "fill", {"id": "pen", "state": "dimmed hov"}, "shape", "type", "warp", "measure", "navigate"]
    body = (
        menubar("harbour-dusk-retouch: Studio", ["M"])
        + tabstrip([{"name": "harbour-dusk-retouch", "on": True, "dirty": True}, {"name": "poster-b-festival"}, {"name": "harbour-0412.raw"}], "Photo")
        + contextbar("paint", "Paint Brush", tz, sz)
        + toolrail(slots, "paint")
        + rail_left(["brushes", "preset-library", "library", "project-bin", "effects-browser", "actions-macros"], top, h)
        + canvas
        + right
        + rail_bottom(["keyframe-timeline", "find-replace", "preflight", "jobs", "problems"], 44, top + h, 1876)
        + tip
        + statusbar(
            [("", "Paint: drag to stroke"), ("Shift", "straight line"), ("Alt", "sample colour"), ("[ ]", "brush size")],
            ['<span class="num">6000 x 4000 px</span>', "RGB 16", '<span class="num">20.0%</span>', '%s<span>Model lane: export proxies 4 of 12</span>' % ui("jobs", 12), '%s<span>Vulkan ready</span>' % ui("cpu", 12)],
        )
    )
    write(fname, page("Studio mock: raster, Photo Layout Preset", body, theme))
