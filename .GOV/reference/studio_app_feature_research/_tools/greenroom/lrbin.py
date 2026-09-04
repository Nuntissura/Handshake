"""
lrbin.py - constant-pool miner for Lightroom Classic's .lrmodule / .lrplugin
PE files.

FINDING THAT MAKES THIS WORK: those PE files embed Lua 5.1 bytecode dumps.
A Lua 5.1 dump writes each string constant as

    0x04  <size:uint32 little-endian>  <size bytes incl. trailing NUL>

and each number constant as

    0x03  <IEEE754 double, little-endian>

So the constant pool can be recovered exactly - including strings containing
punctuation, newlines and embedded Lua source fragments - without executing
anything and without a Lua runtime. That is far more precise than a generic
"printable ASCII run" scan, which splits strings on any non-ASCII byte and
cannot tell a real constant from padding.

This module only READS. It never writes to the install.
"""
from __future__ import annotations

import re
import struct

_PRINTABLE = re.compile(rb"^[\x09\x0a\x0d\x20-\x7e]*$")


def iter_lua_strings(blob: bytes, min_len: int = 1, max_len: int = 1 << 20):
    """Yield (offset, text) for every well-formed Lua 5.1 string constant."""
    n = len(blob)
    i = 0
    find = blob.find
    while True:
        i = find(b"\x04", i)
        if i < 0 or i + 5 > n:
            return
        size = struct.unpack_from("<I", blob, i + 1)[0]
        if size == 0 or size > max_len or i + 5 + size > n:
            i += 1
            continue
        raw = blob[i + 5:i + 5 + size]
        if raw[-1:] != b"\x00":
            i += 1
            continue
        body = raw[:-1]
        if len(body) < min_len:
            i += 1
            continue
        if not _PRINTABLE.match(body):
            i += 1
            continue
        yield i, body.decode("ascii", "replace")
        i += 5 + size


def iter_lua_numbers(blob: bytes):
    """Yield (offset, float) for every Lua 5.1 number constant."""
    n = len(blob)
    i = 0
    while True:
        i = blob.find(b"\x03", i)
        if i < 0 or i + 9 > n:
            return
        val = struct.unpack_from("<d", blob, i + 1)[0]
        if val == val and abs(val) < 1e18:  # not NaN, plausible magnitude
            yield i, val
        i += 1


ZSTR_RE = re.compile(r"^\$\$\$/([^=]{1,300})=(.*)$", re.S)
IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
DOTTED_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)+$")
LUAFILE_RE = re.compile(r"^[\w./\\-]+\.lua$")
REVDNS_RE = re.compile(r"^(com|org|net|io)\.[\w.-]+$")
ASSIGN_RE = re.compile(
    r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+?),?\s*$")


def classify(text: str) -> str:
    if text.startswith("$$$/"):
        return "zstr_localization_key"
    if LUAFILE_RE.match(text):
        return "lua_chunk_name"
    if REVDNS_RE.match(text):
        return "reverse_dns_id"
    if "\n" in text and "=" in text:
        return "lua_source_fragment"
    if DOTTED_RE.match(text):
        return "dotted_path"
    if IDENT_RE.match(text):
        return "identifier"
    if text.startswith("http://") or text.startswith("https://"):
        return "url"
    if re.match(r"^[\w ./\\:-]+\.(png|jpg|jpeg|gif|svg|icns|ico)$", text, re.I):
        return "asset_path"
    return "other"


def mine(path: str, min_len: int = 2):
    """Return dict of classification -> list of unique strings, in order."""
    with open(path, "rb") as fh:
        blob = fh.read()
    seen = set()
    out = {}
    order = []
    for _off, text in iter_lua_strings(blob, min_len=min_len):
        if text in seen:
            continue
        seen.add(text)
        order.append(text)
        out.setdefault(classify(text), []).append(text)
    return out, order, len(blob)
