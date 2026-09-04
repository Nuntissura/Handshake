"""
lrlua.py - minimal reader for Adobe Lightroom's Lua-table serialisation format.

Lightroom persists .lrtemplate, .agprefs, .lrweb and templatePages.lua as plain
Lua source that assigns a single table (`s = { ... }`, `prefs = { ... }`).
This module parses that dialect into Python data. It is a *parser*, not an
evaluator: it accepts table constructors, string/number/boolean/nil literals,
`ZSTR "..."` wrapped strings, long-bracket strings, comments, and identifier or
bracketed keys. Function calls other than ZSTR are captured as opaque markers so
the caller can see that a value was code rather than data.

No Lua runtime is invoked and no shipped file is modified.
"""
from __future__ import annotations

import re

_WS = re.compile(r"(?:\s+|--\[(=*)\[.*?\]\1\]|--[^\n]*)+", re.S)
_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_NUM = re.compile(r"-?(?:0[xX][0-9a-fA-F]+|\d+\.\d*(?:[eE][+-]?\d+)?|"
                  r"\.\d+(?:[eE][+-]?\d+)?|\d+(?:[eE][+-]?\d+)?)")

_ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "a": "\a", "b": "\b",
            "f": "\f", "v": "\v", "\\": "\\", '"': '"', "'": "'", "\n": "\n"}


class LuaParseError(ValueError):
    pass


class OpaqueCall:
    """A function call encountered where a value was expected."""

    def __init__(self, name, raw):
        self.name = name
        self.raw = raw

    def to_json(self):
        return {"__lua_call__": self.name, "__raw__": self.raw[:400]}

    def __repr__(self):  # pragma: no cover
        return f"OpaqueCall({self.name!r})"


