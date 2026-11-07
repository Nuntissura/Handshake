"""Writes ../layouts.json. Validates panel and tool ids against spec section 14 and computes fit_table."""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import layouts_data as L  # noqa: E402

KERNEL = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
SPEC = os.path.join(KERNEL, ".GOV", "spec", "master-spec-v02.229", "spec-modules", "14-studio-creative-suite.md")
SHELL_PANES = {"jobs", "problems"}  # STU-SHL-075 reused shell panes named in default layouts
SPINE = ["context-bar", "tools", "properties", "layers", "colour", "swatches", "history", "info"]
CHROME_V = 28 + 32 + 34 + 24
RES = [(1280, 720, 1), (1920, 1080, 1), (2560, 1440, 1), (2560, 1440, 1.5), (3840, 2160, 1), (3840, 2160, 2)]
TH = L.FIT_RULES["thresholds"]


def read_spec():
    t = open(SPEC, encoding="utf-8").read()
    rows = re.findall(r"^\| ([^|]+) \| `([a-z0-9-]+)` \| `(always_relevant|document_type_specific|task_episodic|selection_contextual)` \| ([^|]+) \| ([^|]+) \|", t, re.M)
    panels = {pid: {"display_name": name.strip(), "class": cls, "default_region": reg.strip(), "profiles": prof.strip()} for name, pid, cls, reg, prof in rows}
    tools, fam = {}, None
    for line in t.split("\n"):
        m = re.match(r"`([a-z_0-9]+)` - .*, (\d+) tools", line)
        if m:
            fam = m.group(1)
            continue
        m = re.match(r"\| [^|]+ \| `([a-z0-9_]+)` \| `([a-z_0-9]+)` \| WORKSPACE > Tools", line)
        if m:
            tools[(m.group(2), m.group(1))] = True
    return panels, tools


def base(pid):
    return pid.split("#")[0].rstrip("*")


def preset_panels(p):
    out = []
    out += p["left"]["panels"] + p["left"].get("registered", [])
    for g in p["right"]["groups"]:
        out += g["tabs"]
    out += p["right"].get("registered", [])
    if p["right_rail"].get("occupant"):
        out.append(p["right_rail"]["occupant"])
    for g in p["bottom"]["groups"]:
        out += g["tabs"]
    out += p["bottom"].get("registered", [])
    return out


def has_time(p):
    return p["bottom"]["state"] == "expanded" and any(t in ("timeline", "keyframe-timeline") for g in p["bottom"]["groups"] for t in g["tabs"])


