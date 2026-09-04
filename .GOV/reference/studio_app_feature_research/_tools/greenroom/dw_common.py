"""dw_common.py -- shared offline helpers for the Dreamweaver 2021 teardown.

Nothing here launches or talks to Dreamweaver. Every function reads bytes off
disk. Where a value is inferred rather than read literally, the caller is
expected to record it under a *_heuristic key.
"""
import codecs
import datetime
import html as htmlmod
import io
import json
import os
import re
import xml.etree.ElementTree as ET

INSTALL_ROOT = r"C:\Program Files\Adobe\Adobe Dreamweaver 2021"
CONFIG = os.path.join(INSTALL_ROOT, "Configuration")

MMSTRING_NS = "{http://www.macromedia.com/schemes/data/string/}"


# --------------------------------------------------------------------------
# generic io
# --------------------------------------------------------------------------
def read_text(path):
    with open(path, "rb") as fh:
        raw = fh.read()
    for bom, enc in ((codecs.BOM_UTF8, "utf-8-sig"),
                     (codecs.BOM_UTF16_LE, "utf-16-le"),
                     (codecs.BOM_UTF16_BE, "utf-16-be")):
        if raw.startswith(bom):
            return raw.decode(enc, "replace")
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError:
        return raw.decode("cp1252", "replace")


def now_iso():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


def install_version():
    p = os.path.join(CONFIG, "version.xml")
    try:
        m = re.search(r'versionnum="([^"]+)"', read_text(p))
        return m.group(1) if m else None
    except OSError:
        return None


def rel(path):
    """Path relative to the install root, forward-slashed, for portable output."""
    try:
        return os.path.relpath(path, INSTALL_ROOT).replace("\\", "/")
    except ValueError:
        return path.replace("\\", "/")


def write_json(path, obj):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(obj, fh, ensure_ascii=False, indent=1)
    return os.path.getsize(path)


# --------------------------------------------------------------------------
# tolerant XML
# --------------------------------------------------------------------------
_ENT = re.compile(r"&(?!#\d+;|#x[0-9A-Fa-f]+;|amp;|lt;|gt;|quot;|apos;)")


class LNode(object):
    """Minimal ElementTree-compatible node produced by the lenient scanner.

    Needed because Dreamweaver's own config XML is not well-formed XML: menu
    attribute values legitimately contain raw '<', '>' and '--' (they hold
    JavaScript source such as applyComment('<!--','-->')). ElementTree rejects
    those files; Dreamweaver's internal parser accepts them, so the rebuild has
    to accept them too.
    """

    __slots__ = ("tag", "attrib", "_children", "text")

    def __init__(self, tag, attrib=None):
        self.tag = tag
        self.attrib = attrib or {}
        self._children = []
        self.text = None

    def append(self, c):
        self._children.append(c)

    def __iter__(self):
        return iter(self._children)

    def __len__(self):
        return len(self._children)

    def iter(self):
        yield self
        for c in self._children:
            for x in c.iter():
                yield x

    def findall_tag(self, name):
        return [n for n in self.iter() if n.tag.split("}")[-1].lower() == name.lower()]


_TAGNAME = re.compile(r"[A-Za-z_:][\w.:\-]*")


def lenient_scan(txt):
    """Tag-soup scanner that keeps quoted attribute values intact.

    Tracks quote state inside a tag so that '<' and '>' appearing inside a
    quoted attribute value do not terminate the tag.
    """
    root = LNode("#document")
    stack = [root]
    i, n = 0, len(txt)
    while i < n:
        lt = txt.find("<", i)
        if lt < 0:
            break
        # text content
        if lt > i:
            t = txt[i:lt].strip()
            if t and stack[-1].text is None:
                stack[-1].text = htmlmod.unescape(t)
        if txt.startswith("<!--", lt):
            end = txt.find("-->", lt + 4)
            i = (end + 3) if end >= 0 else n
            continue
        if txt.startswith("<![CDATA[", lt):
            end = txt.find("]]>", lt + 9)
            body = txt[lt + 9: end if end >= 0 else n]
            if stack[-1].text:
                stack[-1].text += body
            else:
                stack[-1].text = body
            i = (end + 3) if end >= 0 else n
            continue
        if txt.startswith("<!", lt) or txt.startswith("<?", lt):
            end = txt.find(">", lt)
            i = (end + 1) if end >= 0 else n
            continue
        closing = txt.startswith("</", lt)
        j = lt + (2 if closing else 1)
        m = _TAGNAME.match(txt, j)
        if not m:
            i = lt + 1
            continue
        name = m.group(0)
        j = m.end()
        # scan to the real end of the tag, honouring quotes
        q = None
        while j < n:
            ch = txt[j]
            if q:
                if ch == q:
                    q = None
            elif ch in "\"'":
                q = ch
            elif ch == ">":
                break
            j += 1
        body = txt[m.end():j]
        self_closing = body.rstrip().endswith("/")
        if closing:
            for k in range(len(stack) - 1, 0, -1):
                if stack[k].tag == name:
                    del stack[k:]
                    break
        else:
            node = LNode(name, parse_attrs_cased(body))
            stack[-1].append(node)
            if not self_closing:
                stack.append(node)
        i = j + 1
    return root


