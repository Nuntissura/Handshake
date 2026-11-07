"""Writes ../design-system.json from ds_data.py and ds_notes.py. Fails if any contrast pair fails."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import ds_data as d  # noqa: E402
import ds_notes as n  # noqa: E402


def lin(c):
    c = c / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def lum(hx):
    h = hx.lstrip("#")
    r, g, b = int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)
    return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)


def ratio(a, b):
    la, lb = lum(a), lum(b)
    if la < lb:
        la, lb = lb, la
    return (la + 0.05) / (lb + 0.05)


def is_neutral(hx):
    h = hx.lstrip("#")
    return h[0:2] == h[2:4] == h[4:6]


def main():
    rows, failed = [], []
    for theme, table in (("dark", d.SEMANTIC_DARK), ("light", d.SEMANTIC_LIGHT)):
        for fg, bg, need, why in d.CONTRAST_PAIRS:
            r = ratio(table[fg], table[bg])
            row = {"theme": theme, "fg": fg, "bg": bg, "fg_hex": table[fg], "bg_hex": table[bg], "ratio": round(r, 2), "required": need, "use": why, "pass": r >= need}
            rows.append(row)
            if r < need:
                failed.append(row)
    for fg, bg, need, why in d.FIXED_PAIRS:
        r = ratio(d.FIXED[fg][:7], d.FIXED[bg][:7])
        row = {"theme": "fixed", "fg": fg, "bg": bg, "fg_hex": d.FIXED[fg], "bg_hex": d.FIXED[bg], "ratio": round(r, 2), "required": need, "use": why, "pass": r >= need}
        rows.append(row)
        if r < need:
            failed.append(row)
    neutral_prefixes = ("surface.", "edge.", "canvas.", "text.primary", "text.secondary", "text.tertiary", "text.disabled")
    bad_neutral = [
        (t, k, v)
        for t, table in (("dark", d.SEMANTIC_DARK), ("light", d.SEMANTIC_LIGHT))
        for k, v in table.items()
        if k.startswith(neutral_prefixes) and not is_neutral(v)
    ]
    if set(d.SEMANTIC_DARK) != set(d.SEMANTIC_LIGHT):
        sys.exit("theme key mismatch: %s" % sorted(set(d.SEMANTIC_DARK) ^ set(d.SEMANTIC_LIGHT)))
    if bad_neutral:
        sys.exit("chrome tokens not neutral: %s" % bad_neutral)
    if failed:
        for f in failed:
            print("FAIL", f["theme"], f["fg"], f["bg"], f["ratio"], "<", f["required"])
        sys.exit(1)
    out = {
        "schema": "hsk.studio.gui_design.design_system@1",
        "file_id": "STUDIO-GUI-DESIGN-SYSTEM",
        "status": "DRAFT, for engineering; the spec clauses it cites remain the authority",
        "updated_at": "2026-11-07",
        "spec_version": "v02.229",
        "how_to_use": "Generate a token crate from this file; never type a colour or a size in UI code. Tokens are referred to by dotted name; the active theme resolves them.",
        "identity": n.IDENTITY,
        "accent_decision": n.ACCENT_DECISION,
        "shell_conflict": n.SHELL_CONFLICT,
        "scale_rules": n.SCALE_RULES,
        "color": {
            "format": "#RRGGBB opaque sRGB-encoded, #RRGGBBAA only in the fixed table where the colour draws over user content",
            "themes": {"dark": d.SEMANTIC_DARK, "light": d.SEMANTIC_LIGHT},
            "fixed": d.FIXED,
            "surround_preference": {"values": ["canvas.surround", "canvas.surround_mid", "canvas.surround_light"], "default": "canvas.surround", "note": "Colour Layout Preset defaults to canvas.surround_mid; the operator can change it per viewer."},
            "tag_rule": "Layer colour tags render only as a 3px edge in the row and a 2px edge on a clip; they do not tint the whole row, and their name appears in the tooltip so colour is not the only cue.",
        },
        "contrast": {
            "standard": "WCAG 2.2 AA: 4.5:1 for text under 18pt, 3:1 for non-text identifying a control or its state (1.4.11). Inactive components exempt. Ratios are computed from the listed hex values, not rounded up.",
            "pairs_checked": len(rows),
            "all_pass": all(r["pass"] for r in rows),
            "chrome_neutral_verified": True,
            "not_checked": "text.disabled is exempt. Bright user content under overlays is handled by the halo, not by a ratio.",
            "pairs": rows,
        },
        "type": {
            "family": {
                "ui": {"name": "Inter", "license": "SIL OFL 1.1", "weights_used": [400, 500, 600], "note": "Variable font, embedded in the build. Tabular figures via tnum. Reserved Font Name: a modified build cannot keep the name."},
                "code": {"name": "shell code editor font", "note": "UNVERIFIED which font the shell editor uses. Reuse it (SHL-P-18); do not add a second."},
                "fallback": "Noto Sans, Noto Sans CJK and the system UI font through the shaper's fallback chain. UNVERIFIED: not chosen by the spec.",
                "mock_fallback": "The mocks are offline and use Segoe UI where Inter is not installed.",
            },
            "styles": d.TEXT_STYLES,
            "rules": [
                "Numbers use the value or timecode style, always tnum.",
                "Single-line labels never clip silently: ellipsis at the end, full text in L0, and the full string in AccessKit. A control label in a toolbar or tab is not allowed to ellipsize; it overflows instead.",
                "Menu titles keep the literal capitals of STU-SHL-011. All other labels use the descriptor's display_name as written.",
                "IME composition text uses body with a 1px underline in text.primary.",
            ],
        },
        "spacing": {"grid": "All layout lands on the 4px grid. s1 (2px) only inside a control.", "tokens": d.SPACE},
        "size": d.SIZE,
        "radius": d.RADIUS,
        "stroke": d.STROKE,
        "elevation": n.ELEVATION,
        "icon": n.ICON,
        "density": n.DENSITY,
        "state": n.STATE_STYLES,
        "motion": d.MOTION,
        "renderer": n.RENDERER_PRIMITIVES,
        "departures_from_spec": n.DEPARTURES,
        "research_basis": n.RESEARCH_BASIS,
    }
    path = os.path.join(HERE, "..", "design-system.json")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)
    print("ok pairs", len(rows), "min text ratio", min(r["ratio"] for r in rows if r["required"] == 4.5))


if __name__ == "__main__":
    main()