def fit(p, W, H, s):
    w, h = W / s, H / s
    applied = []
    wide = w >= TH["width_class_wide_from"]
    left_state, left_w = p["left"]["state"], p["left"]["w"]
    if left_state == "rail" and wide:
        left_state = "expanded"
        applied.append("FIT-L1")
    right_state, right_w = p["right"]["state"], p["right"]["w"]
    meter = 36 if p["right_rail"].get("visible") else 0
    n_view = len(p["centre"])
    need_w = n_view * TH["centre_min_w_per_viewer"]

    def lw():
        return left_w if left_state == "expanded" else 32

    def rw():
        return right_w if right_state == "expanded" else 32

    def cw():
        return w - 44 - meter - lw() - rw()

    if left_state == "expanded" and meter and w < TH["width_class_compact_below"]:
        left_state = "rail"
        applied.append("FIT-W3")
    if cw() < need_w and left_state == "expanded" and left_w > 240:
        left_w = 240
        applied.append("FIT-W1")
    if cw() < need_w and right_state == "expanded" and right_w > 260:
        right_w = 260
        applied.append("FIT-W2")
    if cw() < need_w and left_state == "expanded":
        left_state = "rail"
        applied.append("FIT-W3")
    viewers_as_tabs = False
    if cw() < need_w and n_view > 1:
        viewers_as_tabs = True
        need_w = TH["centre_min_w_per_viewer"]
        applied.append("FIT-W4")
    if cw() < need_w and right_state == "expanded":
        right_state = "rail"
        applied.append("FIT-W5")

    main_h = h - CHROME_V
    bottom_state, bottom_h = p["bottom"]["state"], p["bottom"]["h"]
    tm = has_time(p)
    bmin = 300 if tm else 180

    def bh():
        return bottom_h if bottom_state == "expanded" else 28

    if main_h - bh() < TH["centre_min_h"] and bottom_state == "expanded" and bottom_h > bmin:
        bottom_h = bmin
        applied.append("FIT-H1")
    if main_h - bh() < TH["centre_min_h"] and bottom_state == "expanded" and not tm:
        bottom_state = "rail"
        applied.append("FIT-H3")
    centre_h = main_h - bh()
    centre_w = cw()
    group_h = []
    if right_state == "expanded":
        foot = 28 if p["right"].get("registered") else 0
        avail = centre_h - foot
        for g in p["right"]["groups"]:
            group_h.append(round(avail * g["share"]))
        collapsed = [i for i, gh in enumerate(group_h) if gh - 26 < p["right"]["groups"][i].get("min_body", TH["group_min_h"])]
        if collapsed:
            applied.append("FIT-H2")
            free = sum(group_h[i] - 26 for i in collapsed)
            group_h = [26 if i in collapsed else gh for i, gh in enumerate(group_h)]
            open_i = [i for i in range(len(group_h)) if i not in collapsed]
            for i in open_i:
                group_h[i] += round(free / len(open_i))
    slots = len(p["rail"]) + len(p.get("command_slots", []))
    rail_foot = 32 + 40
    pitch = 32
    if slots * 32 + rail_foot > main_h:
        pitch = 28
        applied.append("FIT-R1")
    overflow = 0
    if slots * pitch + rail_foot > main_h:
        room = int((main_h - rail_foot) // pitch) - 1
        overflow = slots - room
        applied.append("FIT-R2")
    res = {
        "window": [W, H], "scale": s, "logical": [round(w), round(h)],
        "left": lw(), "left_state": left_state, "right": rw(), "right_state": right_state, "meter": meter,
        "bottom": bh(), "bottom_state": bottom_state,
        "centre": [round(centre_w), round(centre_h)], "viewers_as_tabs": viewers_as_tabs,
        "right_group_heights": group_h, "rail_pitch": pitch, "rail_overflow_slots": overflow, "applied": sorted(set(applied)),
    }
    problems = []
    if centre_w < TH["centre_min_w_per_viewer"]:
        problems.append("centre_w")
    if centre_h < TH["centre_min_h"]:
        problems.append("centre_h")
    res["ok"] = not problems
    res["problems"] = problems
    return res


def main():
    panels, tools = read_spec()
    if len(panels) != 90:
        sys.exit("expected 90 panels, got %d" % len(panels))
    if len(tools) != 362:
        sys.exit("expected 362 tools, got %d" % len(tools))
    errs, gaps, spine_report = [], {}, {}
    known_gaps = {"preview-render", "site-tree", "related-files"}
    for dom, p in L.PRESETS.items():
        ids = [base(x) for x in preset_panels(p)]
        for pid in ids:
            if pid not in panels and pid not in SHELL_PANES:
                if pid in known_gaps:
                    gaps.setdefault(pid, []).append(dom)
                else:
                    errs.append("%s: unknown panel %s" % (dom, pid))
        missing = [s for s in SPINE if s not in ids and s not in ("context-bar", "tools")]
        spine_report[dom] = missing
        if missing:
            errs.append("%s: spine missing %s" % (dom, missing))
        for r in p["rail"]:
            if r not in {g[0] for g in L.RAIL_GROUPS}:
                errs.append("%s: unknown rail group %s" % (dom, r))
        if len(p["rail"]) + len(p.get("command_slots", [])) > 20:
            errs.append("%s: rail over 20" % dom)
        if p["rail"][-1] != "navigate":
            errs.append("%s: navigate must be last" % dom)
        if len(p["right"]["groups"]) > 4:
            errs.append("%s: >4 groups" % dom)
        if p["signature"] != ["timeline"] and p["right"]["groups"][-1]["tabs"][0] != "layers":  # STU-SHL-090 has no layers tab
            errs.append("%s: layers must open the last group" % dom)
    groups_out, count = [], 0
    for gid, fam, icon, ids in L.RAIL_GROUPS:
        for t in ids:
            if (fam, t) not in tools:
                errs.append("group %s: tool %s not in family %s" % (gid, t, fam))
        count += len(ids)
        groups_out.append({"group_id": gid, "family_id": fam, "face_icon": icon, "tools": ids, "author_id": "studio.toolgroup.%s" % gid})
    if errs:
        sys.exit("\n".join(errs))
    panel_min = {}
    for pid, row in panels.items():
        spec = L.SPECIAL_MIN.get(pid)
        if spec:
            m = dict(spec)
        elif pid in L.FIELD_DENSE:
            m = {"class": "field_dense", "min_w": 240, "min_h": 120}
        else:
            m = {"class": "list_dense", "min_w": 200, "min_h": 96}
        panel_min[pid] = {"display_name": row["display_name"], "panel_class": row["class"], "default_region": row["default_region"], **m}
    fit_table = {}
    bad = []
    for dom, p in L.PRESETS.items():
        fit_table[dom] = [fit(p, *r) for r in RES]
        bad += [(dom, x["window"], x["scale"], x["problems"]) for x in fit_table[dom] if not x["ok"]]
    if bad:
        print("fit problems", bad)
    out = {
        "schema": "hsk.studio.gui_design.layouts@1",
        "file_id": "STUDIO-GUI-LAYOUTS",
        "status": "DRAFT, for engineering; spec clauses remain the authority. Items marked proposed need a decision.",
        "updated_at": "2026-11-07",
        "spec_version": "v02.229",
        "vocabulary": "A 'workspace' in the brief is a Layout Preset here (STU-SHL-002). Domain words such as raster and video are design shorthand. The law's names are the profile signature, the named shape and the preset name.",
        "core_logic": "One frame, seven regions, same places. A domain decides which panels are forward and how tall the time band is. Selection, colour and structure always live right, sources always left, time and results always at the bottom, tools always far left, meters far right.",
        "frame": L.FRAME,
        "docking": L.DOCKING,
        "preset_switch": L.SWITCH,
        "domain_map": [
            {"domain": d, "named_shape": p["named_shape"], "signature": p["signature"], "tie_break": p["tie_break"], "clause": p["clause"], "status": p["status"], "extrapolated": p["extrapolated"], "layout_presets": p["layout_presets"], "mock": p.get("mock")}
            for d, p in L.PRESETS.items()
        ],
        "presets": L.PRESETS,
        "tool_rail": {"rules": L.RAIL_RULES, "groups": groups_out, "tools_in_groups": count},
        "panel_minimums": {"note": "Generated from the 90 rows of STU-SHL-113. min_w applies when docked or floating. Field-dense panels switch to compact form under 260. A panel preferring over 320 presents as an overlay in a side edge (STU-SHL-073).", "panels": panel_min},
        "fit_rules": L.FIT_RULES,
        "fit_table": {"note": "Computed by tools/gen_layouts.py from the presets above with the ladder as written; applied lists the rules that fired. Sizes are logical px. rail foot is 72px.", "results": fit_table},
        "global_states": L.GLOBAL_STATES,
        "catalogue_gaps": gaps,
    }
    with open(os.path.join(HERE, "..", "layouts.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)
    print("ok groups", len(groups_out), "tools in groups", count, "gaps", gaps)
    for dom in fit_table:
        print(dom, [(x["logical"], x["centre"], x["applied"]) for x in fit_table[dom][:2]])


if __name__ == "__main__":
    main()