ATTR_CASED_RE = re.compile(r"""([\w:.\-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s/>]+))""")


def parse_attrs_cased(s):
    out = {}
    for m in ATTR_CASED_RE.finditer(s or ""):
        out[m.group(1)] = htmlmod.unescape(
            m.group(2) if m.group(2) is not None else
            m.group(3) if m.group(3) is not None else m.group(4) or "")
    return out


def parse_xml_tolerant(path):
    """Parse a Macromedia/Adobe config XML. Returns (root_or_None, note).

    Escalation: strict ET -> entity/namespace repair -> lenient tag scanner.
    The lenient scanner returns a single element root when the document has
    exactly one, otherwise the synthetic '#document' node.
    """
    txt = read_text(path)
    try:
        return ET.fromstring(txt), "strict"
    except ET.ParseError:
        pass
    fixed = _ENT.sub("&amp;", txt)
    if "MMString:" in fixed and "xmlns:MMString" not in fixed:
        fixed = re.sub(r"(<[A-Za-z_][\w.\-]*)",
                       r"\1 xmlns:MMString='http://www.macromedia.com/schemes/data/string/'",
                       fixed, count=1)
    if "mmstring:" in fixed and "xmlns:mmstring" not in fixed:
        fixed = re.sub(r"(<[A-Za-z_][\w.\-]*)",
                       r"\1 xmlns:mmstring='http://www.macromedia.com/schemes/data/string/'",
                       fixed, count=1)
    try:
        return ET.fromstring(fixed), "repaired_entities_and_ns"
    except ET.ParseError:
        pass
    doc = lenient_scan(txt)
    kids = list(doc)
    if not kids:
        return None, "parse_failed: lenient scanner found no elements"
    return (kids[0] if len(kids) == 1 else doc), "lenient_tag_scanner"


def attrs_of(el):
    """Attribute dict with the MMString namespace collapsed to 'mmstring:<x>'.

    Handles Clark notation from ElementTree ('{uri}name'), the literal
    'MMString:name' / 'mmstring:name' prefixes seen when the lenient scanner
    is used, and leaves every other attribute untouched.
    """
    out = {}
    for k, v in el.attrib.items():
        if k.startswith(MMSTRING_NS):
            out["mmstring:" + k[len(MMSTRING_NS):]] = v
        elif k.lower().startswith("mmstring:"):
            out["mmstring:" + k.split(":", 1)[1]] = v
        elif k.startswith("xmlns"):
            continue
        else:
            out[k] = v
    return out


# --------------------------------------------------------------------------
# HTML-extension surface parsing (Commands / Objects / Behaviors / Inspectors)
# --------------------------------------------------------------------------
# Real user-operable controls only. MMString:loadString is a localisation
# directive, not a control, so it is deliberately excluded here (it is captured
# separately as localized_strings_used).
TAG_RE = re.compile(r"<\s*(input|select|option|textarea|button|"
                    r"mmcolorbutton|mmurlbutton|mmbrowsebutton|mmfilebutton|"
                    r"mmtreecontrol|mmlistcontrol|mmcheckbox|mmtabcontrol)\b([^>]*)>",
                    re.I | re.S)
ATTR_RE = re.compile(r"""([\w:.\-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))""")
SCRIPT_BLOCK_RE = re.compile(r"<script\b[^>]*>(.*?)</script>", re.I | re.S)
SCRIPT_SRC_RE = re.compile(r"<script\b[^>]*\bsrc\s*=\s*[\"']([^\"']+)[\"']", re.I)
FUNC_RE = re.compile(r"^\s*function\s+([A-Za-z_$][\w$]*)\s*\(([^)]*)\)", re.M)
LOADSTRING_TAG_RE = re.compile(r"<\s*MMString:loadString\b[^>]*\bid\s*=\s*[\"']([^\"']+)[\"']",
                               re.I)
