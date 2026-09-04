"""dw_eve.py -- parser for Adobe "Eve" .eve dialog layout files.

Eve is Adobe's declarative dialog-layout language (dvaui). Dreamweaver 2021
ships every native modal dialog as one .eve file under Configuration/Dialogs/Eve.
The grammar is small:

    layout <Name>
    {
        view dialog(key: value, key: value)
        {
            column(key: value)
            {
                static_text(name: "$$$/Some/Key=Default English");
                edit_text(identifier: "IDC_EDIT_X", characters: 6);
            }
        }
    }

Values are: quoted strings, integers, floats, @symbols, bare identifiers,
bracketed lists, and 'true'/'false'. A localizable string is written
"$$$/Path/Key=Default English text" -- the part after the first '=' is the
shipped English, which is why these files are self-describing.

No Adobe code is executed; this is a plain recursive-descent text parse.
"""
import re

_COMMENT = re.compile(r"/\*.*?\*/", re.S)
_LINE_COMMENT = re.compile(r"//[^\n]*")
_IDENT = re.compile(r"[A-Za-z_][\w.\-]*")
_NUM = re.compile(r"-?\d+(?:\.\d+)?")
_WS = re.compile(r"\s+")

DOLLAR = re.compile(r"^\$\$\$/([^=]*)=(.*)$", re.S)


def split_localized(v):
    """'$$$/Key=Text' -> (key, text). Anything else -> (None, value)."""
    if isinstance(v, str):
        m = DOLLAR.match(v)
        if m:
            return m.group(1), m.group(2)
    return None, v


class _P(object):
    def __init__(self, s):
        self.s = s
        self.i = 0
        self.n = len(s)

    def ws(self):
        while self.i < self.n:
            m = _WS.match(self.s, self.i)
            if m:
                self.i = m.end()
                continue
            break

    def peek(self):
        self.ws()
        return self.s[self.i] if self.i < self.n else ""

    def eat(self, ch):
        self.ws()
        if self.i < self.n and self.s[self.i] == ch:
            self.i += 1
            return True
        return False

    def ident(self):
        self.ws()
        m = _IDENT.match(self.s, self.i)
        if not m:
            return None
        self.i = m.end()
        return m.group(0)

    def string(self):
        # assumes current char is a quote
        q = self.s[self.i]
        self.i += 1
        out = []
        while self.i < self.n:
            c = self.s[self.i]
            if c == "\\" and self.i + 1 < self.n:
                nxt = self.s[self.i + 1]
                out.append({"n": "\n", "t": "\t", "r": "\r"}.get(nxt, nxt))
                self.i += 2
                continue
            if c == q:
                self.i += 1
                break
            out.append(c)
            self.i += 1
        return "".join(out)

    def value(self):
        self.ws()
        if self.i >= self.n:
            return None
        c = self.s[self.i]
        if c in "\"'":
            v = self.string()
            # adjacent string concatenation
            while True:
                save = self.i
                self.ws()
                if self.i < self.n and self.s[self.i] in "\"'":
                    v += self.string()
                else:
                    self.i = save
                    break
            return v
        if c == "@":
            self.i += 1
            return "@" + (self.ident() or "")
        if c == "[":
            self.i += 1
            items = []
            while True:
                self.ws()
                if self.i >= self.n or self.s[self.i] == "]":
                    self.i += 1
                    break
                before = self.i
                items.append(self.value())
                self.ws()
                if self.i < self.n and self.s[self.i] == ",":
                    self.i += 1
                if self.i == before:          # no progress -> malformed, step on
                    self.i += 1
            return items
        if c == "{":
            # inline record, e.g. items: [ { name: "...", value: 3 }, ... ]
            self.i += 1
            rec = {}
            while True:
                self.ws()
                if self.i >= self.n or self.s[self.i] == "}":
                    self.i += 1
                    break
                before = self.i
                k = self.ident()
                self.ws()
                if k is not None and self.i < self.n and self.s[self.i] == ":":
                    self.i += 1
                    rec[k] = self.value()
                self.ws()
                if self.i < self.n and self.s[self.i] == ",":
                    self.i += 1
                if self.i == before:
                    self.i += 1
            return rec
        m = _NUM.match(self.s, self.i)
        if m:
            self.i = m.end()
            t = m.group(0)
            return float(t) if "." in t else int(t)
        w = self.ident()
        if w in ("true", "false"):
            return w == "true"
        return w

    def args(self):
        """Parse '(k: v, k: v)' -> dict. Duplicate keys are suffixed #2, #3..."""
        out = {}
        if not self.eat("("):
            return out
        while True:
            self.ws()
            if self.i >= self.n or self.s[self.i] == ")":
                self.i += 1
                break
            before = self.i
            key = self.ident()
            if key is None:
                self.i += 1
                continue
            self.ws()
            if self.i < self.n and self.s[self.i] == ":":
                self.i += 1
                val = self.value()
            else:
                val = True
            k = key
            c = 2
            while k in out:
                k = "%s#%d" % (key, c)
                c += 1
            out[k] = val
            self.ws()
            if self.i < self.n and self.s[self.i] == ",":
                self.i += 1
            if self.i == before:              # no progress -> malformed, step on
                self.i += 1
        return out

    def block(self):
        """Parse '{ node* }' -> list of nodes."""
        nodes = []
        if not self.eat("{"):
            return nodes
        while True:
            self.ws()
            if self.i >= self.n:
                break
            if self.s[self.i] == "}":
                self.i += 1
                break
            nd = self.node()
            if nd is None:
                self.i += 1
                continue
            nodes.append(nd)
        return nodes

    def node(self):
        self.ws()
        words = []
        while True:
            save = self.i
            w = self.ident()
            if w is None:
                self.i = save
                break
            words.append(w)
            self.ws()
            if self.i < self.n and self.s[self.i] == "(":
                break
            if self.i < self.n and self.s[self.i] in "{;}":
                break
            if len(words) > 4:
                break
        if not words:
            return None
        kind = words[-1]
        a = self.args() if self.peek() == "(" else {}
        children = self.block() if self.peek() == "{" else []
        self.eat(";")
        return {"kind": kind, "modifiers": words[:-1], "args": a, "children": children}


