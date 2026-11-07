"""mock-video.html: sequence profile, Edit Layout Preset (STU-SHL-090)."""
import math

from mock_lib import MAIN_H, cbf, contextbar, dd, foot, group, menubar, page, pr, pr2, section, statusbar, sv, tabstrip, toolrail, write, temporal, group_header, name
from mock_icons import ti, ui


def street(w, h):
    return (
        f'<svg width="{w}" height="{h}" viewBox="0 0 640 360" style="display:block"><defs>'
        '<linearGradient id="n" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0b1020"/><stop offset=".6" stop-color="#1a2038"/><stop offset="1" stop-color="#2b2433"/></linearGradient>'
        '<filter id="b2"><feGaussianBlur stdDeviation="2.4"/></filter></defs>'
        '<rect width="640" height="360" fill="url(#n)"/>'
        '<g fill="#10131f"><rect x="0" y="60" width="120" height="240"/><rect x="120" y="110" width="90" height="190"/><rect x="430" y="80" width="100" height="220"/><rect x="530" y="40" width="110" height="260"/></g>'
        '<g filter="url(#b2)" fill="#ffd27a">' + "".join(f'<rect x="{10+(i*37)%600}" y="{70+(i*29)%190}" width="9" height="12" opacity="{.35+(i%5)*.13}"/>' for i in range(46)) + "</g>"
        '<path d="M0 300h640v60H0Z" fill="#16151d"/><path d="M200 360 300 300h40l100 60Z" fill="#23222c"/>'
        '<g filter="url(#b2)"><circle cx="290" cy="288" r="9" fill="#ff5a4a"/><circle cx="350" cy="288" r="9" fill="#ff5a4a"/><rect x="270" y="310" width="100" height="40" fill="#ff5a4a" opacity=".22"/></g>'
        '<path d="M262 298c4-20 12-34 58-34s54 14 58 34v14H262Z" fill="#0d0e14"/></svg>'
    )


def program(w, h):
    return (
        f'<svg width="{w}" height="{h}" viewBox="0 0 640 360" style="display:block"><defs>'
        '<linearGradient id="s2" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2a3556"/><stop offset=".5" stop-color="#b3706c"/><stop offset=".72" stop-color="#f3b06c"/><stop offset="1" stop-color="#2a2433"/></linearGradient>'
        '<radialGradient id="sn"><stop offset="0" stop-color="#fff0c0"/><stop offset="1" stop-color="#ffb060" stop-opacity="0"/></radialGradient></defs>'
        '<rect width="640" height="360" fill="url(#s2)"/><circle cx="440" cy="222" r="90" fill="url(#sn)"/>'
        '<path d="M0 240c120-22 200-6 320-14s210-14 320 2v132H0Z" fill="#141019"/>'
        '<path d="M120 300l90-90 8 90Zm100 0 40-62 40 62Z" fill="#0b0a10"/>'
        '<g font-family="Inter,Segoe UI,sans-serif" fill="#fff"><text x="60" y="96" font-size="34" font-weight="700">Harbour at Dusk</text><text x="62" y="122" font-size="15" font-weight="400" opacity=".9">A short film</text></g>'
        '<rect x="32" y="18" width="576" height="324" fill="none" stroke="#fff" stroke-opacity=".28" stroke-dasharray="3 3"/><rect x="64" y="36" width="512" height="288" fill="none" stroke="#fff" stroke-opacity=".4" stroke-dasharray="3 3"/></svg>'
    )