LOADSTRING_JS_RE = re.compile(r"""(?:dw|dreamweaver|MM)\.loadString\(\s*['"]([^'"]+)['"]""")
TITLE_RE = re.compile(r"<title\b[^>]*>(.*?)</title>", re.I | re.S)
MENU_LOCATION_RE = re.compile(r"<!--\s*MENU-LOCATION\s*=\s*([^\s>-]+)", re.I)
COMMENT_DIRECTIVE_RE = re.compile(r"<!--\s*([A-Z][A-Z0-9_\-]{2,})\s*=\s*([^>]*?)\s*-->")


def parse_attrs(s):
    out = {}
    for m in ATTR_RE.finditer(s or ""):
        out[m.group(1).lower()] = m.group(2) or m.group(3) or m.group(4) or ""
    return out


def _control_kind(tag, a):
    tag = tag.lower()
    if tag == "input":
        return "input:" + (a.get("type") or "text").lower()
    if tag == "select":
        return "select-multiple" if "multiple" in a else "select"
    return tag


def extract_controls(text):
    """Form controls declared in a DW extension HTML surface.

    Returns a list of dicts. `options` on a select carry the literal option
    values shipped in the file; DW frequently fills selects at runtime, and
    a placeholder such as '****' is preserved verbatim rather than dropped.
    """
    controls = []
    current_select = None
    for m in TAG_RE.finditer(text):
        tag = m.group(1).lower()
        a = parse_attrs(m.group(2))
        if tag == "option":
            if current_select is not None:
                # option label is the text after the tag up to </option> or next tag
                tail = text[m.end():m.end() + 400]
                lbl = re.split(r"<", tail, 1)[0].strip()
                current_select["options"].append({
                    "value": a.get("value", lbl),
                    "label": htmlmod.unescape(lbl),
                    "selected": "selected" in a,
                })
            continue
        if tag in ("input", "select", "textarea", "button") or tag.startswith("mm"):
            c = {
                "control_kind": _control_kind(tag, a),
                "name": a.get("name") or a.get("id"),
                "id": a.get("id"),
                "default_value": a.get("value"),
                "checked_by_default": "checked" in a,
                "disabled_by_default": "disabled" in a,
                "size": a.get("size"),
                "maxlength": a.get("maxlength"),
                "css_class": a.get("class"),
                "handlers": {k: v for k, v in a.items() if k.startswith("on")},
                "raw_attributes": a,
            }
            if tag == "select":
                c["options"] = []
                current_select = c
            else:
                current_select = None
            controls.append(c)
    return controls


def extract_js(text, base_dir, follow_src=True, _seen=None):
    """Inline script text plus the text of the surface's OWN <script src> files.

    'Own' means an include that resolves inside the same directory as the
    surface. Includes that reach into Configuration/Shared are recorded by name
    but not inlined: they are shared runtime libraries (dwscripts.js and
    friends) that belong to no single surface, and pulling them in would make
    every surface look as if it implemented hundreds of functions.
    """
    _seen = _seen or set()
    chunks = [m.group(1) for m in SCRIPT_BLOCK_RE.finditer(text)]
    includes, inlined = [], []
    for m in SCRIPT_SRC_RE.finditer(text):
        src = m.group(1)
        includes.append(src)
        if not follow_src:
            continue
        p = os.path.normpath(os.path.join(base_dir, src.replace("/", os.sep)))
        if p in _seen or not os.path.isfile(p):
            continue
        if os.path.dirname(p).lower() != os.path.normpath(base_dir).lower():
            continue
        _seen.add(p)
        try:
            chunks.append(read_text(p))
            inlined.append(src)
        except OSError:
            pass
    return "\n".join(chunks), includes


def js_functions(js_text):
    return [{"name": m.group(1),
             "params": [p.strip() for p in m.group(2).split(",") if p.strip()]}
            for m in FUNC_RE.finditer(js_text)]


BUTTONS_RE = re.compile(r"function\s+commandButtons\s*\(\s*\)\s*\{(.*?)\n\}", re.S)
ARRAY_STR_RE = re.compile(r"""(MM\.BTN_\w+|['"][^'"]*['"])""")


