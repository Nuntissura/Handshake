#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
photoshop-dialog-vocabulary.py

OFFLINE teardown of the installed Adobe Photoshop 2026 declarative UI layout tree.

READ-ONLY. Never launches Photoshop or any application. Parses plain-text Adobe "Eve"
declarative layout files shipped with the installed product and emits a single JSON
document describing the complete dialog / panel / properties / tool-options control
vocabulary.

Primary source (scope required by the task):
    <PS>/Required/layouts/                 .exv + .eve   (classic Eve dialect)

Additional sources (same grammar family, same product, parsed with the same parser and
kept in clearly separated source groups):
    <PS>/Required/drover_layouts/          .eve          (modern "Drover" eve2 dialect)
    <PS>/Required/drover_layouts/drover.eve_schema       (machine-readable eve2 grammar)
    <PS>/Required/OWL/                     .eve
    <PS>/Locales/en_US/Support Files/tw10428_Photoshop_en_US.dat   (UTF-16LE zstring table)
    <PS>/Required/UIColors.txt, <PS>/Required/PSConfig.txt         (checked; see report)

Usage:
    python photoshop-dialog-vocabulary.py [--ps-root <dir>] [--out <file.json>]
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import sys
import traceback
from collections import Counter, OrderedDict, defaultdict

SCHEMA_ID = "handshake.studio_research.photoshop_dialogs.v1"

DEFAULT_PS_ROOT = r"C:\Program Files\Adobe\Adobe Photoshop 2026"
DEFAULT_OUT = (
    r"D:\Projects\LLM projects\Handshake\Handshake Worktrees\wt-gov-kernel\.GOV"
    r"\reference\studio_app_feature_research\_greenroom_20260903\installed_exports"
    r"\photoshop\offline\photoshop_dialogs.json"
)

# ---------------------------------------------------------------------------
# 1. Preprocessor
# ---------------------------------------------------------------------------

_PP_IFDEF = re.compile(r"^[ \t]*#\s*ifdef[ \t]+([A-Za-z_][A-Za-z0-9_]*)")
_PP_IFNDEF = re.compile(r"^[ \t]*#\s*ifndef[ \t]+([A-Za-z_][A-Za-z0-9_]*)")
_PP_ELSE = re.compile(r"^[ \t]*#\s*else\b")
_PP_ENDIF = re.compile(r"^[ \t]*#\s*endif\b")
_PP_OTHER = re.compile(r"^[ \t]*#")

# Documented branch decision: Photoshop layout files carry platform-conditional metric
# and widget blocks guarded by `#ifdef MacEve` / `#ifdef WinEve`. This teardown targets a
# Windows install, so the WinEve branch is KEPT and the MacEve branch is DISCARDED.
# Discarded lines are replaced by empty lines so reported line numbers stay truthful.
PREFERRED_BRANCH = "WinEve"
DISCARDED_BRANCH = "MacEve"


def preprocess(text, stats):
    """Resolve #ifdef MacEve / #ifdef WinEve / #endif. Returns (text, info)."""
    out_lines = []
    # stack entries: dict(symbol, taking, seen_else)
    stack = []
    kept = 0
    dropped = 0
    unknown_symbols = Counter()
    for raw in text.split("\n"):
        m = _PP_IFDEF.match(raw)
        if m:
            sym = m.group(1)
            if sym == PREFERRED_BRANCH:
                taking = True
            elif sym == DISCARDED_BRANCH:
                taking = False
            else:
                # Unknown platform symbol: keep the block (conservative) and record it.
                taking = True
                unknown_symbols[sym] += 1
            stack.append({"symbol": sym, "taking": taking, "seen_else": False})
            out_lines.append("")
            continue
        m = _PP_IFNDEF.match(raw)
        if m:
            sym = m.group(1)
            taking = sym != PREFERRED_BRANCH
            if sym not in (PREFERRED_BRANCH, DISCARDED_BRANCH):
                unknown_symbols[sym] += 1
            stack.append({"symbol": "!" + sym, "taking": taking, "seen_else": False})
            out_lines.append("")
            continue
        if _PP_ELSE.match(raw):
            if stack:
                stack[-1]["taking"] = not stack[-1]["taking"]
                stack[-1]["seen_else"] = True
            out_lines.append("")
            continue
        if _PP_ENDIF.match(raw):
            if stack:
                stack.pop()
            out_lines.append("")
            continue
        if _PP_OTHER.match(raw):
            # Any other '#' directive: blank it out and record.
            stats["other_directives"] += 1
            out_lines.append("")
            continue
        if all(f["taking"] for f in stack):
            out_lines.append(raw)
            if stack:
                kept += 1
        else:
            out_lines.append("")
            dropped += 1
    info = {
        "conditional_lines_kept": kept,
        "conditional_lines_dropped": dropped,
        "unbalanced_ifdef": len(stack) > 0,
        "unknown_conditional_symbols": dict(unknown_symbols),
    }
    return "\n".join(out_lines), info


# ---------------------------------------------------------------------------
# 2. Lexer
# ---------------------------------------------------------------------------

T_IDENT = "ident"
T_AT = "at"          # @keyword
T_STRING = "string"
T_NUMBER = "number"
T_PUNCT = "punct"
T_EOF = "eof"

_MULTI_PUNCT = ("<==", "==", "!=", ">=", "<=", "&&", "||", "->", "::")
_SINGLE_PUNCT = set("(){}[],;:=<>+-*/%!&|?.~^")

_IDENT_START = re.compile(r"[A-Za-z_]")
_IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_NUM_RE = re.compile(r"(?:0[xX][0-9A-Fa-f]+|\d+\.\d*(?:[eE][+-]?\d+)?|\.\d+(?:[eE][+-]?\d+)?|\d+(?:[eE][+-]?\d+)?)")


class Token(object):
    __slots__ = ("kind", "value", "raw", "line", "col")

    def __init__(self, kind, value, raw, line, col):
        self.kind = kind
        self.value = value
        self.raw = raw
        self.line = line
        self.col = col

    def __repr__(self):
        return "Token(%s,%r,L%d)" % (self.kind, self.value, self.line)


class LexError(Exception):
    pass


def tokenize(text):
    tokens = []
    i = 0
    n = len(text)
    line = 1
    line_start = 0
    while i < n:
        c = text[i]
        if c == "\n":
            line += 1
            i += 1
            line_start = i
            continue
        if c in " \t\r\f\v":
            i += 1
            continue
        # comments
        if c == "/" and i + 1 < n:
            if text[i + 1] == "/":
                j = text.find("\n", i)
                i = n if j < 0 else j
                continue
            if text[i + 1] == "*":
                j = text.find("*/", i + 2)
                if j < 0:
                    raise LexError("unterminated block comment at line %d" % line)
                line += text.count("\n", i, j)
                i = j + 2
                continue
        # strings
        if c == "'" or c == '"':
            quote = c
            j = i + 1
            buf = []
            while j < n:
                cj = text[j]
                if cj == "\\" and j + 1 < n:
                    buf.append(text[j + 1])
                    j += 2
                    continue
                if cj == quote:
                    break
                if cj == "\n":
                    line += 1
                buf.append(cj)
                j += 1
            if j >= n:
                raise LexError("unterminated string starting line %d" % line)
            tokens.append(Token(T_STRING, "".join(buf), text[i:j + 1], line, i - line_start))
            i = j + 1
            continue
        # @keyword
        if c == "@":
            m = _IDENT_RE.match(text, i + 1)
            if m:
                tokens.append(Token(T_AT, m.group(0), text[i:m.end()], line, i - line_start))
                i = m.end()
                continue
            tokens.append(Token(T_PUNCT, "@", "@", line, i - line_start))
            i += 1
            continue
        # numbers
        if c.isdigit() or (c == "." and i + 1 < n and text[i + 1].isdigit()):
            m = _NUM_RE.match(text, i)
            if m:
                tokens.append(Token(T_NUMBER, m.group(0), m.group(0), line, i - line_start))
                i = m.end()
                continue
        # identifiers
        if _IDENT_START.match(c):
            m = _IDENT_RE.match(text, i)
            tokens.append(Token(T_IDENT, m.group(0), m.group(0), line, i - line_start))
            i = m.end()
            continue
        # punctuation
        matched = None
        for p in _MULTI_PUNCT:
            if text.startswith(p, i):
                matched = p
                break
        if matched:
            tokens.append(Token(T_PUNCT, matched, matched, line, i - line_start))
            i += len(matched)
            continue
        if c in _SINGLE_PUNCT:
            tokens.append(Token(T_PUNCT, c, c, line, i - line_start))
            i += 1
            continue
        raise LexError("unexpected character %r at line %d" % (c, line))
    tokens.append(Token(T_EOF, None, "", line, 0))
    return tokens


# ---------------------------------------------------------------------------
# 3. Parser (recursive descent)
# ---------------------------------------------------------------------------

SECTION_KEYWORDS = {"constant", "interface", "logic", "external", "invariant", "output"}

