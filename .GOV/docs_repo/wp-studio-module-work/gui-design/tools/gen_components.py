"""Writes ../components.json from components_a and components_b. Validates roles and token names."""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import components_a as A  # noqa: E402
import components_b as B  # noqa: E402
import ds_data as D  # noqa: E402

ROLES = set("""Unknown TextRun Cell Label Image Link Row ListItem ListMarker TreeItem ListBoxOption MenuItem MenuListOption Paragraph GenericContainer CheckBox RadioButton TextInput Button DefaultButton Pane RowHeader ColumnHeader RowGroup List Table Switch Menu MultilineTextInput SearchInput NumberInput Alert AlertDialog Application Canvas Caption ColorWell ComboBox EditableComboBox Complementary Dialog DisclosureTriangle Grid GridCell Group Heading ListBox MenuBar MenuItemCheckBox MenuItemRadio Meter ProgressIndicator RadioGroup Region ScrollBar ScrollView Slider SpinButton Splitter Status Tab TabList TabPanel Timer TitleBar Toolbar Tooltip Tree TreeGrid Window GraphicsObject""".split())
STYLES = {"control", "ghost", "primary", "field", "row", "tab", "tool_slot"}
TOKENS = set(D.SEMANTIC_DARK) | set(D.FIXED)


def main():
    groups = {"chrome": A.CHROME, "tools": A.TOOLS, "docks": A.DOCKS, "controls": B.CONTROLS, "rows_and_time": B.ROWS, "feedback": B.FEEDBACK, "canvas": B.CANVAS}
    errs, seen = [], set()
    tok_re = re.compile(r"(?<![\w.])(surface|canvas|edge|text|accent|focus|status|time|overlay|clip|meter|presence|tag)\.[a-z_0-9]+")
    for g, items in groups.items():
        for c in items:
            if c["id"] in seen:
                errs.append("dup " + c["id"])
            seen.add(c["id"])
            for k in ("id", "name", "spec", "role", "author_id"):
                if k not in c:
                    errs.append("%s missing %s" % (c.get("id"), k))
            first = c["role"].split()[0]
            if first not in ROLES:
                errs.append("%s role %s" % (c["id"], c["role"]))
            if "style" in c and c["style"] not in STYLES:
                errs.append("%s style %s" % (c["id"], c["style"]))
            for m in tok_re.finditer(json.dumps(c)):
                t = m.group(0)
                if t not in TOKENS and not t.startswith(("text.primary", "text.on_accent")) and t not in ("surface.dock",):
                    errs.append("%s token %s" % (c["id"], t))
    if errs:
        sys.exit("\n".join(errs))
    out = {
        "schema": "hsk.studio.gui_design.components@1",
        "file_id": "STUDIO-GUI-COMPONENTS",
        "status": "DRAFT, for engineering; spec clauses remain the authority",
        "updated_at": "2026-11-07",
        "spec_version": "v02.229",
        "reading": {
            "role": "AccessKit Role name, checked against accesskit 0.25.2. Pinned version UNVERIFIED: GraphicsObject and some others may need a Group fallback.",
            "style": "An id under design-system.json state.interaction. Every state not listed here comes from it, focus included.",
            "size": "Logical px at scale 1. Density modes change only what design-system.json density says.",
            "author_id": "Grammar of STU-MDL-100. Never position, never index, never locale text. stable_id derives mechanically (STU-MDL-103).",
            "text": "A style under design-system.json type.styles.",
            "owner": "Kumihimo owns the control mechanic and its pure-Vulkan renderer (STU-ARC-023, D8). Where it lives between Kumihimo and hsk-studio-controls is open decision O5. Each record here is the theme and behaviour input, not a new asset.",
        },
        "universal_rules": [
            "Every interactive control takes a UiDescriptor generated id; no literal strings in tooltip or accessible APIs (STU-SHL-245).",
            "Every control shows hover, active, focus and disabled, and every disabled control exposes its reason (STU-SHL-019).",
            "Focus is always a 2px ring, or an inset ring on a row or tab, in focus.ring with focus.halo. It is shown on keyboard focus only, never removed, and never clipped.",
            "Hover states never fire while a widget owns the pointer (STU-SHL-104).",
            "A press claims at press; panel and tab drags start only from designated chrome (STU-SHL-101, 102).",
            "All on-screen commands have a menu leaf; context menus and rails are projections (STU-SHL-013, 014).",
            "A status is never carried by colour alone.",
            "Every region, panel, tool, Layout Preset and control has a generated manual entry resolved by its author_id, in the same change as its behaviour (STU-MAN-102, STU-SHL-139).",
        ],
        "keyboard": A.GLOBAL_KEYBOARD,
        "groups": groups,
        "count": sum(len(v) for v in groups.values()),
    }
    with open(os.path.join(HERE, "..", "components.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)
    print("ok components", out["count"])


if __name__ == "__main__":
    main()