def command_buttons(js_text):
    """Parse commandButtons() -> ordered [{label_expr, action_expr}] pairs."""
    m = BUTTONS_RE.search(js_text)
    if not m:
        return None
    body = m.group(1)
    arr = re.search(r"new\s+Array\s*\((.*?)\)\s*;", body, re.S)
    if not arr:
        return None
    toks = ARRAY_STR_RE.findall(arr.group(1))
    toks = [t.strip("'\"") for t in toks]
    return [{"label_expr": toks[i], "action_expr": toks[i + 1]}
            for i in range(0, len(toks) - 1, 2)]


RETURN_STR_RE = re.compile(r"return\s+((?:'[^']*'|\"[^\"]*\")(?:\s*\+\s*(?:'[^']*'|\"[^\"]*\"))*)\s*;")


def literal_returns(js_text, func_name):
    """String literals returned by `function func_name()`, concatenation folded."""
    m = re.search(r"function\s+%s\s*\([^)]*\)\s*\{" % re.escape(func_name), js_text)
    if not m:
        return None
    # crude brace matcher from the opening brace
    i = js_text.index("{", m.start())
    depth = 0
    for j in range(i, len(js_text)):
        if js_text[j] == "{":
            depth += 1
        elif js_text[j] == "}":
            depth -= 1
            if depth == 0:
                body = js_text[i:j]
                break
    else:
        body = js_text[i:]
    out = []
    for r in RETURN_STR_RE.finditer(body):
        parts = re.findall(r"'([^']*)'|\"([^\"]*)\"", r.group(1))
        out.append("".join(a or b for a, b in parts))
    return out or None


def js_block(js_text, func_name):
    """Body of `function func_name(...)`, brace-matched. None if absent."""
    m = re.search(r"function\s+%s\s*\([^)]*\)\s*\{" % re.escape(func_name), js_text)
    if not m:
        return None
    i = js_text.index("{", m.start())
    depth = 0
    for j in range(i, len(js_text)):
        if js_text[j] == "{":
            depth += 1
        elif js_text[j] == "}":
            depth -= 1
            if depth == 0:
                return js_text[i + 1:j]
    return js_text[i + 1:]


def _read_js_string(s, i):
    """Read a JS string literal starting at s[i] (a quote). Returns (value, next_i).

    Handles backslash escapes and the backslash-newline continuation that
    Dreamweaver's shipped object scripts use for multi-line markup templates.
    """
    q = s[i]
    i += 1
    out = []
    while i < len(s):
        c = s[i]
        if c == "\\":
            nxt = s[i + 1] if i + 1 < len(s) else ""
            if nxt == "\n":
                i += 2
                continue
            if nxt == "\r":
                i += 3 if s[i + 2:i + 3] == "\n" else 2
                continue
            out.append({"n": "\n", "t": "\t", "r": "\r", "0": "\0"}.get(nxt, nxt))
            i += 2
            continue
        if c == q:
            return "".join(out), i + 1
        out.append(c)
        i += 1
    return "".join(out), i


def js_expression_template(expr):
    """Turn a JS concatenation expression into a template string.

    Literal pieces are kept verbatim; every non-literal piece becomes
    '{{js:<source>}}'. This is a faithful skeleton of what the shipped script
    writes into the document, with the runtime-computed parts marked.
    """
    parts = []
    i = 0
    buf = []
    while i < len(expr):
        c = expr[i]
        if c in "\"'":
            code = "".join(buf).strip().strip("+").strip()
            if code:
                parts.append({"kind": "js", "text": code})
            buf = []
            val, i = _read_js_string(expr, i)
            parts.append({"kind": "literal", "text": val})
            continue
        buf.append(c)
        i += 1
    code = "".join(buf).strip().strip("+").strip()
    if code:
        parts.append({"kind": "js", "text": code})
    template = "".join(p["text"] if p["kind"] == "literal" else "{{js:%s}}" % p["text"]
                       for p in parts)
    return {
        "template": template,
        "literal_only": "".join(p["text"] for p in parts if p["kind"] == "literal"),
        "has_runtime_parts": any(p["kind"] == "js" for p in parts),
        "runtime_part_count": sum(1 for p in parts if p["kind"] == "js"),
    }