def viewer(x, y, w, h, title, svg, tc, dur, focused, marks=None):
    head = f'<div class="abs gh" style="left:{x}px;top:{y}px;width:{w}px"><div class="pt on{"" if focused else " quiet"}" data-a="studio.tab.viewer">{title}</div><div class="grow"></div><div class="pm">{ui("menu", 12)}</div></div>'
    ih = (w - 2) * 9 // 16
    area_h = h - 26 - 20 - 36
    iy = y + 26 + (area_h - ih) // 2
    frame = f'<div class="vp" style="left:{x}px;top:{y+26}px;width:{w}px;height:{area_h}px;background:var(--canvas-viewer)"></div><div class="abs" style="left:{x+1}px;top:{iy}px">{svg(w-2, ih)}</div>'
    sy = y + 26 + area_h
    sc = f'<div class="abs" style="left:{x}px;top:{sy}px;width:{w}px;height:20px;background:var(--surface-strip)">' + "".join(f'<div class="abs" style="left:{i}px;bottom:0;width:1px;height:{8 if (i//16)%5==0 else 4}px;background:var(--text-disabled)"></div>' for i in range(8, w, 16)) + f'<div class="abs" style="left:{int(w*.42)}px;top:0;width:1px;height:20px;background:var(--time-playhead);box-shadow:0 0 0 1px #000"></div><div class="abs" style="left:{int(w*.42)-4}px;top:0;width:9px;height:6px;background:var(--time-playhead-head)"></div></div>'
    tr_y = sy + 20
    btn = lambda i, t, cls="ghost ib": f'<div class="{cls}" title="{t}">{ui(i, 14)}</div>'
    tr = (
        f'<div class="abs flex" style="left:{x}px;top:{tr_y}px;width:{w}px;height:36px;background:var(--surface-panel);box-shadow:inset 0 -1px 0 var(--edge-seam);padding:0 8px;gap:2px">'
        f'<span class="num" style="font-size:13px;font-weight:500;color:var(--text-primary);width:96px">{tc}</span><div class="grow"></div>'
        + btn("in_mark", "Mark In") + btn("out_mark", "Mark Out") + btn("to_in", "Go to In") + btn("step_back", "Step back") + f'<div class="btn ib" title="Play" style="width:32px;color:var(--text-primary)">{ui("play", 14)}</div>' + btn("step_fwd", "Step forward") + btn("to_out", "Go to Out") + btn("loop", "Loop") + btn("fit", "Safe margins")
        + f'<div class="grow"></div><span class="num dim" style="width:96px;text-align:right">{dur}</span></div>'
    )
    return head + frame + sc + tr


def wave(w, h, seed):
    pts = []
    for i in range(0, w, 2):
        a = (math.sin(i * 0.031 + seed) * 0.35 + math.sin(i * 0.11 + seed * 2) * 0.3 + math.sin(i * 0.47 + seed * 3) * 0.2 + 0.25)
        a = max(0.06, abs(a))
        pts.append((i, a))
    m = h / 2
    up = " ".join(f"L{x} {m - a * m * .92:.1f}" for x, a in pts)
    dn = " ".join(f"L{x} {m + a * m * .92:.1f}" for x, a in reversed(pts))
    return f'<svg class="abs" style="left:0;top:0" width="{w}" height="{h}"><path d="M0 {m} {up} {dn} Z" fill="var(--time-waveform-on-clip)" opacity=".85"/></svg>'


def clip(x, w, h, kind, label, sel=False, tag=None, wv=None, thumbs=False):
    col = {"v": "--clip-video", "a": "--clip-audio", "t": "--clip-title", "n": "--clip-nested", "j": "--clip-adjustment"}[kind]
    edge = "box-shadow:inset 0 0 0 2px var(--accent-base);" if sel else "box-shadow:inset 0 0 0 1px rgba(0,0,0,.55);"
    inner = ""
    if thumbs and h >= 40:
        inner += f'<svg class="abs" style="left:1px;top:15px" width="{min(70, w-2)}" height="{h-16}"><rect width="70" height="{h}" fill="#1a2038"/><rect y="{h//2}" width="70" height="{h}" fill="#3a2c3a"/><circle cx="46" cy="{h//3}" r="8" fill="#f3b06c"/></svg>'
    if wv is not None:
        top_ = 15 if h >= 32 else 0
        inner += f'<div class="abs" style="left:0;top:{top_}px;width:{w}px;height:{h-2-top_}px;overflow:hidden">{wave(w, h-2-top_, wv)}</div>'
    lab = f'<span class="abs nowrap cap" style="left:0;top:0;padding:1px 6px;background:var({col});color:var(--clip-text);max-width:{w}px;overflow:hidden;text-overflow:ellipsis;border-radius:2px 0 2px 0">{label}</span>' if w > 40 else ""
    return f'<div class="abs" data-a="studio.timeline.clip" style="left:{x}px;top:1px;width:{w}px;height:{h-2}px;background:var({col});border-radius:2px;overflow:hidden;{edge}">{inner}{lab}</div>'