# Eve cell modifiers that may prefix a declaration inside an `interface:` section.
DECL_MODIFIERS = {"unlink"}


class ParseError(Exception):
    pass


class Node(object):
    """A widget node."""
    __slots__ = ("widget_type", "attrs", "children", "line", "leaf")

    def __init__(self, widget_type, attrs, children, line, leaf):
        self.widget_type = widget_type
        self.attrs = attrs          # list of (key, RawExpr)
        self.children = children    # list of Node
        self.line = line
        self.leaf = leaf


class RawExpr(object):
    __slots__ = ("tokens", "text")

    def __init__(self, tokens, text):
        self.tokens = tokens
        self.text = text

    def single(self):
        """Return the single token if the expression is exactly one token, else None."""
        return self.tokens[0] if len(self.tokens) == 1 else None


def _expr_text(tokens):
    parts = []
    prev = None
    for t in tokens:
        if prev is not None:
            need_space = True
            if t.kind == T_PUNCT and t.value in (",", ";", ")", "]", ":"):
                need_space = False
            if prev.kind == T_PUNCT and prev.value in ("(", "["):
                need_space = False
            if t.kind == T_PUNCT and t.value in ("(", "["):
                need_space = False
            if need_space:
                parts.append(" ")
        parts.append(t.raw)
        prev = t
    return "".join(parts)


class Parser(object):
    def __init__(self, tokens, path):
        self.toks = tokens
        self.i = 0
        self.path = path
        self.warnings = []
        # Every assignment / declaration seen at ANY nesting depth. Photoshop layout files
        # legitimately place 'name = value;' assignments inside widget bodies (e.g.
        # Filters/Dialogs/unsharpMask-1510.exv defines vSliderHeight inside dialog{}), so the
        # symbol table must be collected from the whole token stream, not just the top level.
        self.bindings = []

    # -- token helpers -----------------------------------------------------
    def peek(self, k=0):
        j = self.i + k
        return self.toks[j] if j < len(self.toks) else self.toks[-1]

    def at_punct(self, v, k=0):
        t = self.peek(k)
        return t.kind == T_PUNCT and t.value == v

    def at_ident(self, v=None, k=0):
        t = self.peek(k)
        if t.kind != T_IDENT:
            return False
        return v is None or t.value == v

    def next(self):
        t = self.peek()
        self.i += 1
        return t

    def expect_punct(self, v):
        t = self.peek()
        if not (t.kind == T_PUNCT and t.value == v):
            raise ParseError("expected %r, got %r (%s) at line %d in %s"
                             % (v, t.raw, t.kind, t.line, self.path))
        self.i += 1
        return t

    # -- expressions -------------------------------------------------------
    def parse_expr(self, stops):
        """Collect tokens until one of `stops` (a set of punct values) at nesting depth 0."""
        start = self.i
        depth = 0
        while True:
            t = self.peek()
            if t.kind == T_EOF:
                break
            if t.kind == T_PUNCT:
                if t.value in "([{":
                    depth += 1
                elif t.value in ")]}":
                    if depth == 0 and t.value in stops:
                        break
                    if depth == 0:
                        break
                    depth -= 1
                elif depth == 0 and t.value in stops:
                    break
            self.i += 1
        toks = self.toks[start:self.i]
        return RawExpr(toks, _expr_text(toks))

    # -- widgets -----------------------------------------------------------
    def parse_attr_list(self):
        """Parse the contents of ( ... ). Assumes '(' already consumed."""
        attrs = []
        while True:
            if self.at_punct(")"):
                self.expect_punct(")")
                return attrs
            if self.peek().kind == T_EOF:
                raise ParseError("EOF inside attribute list in %s" % self.path)
            if self.at_punct(","):
                self.next()
                continue
            key_tok = self.peek()
            if key_tok.kind in (T_IDENT, T_AT, T_STRING) and self.at_punct(":", 1):
                self.next()  # key
                self.next()  # ':'
                val = self.parse_expr({",", ")"})
                attrs.append((key_tok.value, val))
                continue
            # positional / unkeyed value (rare) -- record with synthetic key
            val = self.parse_expr({",", ")"})
            if not val.tokens:
                # nothing consumed -> avoid infinite loop
                self.next()
                continue
            attrs.append(("__positional__", val))

    def parse_widget(self):
        name_tok = self.next()
        widget_type = name_tok.value
        attrs = []
        if self.at_punct("("):
            self.expect_punct("(")
            attrs = self.parse_attr_list()
        children = []
        leaf = True
        if self.at_punct("{"):
            leaf = False
            self.expect_punct("{")
            while not self.at_punct("}"):
                if self.peek().kind == T_EOF:
                    raise ParseError("EOF inside widget body of %r in %s" % (widget_type, self.path))
                child = self.parse_statement(in_block=True)
                if isinstance(child, Node):
                    children.append(child)
            self.expect_punct("}")
            if self.at_punct(";"):
                self.next()
        elif self.at_punct(";"):
            self.next()
        return Node(widget_type, attrs, children, name_tok.line, leaf)

    # -- statements --------------------------------------------------------
    def parse_statement(self, in_block=False):
        t = self.peek()
        if t.kind == T_EOF:
            return None
        if t.kind == T_PUNCT and t.value == ";":
            self.next()
            return None
        if t.kind == T_IDENT:
            # layout NAME { ... }
            if t.value == "layout" and self.peek(1).kind == T_IDENT and self.at_punct("{", 2):
                return self.parse_layout()
            # view WIDGET( ... )   (eve dialect root view declaration)
            if t.value == "view" and self.peek(1).kind == T_IDENT and self.at_punct("(", 2):
                self.next()
                node = self.parse_widget()
                return ("view_decl", node)
            # 'unlink NAME : expr ...' -- Eve cell modifier prefix on a declaration.
            if t.value in DECL_MODIFIERS and self.peek(1).kind == T_IDENT and (
                    self.at_punct(":", 2) or self.at_punct("<==", 2) or self.at_punct(";", 2)):
                self.next()  # modifier
                return self.parse_statement(in_block=in_block)
            # assignment:  name = expr ;
            if self.at_punct("=", 1):
                self.next()
                self.next()
                expr = self.parse_expr({";"})
                if self.at_punct(";"):
                    self.next()
                st = ("assign", t.value, expr, t.line)
                self.bindings.append(st)
                return st
            # declaration: name : expr ;   /  name <== expr ;
            if self.at_punct(":", 1) or self.at_punct("<==", 1):
                op = self.peek(1).value
                self.next()
                self.next()
                expr = self.parse_expr({";"})
                if self.at_punct(";"):
                    self.next()
                st = ("decl", t.value, op, expr, t.line)
                self.bindings.append(st)
                return st
            # bare declaration with no initialiser: 'name;' (Eve interface: sections)
            if self.at_punct(";", 1):
                self.next()
                self.next()
                st = ("decl", t.value, ":", RawExpr([], ""), t.line)
                self.bindings.append(st)
                return st
            # widget
            if self.at_punct("(", 1) or self.at_punct("{", 1):
                return self.parse_widget()
        # unrecognised -> skip a token, record
        self.warnings.append("skipped token %r (%s) at line %d" % (t.raw, t.kind, t.line))
        self.next()
        return None

    def parse_layout(self):
        self.next()                     # 'layout'
        name = self.next().value        # layout name
        self.expect_punct("{")
        sections = OrderedDict()
        views = []
        cur_section = "_root"
        while not self.at_punct("}"):
            if self.peek().kind == T_EOF:
                raise ParseError("EOF inside layout %r in %s" % (name, self.path))
            t = self.peek()
            if (t.kind == T_IDENT and t.value in SECTION_KEYWORDS
                    and self.at_punct(":", 1)
                    and not self.at_punct("(", 2)):
                cur_section = t.value
                self.next()
                self.next()
                sections.setdefault(cur_section, [])
                continue
            st = self.parse_statement(in_block=True)
            if st is None:
                continue
            if isinstance(st, Node):
                views.append(st)
            elif isinstance(st, tuple) and st[0] == "view_decl":
                views.append(st[1])
            elif isinstance(st, tuple) and st[0] in ("decl", "assign"):
                sections.setdefault(cur_section, []).append(st)
        self.expect_punct("}")
        return ("layout", name, sections, views)

    def parse_program(self):
        out = []
        while self.peek().kind != T_EOF:
            before = self.i
            st = self.parse_statement()
            if st is not None:
                out.append(st)
            if self.i == before:
                self.next()
        return out


# ---------------------------------------------------------------------------
# 4. zstring handling
# ---------------------------------------------------------------------------

_ZSTRING_PREFIX = "$$$/"


def parse_zstring(s):
    """'$$$/Path/Key=English text' -> (path, text). Returns None if not a zstring."""
    if not isinstance(s, str) or not s.startswith(_ZSTRING_PREFIX):
        return None
    eq = s.find("=")
    if eq < 0:
        return (s, None)
    return (s[:eq], s[eq + 1:])