CALL_NAMES = ("doInsert", "insertHTML", "insertText", "objectTag",
              "dw.getDocumentDOM().insertHTML", "popupCommand", "dwscripts.applyBehavior")
# statement-initial assignment, with or without `var`; `==`/`>=`/`!=` excluded
ASSIGN_RE = re.compile(r"(?:^|[;{}\n])\s*(?:var\s+)?([A-Za-z_$][\w$.\[\]']*)\s*(?:\+)?=\s*(?![=])",
                       re.M)
RETURN_RE = re.compile(r"\breturn\b\s*", re.M)


def _balanced_call_args(s, open_idx):
    depth = 0
    q = None
    for j in range(open_idx, len(s)):
        c = s[j]
        if q:
            if c == "\\":
                continue
            if c == q:
                q = None
            continue
        if c in "\"'":
            q = c
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return s[open_idx + 1:j], j + 1
    return s[open_idx + 1:], len(s)


def extract_insert_templates(body):
    """Markup templates a DW object/behavior script writes into the document.

    Reads: `return <expr>;`, `var x = <expr>;` where the expression contains
    markup, and calls to the shipped insert helpers.
    """
    if not body:
        return {"returns": [], "assignments": [], "calls": [], "popup_commands": []}
    out = {"returns": [], "assignments": [], "calls": [], "popup_commands": []}

    def stmt_expr(s, start):
        depth = 0
        q = None
        for j in range(start, len(s)):
            c = s[j]
            if q:
                if c == "\\":
                    continue
                if c == q:
                    q = None
                continue
            if c in "\"'":
                q = c
            elif c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
                if depth < 0:
                    return s[start:j]
            elif c == ";" and depth == 0:
                return s[start:j]
            elif c == "\n" and depth == 0 and s[start:j].count("'") % 2 == 0 \
                    and s[start:j].count('"') % 2 == 0 and s[start:j].rstrip().endswith(";"):
                return s[start:j]
        return s[start:]

    for m in RETURN_RE.finditer(body):
        e = stmt_expr(body, m.end())
        if "'" in e or '"' in e:
            t = js_expression_template(e)
            if t["literal_only"].strip():
                out["returns"].append(t)
    for m in ASSIGN_RE.finditer(body):
        e = stmt_expr(body, m.end())
        if ("<" in e or "&" in e) and ("'" in e or '"' in e):
            t = js_expression_template(e)
            t["variable"] = m.group(1)
            if t["literal_only"].strip():
                out["assignments"].append(t)
    for name in ("doInsert", "insertHTML", "insertText", "insertObject",
                 "dwscripts.setInnerHTML"):
        for m in re.finditer(r"\b%s\s*\(" % re.escape(name), body):
            args, _ = _balanced_call_args(body, m.end() - 1)
            t = js_expression_template(args)
            t["callee"] = name
            out["calls"].append(t)
    for m in re.finditer(r"popupCommand\s*\(\s*['\"]([^'\"]+)['\"]", body):
        out["popup_commands"].append(m.group(1))
    return out


BODY_RE = re.compile(r"<body\b[^>]*>(.*?)</body>", re.I | re.S)


def body_inner_html(text):
    """The <body> content of a DW extension file.

    Many shipped objects implement objectTag() as `return
    document.body.innerHTML;`. For those, the body of the file IS the markup
    that gets inserted into the user's page, verbatim.
    """
    m = BODY_RE.search(text)
    if not m:
        return None
    inner = m.group(1)
    inner = SCRIPT_BLOCK_RE.sub("", inner).strip()
    return inner or None


def surface_title(text, resolver):
    m = TITLE_RE.search(text)
    if not m:
        return None, None
    inner = m.group(1).strip()
    ls = LOADSTRING_TAG_RE.search(inner)
    if ls:
        val, how = resolver(ls.group(1))
        return val, {"from": "MMString:loadString", "key": ls.group(1), "resolution": how}
    txt = htmlmod.unescape(re.sub(r"<[^>]+>", "", inner)).strip()
    return (txt or None), {"from": "literal_title_text"}


