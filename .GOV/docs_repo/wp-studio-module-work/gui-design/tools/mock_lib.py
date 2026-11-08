"""Shared chrome for the mocks. Every size and colour comes from ds_data and layouts.json, so all four
mocks are the same frame. Only rects, rounded rects, text, icons and images: what the Vulkan pass draws."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import ds_data as D  # noqa: E402
from mock_icons import defs, ti, ui  # noqa: E402

LAYOUTS = json.load(open(os.path.join(HERE, "..", "layouts.json"), encoding="utf-8"))
PANELS = LAYOUTS["panel_minimums"]["panels"]
GROUPS = {g["group_id"]: g for g in LAYOUTS["tool_rail"]["groups"]}
S = D.SIZE
W, H = 1920, 1080
TOP = S["frame.menu_bar_h"] + S["frame.tab_strip_h"] + S["frame.context_bar_h"]
MAIN_H = H - TOP - S["frame.status_bar_h"]
EXTRA_NAMES = {"jobs": "Jobs", "problems": "Problems", "preview-render": "Preview"}
PANEL_ICON = {
    "brushes": "brushes", "preset-library": "presets", "library": "library", "project-bin": "project", "effects-browser": "effects",
    "actions-macros": "actions", "keyframe-timeline": "diamond", "find-replace": "find", "preflight": "preflight", "jobs": "jobs",
    "problems": "problems", "symbols": "symbols", "shape-presets": "shape", "gradient-presets": "gradient", "pattern-presets": "pattern",
    "tracker": "target", "expression-editor": "expr", "keyframe-assistant": "diamond", "layers": "row", "swatches": "presets",
    "render-queue": "jobs", "css-designer": "text", "export": "out_mark", "align-distribute": "align_l", "markers": "diamond", "audio-mixer": "col",
}


def var(tok):
    return "--" + tok.replace(".", "-").replace("_", "-")


def css(theme_table):
    root = ";".join("%s:%s" % (var(k), v) for k, v in theme_table.items())
    fixed = ";".join("%s:%s" % (var(k), v) for k, v in D.FIXED.items())
    return (
        ":root{%s;%s}" % (root, fixed)
        + """