# ---------------------------------------------------------------------------
# 5. Symbol resolution
# ---------------------------------------------------------------------------

RESOLVED_LITERAL = "literal"
RESOLVED_LOCAL_VAR = "resolved_local_variable"
UNRESOLVED_GLOBAL = "unresolved_global_symbol"
UNRESOLVED_EXPR = "unresolved_expression"
KEYWORD_REF = "keyword_reference"


class FileSymbols(object):
    def __init__(self):
        self.table = {}     # name -> RawExpr

    def add(self, name, expr):
        if name not in self.table:
            self.table[name] = expr

    def resolve_string(self, name, _depth=0, _seen=None):
        """Resolve an identifier to a string literal, following intra-file aliases."""
        if _seen is None:
            _seen = set()
        if _depth > 16 or name in _seen:
            return None
        _seen.add(name)
        expr = self.table.get(name)
        if expr is None:
            return None
        tok = expr.single()
        if tok is None:
            return None
        if tok.kind == T_STRING:
            return tok.value
        if tok.kind == T_IDENT:
            return self.resolve_string(tok.value, _depth + 1, _seen)
        return None


def resolve_value(expr, syms, unresolved_globals):
    """
    Resolve a RawExpr to a display value.
    Returns dict(value, zstring_path, resolution, raw).
    """
    raw = expr.text
    tok = expr.single()
    if tok is None:
        return {"value": None, "zstring_path": None, "resolution": UNRESOLVED_EXPR, "raw": raw}
    if tok.kind == T_STRING:
        z = parse_zstring(tok.value)
        if z:
            return {"value": z[1], "zstring_path": z[0], "resolution": RESOLVED_LITERAL, "raw": raw}
        return {"value": tok.value, "zstring_path": None, "resolution": RESOLVED_LITERAL, "raw": raw}
    if tok.kind == T_AT:
        return {"value": None, "zstring_path": None, "resolution": KEYWORD_REF, "raw": raw}
    if tok.kind == T_NUMBER:
        return {"value": tok.value, "zstring_path": None, "resolution": RESOLVED_LITERAL, "raw": raw}
    if tok.kind == T_IDENT:
        s = syms.resolve_string(tok.value)
        if s is not None:
            z = parse_zstring(s)
            if z:
                return {"value": z[1], "zstring_path": z[0],
                        "resolution": RESOLVED_LOCAL_VAR, "raw": raw}
            return {"value": s, "zstring_path": None,
                    "resolution": RESOLVED_LOCAL_VAR, "raw": raw}
        if tok.value in syms.table:
            return {"value": None, "zstring_path": None,
                    "resolution": UNRESOLVED_EXPR, "raw": raw}
        unresolved_globals[tok.value] += 1
        return {"value": None, "zstring_path": None,
                "resolution": UNRESOLVED_GLOBAL, "raw": raw}
    return {"value": None, "zstring_path": None, "resolution": UNRESOLVED_EXPR, "raw": raw}


# ---------------------------------------------------------------------------
# 6. Flattening / classification
# ---------------------------------------------------------------------------

# Attributes that are pure box-model geometry / cosmetic layout hints. They carry no
# semantic meaning for a Rust reimplementation of the control's behaviour, so they are
# dropped from the per-control `attributes` map to keep the JSON tractable. This is a
# documented, deliberate omission -- it is reported in `method.attribute_filtering`.
GEOMETRY_ATTRS = {
    "placement", "spacing", "margin", "margin_left", "margin_right", "margin_top",
    "margin_bottom", "margin_width", "margin_height",
    "horizontal", "vertical", "child_horizontal", "child_vertical",
    "resize_size_horizontal", "resize_size_vertical",
    "resize_location_horizontal", "resize_location_vertical",
    "width", "height", "min_width", "max_width", "min_height", "max_height",
    "maxWidth", "minWidth", "maxHeight", "minHeight",
    "indent", "guide_mask", "guide_balance", "qDebugDraw", "grow", "outset",
}

LABEL_ATTRS = ("name", "title", "label", "text", "layoverText")
# `alt` is documented in drover.eve_schema as "Alternative text (tooltip) for the widget".
# `richtooltip` is deliberately NOT here: inspection of the shipped files shows it is a
# boolean flag (richtooltip: true / false), not tooltip text. It stays in `attributes`.
TOOLTIP_ATTRS = ("tooltip", "alt", "tip", "helpTip", "help_tip")
ID_ATTRS_OSTYPE = ("view_id",)
ID_ATTRS_STRING = ("identifier",)
# `sub_layout(name: "x")` names an INCLUDED LAYOUT, not a display label.
LAYOUT_REF_WIDGETS = {"sub_layout"}
# Attributes that reference ANOTHER control's id. Captured verbatim into related_ids so a
# reimplementation can rebuild the cross-widget wiring (hot text -> edit field, edit field
# -> slider, popup owner -> popup resource, etc.).
RELATED_ID_ATTRS = (
    "hot_text_edit_id", "hotTextEditIdentifier",
    "hot_icon_edit_id", "hotIconEditIdentifier",
    "edit_view_id", "editViewIdentifier", "edit_id",
    "slider_view_id", "sliderViewIdentifier",
    "popup_view_id", "popup_resource_id",
    "cluster_id", "target_id", "targetIdentifier",
    "flyoutControlId", "flyout_id",
    "defaultIdentifier", "cancelIdentifier",
    "resourceIdentifier", "resource_id",
    "bind", "include_view",
)

ITEM_WIDGETS = {"popup_item", "menu_item", "item", "slot_item"}


def attr_lookup(node, keys):
    for k, v in node.attrs:
        if k in keys:
            return k, v
    return None, None


def flatten(node, syms, unresolved_globals, path_stack, out, counters, depth=0):
    is_layout_ref = node.widget_type in LAYOUT_REF_WIDGETS
    label_key, label_expr = (None, None) if is_layout_ref else attr_lookup(node, LABEL_ATTRS)
    tip_key, tip_expr = attr_lookup(node, TOOLTIP_ATTRS)

    label = resolve_value(label_expr, syms, unresolved_globals) if label_expr else None
    tooltip = resolve_value(tip_expr, syms, unresolved_globals) if tip_expr else None

    view_id = None
    id_kind = None
    for k, v in node.attrs:
        if k in ID_ATTRS_OSTYPE:
            tok = v.single()
            if tok is not None and tok.kind == T_STRING:
                view_id = tok.value
                id_kind = "view_id_ostype" if len(tok.value) == 4 else "view_id_string"
            else:
                view_id = v.text
                id_kind = "view_id_expression"
            break
    if view_id is None:
        for k, v in node.attrs:
            if k in ID_ATTRS_STRING:
                tok = v.single()
                if tok is not None and tok.kind == T_STRING:
                    view_id = tok.value
                    id_kind = ("identifier_ostype" if len(tok.value) == 4
                               else "identifier_string")
                elif tok is not None and tok.kind == T_AT:
                    view_id = tok.value
                    id_kind = "identifier_keyword"
                else:
                    view_id = v.text
                    id_kind = "identifier_expression"
                break

    attrs_out = OrderedDict()
    for k, v in node.attrs:
        if k in GEOMETRY_ATTRS:
            counters["geometry_attrs_dropped"] += 1
            continue
        tok = v.single()
        if tok is not None and tok.kind == T_STRING:
            z = parse_zstring(tok.value)
            attrs_out[k] = z[1] if z else tok.value
        else:
            attrs_out[k] = v.text

    def _str_attr(name):
        for k, v in node.attrs:
            if k == name:
                tok = v.single()
                if tok is not None and tok.kind == T_STRING:
                    return tok.value
                return v.text
        return None

    ctrl = OrderedDict()
    ctrl["widget_type"] = node.widget_type
    ctrl["view_id"] = view_id
    ctrl["id_kind"] = id_kind
    ctrl["class_name"] = _str_attr("class_name")
    ctrl["label"] = label["value"] if label else None
    ctrl["label_zstring"] = label["zstring_path"] if label else None
    ctrl["label_resolution"] = label["resolution"] if label else None
    if label and label["resolution"] in (UNRESOLVED_GLOBAL, UNRESOLVED_EXPR, KEYWORD_REF):
        ctrl["label_raw"] = label["raw"]
    ctrl["tooltip"] = tooltip["value"] if tooltip else None
    ctrl["tooltip_zstring"] = tooltip["zstring_path"] if tooltip else None
    ctrl["tooltip_resolution"] = tooltip["resolution"] if tooltip else None
    if tooltip and tooltip["resolution"] in (UNRESOLVED_GLOBAL, UNRESOLVED_EXPR, KEYWORD_REF):
        ctrl["tooltip_raw"] = tooltip["raw"]
    if is_layout_ref:
        ctrl["includes_layout"] = _str_attr("name")
        counters["sub_layout_includes"] += 1
    ctrl["hot_text_edit_id"] = _str_attr("hot_text_edit_id") or _str_attr("hotTextEditIdentifier")
    ctrl["cluster_id"] = _str_attr("cluster_id")
    ctrl["resource_id"] = _str_attr("resource_id")
    related = OrderedDict()
    for k, v in node.attrs:
        if k in RELATED_ID_ATTRS:
            tok = v.single()
            related[k] = tok.value if tok is not None else v.text
    if related:
        ctrl["related_ids"] = related
    ctrl["depth"] = depth
    ctrl["nesting_path"] = list(path_stack)
    ctrl["child_count"] = len(node.children)
    ctrl["source_line"] = node.line
    ctrl["attributes"] = attrs_out

    # inline enumerated choices, where a container declares them as child item widgets
    items = []
    for ch in node.children:
        if ch.widget_type in ITEM_WIDGETS:
            _, il = attr_lookup(ch, LABEL_ATTRS)
            iv = resolve_value(il, syms, unresolved_globals) if il else None
            iid = None
            for k, v in ch.attrs:
                if k in ID_ATTRS_OSTYPE + ID_ATTRS_STRING:
                    tok = v.single()
                    iid = tok.value if tok is not None else v.text
                    break
            items.append(OrderedDict([
                ("id", iid),
                ("label", iv["value"] if iv else None),
                ("zstring", iv["zstring_path"] if iv else None),
            ]))
    if items:
        ctrl["items"] = items

    # strip empty keys to reduce noise but keep the semantic ones always present
    for k in list(ctrl.keys()):
        if ctrl[k] is None and k not in ("view_id", "label", "widget_type"):
            del ctrl[k]

    out.append(ctrl)

    step = node.widget_type
    if view_id:
        step = "%s#%s" % (node.widget_type, view_id)
    path_stack.append(step)
    for ch in node.children:
        flatten(ch, syms, unresolved_globals, path_stack, out, counters, depth + 1)
    path_stack.pop()


