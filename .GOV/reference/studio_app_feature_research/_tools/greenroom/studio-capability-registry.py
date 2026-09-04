#!/usr/bin/env python3
"""Handshake Studio green room: unified cross-app capability registry.

Purpose: answer "what must Studio own so a professional loses nothing when they switch?"
Ingests every captured app under _greenroom_20260903/installed_exports/, normalises each
source into capability rows, dedupes shared capability across apps into ONE Studio capability
(Master Spec STU-SECTION-003 no-double-features), assigns a Studio domain, and measures
coverage against the existing WP-KERNEL-STUDIO microtask contracts.

Sources per app:
  dom_typelib.json          scripting object model  -> primitives, object commands, options
  scripting_api_surface.json (Affinity JSLib)       -> primitives, commands
  ui_strings_dotnet_en-US.json / lproj              -> tools, commands, panels, dialogs, options
  indesign_idrc_survey.json                         -> labels (menus/panels/dialogs), zstring keys
  presets.json                                      -> preset families the app ships
  presets_names_scan.json                           -> stock preset counts + categories
  uxp_manifests.json                                -> built-in panels
  keyboard_shortcuts.json                           -> tool/command shortcut bindings
  tree_manifest.json                                -> library/plugin inventory

Output: studio-capability-registry.json  + studio-coverage-gaps.json
Reference material only; vendor names are provenance, Studio ships its own names.
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
from pathlib import Path

STOP = {"the", "a", "an", "to", "of", "and", "or", "for", "with", "in", "on", "by", "new", "current", "this", "all", "from", "as", "at", "is", "are", "be"}
NOISE = re.compile(r"^(ok|cancel|yes|no|none|default|untitled|error|warning|close|help|apply|reset|done|save|open|copy|paste|cut|undo|redo)$", re.I)
BADCHARS = re.compile(r"[<>{}\[\]|\\^~`=+*#@$%]")

DOMAIN_RULES = [
    ("raw", r"\b(develop|camera raw|raw|demosaic|white balance|dehaze|clarity|texture|vignett|chromatic|lens profile|lens correction|tone curve|hsl|split ton|calibrat|profile browser|catalog|collection|import dialog|cull|flag|reject|rating|keyword|metadata panel|smart preview|virtual copy|snapshot|history panel|before after|soft proof)"),
    ("layout", r"\b(page|spread|master page|parent page|book|index|table of contents|cross.?reference|footnote|endnote|data merge|preflight|package|imposition|column|gutter|text frame|frame fitting|text wrap|baseline grid|story editor|thread|overset|anchored object|section|folio|bleed|slug|printer.?s marks|separation)"),
    ("typography", r"\b(paragraph|character|font|glyph|kern|track|leading|hyphen|justif|opentype|ligature|small caps|superscript|subscript|baseline shift|drop cap|bullet|numbering|tab stop|composer|optical margin|variable font|style sheet|text style|type on a path|vertical text|ruby|kinsoku|mojikumi|tate)"),
    ("color", r"\b(colou?r|swatch|palette|gradient|icc|profile|cmyk|rgb|lab|spot|pantone|tint|overprint|soft.?proof|gamut|lut|curves|levels|white point|colou?r manag|blend if|channel mixer|selective colou?r|posteri[sz]|threshold|gradient map)"),
    ("effects", r"\b(effect|filter|blur|sharpen|noise|distort|stylize|render|liquify|displace|glow|shadow|bevel|emboss|satin|stroke effect|feather|halftone|pixelat|mosaic|wave|zigzag|ripple|twirl|spheri|extrude|3d|lighting effect|smart filter|live filter|adjustment layer)"),
    ("vector", r"\b(path|node|anchor point|b[eé]zier|pen tool|curve|shape|boolean|pathfinder|compound|stroke|dash|arrowhead|corner|join|miter|width tool|blend tool|envelope|warp|mesh|gradient mesh|symbol|artboard|align|distribute|transform|scale|rotate|shear|reflect|offset path|outline stroke|simplify|smooth|scissors|knife|eraser vector|live paint|image trace|perspective grid|isometric)"),
    ("raster", r"\b(pixel|raster|brush|marquee|lasso|magic wand|quick selection|object selection|select subject|select sky|refine edge|mask|channel|clone|heal|patch|content.?aware|dodge|burn|sponge|smudge|blur tool|sharpen tool|crop|resample|image size|canvas size|layer mask|clipping mask|smart object|stack|frequency separation|inpaint|liquif|puppet|perspective warp|hdr merge|panorama|focus merge|dust|spot removal)"),
    ("motion", r"\b(keyframe|timeline|composition|animat|ease|interpolat|motion blur|time remap|track matte|rotoscop|puppet pin|expression|null object|precomp|render queue|frame rate|in point|out point|work area|graph editor|motion path|parent link|camera layer|light layer|3d layer|shape layer|particle|wiggle)"),
    ("video", r"\b(sequence|clip|trim|ripple|roll|slip|slide|razor|multicam|proxy|scopes|waveform|vectorscope|lumetri|audio track|mixer|transition|speed.?duration|nest|source monitor|program monitor|timecode|subclip|marker|caption|subtitle|export media|media encoder)"),
    ("design_system", r"\b(component|variant|instance|library|asset|style|graphic style|object style|preset|template|token|variable|symbol library|shared|team library|swatch group)"),
    ("prototype", r"\b(prototype|interaction|trigger|transition preset|overlay|scroll|hotspot|hyperlink|button|form field|animation preset|smart animate|flow|hover|click.?through)"),
    ("interop", r"\b(import|export|place|link|package|pdf|psd|ai\b|idml|svg|eps|dwg|dxf|jpeg|png|tiff|webp|jxl|heic|exr|dng|save for web|export for screens|round.?trip|missing font|relink|embed|unembed|artboard export|slice)"),
    ("automation", r"\b(action|macro|batch|script|droplet|automate|variable data|conditional text|find.?change|grep|query|preset manager|workspace)"),
    ("whiteboard", r"\b(sticky|whiteboard|figjam|diagram|connector|stamp|vote|timer|cursor chat)"),
    # Second pass: broader vocabulary for rows the specific rules above missed.
    ("raster", r"\b(pixel|bitmap|resolution|dpi|ppi|alpha|opacity|erase|paint|fill bucket|gradient tool|histogram|exposure|shadow.?highlight|denoise|despeckle|unsharp|selection brush|refine|matte|luminosity|tone|dodge|burn|red eye|blemish|inpaint)"),
    ("vector", r"\b(node|handle|segment|corner|cusp|smooth|join|cap|miter|bevel|round|winding|fill rule|outline|expand|divide|trim|merge|crop|exclude|minus front|intersect|unite|geometry|point|vertex|tangent|control point|construction)"),
    ("typography", r"\b(bold|italic|underline|strikethrough|case|uppercase|lowercase|title case|word spacing|letter spacing|indent|align left|align right|centre|center|justify|orphan|widow|keep|column break|page break|frame break|no.?break|soft return|em dash|en dash|quote|apostroph|ellipsis|space|tab)"),
    ("color", r"\b(hue|saturation|luminance|brightness|contrast|chroma|value|opacity|alpha channel|swatch|shade|tint|hex|hsl|hsv|cmy|greyscale|grayscale|monochrome|duotone|colour chord|complementary|analogous|triadic|split)"),
    ("layout", r"\b(margin|bleed|slug|spread|facing|recto|verso|numbering|folio|anchor|inline|above line|custom position|text frame|frame option|inset|vertical justification|first baseline|balance columns|span column|split column)"),
    ("design_system", r"\b(persona|workspace|panel layout|toolbar|context bar|dock|favourite|favorite|recent|history|snapshot|version|revision)"),
    ("interop", r"\b(open|save|save as|save a copy|revert|place|export as|publish|share|print|package|collect|archive|preview|proof)"),
]
KIND_HINTS = [
    ("tool", r"\btool\b"),
    ("panel", r"\bpanel\b|\bstudio\b|\bpalette window\b"),
    ("dialog", r"\bdialog\b|\bsettings\b|\boptions\b\.\.\."),
    ("menu", r"\bmenu\b"),
]



# Rows that are not product capabilities: CSS colour names, internal action/menu identifiers,
# ML model filenames, error sentences, and bare acronyms.
CSS_COLORS = {"aliceblue","antiquewhite","aqua","aquamarine","azure","beige","bisque","black","blanchedalmond","blue","blueviolet","brown","burlywood","cadetblue","chartreuse","chocolate","coral","cornflowerblue","cornsilk","crimson","cyan","darkblue","darkcyan","darkgoldenrod","darkgray","darkgreen","darkgrey","darkkhaki","darkmagenta","darkolivegreen","darkorange","darkorchid","darkred","darksalmon","darkseagreen","darkslateblue","darkslategray","darkslategrey","darkturquoise","darkviolet","deeppink","deepskyblue","dimgray","dimgrey","dodgerblue","firebrick","floralwhite","forestgreen","fuchsia","gainsboro","ghostwhite","gold","goldenrod","gray","green","greenyellow","grey","honeydew","hotpink","indianred","indigo","ivory","khaki","lavender","lavenderblush","lawngreen","lemonchiffon","lightblue","lightcoral","lightcyan","lightgoldenrodyellow","lightgray","lightgreen","lightgrey","lightpink","lightsalmon","lightseagreen","lightskyblue","lightslategray","lightslategrey","lightsteelblue","lightyellow","lime","limegreen","linen","magenta","maroon","mediumaquamarine","mediumblue","mediumorchid","mediumpurple","mediumseagreen","mediumslateblue","mediumspringgreen","mediumturquoise","mediumvioletred","midnightblue","mintcream","mistyrose","moccasin","navajowhite","navy","oldlace","olive","olivedrab","orange","orangered","orchid","palegoldenrod","palegreen","paleturquoise","palevioletred","papayawhip","peachpuff","peru","pink","plum","powderblue","purple","rebeccapurple","red","rosybrown","royalblue","saddlebrown","salmon","sandybrown","seagreen","seashell","sienna","silver","skyblue","slateblue","slategray","slategrey","snow","springgreen","steelblue","tan","teal","thistle","tomato","turquoise","violet","wheat","white","whitesmoke","yellow","yellowgreen"}
INTERNAL_ACTION = re.compile(r"^(k[A-Z]|K[A-Z][a-z]+[A-Z])|Action[-_]?$|Menu$|SubMenu$|Popup$|_Menu$|^ES [A-Z]|^[a-z]+_[a-z0-9]+_(fp16|fp32|int8)$|^[a-z0-9_]+_(fp16|fp32|v\d)$")
ERRORISH = re.compile(r"^(an? |the )?(unknown |internal )?error|failed|cannot|could not|unable|not (found|available|supported)|show an error", re.I)
ACRONYM_ONLY = re.compile(r"^[A-Z]{2,6}[0-9.\-]*$")
SENTENCE_DESC = re.compile(r"^(Create|Choose|Select|Show|Hide|Convert|Apply|Add|Remove|Set|Use|Enable|Disable|Toggle|Lock|Draw|Paint|Erase|Move|Scale|Rotate)\s+.{12,}", re.I)


def is_capability(name: str) -> bool:
    n = name.strip()
    low = n.lower()
    if low in CSS_COLORS:
        return False
    if INTERNAL_ACTION.search(n) or ERRORISH.match(n):
        return False
    if ACRONYM_ONLY.match(n) and len(n) <= 6:
        return False
    if SENTENCE_DESC.match(n) and len(n) > 34:
        return False  # tool tooltip prose, not a capability name
    return True


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def norm(s: str) -> str:
    s = re.sub(r"\.\.\.|…", "", s)
    s = re.sub(r"[^a-z0-9 ]+", " ", s.lower())
    return re.sub(r"\s+", " ", s).strip()


def toks(s: str) -> frozenset:
    return frozenset(t for t in norm(s).split() if t not in STOP and len(t) > 2)


# When a capability's own name carries no domain vocabulary, the UI context it was captured
# from usually does. "Affine Tool" is meaningless alone; captured under "Shape tool reflection"
# it is clearly vector.
CONTEXT_DOMAIN = [
    (r"text style|paragraph|character panel|opentype|glyph|typograph|font usage|hyphenation|justification|indents and tabs|runin|kerning|composite font|kinsoku|mojikumi|ruby", "typography"),
    (r"\binsert\b|special character|dash|quote|break", "typography"),
    (r"document metadata|document preset|new document|spread|page string|short page|master page|section|book|index|toc|preflight|data merge|frame text|text frame|text wrap|story|note.?s? panel|galley|gap tool|layout adjust|scotch|column", "layout"),
    (r"export propert|export label|import|place|link|package|save for web|pdf|epub|jpeg|png|svg|snippet|tagged text|incopy|xmedia", "interop"),
    (r"shape tool|shape name|arrow head|node tool|pen tool|curve|corner|path type|spline|pathfinder|boolean|vector|transform panel|align|perspective|isometric", "vector"),
    (r"brush|adjustment brush|filter brush|tone brush|photo persona|liquify|develop history|astrophotograph|stack history|retouch|selection|mask|channel|pixel persona|inpaint|frequency", "raster"),
    (r"filters|procedural texture|effect|live filter|blend|layer effect|distort|blur|lighting", "effects"),
    (r"colour|color picker|swatch|gradient|palette|icc|proof|separation|ink|overprint|tone map|lut", "color"),
    (r"develop|camera raw|lens|tone curve|calibration|catalog|cull|rating|keyword|import dialog", "raw"),
    (r"grid and snapping|guide|ruler|snap", "layout"),
    (r"macro|script|action|batch|automation|query|find.?change|grep|workspace|preset manager", "automation"),
    (r"asset|library|style|component|symbol|template|token", "design_system"),
    (r"layer command|layer action|layer state|history|undo|selection set|document object|reflectable|reflected|constant description", "document_model"),
    (r"button|behavior|behaviour|hyperlink|bookmark|animation|media|dynamic document|form field", "prototype"),
    (r"dw_command|dw_dialog|dw_panel|dw_inspector|dw_tag|dw_doctype|dw_server|dw_snippet|dw_css|dw_insert|dw_behavior|dw_coloring|dw_starter|dw_connection|dw_scripted|dw_toolbar|dw_object", "web"),
]


def domain_of(text: str, ctx: str = "") -> str:
    t = text.lower()
    for dom, rx in DOMAIN_RULES:
        if re.search(rx, t):
            return dom
    c = (ctx or "").lower()
    for rx, dom in CONTEXT_DOMAIN:
        if re.search(rx, c):
            return dom
    return "cross_cutting"


def kind_of(text: str, default: str) -> str:
    t = text.lower()
    for k, rx in KIND_HINTS:
        if re.search(rx, t):
            return k
    return default


def keep(name: str) -> bool:
    n = name.strip()
    if not (2 < len(n) <= 64):
        return False
    if NOISE.match(n) or BADCHARS.search(n):
        return False
    if n.count(" ") > 7 or n.endswith((":", ";", ",")):
        return False
    if re.search(r"\b(you|your|please|cannot|could not|unable|failed|will be|has been|do you|are you)\b", n, re.I):
        return False
    return True


def add(rows: dict, name: str, app: str, kind: str, evidence: str):
    """Record one capability observation.

    GRD-001: the merge key is (normalised name, kind, domain), NOT the normalised name alone.
    Keying on name alone merged unrelated capabilities that happen to share a display name --
    Affinity's Hue tone-brush tool and Figma's Hue blend-mode enum value became a single row
    that inherited kind=tool and claimed both applications as sources. That corrupted the kind
    field and inflated every cross-app sharing count. Cross-application merging still happens,
    but only between rows that agree on what kind of thing they are and which domain they sit in.
    """
    if not keep(name) or not is_capability(name):
        return
    ctx_for_domain = evidence.split(":", 2)[-1] if ":" in evidence else ""
    k = (norm(name), kind, domain_of(name, ctx_for_domain))
    if not k:
        return
    r = rows.setdefault(k, {"name": name.strip(), "kind": kind, "apps": set(), "evidence": [], "variants": set()})
    r["apps"].add(app)
    r["variants"].add(name.strip())
    if len(r["evidence"]) < 8:
        r["evidence"].append(f"{app}:{evidence}")
    # No kind promotion: kind is part of the merge key now, so a row's kind never changes
    # after creation. A more specific observation of the same name creates its own row.


def ingest_typelib(p: Path, app: str, rows: dict, prim: dict):
    if not p.exists():
        return
    tl = json.loads(p.read_text(encoding="utf-8"))
    for cname, c in tl.get("classes", {}).items():
        prim.setdefault(norm(cname), {"name": cname, "apps": set(), "members": 0})
        prim[norm(cname)]["apps"].add(app)
        prim[norm(cname)]["members"] = max(prim[norm(cname)]["members"], len(c.get("properties", {})) + len(c.get("methods", [])))
        for m in c.get("methods", []):
            nm = re.sub(r"(?<!^)(?=[A-Z])", " ", m["name"]).strip()
            add(rows, nm, app, "command", f"dom:{cname}.{m['name']}")
        for pname in c.get("properties", {}):
            nm = re.sub(r"(?<!^)(?=[A-Z])", " ", pname).strip()
            add(rows, nm, app, "option", f"dom:{cname}.{pname}")


def ingest_affinity_api(p: Path, app: str, rows: dict, prim: dict):
    if not p.exists():
        return
    api = json.loads(p.read_text(encoding="utf-8"))
    for mod, m in api.get("modules", {}).items():
        for c in m.get("classes", []):
            key = norm(c["name"])
            prim.setdefault(key, {"name": c["name"], "apps": set(), "members": 0})
            prim[key]["apps"].add(app)
            prim[key]["members"] = max(prim[key]["members"], c.get("member_count", 0))
            for mem in c.get("members", []):
                nm = re.sub(r"(?<!^)(?=[A-Z])", " ", mem["name"]).strip()
                add(rows, nm, app, "command" if mem["kind"].endswith("method") else "option", f"jslib:{c['name']}.{mem['name']}")


# Affinity string keys carry their own UI context in brackets. Contexts that denote a real
# product surface are kept; the bare/unlabelled context is the raw resource dump and holds
# autocorrect misspellings, internal identifiers, colour names and localisation fragments.
GOOD_CTX = re.compile(
    r"tool|command|menu|panel|dialog|preference|layer|persona|studio|shortcut|"
    r"adjustment|filter|effect|brush|swatch|colou?r|gradient|fill|stroke|blend|"
    r"text|type|paragraph|character|glyph|font|story|frame|table|"
    r"page|spread|master|document|export|import|format|file|"
    r"selection|mask|channel|node|shape|curve|path|transform|align|snap|grid|guide|"
    r"asset|style|preset|macro|action|script|workspace|toolbar|context bar|"
    r"attribute|property|option|setting|controller|reflection|group|insert|view",
    re.I,
)
BAD_CTX = re.compile(r"object detection|typography language|glyph range|typography script|autocorrect|abbreviation|title exception|function description|error|message|alert|licen|registration|product key|onboarding|analytics|telemetry", re.I)
INTERNAL_ID = re.compile(r"^[A-Za-z]+(_[A-Za-z0-9]+){1,}$|^[a-z]+[A-Z][A-Za-z]*$")


def ingest_strings(paths: list[Path], app: str, rows: dict, dropped: collections.Counter):
    for p in paths:
        if not p.exists():
            continue
        data = json.loads(p.read_text(encoding="utf-8-sig"))
        entries = {}
        if "assemblies" in data:
            for a in data["assemblies"]:
                for rs in a.get("resource_sets", []):
                    entries.update(rs.get("entries", {}))
        elif "tables" in data:
            for t in data["tables"].values():
                entries.update(t.get("entries", {}))
        for key, val in entries.items():
            m = re.match(r"^(.*?)\s*\[([^\]]+)\]\s*$", key)
            text, ctx = (m.group(1), m.group(2)) if m else (key, "")
            if not ctx:
                dropped["no_context"] += 1
                continue
            if BAD_CTX.search(ctx):
                dropped["bad_context"] += 1
                continue
            if not GOOD_CTX.search(ctx):
                dropped["unrecognised_context"] += 1
                continue
            if INTERNAL_ID.match(text.strip()):
                dropped["internal_identifier"] += 1
                continue
            if not all(ord(c) < 128 for c in text):
                dropped["non_ascii"] += 1
                continue
            add(rows, text, app, kind_of(ctx, "capability"), f"str:{ctx}")


def ingest_indesign_surface(p: Path, app: str, rows: dict):
    """Use the clean menu/action surface (idrc_MENR + idrc_ACTD).

    The raw idrc label sweep is deliberately NOT used: it returned 118k rows that were mostly
    localized strings in 33 languages, internal identifiers and sentence fragments.
    """
    if not p.exists():
        return
    d = json.loads(p.read_text(encoding="utf-8"))
    for leaf in d.get("menu_leaves", []):
        add(rows, leaf["leaf"], app, "menu", f"menu:{leaf['path'][:60]}")
    for a in d.get("actions", []):
        add(rows, a, app, "command", "action:idrc_ACTD")


def ingest_presets(off: Path, app: str, fams: dict):
    p = off / "presets.json"
    if p.exists():
        d = json.loads(p.read_text(encoding="utf-8"))
        for fam, n in d.get("by_family", {}).items():
            f = fam.replace("preset:", "")
            e = fams.setdefault(f, {"family": f, "apps": {}, "stock_entries": {}, "categories": {}})
            e["apps"][app] = n
    p2 = off / "presets_names_scan.json"
    if p2.exists():
        d = json.loads(p2.read_text(encoding="utf-8"))
        for f in d.get("files", []):
            key = Path(f["file"]).stem
            e = fams.setdefault(key, {"family": key, "apps": {}, "stock_entries": {}, "categories": {}})
            e["stock_entries"][app] = f.get("candidate_name_count", 0)
            e["categories"][app] = len([c for c in f.get("categories_guess", []) if c.get("kind") in ("category", "tree_group")])


def ingest_teardown(off: Path, app: str, rows: dict, prim: dict, stats: dict):
    """Ingest the per-app deep-teardown artifacts produced by the teardown agents.

    These carry the behavioural depth (real preset entries, panels, parameters, enums,
    dialog controls, shortcuts) that the first-pass harvest did not have.
    """
    def load(name):
        p = off / name
        if not p.exists():
            return None
        try:
            return json.loads(p.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            return None

    # real panel inventories (replacing UXP manifest counts)
    for fname, field, namekey in (
        (f"{app}_panels.json", "native_panels", "native_name"),
        (f"{app}_panels.json", "native_panels", "panel_id"),
        (f"{app}_tool_panel_registry.json", "panels", "name"),
        (f"{app}_tool_panel_registry.json", "tools", "name"),
    ):
        d = load(fname)
        if not d:
            continue
        for r in d.get(field, []) or []:
            nm = r.get(namekey) if isinstance(r, dict) else None
            if nm:
                kind = "tool" if field == "tools" else "panel"
                add(rows, str(nm), app, kind, f"teardown:{fname}:{field}")
                stats[f"{app}.{field}"] = stats.get(f"{app}.{field}", 0) + 1

    # real preset entries (replacing file counts)
    for fname in (f"{app}_preset_contents.json", f"{app}_library_contents.json"):
        d = load(fname)
        if not d:
            continue
        for cont in d.get("containers", []) or d.get("libraries", []) or []:
            if not isinstance(cont, dict):
                continue
            fam = cont.get("family") or cont.get("library") or "preset"
            ents = cont.get("entries")
            if isinstance(ents, dict):
                ents = list(ents.values())
            if not isinstance(ents, list):
                ents = []
            for e in ents[:4000]:
                nm = e.get("name") if isinstance(e, dict) else (e if isinstance(e, str) else None)
                if nm:
                    add(rows, str(nm), app, "preset", f"teardown:{fname}:{fam}")
                    stats[f"{app}.preset_entries"] = stats.get(f"{app}.preset_entries", 0) + 1

    # parameter surface -> options, and classes -> primitives
    d = load(f"{app}_parameter_surface.json")
    if d:
        for cname, c in (d.get("classes") or {}).items():
            if not isinstance(c, dict):
                continue
            key = norm(cname)
            prim.setdefault(key, {"name": cname, "apps": set(), "members": 0})
            prim[key]["apps"].add(app)
            prim[key]["members"] = max(prim[key]["members"], (c.get("property_count") or 0) + (c.get("method_count") or 0))
            for pr in (c.get("properties") or []):
                nm = pr.get("name") if isinstance(pr, dict) else pr
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "option", f"teardown:param:{cname}.{nm}")
                    stats[f"{app}.parameters"] = stats.get(f"{app}.parameters", 0) + 1
            for me in (c.get("methods") or []):
                nm = me.get("name") if isinstance(me, dict) else me
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "command", f"teardown:param:{cname}.{nm}")

    # enumerator vocabulary -> options
    d = load(f"{app}_enums.json")
    if d:
        _enums = d.get("enums")
        _enum_iter = _enums if isinstance(_enums, list) else [{"name": k, **(v if isinstance(v, dict) else {})} for k, v in (_enums or {}).items()]
        for en in _enum_iter:
            if not isinstance(en, dict):
                continue
            ename = en.get("name", "")
            for m in (en.get("members") or [])[:400]:
                nm = m.get("name") if isinstance(m, dict) else m
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "option", f"teardown:enum:{ename}")
                    stats[f"{app}.enumerators"] = stats.get(f"{app}.enumerators", 0) + 1

    # effects catalog
    d = load(f"{app}_effects.json")
    if d:
        for e in d.get("effects", []) or []:
            nm = e.get("name") if isinstance(e, dict) else (e if isinstance(e, str) else None)
            if nm:
                add(rows, str(nm), app, "capability", f"teardown:effect")
                stats[f"{app}.effects"] = stats.get(f"{app}.effects", 0) + 1

    # shortcuts -> the commands they bind
    d = load(f"{app}_shortcuts_full.json") or load(f"{app}_shortcuts.json")
    if d:
        for sc in d.get("all_shortcuts", []) or d.get("shortcuts", []) or []:
            nm = sc.get("target_name") if isinstance(sc, dict) else None
            if nm:
                add(rows, str(nm), app, "command", f"teardown:shortcut:{sc.get('section','')}")
                stats[f"{app}.shortcut_targets"] = stats.get(f"{app}.shortcut_targets", 0) + 1

    # develop / text / brush / adjustment parameter models
    for fname, field in ((f"{app}_develop_parameters.json", "parameters"), (f"{app}_text_model.json", "attributes"),
                         (f"{app}_brush_parameters.json", "parameters"), (f"{app}_adjustment_parameters.json", "parameters"),
                         (f"{app}_export_pipeline.json", "settings"), (f"{app}_sdk_api.json", "functions")):
        d = load(fname)
        if not d:
            continue
        for r in d.get(field, []) or []:
            nm = r.get("name") if isinstance(r, dict) else (r if isinstance(r, str) else None)
            if nm:
                kind = "command" if field == "functions" else "option"
                add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, kind, f"teardown:{fname}:{field}")
                stats[f"{app}.{Path(fname).stem}"] = stats.get(f"{app}.{Path(fname).stem}", 0) + 1


    # Lightroom-shaped teardown: develop parameters, export keys, SDK members, templates, profiles
    d = load(f"{app}_develop_parameters.json")
    if d:
        for r in d.get("parameters", []) or []:
            if isinstance(r, dict) and r.get("name"):
                nm = re.sub(r"(?<!^)(?=[A-Z])", " ", str(r["name"])).strip()
                add(rows, nm, app, "option", f"teardown:develop:{r.get('panel','')}")
                stats[f"{app}.develop_parameters"] = stats.get(f"{app}.develop_parameters", 0) + 1

    d = load(f"{app}_export_pipeline.json")
    if d:
        for section in ("still_image_export", "video_export"):
            blk = d.get(section) or {}
            for key in ("keys", "settings", "parameters", "confirmed_export_settings"):
                for r in (blk.get(key) or []):
                    nm = r.get("name") or r.get("key") if isinstance(r, dict) else (r if isinstance(r, str) else None)
                    if nm:
                        add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "option", f"teardown:export:{section}")
                        stats[f"{app}.export_keys"] = stats.get(f"{app}.export_keys", 0) + 1

    d = load(f"{app}_sdk_api.json")
    if d:
        for grp in d.get("sdk_api_surface", []) or []:
            if not isinstance(grp, dict):
                continue
            for m in (grp.get("members") or []):
                nm = m.get("name") if isinstance(m, dict) else (m if isinstance(m, str) else None)
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "command", f"teardown:sdk:{grp.get('role','')}")
                    stats[f"{app}.sdk_members"] = stats.get(f"{app}.sdk_members", 0) + 1

    d = load(f"{app}_templates.json")
    if d:
        for ttype, params in (d.get("parameter_surface_by_type") or {}).items():
            for r in (params or []):
                nm = r.get("name") or r.get("key") if isinstance(r, dict) else (r if isinstance(r, str) else None)
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "option", f"teardown:template:{ttype}")
                    stats[f"{app}.template_params"] = stats.get(f"{app}.template_params", 0) + 1

    d = load(f"{app}_profiles.json")
    if d:
        for blk, label in (("camera_profiles_dcp", "camera_profile"), ("lens_profiles_lcp", "lens_profile")):
            b = d.get(blk) or {}
            for key in ("attribute_vocabulary", "attributes", "models"):
                for r in (b.get(key) or []):
                    nm = r.get("name") or r.get("attribute") if isinstance(r, dict) else (r if isinstance(r, str) else None)
                    if nm:
                        add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "option", f"teardown:{label}")
                        stats[f"{app}.profile_attrs"] = stats.get(f"{app}.profile_attrs", 0) + 1


    # Illustrator-shaped teardown: library entries, native panel catalogue, effect dialogs/params
    d = load(f"{app}_library_contents.json")
    if d:
        for lib in d.get("libraries", []) or []:
            if not isinstance(lib, dict):
                continue
            fam = lib.get("family", "library")
            _ents = lib.get("entries")
            if isinstance(_ents, dict):
                _flat = []
                for _k, _v in _ents.items():
                    if isinstance(_v, list):
                        _flat.extend(_v)
                    else:
                        _flat.append({"name": _k})
                _ents = _flat
            if not isinstance(_ents, list):
                _ents = []
            for e in _ents[:4000]:
                nm = e.get("name") if isinstance(e, dict) else (e if isinstance(e, str) else None)
                if nm:
                    add(rows, str(nm), app, "preset", f"teardown:library:{fam}")
                    stats[f"{app}.library_entries"] = stats.get(f"{app}.library_entries", 0) + 1

    d = load(f"{app}_panels.json")
    if d:
        for r in d.get("panel_catalogue", []) or []:
            nm = r.get("surface_id") if isinstance(r, dict) else None
            if nm:
                add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "panel", f"teardown:panel:{r.get('origin','')}")
                stats[f"{app}.native_panels"] = stats.get(f"{app}.native_panels", 0) + 1

    d = load(f"{app}_effects.json")
    if d:
        for eff, blk in (d.get("serialized_live_effects") or {}).items():
            add(rows, str(eff), app, "capability", "teardown:live_effect")
            stats[f"{app}.live_effects"] = stats.get(f"{app}.live_effects", 0) + 1
            for pname in (blk.get("parameters") or {}) if isinstance(blk, dict) else []:
                add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(pname)).strip(), app, "option", f"teardown:effect_param:{eff}")
                stats[f"{app}.effect_parameters"] = stats.get(f"{app}.effect_parameters", 0) + 1
        for grp, items in (d.get("effect_menu_index") or {}).items():
            for it in (items or []):
                nm = it.get("name") if isinstance(it, dict) else (it if isinstance(it, str) else None)
                if nm:
                    add(rows, str(nm), app, "capability", f"teardown:effect_menu:{grp}")
                    stats[f"{app}.effect_menu_items"] = stats.get(f"{app}.effect_menu_items", 0) + 1
        for plug, blk in (d.get("dialogs") or {}).items():
            if not isinstance(blk, dict):
                continue
            for lay in (blk.get("layouts") or []):
                nm = lay.get("name") if isinstance(lay, dict) else (lay if isinstance(lay, str) else None)
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "dialog", "teardown:eve_dialog")
                    stats[f"{app}.eve_dialogs"] = stats.get(f"{app}.eve_dialogs", 0) + 1
                for pr in (lay.get("parameters") or []) if isinstance(lay, dict) else []:
                    pn = pr.get("name") if isinstance(pr, dict) else None
                    if pn:
                        add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(pn)).strip(), app, "option", "teardown:eve_param")
                        stats[f"{app}.eve_parameters"] = stats.get(f"{app}.eve_parameters", 0) + 1

    d = load(f"{app}_shortcuts.json")
    if d:
        for sc in d.get("shortcuts", []) or []:
            nm = sc.get("command_id") if isinstance(sc, dict) else None
            if nm:
                add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "command", f"teardown:shortcut:{sc.get('section','')}")
                stats[f"{app}.shortcut_commands"] = stats.get(f"{app}.shortcut_commands", 0) + 1


    # InDesign-shaped teardown: full scripting DOM, dialogs, text model, error catalog
    d = load(f"{app}_dom_full.json")
    if d:
        for c in d.get("classes", []) or []:
            if isinstance(c, dict) and c.get("name"):
                key = norm(c["name"])
                prim.setdefault(key, {"name": c["name"], "apps": set(), "members": 0})
                prim[key]["apps"].add(app)
                stats[f"{app}.dom_classes"] = stats.get(f"{app}.dom_classes", 0) + 1
        for pr in d.get("properties", []) or []:
            if isinstance(pr, dict) and pr.get("name"):
                add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(pr["name"])).strip(), app, "option", f"teardown:dom_property:{pr.get('plugin','')}")
                stats[f"{app}.dom_properties"] = stats.get(f"{app}.dom_properties", 0) + 1
        for me in d.get("methods", []) or []:
            if isinstance(me, dict) and me.get("name"):
                add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(me["name"])).strip(), app, "command", f"teardown:dom_method:{me.get('plugin','')}")
                stats[f"{app}.dom_methods"] = stats.get(f"{app}.dom_methods", 0) + 1
        for en in d.get("enumerations", []) or []:
            if not isinstance(en, dict):
                continue
            for v in (en.get("enumerators") or [])[:400]:
                nm = v.get("name") if isinstance(v, dict) else (v if isinstance(v, str) else None)
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "option", f"teardown:dom_enum:{en.get('name','')}")
                    stats[f"{app}.dom_enumerators"] = stats.get(f"{app}.dom_enumerators", 0) + 1

    d = load(f"{app}_dialogs.json")
    if d and d.get("dialogs_and_panels"):
        for r in d["dialogs_and_panels"]:
            if not isinstance(r, dict):
                continue
            nm = r.get("dialog_or_panel")
            if nm:
                add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "dialog", "teardown:view_dialog")
                stats[f"{app}.dialogs_parsed"] = stats.get(f"{app}.dialogs_parsed", 0) + 1
            for lab in (r.get("labels") or [])[:80]:
                if isinstance(lab, str) and 2 < len(lab) <= 48:
                    add(rows, lab, app, "option", f"teardown:dialog_control:{nm}")
                    stats[f"{app}.dialog_controls"] = stats.get(f"{app}.dialog_controls", 0) + 1

    d = load(f"{app}_text_model.json")
    if d:
        for a_ in d.get("attributes", []) or []:
            if isinstance(a_, dict) and a_.get("name"):
                add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(a_["name"])).strip(), app, "option", f"teardown:text_attr:{a_.get('plugin','')}")
                stats[f"{app}.text_attributes"] = stats.get(f"{app}.text_attributes", 0) + 1
        for en in d.get("enumerations", []) or []:
            if not isinstance(en, dict):
                continue
            for v in (en.get("values") or [])[:200]:
                nm = v.get("name") if isinstance(v, dict) else (v if isinstance(v, str) else None)
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "option", f"teardown:text_enum:{en.get('name','')}")
                    stats[f"{app}.text_enumerators"] = stats.get(f"{app}.text_enumerators", 0) + 1


    # Affinity-shaped teardown: KA-parsed presets, brush + adjustment parameter schemas,
    # workspace tool/panel id registry, scripting API detail
    d = load(f"{app}_preset_contents.json")
    if d:
        for cont in d.get("containers", []) or []:
            if not isinstance(cont, dict):
                continue
            fam = Path(str(cont.get("file", "preset"))).stem
            def _leaves(node, depth=0):
                if depth > 6 or not isinstance(node, dict):
                    return
                nm = node.get("name")
                if nm and isinstance(nm, str):
                    yield nm
                for key in ("children", "nodes", "leaves", "presets", "entries"):
                    kids = node.get(key)
                    if isinstance(kids, dict):
                        kids = list(kids.values())
                    if not isinstance(kids, list):
                        continue
                    for ch in kids:
                        yield from _leaves(ch, depth + 1)
            _roots = cont.get("tree") or cont.get("nodes") or cont.get("presets") or []
            if isinstance(_roots, dict):
                _roots = list(_roots.values())
            if not isinstance(_roots, list):
                _roots = []
            for root in _roots:
                for nm in _leaves(root):
                    add(rows, nm, app, "preset", f"teardown:ka:{fam}")
                    stats[f"{app}.ka_presets"] = stats.get(f"{app}.ka_presets", 0) + 1

    d = load(f"{app}_brush_parameters.json")
    if d:
        for b in d.get("brushes", []) or []:
            if isinstance(b, dict) and b.get("name"):
                add(rows, str(b["name"]), app, "preset", f"teardown:brush:{b.get('category','')}")
                stats[f"{app}.brushes"] = stats.get(f"{app}.brushes", 0) + 1
        for pr in d.get("parameter_schema", []) or []:
            if isinstance(pr, dict) and pr.get("tag"):
                add(rows, f"Brush {pr['tag']}", app, "option", "teardown:brush_param")
                stats[f"{app}.brush_parameters"] = stats.get(f"{app}.brush_parameters", 0) + 1

    d = load(f"{app}_adjustment_parameters.json")
    if d:
        for a_ in d.get("adjustments", []) or []:
            if not isinstance(a_, dict):
                continue
            if a_.get("adjustment"):
                add(rows, str(a_["adjustment"]), app, "capability", "teardown:adjustment")
                stats[f"{app}.adjustments"] = stats.get(f"{app}.adjustments", 0) + 1
            for pr in (a_.get("parameter_schema") or []):
                tag = pr.get("tag") if isinstance(pr, dict) else None
                if tag:
                    add(rows, f"{a_.get('adjustment','')} {tag}".strip(), app, "option", "teardown:adjustment_param")
                    stats[f"{app}.adjustment_parameters"] = stats.get(f"{app}.adjustment_parameters", 0) + 1
            for pset in (a_.get("presets") or []):
                pn = pset.get("name") if isinstance(pset, dict) else None
                if pn:
                    add(rows, str(pn), app, "preset", f"teardown:adjustment_preset:{a_.get('adjustment','')}")
                    stats[f"{app}.adjustment_presets"] = stats.get(f"{app}.adjustment_presets", 0) + 1

    d = load(f"{app}_tool_panel_registry.json")
    if d:
        for field, kind in (("tool_ids", "tool"), ("panel_ids", "panel"), ("toolbar_command_ids", "command"), ("boolean_setting_ids", "option")):
            for r in d.get(field, []) or []:
                idc = r.get("id_4cc") if isinstance(r, dict) else None
                if idc:
                    add(rows, f"{kind}:{idc}", app, kind, f"teardown:workspace_id:{field}")
                    stats[f"{app}.{field}"] = stats.get(f"{app}.{field}", 0) + 1

    d = load(f"{app}_scripting_api_detail.json")
    if d:
        for m in d.get("native_api_methods", []) or []:
            if not isinstance(m, dict):
                continue
            cls = m.get("api_class", "")
            for me in (m.get("methods") or [])[:400]:
                nm = me.get("name") if isinstance(me, dict) else (me if isinstance(me, str) else None)
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "command", f"teardown:native_api:{cls}")
                    stats[f"{app}.native_api_methods"] = stats.get(f"{app}.native_api_methods", 0) + 1
        for en in d.get("native_enum_members_recovered", []) or []:
            if not isinstance(en, dict):
                continue
            for v in (en.get("members") or [])[:200]:
                nm = v if isinstance(v, str) else (v.get("name") if isinstance(v, dict) else None)
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "option", f"teardown:native_enum:{en.get('symbol','')}")
                    stats[f"{app}.native_enum_members"] = stats.get(f"{app}.native_enum_members", 0) + 1


    # Figma-shaped teardown: object model parsed from public TypeScript declarations
    d = load("figma_object_model.json")
    if d:
        for n in d.get("node_types", []) or []:
            if not isinstance(n, dict):
                continue
            nt = n.get("node_type", "")
            add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", nt).strip(), app, "capability", "teardown:figma_node")
            key = norm(nt)
            prim.setdefault(key, {"name": nt, "apps": set(), "members": 0})
            prim[key]["apps"].add(app)
            prim[key]["members"] = max(prim[key]["members"], (n.get("resolved_property_count") or 0) + (n.get("resolved_method_count") or 0))
            stats[f"{app}.node_types"] = stats.get(f"{app}.node_types", 0) + 1
            for pr in (n.get("properties") or []):
                pn = pr.get("name") if isinstance(pr, dict) else None
                if pn:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(pn)).strip(), app, "option", f"teardown:figma_prop:{nt}")
                    stats[f"{app}.node_properties"] = stats.get(f"{app}.node_properties", 0) + 1
            for me in (n.get("methods") or []):
                mn = me.get("name") if isinstance(me, dict) else None
                if mn:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(mn)).strip(), app, "command", f"teardown:figma_method:{nt}")
                    stats[f"{app}.node_methods"] = stats.get(f"{app}.node_methods", 0) + 1
        for en in d.get("enums", []) or []:
            if not isinstance(en, dict):
                continue
            for v in (en.get("values") or [])[:300]:
                if isinstance(v, str) and 2 < len(v) <= 48:
                    add(rows, v.replace("_", " ").replace("-", " ").title(), app, "option", f"teardown:figma_enum:{en.get('name','')}")
                    stats[f"{app}.enum_values"] = stats.get(f"{app}.enum_values", 0) + 1
        for i_ in d.get("interfaces", []) or []:
            if not isinstance(i_, dict):
                continue
            nm = i_.get("name", "")
            if nm.endswith(("API", "Api")):
                for me in (i_.get("members") or []):
                    if isinstance(me, dict) and me.get("kind") == "method":
                        add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(me["name"])).strip(), app, "command", f"teardown:figma_api:{nm}")
                        stats[f"{app}.api_methods"] = stats.get(f"{app}.api_methods", 0) + 1


    # Dreamweaver-shaped teardown: command surface, dialogs/inspectors, objects/behaviours,
    # tag libraries and code intelligence, server/site model, snippets and CSS designer
    d = load(f"{app}_command_surface.json")
    if d:
        for e in d.get("flat_command_index", []) or []:
            if not isinstance(e, dict):
                continue
            nm = e.get("label") or e.get("id")
            if nm:
                add(rows, str(nm), app, "command", f"teardown:dw_command:{e.get('menubar_id','')}")
                stats[f"{app}.commands"] = stats.get(f"{app}.commands", 0) + 1
        for sc in d.get("scripted_commands", []) or []:
            nm = sc.get("display_title") if isinstance(sc, dict) else None
            if nm:
                add(rows, str(nm), app, "command", "teardown:dw_scripted_command")
                stats[f"{app}.scripted_commands"] = stats.get(f"{app}.scripted_commands", 0) + 1

    d = load(f"{app}_panels_dialogs.json")
    if d:
        for dl in d.get("native_dialogs", []) or []:
            nm = dl.get("window_title") or dl.get("layout_name") if isinstance(dl, dict) else None
            if nm:
                add(rows, str(nm), app, "dialog", "teardown:dw_dialog")
                stats[f"{app}.dialogs"] = stats.get(f"{app}.dialogs", 0) + 1
        for ins in d.get("property_inspectors", []) or []:
            nm = ins.get("title") if isinstance(ins, dict) else None
            if nm:
                add(rows, str(nm), app, "panel", f"teardown:dw_inspector:{ins.get('binds_to_tag','')}")
                stats[f"{app}.inspectors"] = stats.get(f"{app}.inspectors", 0) + 1
        for pn in d.get("panel_registry", []) or []:
            nm = pn.get("menu_label") or pn.get("panel_id") if isinstance(pn, dict) else None
            if nm:
                add(rows, str(nm), app, "panel", "teardown:dw_panel")
                stats[f"{app}.panels"] = stats.get(f"{app}.panels", 0) + 1
        for tb in d.get("toolbars", []) or []:
            nm = tb.get("label") or tb.get("id") if isinstance(tb, dict) else None
            if nm:
                add(rows, str(nm), app, "command", "teardown:dw_toolbar")

    d = load(f"{app}_objects_behaviors.json")
    if d:
        for cat in d.get("insert_panel", []) or []:
            if not isinstance(cat, dict):
                continue
            for it in (cat.get("items") or []):
                nm = it.get("label") or it.get("id") if isinstance(it, dict) else None
                if nm:
                    add(rows, str(nm), app, "capability", f"teardown:dw_insert:{cat.get('category_label','')}")
                    stats[f"{app}.insert_objects"] = stats.get(f"{app}.insert_objects", 0) + 1
        for ob in d.get("object_implementations", []) or []:
            nm = ob.get("display_title") if isinstance(ob, dict) else None
            if nm:
                add(rows, str(nm), app, "capability", "teardown:dw_object")
        for bh in d.get("behaviors", []) or []:
            nm = bh.get("display_title") if isinstance(bh, dict) else None
            if nm:
                add(rows, str(nm), app, "capability", f"teardown:dw_behavior:{bh.get('group','')}")
                stats[f"{app}.behaviors"] = stats.get(f"{app}.behaviors", 0) + 1

    d = load(f"{app}_code_intelligence.json")
    if d:
        for t in d.get("tags", []) or []:
            nm = t.get("tag_name") if isinstance(t, dict) else None
            if nm:
                add(rows, f"tag {nm}", app, "capability", f"teardown:dw_tag:{t.get('library_id','')}")
                stats[f"{app}.tags"] = stats.get(f"{app}.tags", 0) + 1
        for dt_ in d.get("document_types", []) or []:
            nm = dt_.get("title") or dt_.get("id") if isinstance(dt_, dict) else None
            if nm:
                add(rows, str(nm), app, "capability", "teardown:dw_doctype")
                stats[f"{app}.document_types"] = stats.get(f"{app}.document_types", 0) + 1
        for cs in d.get("code_coloring_schemes", []) or []:
            nm = cs.get("scheme_name") if isinstance(cs, dict) else None
            if nm:
                add(rows, str(nm), app, "option", "teardown:dw_coloring")

    d = load(f"{app}_site_server_model.json")
    if d:
        for sb in d.get("server_behaviors", []) or []:
            nm = sb.get("behavior_name") if isinstance(sb, dict) else None
            if nm:
                add(rows, str(nm), app, "capability", f"teardown:dw_server_behavior:{sb.get('server_model','')}")
                stats[f"{app}.server_behaviors"] = stats.get(f"{app}.server_behaviors", 0) + 1
        for sm in d.get("server_models", []) or []:
            nm = sm.get("model_key") if isinstance(sm, dict) else None
            if nm:
                add(rows, f"server model {nm}", app, "capability", "teardown:dw_server_model")
        for cn in d.get("connections", []) or []:
            nm = cn.get("title") if isinstance(cn, dict) else None
            if nm:
                add(rows, str(nm), app, "dialog", "teardown:dw_connection")

    d = load(f"{app}_templates_css.json")
    if d:
        for sn in d.get("snippets", []) or []:
            nm = sn.get("snippet_name") if isinstance(sn, dict) else None
            if nm:
                add(rows, str(nm), app, "preset", f"teardown:dw_snippet:{sn.get('top_level_group','')}")
                stats[f"{app}.snippets"] = stats.get(f"{app}.snippets", 0) + 1
        for pr in d.get("css_designer_property_surface", []) or []:
            nm = pr.get("display_name") or pr.get("property") if isinstance(pr, dict) else None
            if nm:
                add(rows, str(nm), app, "option", "teardown:dw_css_property")
                stats[f"{app}.css_properties"] = stats.get(f"{app}.css_properties", 0) + 1
        for st in d.get("starter_documents", []) or []:
            nm = st.get("name") if isinstance(st, dict) else None
            if nm:
                add(rows, str(nm), app, "preset", "teardown:dw_starter")


    # Premiere-shaped teardown: effects, Lumetri, export pipeline, sequence model,
    # panels/dialogs, commands, motion graphics, media IO
    d = load(f"{app}_effects_catalogue.json")
    if d:
        for e in d.get("effects", []) or []:
            if not isinstance(e, dict):
                continue
            nm = e.get("display_name") or e.get("effect_key")
            if nm:
                add(rows, str(nm), app, "capability", f"teardown:pp_effect:{e.get('kind','')}")
                stats[f"{app}.effects"] = stats.get(f"{app}.effects", 0) + 1
            for pr in (e.get("parameters") or []):
                pn = (pr.get("name") or pr.get("label")) if isinstance(pr, dict) else None
                if pn:
                    add(rows, str(pn), app, "option", f"teardown:pp_effect_param:{nm}")
                    stats[f"{app}.effect_parameters"] = stats.get(f"{app}.effect_parameters", 0) + 1
        for cat in d.get("effect_categories", []) or []:
            nm = cat.get("label") if isinstance(cat, dict) else None
            if nm:
                add(rows, str(nm), app, "capability", "teardown:pp_effect_category")

    d = load(f"{app}_lumetri_color.json")
    if d:
        lc = d.get("lumetri_color_effect") or {}
        for pr in (lc.get("parameters") or []):
            pn = (pr.get("name") or pr.get("label")) if isinstance(pr, dict) else None
            if pn:
                add(rows, str(pn), app, "option", "teardown:pp_lumetri_param")
                stats[f"{app}.lumetri_parameters"] = stats.get(f"{app}.lumetri_parameters", 0) + 1
        for lk in d.get("shipped_look_presets", []) or []:
            nm = lk.get("preset_name") if isinstance(lk, dict) else None
            if nm:
                add(rows, str(nm), app, "preset", "teardown:pp_look")
                stats[f"{app}.looks"] = stats.get(f"{app}.looks", 0) + 1

    d = load(f"{app}_export_pipeline.json")
    if d:
        for pr in d.get("exporter_parameter_dictionary", []) or []:
            nm = (pr.get("label") or pr.get("identifier")) if isinstance(pr, dict) else None
            if nm:
                add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "option", "teardown:pp_export_param")
                stats[f"{app}.export_parameters"] = stats.get(f"{app}.export_parameters", 0) + 1
        for ps in (d.get("presets") or [])[:2000]:
            nm = ps.get("preset_name") if isinstance(ps, dict) else None
            if nm:
                add(rows, str(nm), app, "preset", f"teardown:pp_export_preset:{ps.get('exporter_name','')}")
                stats[f"{app}.export_presets"] = stats.get(f"{app}.export_presets", 0) + 1

    d = load(f"{app}_sequence_project_model.json")
    if d:
        for sp in (d.get("sequence_presets") or [])[:1000]:
            nm = sp.get("name") if isinstance(sp, dict) else None
            if nm:
                add(rows, str(nm), app, "preset", "teardown:pp_sequence_preset")
                stats[f"{app}.sequence_presets"] = stats.get(f"{app}.sequence_presets", 0) + 1
        for em in d.get("editing_modes", []) or []:
            nm = em.get("name") if isinstance(em, dict) else None
            if nm:
                add(rows, str(nm), app, "option", "teardown:pp_editing_mode")
                stats[f"{app}.editing_modes"] = stats.get(f"{app}.editing_modes", 0) + 1

    d = load(f"{app}_panels_dialogs.json")
    if d:
        for field, label in (("dialogs_and_panels", "pp_dvaui"), ("eve_surfaces", "pp_eve")):
            for sfc in d.get(field, []) or []:
                if not isinstance(sfc, dict):
                    continue
                nm = sfc.get("surface")
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, "dialog", f"teardown:{label}")
                    stats[f"{app}.{field}"] = stats.get(f"{app}.{field}", 0) + 1
                for ct in (sfc.get("controls") or [])[:120]:
                    cn = (ct.get("label") or ct.get("name")) if isinstance(ct, dict) else None
                    if cn and isinstance(cn, str) and 2 < len(cn) <= 48:
                        add(rows, cn, app, "option", f"teardown:{label}_control")
                        stats[f"{app}.surface_controls"] = stats.get(f"{app}.surface_controls", 0) + 1

    d = load(f"{app}_commands_shortcuts.json")
    if d:
        for c in d.get("command_surface", []) or []:
            if not isinstance(c, dict):
                continue
            nm = c.get("label") or c.get("command")
            if nm:
                add(rows, str(nm), app, "command", f"teardown:pp_command:{c.get('namespace','')}")
                stats[f"{app}.commands"] = stats.get(f"{app}.commands", 0) + 1

    d = load(f"{app}_graphics_text.json")
    if d:
        for t in d.get("motion_graphics_templates", []) or []:
            if not isinstance(t, dict):
                continue
            nm = t.get("template_name")
            if nm:
                add(rows, str(nm), app, "preset", "teardown:pp_mogrt")
                stats[f"{app}.mogrt_templates"] = stats.get(f"{app}.mogrt_templates", 0) + 1
            for ct in (t.get("exposed_controls") or []):
                cn = ct.get("name") if isinstance(ct, dict) else None
                if cn:
                    add(rows, str(cn), app, "option", "teardown:pp_mogrt_control")
                    stats[f"{app}.mogrt_controls"] = stats.get(f"{app}.mogrt_controls", 0) + 1

    d = load(f"{app}_media_io.json")
    if d:
        for im in d.get("importers", []) or []:
            nm = (im.get("display_name") or im.get("importer_module")) if isinstance(im, dict) else None
            if nm:
                add(rows, str(nm), app, "capability", "teardown:pp_importer")
                stats[f"{app}.importers"] = stats.get(f"{app}.importers", 0) + 1
        for cm in d.get("codec_and_container_modules", []) or []:
            nm = (cm.get("format_family") or cm.get("module")) if isinstance(cm, dict) else None
            if nm:
                add(rows, str(nm), app, "capability", "teardown:pp_codec")
                stats[f"{app}.codecs"] = stats.get(f"{app}.codecs", 0) + 1


    # After Effects-shaped teardown: effects with typed parameter records, presets, the layer and
    # property model, ExtendScript classes and the expression language, panels, render/output,
    # commands, and the text/shape/mask subsystems.
    d = load(f"{app}_effects_catalogue.json")
    if d:
        for e in d.get("effects", []) or []:
            if not isinstance(e, dict):
                continue
            if e.get("registration_only"):
                continue
            nm = e.get("display_name") or e.get("match_name")
            if nm:
                add(rows, str(nm), app, "capability", f"teardown:ae_effect:{e.get('category','')}")
                stats[f"{app}.effects"] = stats.get(f"{app}.effects", 0) + 1
            for pr in (e.get("parameters") or []):
                pn = (pr.get("name") or pr.get("label")) if isinstance(pr, dict) else None
                if pn:
                    add(rows, str(pn), app, "option", f"teardown:ae_effect_param:{nm}")
                    stats[f"{app}.effect_parameters"] = stats.get(f"{app}.effect_parameters", 0) + 1

    d = load(f"{app}_presets.json")
    if d:
        for ps in (d.get("presets") or [])[:2000]:
            nm = ps.get("preset_name") if isinstance(ps, dict) else None
            if nm:
                add(rows, str(nm), app, "preset", f"teardown:ae_preset:{ps.get('category_path','')}")
                stats[f"{app}.presets"] = stats.get(f"{app}.presets", 0) + 1

    d = load(f"{app}_layer_property_model.json")
    if d:
        for lt in d.get("layer_types", []) or []:
            nm = lt.get("layer_type") if isinstance(lt, dict) else None
            if nm:
                add(rows, f"{nm} layer", app, "capability", "teardown:ae_layer_type")
                stats[f"{app}.layer_types"] = stats.get(f"{app}.layer_types", 0) + 1
        for en_name, en in (d.get("enumerations") or {}).items():
            if not isinstance(en, dict):
                continue
            for v in (en.get("options") or [])[:200]:
                nm = v.get("name") if isinstance(v, dict) else (v if isinstance(v, str) else None)
                if nm:
                    add(rows, str(nm), app, "option", f"teardown:ae_enum:{en_name}")
                    stats[f"{app}.enum_values"] = stats.get(f"{app}.enum_values", 0) + 1
        for ls in d.get("layer_styles", []) or []:
            if not isinstance(ls, dict):
                continue
            nm = ls.get("display_name") or ls.get("match_name")
            if nm:
                add(rows, str(nm), app, "capability", "teardown:ae_layer_style")
            for pr in (ls.get("parameters") or []):
                pn = pr.get("name") if isinstance(pr, dict) else None
                if pn:
                    add(rows, str(pn), app, "option", f"teardown:ae_layer_style_param:{nm}")

    d = load(f"{app}_scripting_expressions.json")
    if d:
        for c in d.get("extendscript_object_model", []) or []:
            if not isinstance(c, dict):
                continue
            cls = c.get("class", "")
            key = norm(cls)
            if key:
                prim.setdefault(key, {"name": cls, "apps": set(), "members": 0})
                prim[key]["apps"].add(app)
                prim[key]["members"] = max(prim[key]["members"], c.get("member_count") or 0)
            for m in (c.get("members") or [])[:400]:
                mn = m.get("name") if isinstance(m, dict) else (m if isinstance(m, str) else None)
                if mn:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(mn)).strip(), app, "command", f"teardown:ae_script:{cls}")
                    stats[f"{app}.scripting_members"] = stats.get(f"{app}.scripting_members", 0) + 1
        el = d.get("expression_language") or {}
        for row in (el.get("ordered_table") or []):
            nm = (row.get("identifier") or row.get("name")) if isinstance(row, dict) else (row if isinstance(row, str) else None)
            if nm:
                add(rows, str(nm), app, "command", "teardown:ae_expression")
                stats[f"{app}.expression_identifiers"] = stats.get(f"{app}.expression_identifiers", 0) + 1

    d = load(f"{app}_panels_dialogs.json")
    if d:
        for field, kind, label in (("control_tree_surfaces", "dialog", "ae_propmap"), ("eve_surfaces", "dialog", "ae_eve")):
            for sfc in d.get(field, []) or []:
                if not isinstance(sfc, dict):
                    continue
                nm = sfc.get("surface_name") or sfc.get("surface_title")
                if nm:
                    add(rows, re.sub(r"(?<!^)(?=[A-Z])", " ", str(nm)).strip(), app, kind, f"teardown:{label}")
                    stats[f"{app}.{field}"] = stats.get(f"{app}.{field}", 0) + 1
                for ct in (sfc.get("controls") or [])[:120]:
                    cn = (ct.get("label") or ct.get("name")) if isinstance(ct, dict) else None
                    if cn and isinstance(cn, str) and 2 < len(cn) <= 48:
                        add(rows, cn, app, "option", f"teardown:{label}_control")
                        stats[f"{app}.surface_controls"] = stats.get(f"{app}.surface_controls", 0) + 1
        for pp in d.get("panel_plugins", []) or []:
            nm = (pp.get("panel_role") or pp.get("panel_plugin")) if isinstance(pp, dict) else None
            if nm:
                add(rows, str(nm), app, "panel", "teardown:ae_panel")
                stats[f"{app}.panels"] = stats.get(f"{app}.panels", 0) + 1

    d = load(f"{app}_render_output.json")
    if d:
        for ex in d.get("exporters", []) or []:
            nm = ex.get("exporter_name") if isinstance(ex, dict) else None
            if nm:
                add(rows, str(nm), app, "capability", "teardown:ae_exporter")
                stats[f"{app}.exporters"] = stats.get(f"{app}.exporters", 0) + 1
        for im in d.get("importers", []) or []:
            nm = im.get("description") if isinstance(im, dict) else None
            if nm:
                add(rows, str(nm), app, "capability", "teardown:ae_importer")
                stats[f"{app}.importers"] = stats.get(f"{app}.importers", 0) + 1
        for mod, params in (d.get("exporter_parameter_strings") or {}).items():
            for pn in (params or [])[:120]:
                if isinstance(pn, str) and 2 < len(pn) <= 48:
                    add(rows, pn, app, "option", f"teardown:ae_codec_param:{mod}")
                    stats[f"{app}.codec_parameters"] = stats.get(f"{app}.codec_parameters", 0) + 1

    d = load(f"{app}_commands_shortcuts.json")
    if d:
        for c in d.get("commands", []) or []:
            if not isinstance(c, dict):
                continue
            nm = c.get("label") or c.get("command_id")
            if nm:
                add(rows, str(nm), app, "command", f"teardown:ae_command:{c.get('category','')}")
                stats[f"{app}.commands"] = stats.get(f"{app}.commands", 0) + 1
        for t in d.get("tools", []) or []:
            nm = t.get("tool_name") if isinstance(t, dict) else None
            if nm:
                add(rows, str(nm), app, "tool", "teardown:ae_tool")
                stats[f"{app}.tools"] = stats.get(f"{app}.tools", 0) + 1

    d = load(f"{app}_text_shape_mask.json")
    if d:
        for section in ("shape_layer", "text_layer", "masks"):
            blk = d.get(section) or {}
            if not isinstance(blk, dict):
                continue
            for grp_name, grp in (blk.get("groups") or {}).items():
                add(rows, re.sub(r"^ADBE ", "", str(grp_name)), app, "capability", f"teardown:ae_{section}")
                stats[f"{app}.{section}_groups"] = stats.get(f"{app}.{section}_groups", 0) + 1
                props = grp.get("properties") if isinstance(grp, dict) else None
                for pr in (props or [])[:200]:
                    pn = (pr.get("display_name") or pr.get("match_name")) if isinstance(pr, dict) else (pr if isinstance(pr, str) else None)
                    if pn:
                        add(rows, re.sub(r"^ADBE ", "", str(pn)), app, "option", f"teardown:ae_{section}_prop")
                        stats[f"{app}.{section}_properties"] = stats.get(f"{app}.{section}_properties", 0) + 1
            for fam in (blk.get("operator_families") or []):
                nm = (fam.get("name") if isinstance(fam, dict) else fam) if fam else None
                if nm:
                    add(rows, str(nm), app, "capability", "teardown:ae_shape_operator")
                    stats[f"{app}.shape_operators"] = stats.get(f"{app}.shape_operators", 0) + 1

    # dialog controls -> the surface each dialog exposes
    d = load(f"{app}_dialogs.json")
    if d:
        for fentry in (d.get("files") or [])[:2000]:
            if not isinstance(fentry, dict):
                continue
            nm = fentry.get("base_name") or fentry.get("file_name")
            if nm:
                add(rows, str(nm).replace("_", " "), app, "dialog", "teardown:dialog")
                stats[f"{app}.dialogs"] = stats.get(f"{app}.dialogs", 0) + 1


def ingest_panels(off: Path, app: str, rows: dict):
    p = off / "uxp_manifests.json"
    if not p.exists():
        return
    for m in json.loads(p.read_text(encoding="utf-8")).get("manifests", []):
        n = m.get("name")
        if n:
            add(rows, n, app, "panel", "uxp:manifest")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--greenroom", type=Path, required=True)
    ap.add_argument("--packet", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    ie = args.greenroom / "installed_exports"
    rows: dict = {}
    prim: dict = {}
    fams: dict = {}
    dropped: collections.Counter = collections.Counter()
    teardown_stats: dict = {}
    apps = sorted(d.name for d in ie.iterdir() if d.is_dir())
    for app in apps:
        off = ie / app / "offline"
        if not off.exists():
            continue
        ingest_typelib(off / "dom_typelib.json", app, rows, prim)
        ingest_affinity_api(off / "scripting_api_surface.json", app, rows, prim)
        ingest_strings([off / "ui_strings_dotnet_en-US.json", off / "ui_strings_lproj_en-US.json"], app, rows, dropped)
        ingest_indesign_surface(off / "indesign_command_surface.json", app, rows)
        ingest_presets(off, app, fams)
        ingest_panels(off, app, rows)
        ingest_teardown(off, app, rows, prim, teardown_stats)
        print(f"  ingested {app}")

    # WP coverage
    mt_tokens = []
    for f in sorted(args.packet.glob("MT-*.json")):
        m = json.loads(f.read_text(encoding="utf-8"))
        blob = m.get("clause", "") + " " + m.get("scope", {}).get("summary", "")
        mt_tokens.append((m["mt_id"], toks(blob)))

    reg = []
    for key, r in rows.items():
        t = toks(r["name"])
        if not t:
            continue
        best, best_mt = 0.0, None
        for mid, mt in mt_tokens:
            if not mt:
                continue
            inter = len(t & mt)
            if not inter:
                continue
            sc = inter / len(t)
            if sc > best:
                best, best_mt = sc, mid
        key_name, key_kind, key_domain = key
        reg.append({
            "id": f"cap.{key_kind}.{key_domain}." + re.sub(r"\s+", "_", key_name)[:48],
            "name": r["name"], "kind": key_kind, "domain": key_domain,
            "source_apps": sorted(r["apps"]), "app_count": len(r["apps"]),
            "vendor_variants": sorted(r["variants"])[:6],
            "evidence": r["evidence"][:5],
            "wp_coverage": {"best_mt": best_mt, "score": round(best, 2), "state": "COVERED" if best >= 0.6 else ("PARTIAL" if best >= 0.34 else "UNCOVERED")},
        })
    reg.sort(key=lambda x: (x["domain"], x["kind"], x["name"].lower()))

    by_dom = collections.Counter(r["domain"] for r in reg)
    by_kind = collections.Counter(r["kind"] for r in reg)
    by_state = collections.Counter(r["wp_coverage"]["state"] for r in reg)
    shared = collections.Counter(r["app_count"] for r in reg)
    doc = {
        "schema_id": "handshake.reference.studio_capability_registry@1",
        "generated_at": now(),
        "purpose": "Single deduplicated inventory of every capability Studio must own for a professional to switch from the source suites without losing functionality. One row per Studio capability; vendor variants recorded as provenance only (STU-SECTION-003).",
        "method": "Mechanical union over installed-app extractions: scripting object models, UI string tables, resource labels, built-in panel manifests, preset families. Deduped by normalised name across apps. WP coverage by token overlap against the 509 existing microtask clauses. No LLM judgement; PARTIAL/UNCOVERED are candidates for triage, not verdicts.",
        "apps_ingested": apps,
        "string_rows_dropped": dict(dropped),
        "teardown_ingested": dict(sorted(teardown_stats.items())),
        "totals": {"capabilities": len(reg), "by_domain": dict(by_dom), "by_kind": dict(by_kind), "wp_coverage": dict(by_state), "capabilities_shared_by_n_apps": dict(sorted(shared.items()))},
        "document_primitives": sorted(({"name": v["name"], "apps": sorted(v["apps"]), "max_members": v["members"]} for v in prim.values()), key=lambda x: -x["max_members"])[:400],
        "preset_families": sorted(fams.values(), key=lambda f: -sum(f["stock_entries"].values() or [0])),
        "capabilities": reg,
    }
    (args.out / "studio-capability-registry.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    gaps = [r for r in reg if r["wp_coverage"]["state"] == "UNCOVERED"]
    gd = collections.Counter(r["domain"] for r in gaps)
    (args.out / "studio-coverage-gaps.json").write_text(json.dumps({
        "schema_id": "handshake.reference.studio_coverage_gaps@1", "generated_at": now(),
        "note": "Capabilities with no token overlap against any of the 509 existing microtask clauses. Candidate list for microtask authoring; expect false positives where the packet uses different wording.",
        "count": len(gaps), "by_domain": dict(gd), "gaps": gaps}, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"\n[registry] capabilities={len(reg)}  primitives={len(prim)}  preset_families={len(fams)}")
    print(f"[registry] by domain: {dict(by_dom)}")
    print(f"[registry] WP coverage: {dict(by_state)}")
    print(f"[registry] shared across N apps: {dict(sorted(shared.items()))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