def timeline(w, h):
    head = 180
    tracks = [
        ("V3", 24, "v", [(980, 200, "t", "Lower third: pier", False, None, False)]),
        ("V2", 32, "v", [(120, 260, "t", "Title: Harbour at Dusk", True, None, False), (640, 160, "j", "Adjustment: warm", False, None, False)]),
        ("V1", 56, "v", [(0, 360, "v", "harbour-wide-01", False, None, True), (360, 240, "v", "street-night-02", False, None, True), (600, 180, "v", "tug-close-04", False, None, True), (780, 300, "n", "Nested: pier walk", False, None, True), (1080, 180, "v", "sunset-pan-07", False, None, True)]),
        ("A1", 40, "a", [(0, 600, "a", "dialogue-take-3", False, 1.1, False), (600, 480, "a", "dialogue-take-4", False, 2.4, False), (1080, 180, "a", "dialogue-take-6", False, 3.7, False)]),
        ("A2", 32, "a", [(0, 1260, "a", "harbour-ambience", False, 5.2, False)]),
        ("A3", 24, "a", [(240, 760, "a", "score-strings-b", False, 7.9, False)]),
    ]
    ruler_h = 28
    bar = 28
    y = bar + ruler_h
    t = ""
    hdr = lambda lab, target: (
        f'<div class="flex" style="width:{head}px;height:100%;padding:0 6px 0 8px;gap:2px;background:var(--surface-strip);box-shadow:inset -1px 0 0 var(--edge-seam);flex:none">'
        f'<span class="strong" style="width:22px;color:{"var(--accent-text)" if target else "var(--text-secondary)"}">{lab}</span>'
        f'<div class="ghost ib" style="width:20px;height:20px" title="Lock">{ui("unlock", 12)}</div>'
        + ('<div class="ghost ib" style="width:20px;height:20px" title="Output">%s</div>' % ui("eye", 12) if lab[0] == "V" else '<div class="ghost ib cap strong" style="width:20px;height:20px" title="Mute">M</div><div class="ghost ib cap strong" style="width:20px;height:20px" title="Solo">S</div>')
        + "</div>"
    )
    for i, (lab, th, kind, clips) in enumerate(tracks):
        cl = "".join(clip(c[0], c[1], th, c[2], c[3], c[4], None, c[5], c[6]) for c in clips)
        bg = "var(--time-track-bg)" if i % 2 == 0 else "var(--time-track-alt-bg)"
        t += f'<div class="flex" data-a="studio.timeline.track.{lab.lower()}" style="height:{th}px;box-shadow:inset 0 -1px 0 var(--edge-seam)">{hdr(lab, lab in ("V1", "A1"))}<div class="grow" style="position:relative;height:100%;background:{bg};overflow:hidden">{cl}</div></div>'
    ticks = ""
    for i in range(0, w - head, 12):
        major = (i // 12) % 10 == 0
        ticks += f'<div class="abs" style="left:{i}px;bottom:0;width:1px;height:{10 if major else 4}px;background:var(--text-disabled)"></div>'
        if major:
            s = i // 12
            ticks += f'<div class="abs num cap dim" style="left:{i+4}px;top:2px">00:{s//60:02d}:{s%60:02d}:00</div>'
    range_ = '<div class="abs" style="left:120px;right:%dpx;bottom:0;height:6px;background:var(--time-range-fill)"></div>' % (w - head - 1080)
    marks = "".join(f'<div class="abs" style="left:{m}px;bottom:0;width:0;height:0;border-left:5px solid transparent;border-right:5px solid transparent;border-top:8px solid var(--time-marker);margin-left:-5px"></div>' for m in (360, 780))
    ruler = f'<div class="flex" style="height:{ruler_h}px;box-shadow:inset 0 -1px 0 var(--edge-seam)"><div class="flex" style="width:{head}px;height:100%;padding:0 8px;background:var(--surface-strip);box-shadow:inset -1px 0 0 var(--edge-seam);flex:none"><span class="num" style="font-size:13px;font-weight:500">00:00:42:11</span></div><div class="grow" style="position:relative;height:100%;background:var(--time-ruler-bg);overflow:hidden">{range_}{ticks}{marks}</div></div>'
    bar_html = (
        f'<div class="flex" style="height:{bar}px;padding:0 8px;gap:4px;background:var(--surface-panel);box-shadow:inset 0 -1px 0 var(--edge-subtle)"><span class="strong nowrap">Harbour cut 03</span><span class="dim nowrap">1920 x 1080, 25 fps</span><div class="grow"></div>'
        '<div class="ghost on" style="height:20px">Snap</div><div class="ghost on" style="height:20px">Linked</div><div class="ghost" style="height:20px">Markers</div><div class="ghost ib" style="width:20px;height:20px" title="Add track">%s</div><div style="width:1px;height:16px;background:var(--edge-subtle);margin:0 4px"></div><span class="cl">Zoom</span>%s</div>' % (ui("plus", 12), sv("48", "%", meter=48))
    )
    px = head + 508
    ph = f'<div class="abs" data-a="studio.timeline.playhead" style="left:{px}px;top:{bar+6}px;width:1px;bottom:12px;background:var(--time-playhead);box-shadow:0 0 0 1px #000"></div><div class="abs" style="left:{px-5}px;top:{bar+ruler_h-12}px;width:11px;height:12px;background:var(--time-playhead-head);clip-path:polygon(0 0,100% 0,100% 60%,50% 100%,0 60%)"></div>'
    sb = f'<div class="abs" style="left:{head}px;right:0;bottom:0;height:10px;background:var(--surface-panel)"><div class="abs" style="left:4px;right:220px;top:2px;height:6px;background:var(--surface-control-hover);border-radius:3px"></div></div>'
    return f'<div style="position:relative;height:100%;overflow:hidden">{bar_html}{ruler}{t}{ph}{sb}</div>'


def mixer():
    strips = [("A1", -6, 62), ("A2", -14, 40), ("A3", -18, 30), ("Mix", -3, 70)]
    out = ""
    for nm, db, lvl in strips:
        isolate = nm == "Mix"
        out += (
            f'<div style="width:{84}px;flex:none;height:100%;display:flex;flex-direction:column;align-items:center;padding:6px 0;gap:4px;box-shadow:inset -1px 0 0 var(--edge-subtle);{"background:var(--surface-control-active);" if isolate else ""}">'
            f'<span class="strong">{nm}</span>'
            + sv("0.0", "", w=72, meter=50)
            + '<div class="flex" style="gap:2px"><div class="ghost ib cap strong" style="width:24px" title="Mute">M</div><div class="ghost ib cap strong" style="width:24px" title="Solo">S</div></div>'
            f'<div class="grow" style="display:flex;gap:6px;align-items:stretch;justify-content:center;min-height:0;width:100%">'
            f'<div style="position:relative;width:4px;background:var(--surface-rail-track);border-radius:2px"><div class="abs" style="left:-6px;top:{100-lvl}%;width:16px;height:12px;background:var(--text-primary);border-radius:2px;margin-top:-6px;box-shadow:0 0 0 1px var(--edge-seam)"></div></div>'
            + "".join(f'<div style="position:relative;width:6px;background:var(--meter-track)"><div class="abs" style="left:0;right:0;bottom:0;height:{lvl-k*4}%;background:linear-gradient(to top,var(--meter-low) 0 65%,var(--meter-mid) 65% 85%,var(--meter-high) 85%);background-size:100% {10000/(lvl-k*4):.0f}%;background-position:bottom"></div></div>' for k in (0, 1))
            + f'</div><span class="num cap dim">{db}.0 dB</span></div>'
        )
    return f'<div class="flex" style="height:100%;align-items:stretch;overflow:hidden">{out}</div>'


def project():
    row = lambda ic, nm, r="", ind=0, sel=False: f'<div class="tree{" sel" if sel else ""}" style="padding-left:{8+ind*16}px">{ui(ic, 14)}<span class="grow ell">{nm}</span><span class="num dim cap">{r}</span></div>'
    rows = (
        row("chev_down", "Sequences")
        + row("timeline", "Harbour cut 03", "02:14:06", 1, True)
        + row("timeline", "Harbour cut 02", "02:31:18", 1)
        + row("chev_down", "Footage")
        + row("image", "harbour-wide-01", "00:18:12", 1)
        + row("image", "street-night-02", "00:12:04", 1)
        + row("image", "tug-close-04", "00:09:21", 1)
        + row("image", "sunset-pan-07", "00:15:00", 1)
        + row("image", "pier-walk-09", "00:22:16", 1)
        + row("chev_down", "Audio")
        + row("col", "dialogue-take-3", "00:24:00", 1)
        + row("col", "dialogue-take-4", "00:19:12", 1)
        + row("col", "harbour-ambience", "02:30:00", 1)
        + row("col", "score-strings-b", "01:40:00", 1)
        + row("chev_right", "Graphics")
        + row("chev_right", "Nests")
    )
    return (
        '<div class="flex" style="height:36px;padding:0 8px 0 8px;gap:6px"><div class="search" style="width:auto;flex:1">%s<span>Filter 41 items</span></div><div class="ghost ib" title="List">%s</div><div class="ghost ib" title="Grid">%s</div></div>'
        % (ui("search", 14), ui("menu", 14), ui("presets", 14))
        + f'<div class="flex" style="height:24px;padding:0 10px 0 8px;box-shadow:inset 0 -1px 0 var(--edge-subtle);color:var(--text-tertiary)"><span class="grow">Name</span><span>Duration</span></div>{rows}'
    )


def props():
    return (
        '<div class="pad">'
        '<div class="flex" style="height:24px;margin-bottom:4px;gap:6px"><span style="color:var(--text-tertiary)">%s</span><span class="strong grow ell">Title: Harbour at Dusk</span><span class="dim">Clip</span></div>' % ui("clip", 14)
        + section("Motion", True, '<span class="dim" title="Reset">%s</span>' % ui("reset", 12))
        + pr("Position X", sv("960.0", "", meter=50), temporal("static"))
        + pr("Position Y", sv("540.0", "", meter=50), temporal("static"))
        + pr("Scale", sv("104.0", "%", cls="anim", meter=52), temporal("animated"))
        + pr("Rotation", sv("0.0", "°", meter=0), temporal("static"))
        + section("Opacity")
        + pr("Opacity", sv("88", "%", cls="expr"), temporal("expression"))
        + '<div class="pr" style="gap:6px"><span class="dim cap nowrap" style="width:98px">Set by expression</span><div class="btn">Edit</div><div class="btn">Disable</div></div>'
        + pr("Blend", dd("Normal", 104))
        + section("Time Remapping", False)
        + section("Lower Third Glow", True, '<span class="dim">%s</span>' % ui("eye", 12))
        + pr("Radius", sv("24.0", "px", meter=24), temporal("static"))
        + pr("Intensity", sv("36", "%", meter=36), temporal("static"))
        + pr("Colour", '<div class="swbtn" style="width:72px"><span class="sw" style="background:#ffd27a"></span><span class="num">FFD27A</span></div>')
        + "</div>"
    )


def scopes(w, h):
    pts = "".join(f'<rect x="{i*2}" y="{20+int(36*(1+math.sin(i*.14)*.5))+(i*7)%22}" width="2" height="{14+(i*13)%28}" fill="#7ee0a0" opacity=".55"/>' for i in range(62))
    vec = "".join(f'<circle cx="{58+int(math.cos(i*.7)*(10+i%26))}" cy="{58+int(math.sin(i*.7)*(10+i%26)*.8)}" r="1" fill="#7ee0a0" opacity=".7"/>' for i in range(90))
    return (
        '<div class="flex" style="height:100%;gap:1px;background:var(--edge-seam)">'
        f'<div style="flex:1;height:100%;background:var(--canvas-viewer);position:relative"><div class="abs cap dim" style="left:6px;top:4px">Waveform</div><svg class="abs" style="left:10px;top:18px" width="124" height="96" viewBox="0 0 124 96">{pts}</svg></div>'
        f'<div style="flex:1;height:100%;background:var(--canvas-viewer);position:relative"><div class="abs cap dim" style="left:6px;top:4px">Vectorscope</div><svg class="abs" style="left:14px;top:18px" width="116" height="116"><circle cx="58" cy="58" r="50" fill="none" stroke="#444"/><path d="M58 8v100M8 58h100" stroke="#333"/>{vec}</svg></div></div>'
    )


def meters(x, y, h):
    bar = lambda lvl: f'<div style="position:relative;width:8px;height:100%;background:var(--meter-track)"><div class="abs" style="left:0;right:0;bottom:0;height:{lvl}%;background:linear-gradient(to top,var(--meter-low) 0,var(--meter-low) {70000//lvl/10:.0f}%,var(--meter-mid) {70000//lvl/10:.0f}%,var(--meter-mid) {90000//lvl/10:.0f}%,var(--meter-high) {90000//lvl/10:.0f}%)"></div></div>'
    ticks = "".join(f'<div class="abs num" style="right:2px;top:{t}%;font-size:11px;line-height:14px;color:var(--text-tertiary);margin-top:-7px" hidden>{v}</div>' for t, v in ())
    return (
        f'<div class="abs" data-a="studio.panel.audio-meters" style="left:{x}px;top:{y}px;width:36px;height:{h}px;background:var(--surface-frame);box-shadow:inset 1px 0 0 var(--edge-seam);padding:6px 0 6px 1px;display:flex;flex-direction:column;align-items:center;gap:4px">'
        f'<span class="cap strong sec">dB</span><div class="grow" style="display:flex;gap:4px;min-height:0;width:100%;justify-content:center">{bar(68)}{bar(61)}</div><span class="num cap dim">-6</span></div>'
    )


def build():
    top = 94
    bh = 320
    uh = MAIN_H - bh
    right_h = uh
    foot_h = 28
    g1 = round((right_h - foot_h) * 0.6)
    g2 = right_h - foot_h - g1
    meter_x = 1884
    rx = meter_x - 300
    left = (
        f'<div class="abs dock" style="left:44px;top:{top}px;width:300px;height:{uh}px;box-shadow:inset -1px 0 0 var(--edge-seam)">'
        + group(["project-bin", "media-browser", "effects-browser", "library", "markers", "captions", "info", "history"], "project-bin", 299, uh, project(), focused=False)
        + "</div>"
    )
    right = (
        f'<div class="abs dock flex" style="left:{rx}px;top:{top}px;width:300px;height:{right_h}px;flex-direction:column;align-items:stretch;box-shadow:inset 1px 0 0 var(--edge-seam)">'
        + group(["properties", "effect-controls", "graphics-templates"], "properties", 300, g1, props(), thumb=(4, 200))
        + group(["colour", "scopes", "metadata", "find-replace"], "scopes", 300, g2, scopes(300, g2 - 26), focused=False)
        + foot(["tracker", "expression-editor", "keyframe-assistant", "layers", "swatches"])
        + "</div>"
    )
    vw = (rx - 344) // 2
    centre = viewer(344, top, vw, uh, "Source: street-night-02", street, "00:00:04:18", "00:00:12:04", False) + viewer(344 + vw, top, rx - 344 - vw, uh, "Program: Harbour cut 03", program, "00:00:42:11", "00:02:14:06", True)
    seam = f'<div class="abs" style="left:{344+vw}px;top:{top}px;width:1px;height:{uh}px;background:var(--edge-seam)"></div>'
    by = top + uh
    tw = round(1840 * 0.8)
    tl_h = group_header(["timeline", "keyframe-timeline"], "timeline", tw)
    mx_h = group_header(["audio-mixer"], "audio-mixer", 1840 - tw, focused=False)
    bottom = (
        f'<div class="abs dock" style="left:44px;top:{by}px;width:1840px;height:{bh}px;box-shadow:inset 0 1px 0 var(--edge-seam);display:flex">'
        f'<div style="width:{tw}px;height:100%;display:flex;flex-direction:column;box-shadow:inset -1px 0 0 var(--edge-seam)">{tl_h}<div style="flex:1;min-height:0">{timeline(tw, bh-26)}</div></div>'
        f'<div style="width:{1840-tw}px;height:100%;display:flex;flex-direction:column">{mx_h}<div style="flex:1;min-height:0">{mixer()}</div></div></div>'
    )
    tz = ['<div class="ghost on" style="padding:0 8px">Snap</div>', '<div class="ghost on" style="padding:0 8px">Linked Selection</div>', '<div class="ghost" style="padding:0 8px">All Tracks</div>']
    sz = ['<span class="dim nowrap">Clip</span>', cbf("Speed", "100", "%", meter=50), cbf("Volume", "0.0", "dB", meter=50), cbf("Opacity", "88", "%", meter=88), '<div class="btn">%s Keyframes</div>' % ui("diamond", 12)]
    body = (
        menubar("harbour-short-film: Studio", ["M", "C"])
        + tabstrip([{"name": "harbour-short-film", "on": True, "dirty": True}, {"name": "pier-walk-comp"}], "Edit")
        + contextbar("timeline", "Razor", tz, sz)
        + toolrail(["select", "timeline", "pen", "type", "shape", "navigate"], "timeline", fg="#FFFFFF", bg="#000000")
        + left + centre + seam + right + meters(meter_x, top, MAIN_H) + bottom
        + statusbar(
            [("", "Razor: click a clip to cut it"), ("Shift", "cut all tracks"), ("Alt", "unlinked side only")],
            ['<span class="num">1920 x 1080, 25 fps</span>', "Rec. 709", "Playback: Full", '%s<span>Render: 2 of 5</span>' % ui("jobs", 12), '%s<span>Vulkan ready</span>' % ui("cpu", 12)],
        )
    )
    write("mock-video.html", page("Studio mock: video, Edit Layout Preset", body))