class _P:
    def __init__(self, src: str):
        self.s = src
        self.i = 0
        self.n = len(src)

    def ws(self):
        while True:
            m = _WS.match(self.s, self.i)
            if not m or m.end() == self.i:
                return
            self.i = m.end()

    def peek(self):
        self.ws()
        return self.s[self.i] if self.i < self.n else ""

    def expect(self, ch):
        self.ws()
        if self.i >= self.n or self.s[self.i] != ch:
            got = self.s[self.i:self.i + 24] if self.i < self.n else "<eof>"
            raise LuaParseError(f"expected {ch!r} at {self.i}, saw {got!r}")
        self.i += 1

    # -- literals ----------------------------------------------------------
    def long_string(self):
        m = re.compile(r"\[(=*)\[").match(self.s, self.i)
        if not m:
            return None
        eq = m.group(1)
        close = "]" + eq + "]"
        j = self.s.find(close, m.end())
        if j < 0:
            raise LuaParseError("unterminated long string")
        val = self.s[m.end():j]
        self.i = j + len(close)
        return val.lstrip("\n")

    def quoted_string(self):
        q = self.s[self.i]
        self.i += 1
        out = []
        while self.i < self.n:
            c = self.s[self.i]
            if c == "\\":
                self.i += 1
                e = self.s[self.i]
                if e in _ESCAPES:
                    out.append(_ESCAPES[e])
                    self.i += 1
                elif e == "x":
                    out.append(chr(int(self.s[self.i + 1:self.i + 3], 16)))
                    self.i += 3
                elif e.isdigit():
                    m = re.compile(r"\d{1,3}").match(self.s, self.i)
                    out.append(chr(int(m.group(0))))
                    self.i = m.end()
                else:
                    out.append(e)
                    self.i += 1
            elif c == q:
                self.i += 1
                return "".join(out)
            else:
                out.append(c)
                self.i += 1
        raise LuaParseError("unterminated string")

    def value(self):
        self.ws()
        if self.i >= self.n:
            raise LuaParseError("eof in value")
        c = self.s[self.i]
        if c == "{":
            return self.table()
        if c in "\"'":
            return self.quoted_string()
        if c == "[" and re.compile(r"\[=*\[").match(self.s, self.i):
            return self.long_string()
        m = _NUM.match(self.s, self.i)
        if m and (c.isdigit() or c == "-" or c == "."):
            self.i = m.end()
            t = m.group(0)
            if t.lower().startswith("0x") or t.lower().startswith("-0x"):
                return int(t, 16)
            return float(t) if ("." in t or "e" in t or "E" in t) else int(t)
        m = _NAME.match(self.s, self.i)
        if m:
            word = m.group(0)
            self.i = m.end()
            if word == "true":
                return True
            if word == "false":
                return False
            if word == "nil":
                return None
            if word == "ZSTR":
                self.ws()
                if self.s[self.i] in "\"'":
                    return {"__zstr__": self.quoted_string()}
                return {"__zstr__": None}
            # identifier / call / dotted path
            start = self.i - len(word)
            while True:
                self.ws()
                if self.i < self.n and self.s[self.i] in ".:":
                    self.i += 1
                    m2 = _NAME.match(self.s, self.i)
                    if not m2:
                        break
                    self.i = m2.end()
                    continue
                break
            self.ws()
            if self.i < self.n and self.s[self.i] in "({\"'":
                depth = 0
                j = self.i
                if self.s[j] == "(":
                    while j < self.n:
                        if self.s[j] == "(":
                            depth += 1
                        elif self.s[j] == ")":
                            depth -= 1
                            if depth == 0:
                                j += 1
                                break
                        j += 1
                    raw = self.s[start:j]
                    self.i = j
                    return OpaqueCall(word, raw)
            return OpaqueCall(word, self.s[start:self.i])
        raise LuaParseError(f"unparsable value at {self.i}: "
                            f"{self.s[self.i:self.i + 30]!r}")

    def table(self):
        self.expect("{")
        out = {}
        arr = []
        while True:
            self.ws()
            if self.i >= self.n:
                raise LuaParseError("eof in table")
            if self.s[self.i] == "}":
                self.i += 1
                break
            if self.s[self.i] in ",;":
                self.i += 1
                continue
            save = self.i
            key = None
            if self.s[self.i] == "[":
                nxt = re.compile(r"\[=*\[").match(self.s, self.i)
                if not nxt:
                    self.i += 1
                    key = self.value()
                    self.expect("]")
                    self.expect("=")
            else:
                m = _NAME.match(self.s, self.i)
                if m:
                    j = m.end()
                    k = _WS.match(self.s, j)
                    if k:
                        j = k.end()
                    if j < self.n and self.s[j] == "=" and self.s[j + 1] != "=":
                        key = m.group(0)
                        self.i = j + 1
            if key is None:
                self.i = save
                arr.append(self.value())
            else:
                out[key if isinstance(key, str) else str(key)] = self.value()
        if arr and not out:
            return arr
        if arr:
            out["__array__"] = arr
        return out


def parse_table(src: str):
    """Parse the first `name = { ... }` assignment in the source."""
    m = re.search(r"(?m)^[ \t]*(?:local\s+)?([A-Za-z_][\w.]*)\s*=\s*(?=\{)", src)
    if not m:
        p = _P(src)
        p.ws()
        if p.peek() == "{":
            return "<anonymous>", p.table()
        raise LuaParseError("no table assignment found")
    p = _P(src)
    p.i = m.end()
    return m.group(1), p.table()


def parse_all_tables(src: str):
    """Parse every top-level `name = { ... }` assignment."""
    out = []
    for m in re.finditer(r"(?m)^[ \t]*(?:local\s+)?([A-Za-z_][\w.]*)\s*=\s*(?=\{)",
                         src):
        p = _P(src)
        p.i = m.end()
        try:
            out.append((m.group(1), p.table()))
        except LuaParseError:
            continue
    return out


def jsonable(obj):
    if isinstance(obj, OpaqueCall):
        return obj.to_json()
    if isinstance(obj, dict):
        return {k: jsonable(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [jsonable(v) for v in obj]
    return obj


def read(path: str):
    with open(path, "rb") as fh:
        raw = fh.read()
    if raw[:3] == b"\xef\xbb\xbf":
        raw = raw[3:]
    for enc in ("utf-8", "cp1252", "latin-1"):
        try:
            return raw.decode(enc)
        except UnicodeDecodeError:
            continue
    return raw.decode("utf-8", "replace")