# ---------------------------------------------------------------------------
# 7. Surface classification from folder taxonomy
# ---------------------------------------------------------------------------

SURFACE_KINDS = {"Dialogs", "Panels", "Properties", "Tools", "Bars", "Flyouts",
                 "Core", "Workspaces", "dialogs", "panels", "properties", "tools",
                 "bars", "flyouts", "shared", "controls", "view", "debug"}

_ID_SUFFIX_RE = re.compile(r"-(\d+)$")


def classify(rel_path):
    """Classify a layout file from its position in the folder taxonomy."""
    parts = rel_path.replace("\\", "/").split("/")
    fname = parts[-1]
    dirs = parts[:-1]
    stem = os.path.splitext(fname)[0]
    m = _ID_SUFFIX_RE.search(stem)
    resource_id = int(m.group(1)) if m else None
    base_name = stem[:m.start()] if m else stem

    unused = bool(dirs) and dirs[0] == "Unused"
    eff = dirs[1:] if unused else dirs

    workspace = None
    if "Workspaces" in eff:
        wi = eff.index("Workspaces")
        if wi + 1 < len(eff):
            workspace = eff[wi + 1]

    kind = None
    for d in reversed(eff):
        if d in SURFACE_KINDS and d != "Workspaces":
            kind = d
            break
    domain = eff[0] if eff else None
    if domain == kind:
        domain = None

    return OrderedDict([
        ("relative_path", "/".join(parts)),
        ("directory", "/".join(dirs)),
        ("file_name", fname),
        ("base_name", base_name),
        ("resource_id", resource_id),
        ("resource_id_source", "numeric filename suffix" if resource_id is not None else None),
        ("domain", domain),
        ("surface_kind", kind),
        ("workspace", workspace),
        ("unused_tree", unused),
    ])


# ---------------------------------------------------------------------------
# 8. File reading with encoding detection
# ---------------------------------------------------------------------------

def read_text(path):
    with open(path, "rb") as fh:
        data = fh.read()
    if data.startswith(b"\xff\xfe"):
        return data.decode("utf-16-le", errors="replace").lstrip("\ufeff"), "utf-16-le-bom"
    if data.startswith(b"\xfe\xff"):
        return data.decode("utf-16-be", errors="replace").lstrip("\ufeff"), "utf-16-be-bom"
    if data.startswith(b"\xef\xbb\xbf"):
        return data[3:].decode("utf-8", errors="replace"), "utf-8-bom"
    try:
        return data.decode("utf-8"), "utf-8"
    except UnicodeDecodeError:
        return data.decode("latin-1"), "latin-1-fallback"


# ---------------------------------------------------------------------------
# 9. Root widget / dialog identification
# ---------------------------------------------------------------------------

DIALOG_ROOT_TYPES = {"dialog", "denoise_dialog"}


def is_dialog_root(node):
    return node.widget_type == "dialog" or node.widget_type.endswith("_dialog")


# ---------------------------------------------------------------------------
# 10. eve_schema (drover eve2 grammar) extraction
# ---------------------------------------------------------------------------

_SCHEMA_DEF_RE = re.compile(
    r"^def\s+([A-Za-z_][A-Za-z0-9_]*)\s*(\(([^)]*)\))?\s*(?::\s*([^{;]+))?\s*([{;])",
    re.M)
_SCHEMA_ENUM_RE = re.compile(r"^enum\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{", re.M)