def read_surface(path, resolver):
    """Full control/entry-point inventory for one DW extension HTML surface."""
    text = read_text(path)
    base = os.path.dirname(path)
    js, includes = extract_js(text, base)
    title, title_src = surface_title(text, resolver)
    directives = {m.group(1): m.group(2) for m in COMMENT_DIRECTIVE_RE.finditer(text)}
    string_keys = sorted(set(LOADSTRING_TAG_RE.findall(text)) |
                         set(LOADSTRING_JS_RE.findall(js)))
    resolved = {}
    for k in string_keys:
        v, how = resolver(k)
        if v is not None:
            resolved[k] = v
    fns = js_functions(js)
    return {
        "file": rel(path),
        "title": title,
        "title_source": title_src,
        "html_comment_directives": directives,
        "controls": extract_controls(text),
        "js_includes": includes,
        "js_functions": [f["name"] for f in fns],
        "js_function_signatures": fns,
        "command_buttons": command_buttons(js),
        "localized_strings_used": resolved,
        "bytes": os.path.getsize(path),
    }


# --------------------------------------------------------------------------
# walking
# --------------------------------------------------------------------------
def walk(root, exts=None, skip_dirs=()):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in skip_dirs]
        for fn in sorted(filenames):
            if exts and os.path.splitext(fn)[1].lower() not in exts:
                continue
            yield os.path.join(dirpath, fn)


AI_PATTERN = re.compile(
    r"sensei|generative|\bAI\b|machine.?learning|content.?aware|firefly|"
    r"neural|\bGPT\b|copilot|autocomplete.?model", re.I)

_AI_FILENAME_HITS = None


def _ai_filename_hits():
    global _AI_FILENAME_HITS
    if _AI_FILENAME_HITS is None:
        hits = []
        for dirpath, _dn, filenames in os.walk(CONFIG):
            for fn in filenames:
                if AI_PATTERN.search(fn):
                    hits.append(rel(os.path.join(dirpath, fn)))
        _AI_FILENAME_HITS = sorted(hits)
    return _AI_FILENAME_HITS


# Manually adjudicated: these filename matches are not AI features.
AI_FALSE_POSITIVES = {
    "Configuration/Content/HelloWelcome/HTML5/offline/images/ai.png":
        "the Adobe Illustrator (.ai) file-type icon on the welcome screen, "
        "not an artificial-intelligence feature",
}


def excluded_ai(surface_name, candidates=(), extra_note=None):
    """Run the AI-feature sweep for real and report what it found.

    `candidates` is any iterable of identifiers/labels belonging to this
    surface. The result records the regex used, the raw hits, and the
    adjudication of each hit, so the exclusion is auditable rather than
    asserted.
    """
    raw = [c for c in candidates if c and AI_PATTERN.search(str(c))]
    fn_hits = _ai_filename_hits()
    adjudicated = []
    excluded_features = []
    for h in sorted(set(list(raw) + fn_hits)):
        reason = AI_FALSE_POSITIVES.get(h)
        if reason:
            adjudicated.append({"hit": h, "verdict": "not an AI feature",
                                "reason": reason})
        else:
            adjudicated.append({"hit": h, "verdict": "AI feature - EXCLUDED"})
            excluded_features.append(h)
    return {
        "policy": "Adobe AI / generative features are out of scope for the "
                  "Handshake Studio rebuild and are excluded from the "
                  "behavioural specification above.",
        "surface": surface_name,
        "sweep_pattern": AI_PATTERN.pattern,
        "sweep_scope": ["every filename under Configuration/",
                        "every identifier and label belonging to this surface"],
        "raw_hit_count": len(adjudicated),
        "raw_hits_adjudicated": adjudicated,
        "excluded_ai_features": excluded_features,
        "conclusion": ("Dreamweaver 21.8.1 ships no AI or generative feature on "
                       "this surface; every raw hit is a false positive listed "
                       "above."
                       if not excluded_features else
                       "AI features found and excluded; see excluded_ai_features."),
        "note": extra_note,
    }


def envelope(schema_id, method, extra=None):
    env = {
        "schema_id": schema_id,
        "schema_version": "1.0.0",
        "generated_at": now_iso(),
        "app_launched": False,
        "source": {
            "product": "Adobe Dreamweaver 2021",
            "install_root": INSTALL_ROOT,
            "version_num": install_version(),
            "read_mode": "filesystem read-only; no process started, no COM, no type library",
        },
        "method": method,
        "labelling_convention": {
            "parsed": "value read literally out of a shipped file",
            "resolved": "an mmstring/$$$ key read from a shipped file and looked up in the shipped ZString table",
            "heuristic": "value inferred by this tool from naming or structure, not stated in any file",
        },
    }
    if extra:
        env.update(extra)
    return env