*{box-sizing:border-box;margin:0;padding:0}
html,body{width:1920px;height:1080px;overflow:hidden;background:var(--surface-frame)}
body{font:400 12px/16px Inter,"Segoe UI",system-ui,sans-serif;color:var(--text-primary);position:relative;-webkit-font-smoothing:antialiased;user-select:none}
.ic{display:block;flex:none}
.abs{position:absolute}
.num{font-variant-numeric:tabular-nums;font-feature-settings:"tnum"}
.cap{font-size:11px;line-height:14px}
.dim{color:var(--text-tertiary)}.sec{color:var(--text-secondary)}.strong{font-weight:600}
.flex{display:flex;align-items:center}
.grow{flex:1;min-width:0}
.ell{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.nowrap{white-space:nowrap}

/* menu bar */
.mb{left:0;top:0;width:1920px;height:28px;background:var(--surface-frame);display:flex;align-items:center;box-shadow:inset 0 -1px 0 var(--edge-seam)}
.mod{width:32px;height:28px;display:flex;align-items:center;justify-content:center;color:var(--accent-base)}
.mt{height:28px;padding:0 8px;display:flex;align-items:center;font-size:11px;line-height:16px;font-weight:600;letter-spacing:.4px;color:var(--text-primary);white-space:nowrap}
.mt.dimmed{color:var(--text-secondary)}
.wbtn{width:46px;height:28px;display:flex;align-items:center;justify-content:center;color:var(--text-secondary)}

/* document tab strip */
.ts{left:0;top:28px;width:1920px;height:32px;background:var(--surface-strip);display:flex;align-items:stretch}
.dt{width:220px;display:flex;align-items:center;gap:6px;padding:0 6px 0 12px;color:var(--text-secondary);border-right:1px solid var(--edge-seam);white-space:nowrap}
.dt.on{background:var(--surface-panel);color:var(--text-primary);box-shadow:inset 0 2px 0 var(--accent-base);font-weight:600}
.dt .x{width:20px;height:20px;display:flex;align-items:center;justify-content:center;color:var(--text-tertiary);flex:none}
.dot{width:6px;height:6px;border-radius:3px;background:currentColor;flex:none}
.ghost{height:24px;padding:0 8px;display:inline-flex;align-items:center;gap:4px;color:var(--text-secondary);border-radius:3px;white-space:nowrap}
.ghost.ib{width:24px;padding:0;justify-content:center}
.ghost.on{background:var(--surface-row-selected);color:var(--text-primary)}
.btn{height:24px;padding:0 8px;min-width:56px;display:inline-flex;align-items:center;justify-content:center;gap:4px;background:var(--surface-control);border-radius:3px;white-space:nowrap}
.btn.ib{min-width:0;width:24px;padding:0;color:var(--text-secondary)}
.btn.on{background:var(--surface-row-selected);box-shadow:inset 2px 0 0 var(--accent-base);color:var(--text-primary)}
.search{width:240px;height:24px;background:var(--surface-field);box-shadow:inset 0 -1px 0 var(--edge-control);border-radius:2px 2px 0 0;display:flex;align-items:center;gap:6px;padding:0 8px;color:var(--text-tertiary)}
.kbd{padding:0 4px;height:16px;line-height:16px;border-radius:3px;background:var(--surface-control);color:var(--text-secondary);font-size:11px;white-space:nowrap;display:inline-block}

/* context bar */
.cb{left:0;top:60px;width:1920px;height:34px;background:var(--surface-panel);box-shadow:inset 0 -1px 0 var(--edge-seam);display:flex;align-items:center;padding:0 8px 0 4px;gap:8px}
.chip{height:24px;padding:0 8px 0 4px;display:flex;align-items:center;gap:4px;font-weight:600;white-space:nowrap}
.zdiv{width:1px;height:20px;background:var(--edge-subtle);flex:none}
.cl{color:var(--text-secondary);white-space:nowrap}
.pair{display:flex;align-items:center;gap:6px}
.dd{height:24px;padding:0 4px 0 8px;display:flex;align-items:center;justify-content:space-between;gap:4px;background:var(--surface-field);box-shadow:inset 0 -1px 0 var(--edge-control);border-radius:2px 2px 0 0;white-space:nowrap;color:var(--text-primary)}
.dd .ic{color:var(--text-tertiary)}
.sv{position:relative;height:24px;width:72px;background:var(--surface-field);box-shadow:inset 0 -1px 0 var(--edge-control);border-radius:2px 2px 0 0;display:flex;align-items:center;justify-content:flex-end;padding:0 6px;gap:2px;font-variant-numeric:tabular-nums;font-feature-settings:"tnum";white-space:nowrap;flex:none}
.sv .u{color:var(--text-tertiary)}
.sv i{position:absolute;left:0;bottom:0;height:2px;background:var(--text-secondary)}
.sv.hov{background:var(--surface-field-hover);box-shadow:inset 2px 0 0 var(--accent-base),inset 0 -1px 0 var(--text-secondary)}
.sv.expr{background:transparent;box-shadow:inset 0 -1px 0 var(--edge-subtle);color:var(--text-secondary)}
.sv.anim{box-shadow:inset 0 -1px 0 var(--edge-control)}
.sw{width:18px;height:18px;border-radius:2px;box-shadow:inset 0 0 0 1px var(--edge-control);flex:none}
.swbtn{height:24px;display:flex;align-items:center;gap:4px;padding:0 4px;background:var(--surface-field);box-shadow:inset 0 -1px 0 var(--edge-control);border-radius:2px 2px 0 0}

/* tool rail */
.tr{left:0;top:94px;width:44px;height:962px;background:var(--surface-frame);box-shadow:inset -1px 0 0 var(--edge-seam);display:flex;flex-direction:column;align-items:flex-start;padding-left:4px}
.sl{position:relative;width:36px;height:32px;display:flex;align-items:center;justify-content:center;color:var(--text-secondary);border-radius:0 3px 3px 0;flex:none}
.sl.on{background:var(--surface-slot-active);color:var(--text-primary);box-shadow:inset 2px 0 0 var(--accent-base);--hp:var(--accent-base)}
.sl.dimmed{color:var(--text-disabled)}
.sl.hov{background:var(--surface-control);color:var(--text-primary)}
.sl.more::after{content:"";position:absolute;right:2px;bottom:2px;width:0;height:0;border-style:solid;border-width:0 0 4px 4px;border-color:transparent transparent var(--text-tertiary) transparent}
.proxy{position:relative;width:36px;height:36px;flex:none;margin-right:0}
.proxy b{position:absolute;display:block;width:22px;height:22px;box-shadow:0 0 0 1px var(--edge-seam),inset 0 0 0 1px var(--edge-control);border-radius:2px}

/* docks */
.dock{background:var(--surface-panel)}
.rail-l{background:var(--surface-strip);display:flex;flex-direction:column;align-items:center}
.ri{width:32px;height:32px;display:flex;align-items:center;justify-content:center;color:var(--text-secondary);flex:none}
.ri.on{color:var(--text-primary);box-shadow:inset 2px 0 0 var(--accent-base);background:var(--surface-panel)}
.rail-b{background:var(--surface-strip);border-top:1px solid var(--edge-seam);display:flex;align-items:center;padding:0 4px;gap:2px}
.gh{height:26px;background:var(--surface-strip);display:flex;align-items:stretch;flex:none;box-shadow:inset 0 -1px 0 var(--edge-seam)}
.gh .disc,.gh .pm{width:20px;display:flex;align-items:center;justify-content:center;color:var(--text-tertiary);flex:none}
.pt{display:flex;align-items:center;padding:0 8px;color:var(--text-secondary);white-space:nowrap;flex:none}
.pt.on{background:var(--surface-panel);color:var(--text-primary);font-weight:600;box-shadow:inset 0 2px 0 var(--accent-base)}
.pt.on.quiet{box-shadow:inset 0 2px 0 var(--edge-control)}
.gh .over{display:flex;align-items:center;padding:0 4px;color:var(--text-tertiary);flex:none;gap:1px;white-space:nowrap}
.grp{display:flex;flex-direction:column;overflow:hidden;border-top:1px solid var(--edge-seam);position:relative}
.grp:first-child{border-top:0}
.gb{flex:1;position:relative;overflow:hidden;background:var(--surface-panel);min-height:0}
.foot{height:28px;background:var(--surface-strip);border-top:1px solid var(--edge-seam);display:flex;align-items:center;gap:2px;padding:0 4px;flex:none}
.foot .ri{width:24px;height:24px;border-radius:3px}
.thumb{position:absolute;right:2px;width:6px;background:var(--surface-control-hover);border-radius:3px}

/* parameter rows */
.pad{padding:6px 22px 6px 12px}
.sh{height:24px;display:flex;align-items:center;gap:4px;font-weight:600;margin:0 -22px 2px -12px;padding:0 22px 0 8px;box-shadow:inset 0 1px 0 var(--edge-subtle)}
.sh:first-child{box-shadow:none}
.sh .ic{color:var(--text-tertiary)}
.pr{height:24px;margin-bottom:4px;display:flex;align-items:center}
.pr .l{width:96px;margin-right:8px;color:var(--text-secondary);white-space:nowrap;flex:none}
.pr .l2{width:24px;color:var(--text-tertiary);flex:none}
.pr .gap{width:8px;flex:none}
.aff{width:20px;height:20px;display:flex;align-items:center;justify-content:center;color:var(--text-tertiary);flex:none}
.aff.lit{color:var(--accent-base)}
.rail{position:relative;height:4px;background:var(--surface-rail-track);border-radius:2px;margin:0 0 8px 0}
.rail b{position:absolute;top:-4px;width:12px;height:12px;border-radius:6px;background:var(--text-primary);box-shadow:0 0 0 1px var(--edge-seam)}
.rail i{position:absolute;left:0;top:0;bottom:0;background:var(--text-tertiary);border-radius:2px}
.seg{height:24px;display:flex;align-items:stretch;background:var(--surface-field);border-radius:3px;overflow:hidden}
.seg span{width:28px;display:flex;align-items:center;justify-content:center;color:var(--text-secondary)}
.seg span.on{background:var(--surface-row-selected);color:var(--text-primary);box-shadow:inset 0 -2px 0 var(--accent-base)}

/* rows */
.lr{position:relative;height:32px;display:flex;align-items:center;padding-right:6px}
.lr.sel{background:var(--surface-row-selected);box-shadow:inset 2px 0 0 var(--accent-base)}
.lr .tag{width:3px;height:32px;flex:none}
.lr .ey{width:22px;height:32px;display:flex;align-items:center;justify-content:center;color:var(--text-tertiary);flex:none}
.lth{width:24px;height:24px;flex:none;border-radius:2px;box-shadow:inset 0 0 0 1px var(--edge-seam);margin-right:6px;overflow:hidden}
.tree{height:24px;display:flex;align-items:center;gap:4px;padding-right:8px;color:var(--text-primary);white-space:nowrap}
.tree.sel{background:var(--surface-row-selected);box-shadow:inset 2px 0 0 var(--accent-base)}
.tree .ic{color:var(--text-tertiary)}
.actbar{height:28px;border-top:1px solid var(--edge-subtle);display:flex;align-items:center;justify-content:flex-end;gap:2px;padding:0 6px;background:var(--surface-panel)}

/* status bar */
.st{left:0;top:1056px;width:1920px;height:24px;background:var(--surface-frame);box-shadow:inset 0 1px 0 var(--edge-seam);display:flex;align-items:center;padding:0 8px;gap:12px;color:var(--text-secondary);white-space:nowrap}
.st .hint{display:flex;align-items:center;gap:6px;flex:1;min-width:0;overflow:hidden}
.st .rd{display:flex;align-items:center;gap:4px}
.st .sp{width:1px;height:14px;background:var(--edge-subtle)}
.pres{width:18px;height:18px;border-radius:9px;display:flex;align-items:center;justify-content:center;color:#0b0b0b;font-size:11px;font-weight:600;flex:none}

/* floating */
.pop{position:absolute;background:var(--surface-raised);box-shadow:0 0 0 1px var(--edge-seam),0 2px 8px rgba(0,0,0,.4);border-radius:6px;z-index:20}
.mi{height:24px;display:flex;align-items:center;gap:8px;padding:0 8px;white-space:nowrap;color:var(--text-primary)}
.mi.sel{background:var(--surface-row-selected);box-shadow:inset 2px 0 0 var(--accent-base)}
.mi .ic{color:var(--text-secondary)}
.key{margin-left:auto;padding-left:16px;color:var(--text-tertiary)}

/* canvas */
.ruler{background:var(--surface-strip);color:var(--text-tertiary);font-size:11px;line-height:14px;overflow:hidden;position:absolute}
.vp{position:absolute;overflow:hidden}
.ovchip{position:absolute;background:var(--overlay-chip-bg);color:var(--overlay-chip-text);font-size:11px;line-height:14px;padding:2px 6px;border-radius:3px;white-space:nowrap;font-variant-numeric:tabular-nums}
.band{position:absolute;display:flex;align-items:center;gap:8px;height:28px;padding:0 8px}
"""
    )


def name(pid):
    p = pid.split("#")[0].rstrip("*")
    if p in EXTRA_NAMES:
        return EXTRA_NAMES[p]
    return PANELS[p]["display_name"]


def page(title, body, theme="dark"):
    table = D.SEMANTIC_DARK if theme == "dark" else D.SEMANTIC_LIGHT
    return (
        '<!DOCTYPE html>\n<html lang="en"><head><meta charset="utf-8"><title>%s</title><meta name="viewport" content="width=1920"><style>%s</style></head><body data-theme="%s">%s%s</body></html>\n'
        % (title, css(table), theme, defs(), body)
    )


MENU = ["FILE", "EDIT", "VIEW", "DOCUMENT", "SELECT", "OBJECT", "TYPE", "COLOR", "EFFECTS", "MOTION", "INSERT", "CODE", "AUTOMATE", "WORKSPACE", "GO", "RUN", "HELP", "EDITORS", "OPERATOR"]


def menubar(title, models=None):
    m = "".join('<div class="mt" data-a="menu.%s">%s</div>' % (t.lower(), t) for t in MENU)
    pres = '<div class="pres" style="background:var(--presence-operator)" title="Operator">O</div>'
    for i, lab in enumerate(models or []):
        pres += '<div class="pres" style="background:var(--presence-model-%d)" title="Model lane">%s</div>' % (i + 1, lab)
    return (
        '<div class="abs mb"><div class="mod" data-a="shell.module-switch">%s</div>%s<div class="grow nowrap sec" style="text-align:center">%s</div><div class="flex" style="gap:4px;padding-right:8px">%s</div><div class="wbtn">%s</div><div class="wbtn">%s</div><div class="wbtn">%s</div></div>'
        % (ui("module"), m, title, pres, ui("window_min"), ui("window_max"), ui("close"))
    )


def tabstrip(docs, preset, modified=False):
    t = ""
    for d in docs:
        on = " on" if d.get("on") else ""
        dot = '<span class="dot" style="color:var(--text-secondary)"></span>' if d.get("dirty") else ""
        t += '<div class="dt%s"><span class="grow ell">%s</span>%s<span class="x">%s</span></div>' % (on, d["name"], dot, ui("close", 12))
    mod = '<span class="dot" style="color:var(--text-primary)"></span>' if modified else ""
    return (
        '<div class="abs ts">%s<div class="ghost ib" style="margin:4px 0 0 4px">%s</div><div class="grow"></div>'
        '<div class="flex" style="gap:6px;padding-right:8px"><div class="search">%s<span class="grow">Search tools, commands, help</span><span class="kbd">Ctrl K</span></div>'
        '<div class="sp" style="width:1px;height:20px;background:var(--edge-subtle)"></div><span class="dim">Layout</span><div class="ghost" style="color:var(--text-primary)" data-a="studio.layout-preset.dropdown">%s%s%s</div><div class="ghost ib" title="Panel layout">%s</div></div></div>'
        % (t, ui("plus", 12), ui("search", 14), preset, mod, ui("chev_down", 12), ui("layout", 14))
    )


def sv(v, unit="", w=72, meter=None, cls=""):
    i = '<i style="width:%d%%"></i>' % meter if meter is not None else ""
    u = '<span class="u">%s</span>' % unit if unit else ""
    return '<div class="sv %s" style="width:%dpx">%s<span>%s</span>%s</div>' % (cls, w, i, v, u)


def cbf(label, v, unit="", w=72, meter=None):
    return '<div class="pair"><span class="cl">%s</span>%s</div>' % (label, sv(v, unit, w, meter))


def dd(text, w=None):
    st = ' style="width:%dpx"' % w if w else ""
    return '<div class="dd"%s><span>%s</span>%s</div>' % (st, text, ui("chev_down", 12))


def contextbar(tool_icon, tool_name, tool_zone, selection_zone, overflow=True):
    ov = '<div class="ghost ib" title="More options">%s</div><div class="ghost ib" title="Reset tool">%s</div>' % (ui("more", 14), ui("reset", 14))
    return (
        '<div class="abs cb"><div class="chip" data-a="studio.options.identity">%s<span>%s</span></div>%s<div class="zdiv"></div>%s%s<div class="grow"></div><div class="zdiv"></div>%s</div>'
        % (ti(tool_icon), tool_name, dd("Default"), "".join(tool_zone), ov if overflow else "", "".join(selection_zone))
    )


def toolrail(slots, active, fg="#D8A25E", bg="#1B2A3A", extra_top=""):
    h = ""
    for s in slots:
        gid = s if isinstance(s, str) else s["id"]
        icon = gid if isinstance(s, str) else s.get("icon", gid)
        c = "sl more"
        if gid == active:
            c += " on"
        if not isinstance(s, str) and s.get("state"):
            c += " " + s["state"]
        h += '<div class="%s" data-a="studio.toolgroup.%s">%s</div>' % (c, gid.replace("_", "-"), ti(icon))
    return (
        '<div class="abs tr">%s<div style="display:flex;flex-direction:column;padding-top:2px">%s</div><div class="grow"></div>'
        '<div class="sl" title="All tools by family" data-a="studio.tool.family-browse">%s</div>'
        '<div class="proxy" style="margin:4px 0 6px"><b style="left:11px;top:11px;background:%s"></b><b style="left:3px;top:3px;background:%s"></b></div></div>'
        % (extra_top, h, ui("presets"), bg, fg)
    )


def est_w(text):
    return len(text) * 6.3 + 16


def group_header(tabs, active, width, focused=True, collapsed=False):
    room = width - 40
    act = name(active)
    used = est_w(act)
    show = []
    hidden = 0
    for t in tabs:
        if t == active:
            show.append(t)
            continue
        w = est_w(name(t))
        if used + w + 30 <= room and hidden == 0:
            show.append(t)
            used += w
        else:
            hidden += 1
    h = "".join('<div class="pt%s%s" data-a="studio.tab.%s">%s</div>' % (" on" if t == active else "", " quiet" if t == active and not focused else "", t.split("#")[0].rstrip("*"), name(t)) for t in show)
    over = '<div class="over cap" title="More tabs">+%d%s</div>' % (hidden, ui("chev_down", 10)) if hidden else ""
    return '<div class="gh"><div class="disc">%s</div>%s%s<div class="grow"></div><div class="pm">%s</div></div>' % (ui("chev_right" if collapsed else "chev_down", 12), h, over, ui("menu", 12))


def group(tabs, active, width, height, body, focused=True, thumb=None):
    t = ""
    if thumb:
        t = '<div class="thumb" style="top:%dpx;height:%dpx"></div>' % thumb
    return '<div class="grp" style="height:%dpx;width:%dpx">%s<div class="gb">%s%s</div></div>' % (height, width, group_header(tabs, active, width, focused), body, t)


def collapsed_group(tabs, active, width):
    return '<div class="grp" style="height:26px;width:%dpx">%s</div>' % (width, group_header(tabs, active, width, collapsed=True))


def foot(panels):
    return '<div class="foot" title="Registered panels: click opens, Shift-click docks">%s</div>' % "".join(
        '<div class="ri" title="%s" data-a="studio.rail.%s">%s</div>' % (name(p), p, ui(PANEL_ICON.get(p, "presets"), 14)) for p in panels
    )


def rail_left(panels, top, height, left=44):
    items = "".join('<div class="ri" title="%s" data-a="studio.rail.left.%s">%s</div>' % (name(p), p, ui(PANEL_ICON.get(p, "presets"))) for p in panels)
    return '<div class="abs rail-l" style="left:%dpx;top:%dpx;width:32px;height:%dpx;border-right:1px solid var(--edge-seam)"><div class="ri" title="Expand Browse Dock">%s</div><div style="width:20px;height:1px;background:var(--edge-subtle);margin:0 0 4px"></div>%s</div>' % (left, top, height, ui("dbl_right", 14), items)


def rail_bottom(panels, left, top, width):
    items = "".join('<div class="ghost" style="height:24px;padding:0 6px" data-a="studio.rail.bottom.%s">%s<span class="cap">%s</span></div>' % (p, ui(PANEL_ICON.get(p, "presets"), 14), name(p)) for p in panels)
    return '<div class="abs rail-b" style="left:%dpx;top:%dpx;width:%dpx;height:28px">%s<div class="grow"></div><div class="ghost ib" title="Expand Time and Results Dock">%s</div></div>' % (left, top, width, items, ui("dbl_up", 14))


def statusbar(hint, readouts):
    hs = ""
    for k, t in hint:
        hs += ('<span class="kbd">%s</span>' % k if k else "") + '<span>%s</span>' % t
    rd = '<div class="sp"></div>'.join('<div class="rd">%s</div>' % r for r in readouts)
    return '<div class="abs st"><div class="hint">%s</div>%s</div>' % (hs, rd)


def pr(label, field, aff=""):
    return '<div class="pr"><span class="l">%s</span>%s%s</div>' % (label, field, aff)


def pr2(a, av, b, bv, unit="px"):
    return '<div class="pr"><span class="l2">%s</span>%s<span class="gap"></span><span class="l2">%s</span>%s</div>' % (a, sv(av, unit), b, sv(bv, unit))


def section(title, open_=True, actions=""):
    return '<div class="sh">%s<span class="grow">%s</span>%s</div>' % (ui("chev_down" if open_ else "chev_right", 12), title, actions)


def temporal(state="static"):
    if state == "static":
        return '<div class="aff" title="Animate">%s</div>' % ui("stopwatch", 14)
    if state == "animated":
        return '<div class="aff lit" title="Animated">%s</div><div class="aff" title="Previous key">%s</div><div class="aff lit" title="Key at playhead">%s</div><div class="aff" title="Next key">%s</div>' % (ui("stopwatch", 14), ui("prev", 12), ui("diamond", 12), ui("next", 12))
    if state == "expression":
        return '<div class="aff" title="Expression" style="color:var(--text-secondary)">%s</div>' % ui("expr", 14)
    return ""


def thumb_svg(c1, c2):
    return '<svg width="24" height="24" style="display:block"><rect width="24" height="24" fill="%s"/><rect y="13" width="24" height="11" fill="%s"/></svg>' % (c1, c2)


def layer_row(name_, kind="image", thumb="", tag=None, sel=False, hidden=False, indent=0, badges="", locked=False):
    t = '<div class="tag" style="background:%s"></div>' % (tag or "transparent")
    eye = ui("eye_off" if hidden else "eye", 14)
    ind = '<div style="width:%dpx;flex:none"></div>' % (indent * 16) if indent else ""
    lk = '<span style="color:var(--text-tertiary)">%s</span>' % ui("lock", 12) if locked else ""
    tcol = "var(--text-disabled)" if hidden else "var(--text-primary)"
    return '<div class="lr%s">%s<div class="ey">%s</div>%s<div class="lth">%s</div><span class="grow ell" style="color:%s">%s</span>%s%s</div>' % (" sel" if sel else "", t, eye, ind, thumb, tcol, name_, badges, lk)


def badge(icon):
    return '<span style="color:var(--text-tertiary);margin-left:4px">%s</span>' % ui(icon, 12)


def write(fname, html_):
    path = os.path.join(HERE, "..", fname)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(html_)
    print(fname, len(html_))


__all__ = [n for n in dir() if not n.startswith("__")]