def parse_eve(text):
    """Return list of {layout_name, root_nodes} declared in one .eve file."""
    txt = _COMMENT.sub(" ", text)
    txt = _LINE_COMMENT.sub(" ", txt)
    p = _P(txt)
    layouts = []
    while True:
        p.ws()
        if p.i >= p.n:
            break
        save = p.i
        w = p.ident()
        if w is None:
            p.i += 1
            continue
        if w == "layout":
            name = p.ident()
            nodes = p.block()
            layouts.append({"layout_name": name, "nodes": nodes})
        else:
            p.i = save
            nd = p.node()
            if nd is None:
                p.i += 1
    return layouts


# --------------------------------------------------------------------------
# semantic flattening: which nodes are actual user-operable controls
# --------------------------------------------------------------------------
CONTAINERS = {"row", "column", "group", "panel", "tab_group", "cluster",
              "multi_subview_container", "scroll_view"}

CONTROL_SEMANTICS = {
    # kind: (control_role, value_kind)
    "edit_text": ("text field", "string"),
    "dva_edit_number": ("numeric field", "number"),
    "checkbox": ("checkbox", "boolean"),
    "radio_button": ("radio button", "enum member"),
    "radiogroup": ("radio group", "enum"),
    "popup": ("dropdown", "enum"),
    "combobox": ("editable dropdown", "enum or free text"),
    "list_box": ("list box", "enum"),
    "list_control": ("list control", "collection"),
    "multicolumn_tree": ("multi-column tree", "collection"),
    "sft_tree": ("tree", "collection"),
    "slider": ("slider", "number"),
    "spin_button": ("spinner", "number"),
    "button": ("push button", "action"),
    "ownerdrawn_button": ("icon button", "action"),
    "link_button": ("hyperlink button", "action"),
    "progress_bar": ("progress bar", "readonly number"),
    "image": ("image", "readonly"),
    "static_text": ("label", "readonly"),
    "display_text": ("read-only text", "readonly"),
    "separator": ("separator", "none"),
    "placeholder": ("spacer", "none"),
    "subview": ("embedded subview", "container"),
}

VALUE_ARG_KEYS = ("value", "default", "min_value", "max_value", "min", "max",
                  "increment", "characters", "digits", "precision", "items",
                  "list", "readonly", "multiselect", "password", "alignment",
                  "num_visible_items", "small_increment", "large_increment")


def flatten_controls(nodes, path=()):
    out = []
    for nd in nodes:
        kind = nd["kind"]
        a = nd["args"]
        key, label = split_localized(a.get("name"))
        role, vkind = CONTROL_SEMANTICS.get(kind, (None, None))
        rec = {
            "kind": kind,
            "control_role": role,
            "value_kind": vkind,
            "is_container": kind in CONTAINERS,
            "identifier": a.get("identifier"),
            "label": label if isinstance(label, str) else None,
            "label_string_key": key,
            "container_path": list(path),
        }
        for k in VALUE_ARG_KEYS:
            if k in a:
                kk, vv = split_localized(a[k])
                rec[k] = vv
                if kk:
                    rec[k + "_string_key"] = kk
        geom = {k: a[k] for k in ("width", "height", "margin", "spacing",
                                  "horizontal", "vertical", "characters")
                if k in a}
        if geom:
            rec["geometry"] = geom
        rec["all_args"] = a
        out.append(rec)
        if nd["children"]:
            out.extend(flatten_controls(nd["children"],
                                        path + (kind + (":" + str(a.get("identifier"))
                                                        if a.get("identifier") else ""),)))
    return out