def _match_brace(text, open_idx):
    depth = 0
    i = open_idx
    n = len(text)
    while i < n:
        c = text[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        elif c == "/" and i + 1 < n and text[i + 1] == "/":
            j = text.find("\n", i)
            i = n if j < 0 else j
            continue
        i += 1
    return -1


def parse_eve_schema(text):
    """Extract def/enum declarations from drover.eve_schema.

    HEURISTIC: this uses anchored regex + brace matching, not the full recursive-descent
    parser, because the schema file is a grammar-description dialect (def/enum/inheritance)
    distinct from the layout dialect. Labelled heuristic in the JSON.
    """
    defs = []
    for m in _SCHEMA_DEF_RE.finditer(text):
        name = m.group(1)
        flags_raw = (m.group(3) or "").strip()
        bases = [b.strip() for b in (m.group(4) or "").split(",") if b.strip()]
        body = ""
        if m.group(5) == "{":
            open_idx = text.index("{", m.end() - 1)
            close_idx = _match_brace(text, open_idx)
            if close_idx > 0:
                body = text[open_idx + 1:close_idx]
        attr_names = []
        depth = 0
        for line in body.split("\n"):
            stripped = line.strip()
            if depth == 0:
                am = re.match(r"^([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+?)\s*(\{|;|$)", stripped)
                if am and not stripped.startswith("//"):
                    attr_names.append({"name": am.group(1), "type": am.group(2).strip()})
            depth += stripped.count("{") - stripped.count("}")
            if depth < 0:
                depth = 0
        flags = {}
        for fm in re.finditer(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([A-Za-z0-9_]+)", flags_raw):
            flags[fm.group(1)] = fm.group(2)
        defs.append(OrderedDict([
            ("name", name),
            ("flags", flags),
            ("inherits", bases),
            ("attributes", attr_names),
            ("abstract", name.startswith("_abstract")),
        ]))
    enums = []
    for m in _SCHEMA_ENUM_RE.finditer(text):
        name = m.group(1)
        open_idx = text.index("{", m.end() - 1)
        close_idx = _match_brace(text, open_idx)
        body = text[open_idx + 1:close_idx] if close_idx > 0 else ""
        vals = []
        for line in body.split("\n"):
            s = line.strip().rstrip(",")
            if not s or s.startswith("//"):
                continue
            vals.append(s)
        enums.append(OrderedDict([("name", name), ("values", vals)]))
    return defs, enums


# ---------------------------------------------------------------------------
# 11. tw10428 .dat string table
# ---------------------------------------------------------------------------

_DAT_LINE_RE = re.compile(r'^\s*"(\$\$\$/[^"]*)"\s*$')


def parse_dat_string_table(path):
    text, enc = read_text(path)
    entries = []
    bad = 0
    for line in text.split("\n"):
        line = line.rstrip("\r")
        if not line.strip():
            continue
        m = _DAT_LINE_RE.match(line)
        if m:
            z = parse_zstring(m.group(1))
            if z:
                entries.append((z[0], z[1]))
                continue
        # tolerate unquoted lines
        s = line.strip().strip('"')
        z = parse_zstring(s)
        if z:
            entries.append((z[0], z[1]))
        else:
            bad += 1
    return entries, enc, bad


# ---------------------------------------------------------------------------
# 12. Main
# ---------------------------------------------------------------------------

def collect_symbols(statements, syms):
    for st in statements:
        if isinstance(st, tuple):
            if st[0] == "assign":
                syms.add(st[1], st[2])
            elif st[0] == "decl":
                syms.add(st[1], st[3])
            elif st[0] == "layout":
                for _sec, decls in st[2].items():
                    for d in decls:
                        if d[0] == "decl":
                            syms.add(d[1], d[3])
                        elif d[0] == "assign":
                            syms.add(d[1], d[2])


def top_level_widgets(statements):
    out = []
    for st in statements:
        if isinstance(st, Node):
            out.append((st, None))
        elif isinstance(st, tuple) and st[0] == "view_decl":
            out.append((st[1], None))
        elif isinstance(st, tuple) and st[0] == "layout":
            for v in st[3]:
                out.append((v, st[1]))
    return out


def process_file(abs_path, rel_path, source_group, stats_bucket, unresolved_globals,
                 zstrings, counters):
    meta = classify(rel_path)
    meta["source_group"] = source_group
    text, enc = read_text(abs_path)
    meta["encoding"] = enc
    meta["extension"] = os.path.splitext(abs_path)[1].lower().lstrip(".")

    # harvest every zstring literal in the raw text before any branch pruning, so the
    # string table is complete regardless of the WinEve/MacEve choice
    for m in re.finditer(r"['\"](\$\$\$/[^'\"]*)['\"]", text):
        z = parse_zstring(m.group(1))
        if z and z[1] is not None:
            zstrings.setdefault(z[0], {"text": z[1], "sources": set()})
            zstrings[z[0]]["sources"].add(source_group)
            if zstrings[z[0]]["text"] != z[1]:
                counters["zstring_text_conflicts"] += 1

    pp_stats = {"other_directives": 0}
    text2, pp_info = preprocess(text, pp_stats)
    meta["preprocessor"] = pp_info

    tokens = tokenize(text2)
    parser = Parser(tokens, rel_path)
    statements = parser.parse_program()

    syms = FileSymbols()
    collect_symbols(statements, syms)
    collect_symbols(parser.bindings, syms)   # assignments nested inside widget bodies
    meta["file_variables"] = len(syms.table)
    if parser.warnings:
        meta["parser_warnings"] = parser.warnings[:20]
        meta["parser_warning_count"] = len(parser.warnings)
        counters["files_with_warnings"] += 1

    surfaces = []
    for root, layout_name in top_level_widgets(statements):
        controls = []
        flatten(root, syms, unresolved_globals, [], controls, counters)
        root_ctrl = controls[0] if controls else {}
        title = root_ctrl.get("label")
        surf = OrderedDict()
        surf["surface_id"] = "%s::%s" % (rel_path, root_ctrl.get("view_id") or root.widget_type)
        surf["layout_name"] = layout_name
        surf["root_widget_type"] = root.widget_type
        surf["is_dialog_root"] = is_dialog_root(root)
        surf["title"] = title
        surf["title_zstring"] = root_ctrl.get("label_zstring")
        surf["class_name"] = root_ctrl.get("class_name")
        surf["target_id"] = root_ctrl.get("attributes", {}).get("target_id")
        surf["root_view_id"] = root_ctrl.get("view_id")
        surf["control_count"] = len(controls)
        surf["controls"] = controls
        surfaces.append(surf)

    meta["surface_count"] = len(surfaces)
    meta["surfaces"] = surfaces
    return meta


def walk_files(root, exts):
    out = []
    for dirpath, _dirnames, filenames in os.walk(root):
        for fn in sorted(filenames):
            if os.path.splitext(fn)[1].lower() in exts:
                ap = os.path.join(dirpath, fn)
                rp = os.path.relpath(ap, root).replace("\\", "/")
                out.append((ap, rp))
    out.sort(key=lambda x: x[1])
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ps-root", default=DEFAULT_PS_ROOT)
    ap.add_argument("--out", default=DEFAULT_OUT)
    args = ap.parse_args()

    ps_root = args.ps_root
    layouts_root = os.path.join(ps_root, "Required", "layouts")
    drover_root = os.path.join(ps_root, "Required", "drover_layouts")
    owl_root = os.path.join(ps_root, "Required", "OWL")
    locale_support = os.path.join(ps_root, "Locales", "en_US", "Support Files")

    if not os.path.isdir(layouts_root):
        sys.stderr.write("FATAL: layouts root not found: %s\n" % layouts_root)
        return 2

    unresolved_globals = Counter()
    zstrings = {}
    counters = Counter()

    source_groups = []
    all_files = []

    # -- primary source ----------------------------------------------------
    layout_files = walk_files(layouts_root, {".exv", ".eve"})
    other_layout_files = [
        (ap, rp) for ap, rp in walk_files(layouts_root, {".json", ".txt", ".dat", ".xml"})
    ]
    source_groups.append(OrderedDict([
        ("id", "required_layouts"),
        ("root", layouts_root),
        ("role", "primary"),
        ("description",
         "Classic Adobe Eve declarative UI layout tree. Defines Photoshop dialogs, panels, "
         "properties views, tool-options bars and flyouts with 4-character OSType view_id "
         "keys that correspond to Action Descriptor parameter keys."),
        ("layout_file_count", len(layout_files)),
        ("layout_file_count_by_extension",
         dict(Counter(os.path.splitext(rp)[1].lower().lstrip(".") for _a, rp in layout_files))),
        ("non_layout_files", [rp for _a, rp in other_layout_files]),
        ("total_files_in_tree", len(layout_files) + len(other_layout_files)),
    ]))
    all_files.extend([(ap, rp, "required_layouts") for ap, rp in layout_files])

    # -- additional: drover_layouts ---------------------------------------
    drover_files = []
    if os.path.isdir(drover_root):
        drover_files = walk_files(drover_root, {".eve"})
        source_groups.append(OrderedDict([
            ("id", "required_drover_layouts"),
            ("root", drover_root),
            ("role", "additional"),
            ("description",
             "Modern Adobe 'Drover' eve2 declarative UI layout tree shipped alongside the "
             "classic tree. Uses string `identifier:` control ids rather than 4-char OSType "
             "view_id keys. Parsed with the same parser; kept in a separate source group."),
            ("layout_file_count", len(drover_files)),
            ("layout_file_count_by_extension",
             dict(Counter(os.path.splitext(rp)[1].lower().lstrip(".") for _a, rp in drover_files))),
            ("total_files_in_tree", len(walk_files(drover_root, {".eve", ".eve_schema"}))),
        ]))
        all_files.extend([(ap, rp, "required_drover_layouts") for ap, rp in drover_files])

    # -- additional: OWL ---------------------------------------------------
    owl_files = []
    if os.path.isdir(owl_root):
        owl_files = walk_files(owl_root, {".eve"})
        source_groups.append(OrderedDict([
            ("id", "required_owl"),
            ("root", owl_root),
            ("role", "additional"),
            ("description",
             "Adobe OWL (application frame / app bar / grid / screen mode / view options) "
             "Eve layouts."),
            ("layout_file_count", len(owl_files)),
            ("total_files_in_tree", len(walk_files(owl_root, {".eve", ".adm"}))),
        ]))
        all_files.extend([(ap, rp, "required_owl") for ap, rp in owl_files])

    # -- parse everything --------------------------------------------------
    files_out = []
    parse_failures = []
    for ap, rp, group in all_files:
        try:
            meta = process_file(ap, rp, group, None, unresolved_globals, zstrings, counters)
            files_out.append(meta)
        except Exception as exc:  # noqa: BLE001 - failures are reported, not swallowed
            parse_failures.append(OrderedDict([
                ("source_group", group),
                ("relative_path", rp),
                ("error_type", type(exc).__name__),
                ("error", str(exc)),
                ("traceback_tail", traceback.format_exc().strip().split("\n")[-1]),
            ]))

    # -- string table: layout-tree zstrings + external tables -------------
    string_table = {}
    for path, rec in zstrings.items():
        string_table[path] = OrderedDict([
            ("text", rec["text"]),
            ("sources", sorted(rec["sources"])),
        ])

    dat_path = os.path.join(locale_support, "tw10428_Photoshop_en_US.dat")
    dat_info = OrderedDict([("path", dat_path), ("present", os.path.isfile(dat_path))])
    if dat_info["present"]:
        entries, enc, bad = parse_dat_string_table(dat_path)
        dat_info["encoding"] = enc
        dat_info["entries_parsed"] = len(entries)
        dat_info["lines_not_matching_zstring_form"] = bad
        added = 0
        merged = 0
        conflicts = 0
        for zpath, ztext in entries:
            if zpath in string_table:
                merged += 1
                if string_table[zpath]["text"] != ztext:
                    conflicts += 1
                    string_table[zpath]["conflicting_text_locale_dat"] = ztext
                string_table[zpath]["sources"] = sorted(
                    set(string_table[zpath]["sources"]) | {"locale_tw10428_dat"})
            else:
                added += 1
                string_table[zpath] = OrderedDict([
                    ("text", ztext), ("sources", ["locale_tw10428_dat"])])
        dat_info["new_zstring_paths_added"] = added
        dat_info["zstring_paths_already_present"] = merged
        dat_info["text_conflicts_with_layout_tree"] = conflicts

    other_string_sources = []
    for name, note in (
        ("Required/UIColors.txt",
         "Photoshop Color Values table. Inspected: contains no $$$/ zstrings. Not folded in."),
        ("Required/PSConfig.txt",
         "Default configuration file. Inspected: single comment line, no strings. Not folded in."),
        ("Locales/en_US/Support Files/pack.inf",
         "UTF-16 locale pack descriptor (version/prefstring/localeid). No zstrings."),
        ("Locales/en_US/Support Files/Shortcuts/Win/OS Shortcuts.txt",
         "Windows OS keyboard shortcut list. Checked for zstrings."),
        ("Required/Default Menus.mnu",
         "Binary-prefixed menu resource. Checked for zstrings."),
    ):
        p = os.path.join(ps_root, name.replace("/", os.sep))
        rec = OrderedDict([("path", p), ("present", os.path.isfile(p)), ("note", note)])
        if rec["present"]:
            try:
                t, enc = read_text(p)
                found = re.findall(r"(\$\$\$/[^'\"\r\n]*)", t)
                rec["encoding"] = enc
                rec["zstrings_found"] = len(found)
                if found:
                    added = 0
                    for f in found:
                        z = parse_zstring(f)
                        if z and z[1] is not None and z[0] not in string_table:
                            string_table[z[0]] = OrderedDict([
                                ("text", z[1]), ("sources", ["other:" + name])])
                            added += 1
                    rec["new_zstring_paths_added"] = added
            except Exception as exc:  # noqa: BLE001
                rec["read_error"] = "%s: %s" % (type(exc).__name__, exc)
        other_string_sources.append(rec)

    # -- eve_schema --------------------------------------------------------
    schema_path = os.path.join(drover_root, "drover.eve_schema")
    schema_section = OrderedDict([("path", schema_path), ("present", os.path.isfile(schema_path))])
    if schema_section["present"]:
        stext, senc = read_text(schema_path)
        sdefs, senums = parse_eve_schema(stext)
        schema_section["encoding"] = senc
        schema_section["extraction_method"] = (
            "HEURISTIC regex + brace matching, not the recursive-descent layout parser. "
            "The schema file uses a grammar-description dialect (def/enum with inheritance) "
            "distinct from the layout dialect.")
        schema_section["is_heuristic"] = True
        schema_section["widget_definition_count"] = len(sdefs)
        schema_section["enum_count"] = len(senums)
        schema_section["widget_definitions"] = sdefs
        schema_section["enums"] = senums

    # -- indexes -----------------------------------------------------------
    widget_type_counts = Counter()
    class_name_counts = Counter()
    view_id_index = defaultdict(lambda: {"labels": Counter(), "widget_types": Counter(),
                                         "class_names": Counter(), "surfaces": []})
    total_controls = 0
    surface_count = 0
    surfaces_by_kind = Counter()
    surfaces_by_group = Counter()
    dialog_root_count = 0
    layout_includes = defaultdict(list)
    attr_key_counts = Counter()

    for f in files_out:
        for s in f["surfaces"]:
            surface_count += 1
            surfaces_by_kind[f.get("surface_kind") or "(none)"] += 1
            surfaces_by_group[f["source_group"]] += 1
            if s["is_dialog_root"]:
                dialog_root_count += 1
            for c in s["controls"]:
                total_controls += 1
                widget_type_counts[c["widget_type"]] += 1
                for ak in c.get("attributes", {}):
                    attr_key_counts[ak] += 1
                if c.get("includes_layout"):
                    layout_includes[c["includes_layout"]].append(OrderedDict([
                        ("file", f["relative_path"]),
                        ("source_group", f["source_group"]),
                        ("host_layout", s["layout_name"]),
                    ]))
                if c.get("class_name"):
                    class_name_counts[c["class_name"]] += 1
                vid = c.get("view_id")
                if vid:
                    ent = view_id_index[vid]
                    ent["widget_types"][c["widget_type"]] += 1
                    if c.get("class_name"):
                        ent["class_names"][c["class_name"]] += 1
                    if c.get("label"):
                        ent["labels"][c["label"]] += 1
                    ref = OrderedDict([
                        ("file", f["relative_path"]),
                        ("source_group", f["source_group"]),
                        ("surface", s["surface_id"]),
                        ("surface_title", s["title"]),
                    ])
                    ent["surfaces"].append(ref)

    view_id_index_out = OrderedDict()
    for vid in sorted(view_id_index.keys()):
        ent = view_id_index[vid]
        view_id_index_out[vid] = OrderedDict([
            ("length", len(vid)),
            ("is_four_char_ostype", len(vid) == 4),
            ("use_count", len(ent["surfaces"])),
            ("widget_types", dict(ent["widget_types"])),
            ("class_names", dict(ent["class_names"])),
            ("labels", [lbl for lbl, _n in ent["labels"].most_common()]),
            ("used_in", ent["surfaces"]),
        ])

    # -- directory taxonomy -----------------------------------------------
    taxonomy = OrderedDict()
    for f in files_out:
        d = f["directory"] or "(root)"
        e = taxonomy.setdefault(d, OrderedDict([
            ("source_group", f["source_group"]),
            ("domain", f.get("domain")),
            ("surface_kind", f.get("surface_kind")),
            ("workspace", f.get("workspace")),
            ("unused_tree", f.get("unused_tree")),
            ("file_count", 0),
            ("surface_count", 0),
            ("control_count", 0),
        ]))
        e["file_count"] += 1
        e["surface_count"] += f["surface_count"]
        e["control_count"] += sum(s["control_count"] for s in f["surfaces"])

    # -- resource id registry ---------------------------------------------
    res_ids_path = os.path.join(layouts_root, "Unused", "resource_ids.json")
    resource_id_registry = OrderedDict([
        ("path", res_ids_path), ("present", os.path.isfile(res_ids_path))])
    if resource_id_registry["present"]:
        try:
            with open(res_ids_path, "r", encoding="utf-8") as fh:
                rj = json.load(fh)
            rows = rj.get("resource_ids", [])
            resource_id_registry["entry_count"] = len(rows)
            resource_id_registry["note"] = (
                "Shipped id->name->origin registry. Cross-referenced against the numeric "
                "filename suffixes to name dialog resource ids.")
            resource_id_registry["entries"] = rows
        except Exception as exc:  # noqa: BLE001
            resource_id_registry["read_error"] = "%s: %s" % (type(exc).__name__, exc)

    # cross-reference file resource ids against the registry
    reg_by_id = {}
    for row in resource_id_registry.get("entries", []) or []:
        try:
            reg_by_id[int(row.get("id"))] = row
        except (TypeError, ValueError):
            pass
    matched_ids = 0
    for f in files_out:
        rid = f.get("resource_id")
        if rid is not None and rid in reg_by_id:
            f["resource_id_registry_name"] = reg_by_id[rid].get("name")
            f["resource_id_registry_origin"] = reg_by_id[rid].get("origin_r")
            matched_ids += 1

    # -- assemble ----------------------------------------------------------
    now = _dt.datetime.now(_dt.timezone.utc).replace(microsecond=0).isoformat().replace(
        "+00:00", "Z")

    files_with_zero_surfaces = [f["relative_path"] for f in files_out if f["surface_count"] == 0]

    doc = OrderedDict()
    doc["schema_id"] = SCHEMA_ID
    doc["generated_at"] = now
    doc["generator"] = OrderedDict([
        ("script", os.path.abspath(__file__)),
        ("python", sys.version.split()[0]),
        ("mode", "offline, read-only, no application launched"),
    ])
    doc["product"] = OrderedDict([
        ("name", "Adobe Photoshop 2026"),
        ("install_root", ps_root),
    ])

    doc["method"] = OrderedDict([
        ("overview",
         "Every source file is read as bytes, BOM-sniffed for UTF-16/UTF-8, decoded, then "
         "passed through a three-stage pipeline: (1) a line-oriented preprocessor that "
         "resolves #ifdef platform blocks, (2) a hand-written lexer, (3) a hand-written "
         "recursive-descent parser for the Adobe Eve declarative layout grammar. Nothing "
         "is regex-scraped except where explicitly labelled heuristic below."),
        ("preprocessor",
         "The only conditional symbols present in the tree are MacEve and WinEve "
         "(158 '#ifdef MacEve' and 164 '#ifdef WinEve' occurrences against 322 '#endif'). "
         "DOCUMENTED DECISION: the WinEve branch is KEPT and the MacEve branch is DISCARDED, "
         "because this is a Windows install. Discarded lines are replaced with empty lines so "
         "source_line numbers remain accurate. Per-file counts of kept/dropped conditional "
         "lines are in files[].preprocessor. Any other #ifdef symbol encountered would be "
         "kept (conservative) and reported in files[].preprocessor.unknown_conditional_symbols."),
        ("lexer",
         "Handles // line comments, /* */ block comments, single- and double-quoted strings "
         "with backslash escapes, decimal/hex/float numbers, identifiers, @keyword tokens, "
         "and multi-character operators (<==, ==, !=, >=, <=, &&, ||)."),
        ("parser",
         "Recursive descent over four statement forms: (a) 'layout NAME { ... }' blocks with "
         "constant:/interface:/logic: declaration sections and 'view WIDGET(...) {...}' roots "
         "(the .eve dialect); (b) 'name = expression;' file-level assignments (the .exv "
         "dialect); (c) 'name : expression;' and 'name <== expression;' declarations; "
         "(d) widget statements 'type(attr: value, ...) { children }' or leaf widgets "
         "terminated by ';'. Nested parentheses, brackets and braces are tracked by depth. "
         "Numeric and boolean expressions are NOT evaluated -- they are preserved verbatim as "
         "raw strings."),
        ("surface_classification",
         "Each file's surface kind is taken from the shipped folder taxonomy, not guessed: the "
         "path segment matching {Dialogs, Panels, Properties, Tools, Bars, Flyouts, Core, "
         "shared, controls, dialogs, panels, properties, tools} nearest the file becomes "
         "surface_kind; the first segment becomes domain; a segment after 'Workspaces' becomes "
         "workspace; a leading 'Unused' segment sets unused_tree=true and is stripped before "
         "domain/kind resolution."),
        ("resource_id",
         "The numeric suffix in the file name (e.g. 'brightness-1780' -> 1780) is recorded as "
         "the Photoshop dialog resource id, and cross-referenced against the shipped "
         "Unused/resource_ids.json registry to attach the registry name and origin where the "
         "id matches."),
        ("zstring_resolution",
         "A literal of the form '$$$/Path/Key=English text' is split at the FIRST '=' after the "
         "'$$$/' prefix; the left side is recorded as zstring_path and the right side as the "
         "English display text (so display text containing '=' survives). When an attribute "
         "value is a bare identifier it is resolved against the file's own symbol table "
         "(assignments plus constant:/interface: declarations), following aliases up to 16 "
         "levels with a cycle guard. The symbol table is collected from EVERY nesting depth, "
         "not just the file's top level, because Photoshop layout files legitimately place "
         "'name = value;' assignments inside widget bodies. The resolution status is recorded "
         "per value as one of "
         "literal / resolved_local_variable / unresolved_global_symbol / unresolved_expression "
         "/ keyword_reference. Symbols such as gOKString and gCancelString are compiled into "
         "the Photoshop binary and are defined in NO layout file; they are reported honestly as "
         "unresolved_global_symbol with the raw variable name preserved, never guessed."),
        ("string_table",
         "Built in two passes. Pass 1 scans the RAW pre-preprocessor text of every layout file "
         "with a literal-anchored regex for '$$$/...' inside quotes, so strings inside the "
         "discarded MacEve branch are still captured. Pass 2 folds in the UTF-16LE locale table "
         "Locales/en_US/Support Files/tw10428_Photoshop_en_US.dat, whose lines are quoted "
         "'\"$$$/Path=Text\"' records. Provenance per zstring path is recorded in .sources. "
         "Text conflicts between sources are reported, never silently overwritten."),
        ("control_flattening",
         "Every parsed widget tree is flattened depth-first into a per-surface FLAT control "
         "list. Each control records widget_type, view_id (from view_id: or identifier:), "
         "id_kind, class_name, resolved label + its zstring path, resolved tooltip + its "
         "zstring path, hot_text_edit_id, cluster_id, resource_id, related_ids, depth, "
         "nesting_path (ancestor chain as 'widget_type#view_id' steps), child_count, "
         "source_line and the remaining attributes. Label text is read from the first present "
         "of %s; tooltip text from the first present of %s -- 'alt' is included because "
         "drover.eve_schema documents it verbatim as \"Alternative text (tooltip) for the "
         "widget\". related_ids captures every attribute that references another control's id "
         "(%s) so cross-widget wiring (hot text -> edit field, edit field -> slider, control -> "
         "menu resource) is recoverable."
         % (", ".join(LABEL_ATTRS), ", ".join(TOOLTIP_ATTRS), ", ".join(RELATED_ID_ATTRS))),
        ("grammar_features_supported",
         "Beyond the core widget/attribute grammar the parser handles: the 'unlink' cell "
         "modifier prefixing an interface declaration; bare uninitialised interface "
         "declarations of the form 'name;'; the '<==' Eve cell-binding operator; '@keyword' "
         "cell references; ternary '?:' and boolean expressions (captured raw, never "
         "evaluated); and shipped files containing one unbalanced trailing '}' (recovered by "
         "skipping the token and recording a parse anomaly)."),
        ("attribute_filtering",
         "Per-control `attributes` omits pure box-model geometry keys (%s) which carry no "
         "semantic meaning for a reimplementation. Every other attribute is retained verbatim. "
         "Count of dropped geometry attributes is in parse_stats.geometry_attributes_dropped."
         % ", ".join(sorted(GEOMETRY_ATTRS))),
        ("enumerated_choices",
         "Inline enumerated choices are emitted as controls[].items when a container declares "
         "child item widgets (popup_item / menu_item / item / slot_item). In the classic .exv "
         "tree popups almost never declare inline items: they reference a menu resource by "
         "numeric resource_id, whose item list lives in the compiled binary and is NOT "
         "recoverable from the layout files. This is recorded in unknowns."),
        ("indexes",
         "Global indexes are built by a single pass over every emitted control: view_id_index "
         "(distinct control id -> every surface using it, with labels, widget types and class "
         "names), widget_type_index (distinct widget type -> occurrence count), and "
         "class_name_index (distinct class_name -> occurrence count)."),
        ("branch_choice_effect",
         "WinEve was chosen over MacEve. This affects only platform-conditional metric "
         "assignments and, in a small number of files, platform-conditional widgets. Any widget "
         "that exists ONLY inside a MacEve block is therefore absent from the control lists; the "
         "zstrings inside those blocks are still present in the string table (see string_table "
         "above)."),
    ])

    doc["source_files"] = OrderedDict([
        ("source_groups", source_groups),
        ("total_layout_files_discovered", len(all_files)),
        ("primary_scope_note",
         "Required/layouts contains %d layout files (%d .exv + %d .eve) plus 1 non-layout "
         "JSON file (Unused/resource_ids.json), for %d files in the tree in total."
         % (len(layout_files),
            sum(1 for _a, rp in layout_files if rp.endswith(".exv")),
            sum(1 for _a, rp in layout_files if rp.endswith(".eve")),
            len(layout_files) + len(other_layout_files))),
        ("locale_string_table", dat_info),
        ("other_string_sources_checked", other_string_sources),
        ("resource_id_registry", OrderedDict([
            (k, v) for k, v in resource_id_registry.items() if k != "entries"
        ] + [("resource_ids_matched_to_layout_files", matched_ids)])),
    ])

    doc["parse_stats"] = OrderedDict([
        ("layout_files_attempted", len(all_files)),
        ("layout_files_parsed_ok", len(files_out)),
        ("layout_files_failed", len(parse_failures)),
        ("failures", parse_failures),
        ("files_with_parser_warnings", counters["files_with_warnings"]),
        ("parse_anomalies", [
            OrderedDict([
                ("relative_path", f["relative_path"]),
                ("source_group", f["source_group"]),
                ("warning_count", f["parser_warning_count"]),
                ("warnings", f["parser_warnings"]),
            ])
            for f in files_out if f.get("parser_warning_count")
        ]),
        ("parse_anomaly_note",
         "Every anomaly below is a token the parser could not place, recovered by skipping "
         "that single token and continuing. Independent verification by counting braces in "
         "the raw bytes shows the '}' anomalies are genuine: those shipped files contain one "
         "more '}' than '{'. No content was lost -- the stray brace is the final character of "
         "the file in each case."),
        ("files_yielding_zero_surfaces", len(files_with_zero_surfaces)),
        ("files_yielding_zero_surfaces_list", files_with_zero_surfaces),
        ("surfaces_emitted", surface_count),
        ("surfaces_with_dialog_root", dialog_root_count),
        ("surfaces_by_source_group", dict(surfaces_by_group)),
        ("surfaces_by_surface_kind", dict(surfaces_by_kind)),
        ("controls_emitted", total_controls),
        ("distinct_widget_types", len(widget_type_counts)),
        ("distinct_class_names", len(class_name_counts)),
        ("distinct_view_ids", len(view_id_index_out)),
        ("distinct_four_char_ostype_view_ids",
         sum(1 for v in view_id_index_out.values() if v["is_four_char_ostype"])),
        ("distinct_zstring_paths", len(string_table)),
        ("geometry_attributes_dropped", counters["geometry_attrs_dropped"]),
        ("zstring_text_conflicts_within_layout_tree", counters["zstring_text_conflicts"]),
        ("distinct_unresolved_global_symbols", len(unresolved_globals)),
    ])

    doc["directory_taxonomy"] = taxonomy

    doc["widget_type_index"] = OrderedDict(
        (k, v) for k, v in sorted(widget_type_counts.items(), key=lambda kv: (-kv[1], kv[0])))
    doc["class_name_index"] = OrderedDict(
        (k, v) for k, v in sorted(class_name_counts.items(), key=lambda kv: (-kv[1], kv[0])))
    doc["view_id_index"] = view_id_index_out
    doc["attribute_key_index"] = OrderedDict(
        (k, v) for k, v in sorted(attr_key_counts.items(), key=lambda kv: (-kv[1], kv[0])))
    doc["layout_composition_index"] = OrderedDict([
        ("note",
         "Layouts compose via the `sub_layout(name: \"<layout>\")` widget. This maps each "
         "included layout name to every layout file that includes it. Included layout names "
         "are Eve layout names, not file paths; resolve them against files[].surfaces[]"
         ".layout_name."),
        ("total_sub_layout_includes", counters["sub_layout_includes"]),
        ("included_layout_count", len(layout_includes)),
        ("included_by", OrderedDict(sorted(layout_includes.items()))),
    ])

    doc["unresolved_global_symbols"] = OrderedDict(
        (k, v) for k, v in sorted(unresolved_globals.items(), key=lambda kv: (-kv[1], kv[0])))

    doc["string_table"] = OrderedDict(sorted(string_table.items()))
    doc["eve_schema"] = schema_section
    doc["resource_id_registry_entries"] = resource_id_registry.get("entries", [])

    doc["files"] = files_out

    doc["heuristics"] = [
        OrderedDict([
            ("id", "H1"),
            ("heuristic", True),
            ("what", "surface_kind / domain / workspace classification"),
            ("basis", "Folder-name matching against a fixed set of known surface-kind folder "
                      "names. Files in folders outside that set (e.g. Required/layouts/OCIO) "
                      "get surface_kind=null."),
            ("risk", "A shipped folder whose name is not in the set is left unclassified rather "
                     "than mis-classified."),
        ]),
        OrderedDict([
            ("id", "H2"),
            ("heuristic", True),
            ("what", "is_dialog_root flag"),
            ("basis", "True when the root widget type is exactly 'dialog' or ends with "
                      "'_dialog'. Modal-ness is not declared in the layout files."),
            ("risk", "Some roots that are modal at runtime use a generic 'view' root and are "
                     "therefore not flagged."),
        ]),
        OrderedDict([
            ("id", "H3"),
            ("heuristic", True),
            ("what", "eve_schema widget/enum extraction"),
            ("basis", "Anchored regex + brace matching over drover.eve_schema, NOT the "
                      "recursive-descent parser. The schema uses a grammar-description dialect."),
            ("risk", "Attribute lists inside deeply nested schema bodies may be under-reported. "
                     "Definition names, inheritance and enum value lists are reliable."),
        ]),
        OrderedDict([
            ("id", "H4"),
            ("heuristic", True),
            ("what", "id_kind = four-character OSType classification"),
            ("basis", "A view_id / identifier string literal of exactly 4 characters is "
                      "labelled an OSType key. Length is the only available signal."),
            ("risk", "A coincidental 4-character non-OSType id would be mislabelled. In the "
                     "classic .exv tree essentially all view_id values are OSType keys; in the "
                     "drover tree identifiers are descriptive strings and are labelled "
                     "identifier_string."),
        ]),
        OrderedDict([
            ("id", "H5"),
            ("heuristic", True),
            ("what", "attribute geometry filtering"),
            ("basis", "A fixed blocklist of box-model attribute names is dropped from "
                      "controls[].attributes."),
            ("risk", "If a dropped name is ever reused for a semantic purpose, that value is "
                     "lost from the per-control attribute map. The full blocklist is published "
                     "in method.attribute_filtering."),
        ]),
        OrderedDict([
            ("id", "H6"),
            ("heuristic", True),
            ("what", "string_table harvesting pass 1"),
            ("basis", "Literal-anchored regex over the RAW file text for quoted '$$$/...' "
                      "sequences, used INSTEAD of the parser so that strings inside the "
                      "discarded MacEve branch are not lost."),
            ("risk", "A '$$$/' sequence inside a comment would be harvested. Structural "
                     "attachment of strings to controls always comes from the parser, never "
                     "from this pass."),
        ]),
    ]

    doc["unknowns"] = [
        OrderedDict([
            ("id", "U1"),
            ("what", "Global Eve symbols such as gOKString, gCancelString, gGap, gLargeSpace, "
                     "gDialogIconWidth."),
            ("status", "NOT RESOLVED. These are defined nowhere in the shipped layout tree; "
                       "they are compiled into the Photoshop binary. Their raw names are "
                       "preserved and their resolution status is unresolved_global_symbol. "
                       "Their values are NOT guessed."),
            ("full_list_at", "unresolved_global_symbols"),
        ]),
        OrderedDict([
            ("id", "U2"),
            ("what", "Popup / menu item lists referenced by numeric resource_id."),
            ("status", "NOT RECOVERABLE from the layout tree. Classic .exv popups declare "
                       "'resource_id: N' and the actual item list lives in a compiled menu "
                       "resource. The resource_id is recorded on the control; the items are not."),
        ]),
        OrderedDict([
            ("id", "U3"),
            ("what", "Runtime modality, tab order, enable/disable logic, validation rules and "
                     "value ranges not declared as attributes."),
            ("status", "NOT PRESENT in the layout files. Eve layouts declare structure, "
                       "identity and static presentation only."),
        ]),
        OrderedDict([
            ("id", "U4"),
            ("what", "Mapping from view_id OSType keys to Action Descriptor keys."),
            ("status", "NOT VERIFIED BY THIS SCRIPT. The layout files supply the 4-character "
                       "view_id keys; the claim that they correspond to Action Descriptor "
                       "parameter keys is an external premise supplied to this teardown and is "
                       "not proven by anything in these files."),
        ]),
        OrderedDict([
            ("id", "U5"),
            ("what", "Widgets that exist only inside '#ifdef MacEve' blocks."),
            ("status", "ABSENT from the control lists by the documented WinEve branch choice. "
                       "Per-file dropped conditional line counts are in "
                       "files[].preprocessor.conditional_lines_dropped."),
        ]),
        OrderedDict([
            ("id", "U6"),
            ("what", "Files that parse successfully but yield zero surfaces."),
            ("status", "Listed in parse_stats.files_yielding_zero_surfaces_list. These are "
                       "files whose top level contains only declarations, or whose root "
                       "construct the parser did not recognise as a widget."),
        ]),
        OrderedDict([
            ("id", "U7"),
            ("what", "Layouts pulled in by `sub_layout(name: ...)`."),
            ("status", "The include EDGE is captured (layout_composition_index) but the "
                       "controls of an included layout are NOT inlined into the including "
                       "surface's control list. Each layout is emitted once, under its own "
                       "file. A consumer wanting the fully composed tree must follow the "
                       "include graph."),
        ]),
        OrderedDict([
            ("id", "U8"),
            ("what", "Misspelled keys verified in the shipped source. (a) 'iew_id' -- one "
                     "widget attribute, Unused/3D/Dialogs/3DGenerateUVs-5143.exv line 20, a "
                     "truncated 'view_id' on a cluster (the same widget also carries a "
                     "misspelled 'vhorizontal'). (b) 'resourse_id' -- one occurrence, "
                     "Preferences/Dialogs/preferences-3080.exv line 1039, inside the "
                     "expression menu_width(resourse_id: 3093); it is a function-call argument, "
                     "NOT a widget attribute, so it correctly does not appear in "
                     "attribute_key_index."),
            ("status", "Preserved verbatim, never silently corrected, because a misspelled key "
                       "is presumably ignored by Photoshop's own layout loader too and guessing "
                       "intent would fabricate data. The 3DGenerateUVs cluster carrying "
                       "'iew_id' therefore has view_id=null in this export."),
        ]),
        OrderedDict([
            ("id", "U9"),
            ("what", "Surface titles for dialogs whose root declares no name attribute "
                     "(e.g. Filters/Dialogs/unsharpMask-1510.exv)."),
            ("status", "title=null. Verified by reading the source: those dialog() roots "
                       "genuinely carry no name: attribute; the window title comes from a "
                       "compiled resource. Where the numeric filename id matches the shipped "
                       "Unused/resource_ids.json registry, the registry's name is attached as "
                       "files[].resource_id_registry_name -- that is a REGISTRY name, not a "
                       "parsed dialog title, and the two are kept in separate fields."),
        ]),
    ]

    out_path = args.out
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)

    size = os.path.getsize(out_path)
    sys.stderr.write(
        "WROTE %s (%.2f MB)\n"
        "  layout files attempted : %d\n"
        "  parsed ok              : %d\n"
        "  failed                 : %d\n"
        "  surfaces               : %d\n"
        "  controls               : %d\n"
        "  distinct widget types  : %d\n"
        "  distinct class names   : %d\n"
        "  distinct view_ids      : %d\n"
        "  distinct zstrings      : %d\n"
        % (out_path, size / 1048576.0, len(all_files), len(files_out), len(parse_failures),
           surface_count, total_controls, len(widget_type_counts), len(class_name_counts),
           len(view_id_index_out), len(string_table)))
    for f in parse_failures:
        sys.stderr.write("  FAIL %s :: %s: %s\n"
                         % (f["relative_path"], f["error_type"], f["error"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
