#!/usr/bin/env python3
"""Handshake Studio green room: per-type-code structural map of InDesign .idrc resources.

OFFLINE ONLY. This tool reads installed files. It never launches InDesign or any Adobe
executable, never touches COM/ExtendScript, and never writes into the install tree.

WHAT THIS IS
------------
InDesign 2026 ships ~29k `.idrc` resource files grouped into directories named
`idrc_<CODE>` where CODE is a 4-character resource type code (56 distinct codes at the
app root and under each plug-in's "(X Resources)" folder). A sibling tool
(`indesign-idrc-scan.py`) did a string-level survey; `indesign-sce2-parse.py` fully
parsed the SCE2 scripting model. This tool answers the next question for EVERY code:
what does the container actually hold, is its binary layout decodable, and what could a
Rust reimplementation extract from it.

For each code it produces:
  * real file/byte/plug-in/id-range counts (a file count is never reported as a record count)
  * a decoder result when the record grammar was actually verified byte-exactly
  * explicit structure probes with pass/fail for codes that were not decoded
  * magic detection (PNG / SVG / XML / ICC / TSMP / VIEW 0x3333 / zlib / ABC / SWF)
  * readable string and $$$/ZString samples
  * the first 32 header bytes of the largest file

VERIFICATION RULE
-----------------
A decoder is only marked "parsed" when the walk consumes the file byte-exactly (and,
where the container declares a count, when the parsed record count equals the declared
count) for a stated fraction of the code's files. Everything else is "partially_parsed"
or "not_decoded", with the failed hypothesis recorded in `structure_probes`.

GRAMMARS REUSED FROM EARLIER PASSES (not re-derived here, cited as prior work)
-----------------------------------------------------------------------------
SCE2  scripting object model  -- indesign-sce2-parse.py
uetb  error-code catalog      -- u16 count, {u32 code, u8 len, ASCII name}
PMST  localized string tables -- u32 locale, u32 8, u32 count, {u16 klen,key,u16 vlen,utf8}
VIEW  widget/dialog trees     -- nodes begin 0x33 0x33 + u32 boss class id
LOCR  locale->resource index  -- 'TSMP', u32 count, {u16 a, u16 locale, u32 resource_id}
CLST/FACT/PVER/SCML/MENR/ACTD -- see the per-code evidence field

PARTIALLY CONFIRMED (entry grammar verified, container framing NOT reversed)
---------------------------------------------------------------------------
SCML / SCMA  10-byte file header {u16,u32,u32}; block header {u32 block_id, u32 a, u32 b,
             u16 entry_count}; entry {u16 value_type, u16 key, value} with value sizes
             2->2B, 3->4B, 7->4B signed int, 9->8B double, keys sequential from 1. What
             separates one block from the next is NOT reversed: value_type 0x8001 turns up
             where the next block header should start. The three-rung probe ladder in the
             output records every trailer size tested and how far each got. These tables
             hold DEFAULT PREFERENCE VALUES (the PDF Resources instance = PDF export
             defaults), which makes finishing them high value.

GRAMMARS FIRST DERIVED BY THIS TOOL (2026-09-04)
------------------------------------------------
PLUG  plug-in prerequisite table   FTTB  file-type/OSType/MIME table
CNTL  Win32 control class + style  Colr  UI colour palette (RGB doubles)
EVE_  Adobe Eve layout DSL source  ADAM  Adobe Adam property-model source
rulr  ruler subdivision table      HOTC  cursor hotspot (x,y)
TIPS  tooltip table                ACCF  accessibility caption table
TOCL  panel ordering record        KLST/PLST  menu-path registration
CLAS/ACLS/ISui  boss class + interface/implementation pairs
CTAG/ITAG  4CC-tagged class-id lists
IALS  interface->implementation pair list      ILST  flat u32 id list
ILTP  12-byte id triple list                   VRLS  12-byte version list
SERV  service registry             petb  presentable-error message table
"""
from __future__ import annotations

import argparse
import binascii
import collections
import datetime as dt
import json
import os
import re
import struct
import sys

# --------------------------------------------------------------------------------------
# constants
# --------------------------------------------------------------------------------------

ASCII_RUN = re.compile(rb"[\x20-\x7e]{4,}")
UTF16_RUN = re.compile(rb"(?:[\x20-\x7e]\x00){4,}")
ZSTRING = re.compile(r"\$\$\$/[A-Za-z0-9_./\-]+")

PNG_SIG = b"\x89PNG\r\n\x1a\n"
MAGICS = {
    "png": PNG_SIG,
    "svg": b"<svg",
    "xml_decl": b"<?xml",
    "html": b"<html",
    "tsmp": b"TSMP",
    "zlib_78_9c": b"\x78\x9c",
    "gzip": b"\x1f\x8b",
    "icc_acsp": b"acsp",
    "swf_cws": b"CWS",
    "utf8_bom": b"\xef\xbb\xbf",
}

# Maximum payload bytes read per code for the deep pass (keeps runtime sane on 37 MB codes).
DEFAULT_CAP = 12 * 1024 * 1024
# Minimum number of files sampled per code (or all files when the code has fewer).
MIN_SAMPLE = 12

# Codes whose grammar is confirmed only in part. Presence here forces decoded="partially_parsed"
# and supplies the record_structure text, because no full decoder exists.
PARTIAL_RECORD_STRUCTURE = {
    "SCML": (
        "CONFIRMED: 10-byte file header {u16, u32, u32}; then a block header at offset 10 of "
        "{u32 block_id, u32 a, u32 b, u16 entry_count}; then entry_count entries, each "
        "{u16 value_type, u16 key, value} with value sizes 2->2 bytes, 3->4 bytes, "
        "7->4-byte signed int, 9->8-byte IEEE double, and keys running sequentially from 1. "
        "NOT REVERSED: what separates one block from the next. Immediately after the declared "
        "entry list, value_type 0x8001 (32769) appears where the next block header should be, so "
        "the inter-block framing is wrong under every trailer size tested (0, 2, 4, 6, 8, 10, 12, "
        "16 bytes). Consequently the file-level walk succeeds for only a handful of files; the "
        "per-block entry grammar above is the verified part."),
    "SCMA": None,  # filled in below: identical grammar
    "VIEW": (
        "u32 root id at offset 0, then a node stream in which each node begins with the 2-byte "
        "magic 0x33 0x33 followed by a u32 boss class id. Node payload layout is per-class and is "
        "not reversed here; see the cross_reference for the node-level extraction."),
}
PARTIAL_RECORD_STRUCTURE["SCMA"] = PARTIAL_RECORD_STRUCTURE["SCML"].replace(
    "10-byte file header", "10-byte file header (same container as SCML)")

# Sibling green-room artifacts that decode a code more deeply than this survey does. Totals are
# read from the artifact at run time rather than restated from memory, so the numbers in this
# file cannot drift from the artifact they cite.
CROSS_REFS = {
    "SCE2": ("indesign-sce2-parse.py", "indesign_dom_full.json",
             ["sce2_resource_files_found", "sce2_resource_files_parsed", "sce2_resource_files_failed",
              "suites", "classes", "properties", "methods", "method_parameters", "enumerations",
              "enumerators", "typedefs", "member_tables", "class_property_edges",
              "class_method_edges", "resync_bytes_skipped"]),
    "VIEW": ("indesign-view-dialogs.py", "indesign_dialogs.json",
             ["view_resource_files", "view_bytes", "nodes_found", "distinct_class_ids_confirmed",
              "distinct_dialogs_or_panels", "labels_extracted", "dialog_labels_resolved_from_pmst",
              "dialog_labels_total", "resources_with_source_path"]),
    "uetb": ("indesign-error-catalog.py", "indesign_error_catalog.json",
             ["uetb_resource_files", "petb_resource_files", "uetb_files_fully_consumed",
              "error_entries_parsed", "distinct_error_codes", "errors_with_english_message"]),
    "petb": ("indesign-error-catalog.py", "indesign_error_catalog.json",
             ["uetb_resource_files", "petb_resource_files", "uetb_files_fully_consumed",
              "error_entries_parsed", "distinct_error_codes", "errors_with_english_message"]),
}

CROSS_REF_NOTES = {
    "SCE2": "Fully decoded: every SCE2 file parsed with zero failures. The residual "
            "resync_bytes_skipped are variable-length method-parameter trailers, which the "
            "sibling tool classifies rather than invents.",
    "VIEW": "Node-level extraction is complete; still missing are the class-id -> widget-name map "
            "(the install ships no such table) and the per-class node payload layout.",
    "uetb": "All uetb files plus the single petb file fully consumed. Tab, CR and LF are legal "
            "inside a uetb symbol because the InDesign core table stores multi-line English "
            "messages in that field.",
    "petb": "Parsed together with uetb by the same sibling tool.",
}

CONTENT_CLASSES = {
    "scripting_model", "error_catalog", "localized_strings", "ui_layout", "locale_index",
    "class_registry", "settings_table", "raster_asset", "vector_asset", "version_info",
    "keyboard_shortcuts", "unknown_numeric", "unknown_mixed", "other",
}


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


# --------------------------------------------------------------------------------------
# tiny binary reader helpers
# --------------------------------------------------------------------------------------

class Eof(Exception):
    pass


def u8(d, o):
    if o + 1 > len(d):
        raise Eof()
    return d[o]


def u16(d, o):
    if o + 2 > len(d):
        raise Eof()
    return struct.unpack_from("<H", d, o)[0]


def u32(d, o):
    if o + 4 > len(d):
        raise Eof()
    return struct.unpack_from("<I", d, o)[0]


def f64(d, o):
    if o + 8 > len(d):
        raise Eof()
    return struct.unpack_from("<d", d, o)[0]


def fourcc_rev(d, o):
    if o + 4 > len(d):
        raise Eof()
    return d[o:o + 4][::-1].decode("latin-1")


def pstr16(d, o):
    n = u16(d, o)
    if o + 2 + n > len(d):
        raise Eof()
    s = d[o + 2:o + 2 + n]
    if not all(32 <= b < 127 for b in s):
        raise Eof()
    return s.decode("latin-1"), o + 2 + n


# --------------------------------------------------------------------------------------
# decoders: each returns (consumed_all: bool, record_count: int, note: str)
# A decoder MUST raise Eof / return False rather than guess.
# --------------------------------------------------------------------------------------

def dec_plug(d):
    """PLUG: u16 format marker (0xfffe in 96/98 files, 0x0101 in the other two), u32 count,
    count*{u32 plugin_id, pstr16 plugin_name, u32 required_major, u32 required_minor}."""
    marker = u16(d, 0)
    if marker not in (0xFFFE, 0x0101):
        return False, 0, "unknown format marker 0x%04x" % marker
    n = u32(d, 2)
    o = 6
    out = []
    for _ in range(n):
        pid = u32(d, o)
        name, o = pstr16(d, o + 4)
        maj, mino = u32(d, o), u32(d, o + 4)
        o += 8
        out.append((pid, name, maj, mino))
    return o == len(d), n, ("deps=%d ver=%d.%d first=%s" % (n, out[0][2], out[0][3], out[0][1])) if out else "empty"


def dec_fttb(d):
    """FTTB: u16 block_count, blocks{u32 group_id, u16 n, n*{4cc type, 4cc creator, pstr16 ext, pstr16 mime}}."""
    nb = u16(d, 0)
    o = 2
    blocks = 0
    recs = 0
    sample = None
    while o < len(d):
        gid = u32(d, o)
        cnt = u16(d, o + 4)
        o += 6
        for _ in range(cnt):
            t = fourcc_rev(d, o)
            c = fourcc_rev(d, o + 4)
            o += 8
            ext, o = pstr16(d, o)
            mime, o = pstr16(d, o)
            recs += 1
            if sample is None:
                sample = (hex(gid), t, c, ext, mime)
        blocks += 1
    return (o == len(d) and blocks == nb), recs, "blocks=%d/%d records=%d first=%s" % (blocks, nb, recs, sample)


def dec_cntl(d):
    """CNTL: u32 count, count*{pstr16 win32_class, u32 dwStyle, u32 dwExStyle}."""
    n = u32(d, 0)
    o = 4
    out = []
    for _ in range(n):
        name, o = pstr16(d, o)
        style, ex = u32(d, o), u32(d, o + 4)
        o += 8
        out.append((name, "0x%08x" % style, "0x%08x" % ex))
    return o == len(d), n, "controls=%s" % (out[:3],)


def dec_colr(d):
    """Colr: u32 count, count*{double r, double g, double b} in 0..1."""
    n = u32(d, 0)
    if 4 + n * 24 != len(d):
        return False, 0, "count %d does not fit 24-byte RGB triples in %d bytes" % (n, len(d))
    cols = []
    for i in range(n):
        r, g, b = struct.unpack_from("<3d", d, 4 + i * 24)
        if not all(-0.001 <= v <= 1.001 for v in (r, g, b)):
            return False, n, "value out of 0..1 at index %d" % i
        cols.append((round(r, 6), round(g, 6), round(b, 6)))
    return True, n, "rgb_doubles=%d first=%s" % (n, cols[:4])


def dec_eve(d):
    """EVE_ / ADAM: u16 length, ASCII source text of the Adobe Eve / Adam declarative UI language."""
    n = u16(d, 0)
    if 2 + n != len(d):
        return False, 0, "u16 length %d != filesize-2 (%d)" % (n, len(d) - 2)
    txt = d[2:].decode("latin-1")
    head = txt.strip().splitlines()[0][:90] if txt.strip() else ""
    return True, 1, "text_bytes=%d first_line=%r" % (n, head)


def dec_rulr(d):
    """rulr: pstr16 font, double font_size, u32 a, u32 b, u32 n, n*{double zoom, double spacing,
    u32 k, k*{u16 divisions, u16 unit}}."""
    font, o = pstr16(d, 0)
    size = f64(d, o)
    o += 8
    o += 8  # two u32 (id / flags)
    n = u32(d, o)
    o += 4
    rows = []
    for _ in range(n):
        zoom = f64(d, o)
        spacing = f64(d, o + 8)
        o += 16
        k = u32(d, o)
        o += 4
        subs = []
        for _ in range(k):
            subs.append((u16(d, o), u16(d, o + 2)))
            o += 4
        rows.append((round(zoom, 4), round(spacing, 3), subs))
    return o == len(d), n, "font=%s size=%g steps=%d first=%s" % (font, size, n, rows[:2])


def dec_hotc(d):
    """HOTC: {u16 hotspot_x, u16 hotspot_y} -- cursor hotspot for the same-id PNGC/SVGC image."""
    if len(d) != 4:
        return False, 0, "not 4 bytes"
    x, y = struct.unpack("<HH", d)
    if x > 256 or y > 256:
        return False, 1, "hotspot out of plausible range (%d,%d)" % (x, y)
    return True, 1, "hotspot=(%d,%d)" % (x, y)


def dec_tips(d):
    """TIPS: u32 count, count*{u32 widget_id, pstr16 tip_key_or_text}."""
    n = u32(d, 0)
    o = 4
    out = []
    for _ in range(n):
        wid = u32(d, o)
        s, o = pstr16(d, o + 4)
        out.append((hex(wid), s))
    return o == len(d), n, "tips=%d first=%s" % (n, out[:3])


def dec_accf(d):
    """ACCF: u32 count, count*{u32 widget_id, u32 flag, pstr16 caption}."""
    n = u32(d, 0)
    o = 4
    out = []
    for _ in range(n):
        wid = u32(d, o)
        flag = u32(d, o + 4)
        s, o = pstr16(d, o + 8)
        out.append((hex(wid), flag, s))
    return o == len(d), n, "captions=%d first=%s" % (n, out[:3])


def dec_tocl(d):
    """TOCL: {u32 widget_id, u16 f1, pstr16 name, u32 related_id, u16 f2, u32 z, double order}."""
    wid = u32(d, 0)
    f1 = u16(d, 4)
    name, o = pstr16(d, 6)
    rel = u32(d, o)
    f2 = u16(d, o + 4)
    z = u32(d, o + 6)
    order = f64(d, o + 10)
    o += 18
    return o == len(d), 1, "id=%s name=%r related=%s order=%g flags=(%d,%d,%d)" % (
        hex(wid), name, hex(rel), order, f1, f2, z)


def _menu_records(d, o, n):
    out = []
    for _ in range(n):
        a = u32(d, o)
        flags = u32(d, o + 4)
        b = u32(d, o + 8)
        o += 12
        path, o = pstr16(d, o)
        parent, o = pstr16(d, o)
        pos = f64(d, o)
        o += 8
        c = u32(d, o)
        tail16 = u16(d, o + 4)
        if o + 10 > len(d):
            raise Eof()
        tag = d[o + 6:o + 10][::-1].decode("latin-1")
        o += 10
        out.append((hex(a), hex(b), path, parent, pos, hex(c), tail16, tag))
    return out, o


def dec_plst(d):
    """PLST: u32 count, count*{u32 id_a, u32 flags, u32 id_b, pstr16 menu_path,
    pstr16 parent_menu_path, double menu_position, u32 id_c, u16 tail, 4cc trailing tag
    (byte-reversed; 'CPNS' in most files, 'pgsp' or 0x00000000 in a few)}."""
    n = u32(d, 0)
    out, o = _menu_records(d, 4, n)
    return o == len(d), n, "entries=%d first=%s" % (n, out[:2])


def dec_klst(d):
    """KLST: u16 version, u16 count, u16 pad, count*(the PLST menu-registration record)."""
    ver = u16(d, 0)
    n = u16(d, 2)
    out, o = _menu_records(d, 6, n)
    return o == len(d), n, "version=%d entries=%d first=%s" % (ver, n, out[:2])


def dec_menr(d):
    """MENR: u32 count, count*{u32 action_id, pstr16 menu_path, double menu_position,
    u32, u32, u16}."""
    n = u32(d, 0)
    o = 4
    out = []
    for _ in range(n):
        aid = u32(d, o)
        path, o = pstr16(d, o + 4)
        pos = f64(d, o)
        o += 18
        if o > len(d):
            raise Eof()
        if len(out) < 3:
            out.append((hex(aid), path, pos))
    return o == len(d), n, "menu_items=%d first=%s" % (n, out)


def dec_actd(d):
    """ACTD: u32 count, count*{u32 action_id, u32 command_id, pstr16 action_name,
    pstr16 shortcut_area, u16, u32, u32, u16}."""
    n = u32(d, 0)
    o = 4
    out = []
    for _ in range(n):
        aid = u32(d, o)
        cid = u32(d, o + 4)
        name, o = pstr16(d, o + 8)
        area, o = pstr16(d, o)
        o += 12
        if o > len(d):
            raise Eof()
        if len(out) < 3:
            out.append((hex(aid), hex(cid), name, area))
    return o == len(d), n, "actions=%d first=%s" % (n, out)


def dec_guid(d):
    """GUID: {u32 release_index, u32 variant, pstr16 release_name, 3 * guid[16], u32, u32}."""
    idx, var = u32(d, 0), u32(d, 4)
    name, o = pstr16(d, 8)
    if o + 48 + 8 != len(d):
        return False, 0, "3 GUIDs + 8 trailing bytes do not fit (%d left)" % (len(d) - o)
    guids = [binascii.hexlify(d[o + i * 16:o + (i + 1) * 16]).decode() for i in range(3)]
    return True, 3, "release=%r index=%d variant=%d guids=%s" % (name, idx, var, guids)


def dec_pver(d):
    """PVER: a numeric prefix (u32 words, but with a 2- or 4-byte variable field before the tail
    that is NOT decoded) followed by u32 then pstr16 ASCII version string, e.g. '21.5.1.73'.
    Only the trailing version record is verified byte-exactly; the numeric prefix length is
    reported, not interpreted."""
    ver = None
    voff = None
    for o in range(max(0, len(d) - 40), max(0, len(d) - 2)):
        ln = u16(d, o)
        if 5 <= ln <= 32 and o + 2 + ln == len(d):
            s = d[o + 2:]
            if all(48 <= b <= 57 or b == 46 for b in s):
                ver = s.decode("latin-1")
                voff = o
                break
    if ver is None:
        return False, 0, "no trailing u16-length-prefixed dotted version string"
    pre = voff - 4
    return True, 1, "numeric_prefix_bytes=%d (u32-aligned=%s) version_string=%r" % (
        pre, pre % 4 == 0, ver)


def _pairs_after_two_u32(d):
    a = u32(d, 0)
    n = u32(d, 4)
    if 8 + n * 8 != len(d):
        raise Eof()
    prs = [(hex(u32(d, 8 + i * 8)), hex(u32(d, 12 + i * 8))) for i in range(n)]
    return a, n, prs


def dec_clas(d):
    """CLAS / ACLS / ISui: {u32 class_id, u32 base_id_or_owner, u32 pair_count,
    pair_count*{u32 interface_id, u32 implementation_id}} -- but observed as
    {u32 id_a, u32 count, count*{u32,u32}} for ISui. Both shapes are the same 8-byte-pair list;
    the discriminator is whether field 2 equals the pair count."""
    # shape A: id, base, count, pairs
    try:
        cid, base, n = u32(d, 0), u32(d, 4), u32(d, 8)
        if 12 + n * 8 == len(d):
            prs = [(hex(u32(d, 12 + i * 8)), hex(u32(d, 16 + i * 8))) for i in range(n)]
            return True, n, "shape=id+base+count class=%s base=%s pairs=%d first=%s" % (
                hex(cid), hex(base), n, prs[:3])
    except Eof:
        pass
    # shape B: id, count, pairs
    a, n, prs = _pairs_after_two_u32(d)
    return True, n, "shape=id+count id=%s pairs=%d first=%s" % (hex(a), n, prs[:3])


def dec_isui(d):
    """ISui: {u32 owner_id, u32 pair_count, pair_count*{u32, u32}}."""
    a, n, prs = _pairs_after_two_u32(d)
    return True, n, "owner=%s pairs=%d first=%s" % (hex(a), n, prs[:3])


def dec_clst(d):
    """CLST: {u32 record_count, record_count*{u16 kind (1, 2 or 3), u32 class_id, u32 base_id,
    u32 pair_count, pair_count*{u32 interface_id, u32 implementation_id}}}.
    CORRECTION to prior work, which described CLST as '12-byte-ish records of u16/u32 ids':
    the records are variable length and carry an explicit interface/implementation pair list."""
    cnt = u32(d, 0)
    o = 4
    recs = 0
    kinds = collections.Counter()
    first = None
    while o < len(d):
        if o + 14 > len(d):
            raise Eof()
        k = u16(d, o)
        cid, base, m = u32(d, o + 2), u32(d, o + 6), u32(d, o + 10)
        if m > 100000 or o + 14 + m * 8 > len(d):
            raise Eof()
        if first is None:
            first = (k, hex(cid), hex(base), m)
        kinds[k] += 1
        o += 14 + m * 8
        recs += 1
    return (o == len(d) and recs == cnt), recs, "records=%d/%d kinds=%s first=%s" % (
        recs, cnt, dict(kinds), first)


def dec_ials(d):
    """IALS: {u32 pair_count, pair_count*{u32 interface_id, u32 implementation_id}}."""
    n = u32(d, 0)
    if 4 + n * 8 != len(d):
        return False, 0, "count %d does not fit 8-byte pairs in %d bytes" % (n, len(d))
    prs = [(hex(u32(d, 4 + i * 8)), hex(u32(d, 8 + i * 8))) for i in range(n)]
    return True, n, "pairs=%d first=%s" % (n, prs[:3])


def dec_ilst(d):
    """ILST: {u32 count, count*u32 id}."""
    n = u32(d, 0)
    if 4 + n * 4 != len(d):
        return False, 0, "count %d does not fit u32 ids in %d bytes" % (n, len(d))
    return True, n, "ids=%d first=%s" % (n, [hex(u32(d, 4 + i * 4)) for i in range(min(n, 4))])


def dec_iltp(d):
    """ILTP: {u32 count, count*{u32 a, u32 b, u32 c}}."""
    n = u32(d, 0)
    if 4 + n * 12 != len(d):
        return False, 0, "count %d does not fit 12-byte triples in %d bytes" % (n, len(d))
    tri = [(hex(u32(d, 4 + i * 12)), hex(u32(d, 8 + i * 12)), hex(u32(d, 12 + i * 12)))
           for i in range(min(n, 3))]
    return True, n, "triples=%d first=%s" % (n, tri)


def dec_vrls(d):
    """VRLS: {u16 count, count*12-byte record observed as (u16 0, u32 A, u32 B, u16 0)}."""
    n = u16(d, 0)
    if 2 + n * 12 != len(d):
        return False, 0, "count %d does not fit 12-byte records in %d bytes" % (n, len(d))
    recs = []
    for i in range(min(n, 4)):
        o = 2 + i * 12
        recs.append((u16(d, o), u32(d, o + 2), u32(d, o + 6), u16(d, o + 10)))
    return True, n, "records=%d first=%s" % (n, recs)


def dec_ctag(d):
    """CTAG / ITAG: {u32 tag_or_owner_id, u32 count, count*u32 class_id}."""
    a = u32(d, 0)
    n = u32(d, 4)
    if 8 + n * 4 != len(d):
        return False, 0, "count %d does not fit u32 ids in %d bytes" % (n, len(d))
    return True, n, "owner=%s ids=%d first=%s" % (hex(a), n, [hex(u32(d, 8 + i * 4)) for i in range(min(n, 4))])


def dec_serv(d):
    """SERV: {u32 service_id, u32 provider_id, pstr16 name, u32 flags, u16 tail}."""
    sid, pid = u32(d, 0), u32(d, 4)
    name, o = pstr16(d, 8)
    flags = u32(d, o)
    tail = u16(d, o + 4)
    o += 6
    return o == len(d), 1, "service=%s provider=%s name=%r flags=%d tail=%d" % (
        hex(sid), hex(pid), name, flags, tail)


def _printable(b):
    return 32 <= b < 127 or b in (9, 10, 13)


def dec_petb(d):
    """petb: {u32 count, u16 pad, count*{u32 error_code, u8 len, ASCII message}}."""
    n = u32(d, 0)
    o = 6
    out = []
    for _ in range(n):
        code = u32(d, o)
        ln = u8(d, o + 4)
        s = d[o + 5:o + 5 + ln]
        if len(s) != ln or not all(_printable(b) for b in s):
            return False, 0, "bad pascal string at %d" % o
        o += 5 + ln
        out.append(("0x%08x" % code, s.decode("latin-1")))
    return o == len(d), n, "messages=%d first=%s" % (n, out[:3])


def dec_uetb(d):
    """uetb (prior work, re-verified): {u16 count, count*{u32 error_code, u8 len, ASCII symbol}}.
    The error code's high 16 bits are the plug-in/service id and the low 16 bits the ordinal."""
    n = u16(d, 0)
    o = 2
    out = []
    for _ in range(n):
        code = u32(d, o)
        ln = u8(d, o + 4)
        s = d[o + 5:o + 5 + ln]
        if len(s) != ln or not all(_printable(b) for b in s):
            return False, 0, "bad pascal string at %d" % o
        o += 5 + ln
        out.append(("0x%08x" % code, s.decode("latin-1")))
    return o == len(d), n, "errors=%d first=%s" % (n, out[:2])


def dec_pmst(d):
    """PMST (prior work): {u32 locale, u32 8, u32 count, count*{pstr16 key, pstr16 utf8 value}}."""
    loc = u32(d, 0)
    n = u32(d, 8)
    o = 12
    out = []
    for _ in range(n):
        kl = u16(d, o)
        key = d[o + 2:o + 2 + kl]
        o += 2 + kl
        vl = u16(d, o)
        val = d[o + 2:o + 2 + vl]
        o += 2 + vl
        if len(key) != kl or len(val) != vl:
            return False, 0, "string overrun at %d" % o
        if len(out) < 2:
            out.append((key.decode("latin-1")[:40], val.decode("utf-8", "replace")[:40]))
    return o == len(d), n, "locale=%d entries=%d first=%s" % (loc, n, out)


def dec_locr(d):
    """LOCR: {4cc target_type_code stored byte-reversed, u32 count,
    count*{u16 a, u16 locale, u32 resource_id}}. The leading 4CC is NOT always 'TSMP': it is
    the type code of the resource family the index points at, byte-reversed on disk -- observed
    WEIV=VIEW, TSMP=PMST, RNEM=MENR, DTCA=ACTD, TSLK=KLST, FDLT=TLDF, _EVE=EVE_, TSLP=PLST."""
    tgt = fourcc_rev(d, 0)
    if not all(32 <= b < 127 for b in d[:4]):
        return False, 0, "leading 4 bytes are not a printable 4CC"
    n = u32(d, 4)
    if 8 + n * 8 != len(d):
        return False, 0, "count %d does not fit 8-byte records in %d bytes" % (n, len(d))
    recs = [(u16(d, 8 + i * 8), u16(d, 10 + i * 8), u32(d, 12 + i * 8)) for i in range(min(n, 3))]
    return True, n, "target_code=%s entries=%d first=%s" % (tgt, n, recs)


def dec_sce2(d):
    """SCE2 (prior work -- fully parsed by indesign-sce2-parse.py, header re-verified here):
    u32 section_count, section_count*16 bytes, u32 element_count, then elements. Element =
    u16 kind (1 suite, 2 class, 3 method, 4 property, 5 enumeration, 11 typedef) + 46-byte
    header + byte-reversed 4CC tag + pstr16 name + pstr16 description + kind-specific body.
    Short-header kinds 7/8/9 = u16 kind + 2*u32 0x7fffffff + u32 id + u32 count + entries."""
    sc = u32(d, 0)
    if sc > 4096:
        return False, 0, "implausible section count %d" % sc
    o = 4 + sc * 16
    ec = u32(d, o)
    o += 4
    if ec > 100000:
        return False, 0, "implausible element count %d" % ec
    if ec == 0:
        return o == len(d), 0, "sections=%d elements=0 (empty scripting resource)" % sc
    kind = u16(d, o)
    if not 1 <= kind <= 12:
        return False, 0, "first element kind %d outside the known 1..12 range" % kind
    name = None
    if kind in (1, 2, 3, 4, 5, 11) and o + 50 < len(d):
        try:
            name, _ = pstr16(d, o + 50)
        except Eof:
            name = None
    return True, ec, ("sections=%d elements=%d first_kind=%d first_name=%r "
                      "(element bodies parsed by indesign-sce2-parse.py, not re-walked here)"
                      % (sc, ec, kind, name))


def dec_fact(d):
    """FACT (prior work): flat array of u32 class ids, no header."""
    if len(d) % 4:
        return False, 0, "size %d not a multiple of 4" % len(d)
    n = len(d) // 4
    return True, n, "u32_ids=%d first=%s" % (n, [hex(u32(d, i * 4)) for i in range(min(n, 4))])


def dec_raw_png(d):
    """PNGA/PNGC/PNGD/PNGK/PNGR: raw PNG payload, no wrapper. Reports IHDR dimensions.
    A handful of shipped files carry the CRLF-mangled signature 89 50 4E 47 0D 0D 0A 1A 0D 0A
    (an LF->CRLF text-mode conversion applied to a binary file); those are reported separately."""
    off = 0
    mangled = False
    if d[:8] != PNG_SIG:
        if d[:10] == b"\x89PNG\r\r\n\x1a\r\n":
            off, mangled = 2, True
        else:
            return False, 0, "no PNG signature at offset 0"
    if d[12 + off:16 + off] != b"IHDR":
        return False, 0, "PNG signature but no IHDR at offset %d" % (12 + off)
    w, h = struct.unpack_from(">II", d, 16 + off)
    depth, ctype = d[24 + off], d[25 + off]
    return True, 1, "wrapper_bytes=0 png=%dx%d depth=%d colour_type=%d%s" % (
        w, h, depth, ctype, " CRLF-MANGLED-SIGNATURE" if mangled else "")


def dec_raw_svg(d):
    """SVGA/SVGC/SVGD: raw SVG/XML text, no wrapper. Reports the offset of the <svg element."""
    i = d.find(b"<svg")
    if i < 0:
        return False, 0, "no <svg element"
    pre = d[:i].strip()
    if pre and not pre.startswith(b"<?xml"):
        return False, 0, "unexpected %d bytes before <svg" % i
    head = d[i:i + 140].decode("latin-1", "replace").replace("\n", " ")
    return True, 1, "wrapper_bytes=%d (xml prolog only) head=%r" % (i, head)


def dec_crct(d):
    """CRCT: 8-byte all-zero registration marker, one per plug-in."""
    if len(d) == 8 and d == b"\x00" * 8:
        return True, 0, "8 zero bytes (marker, carries no records)"
    return False, 0, "not 8 zero bytes"


def dec_tldf(d):
    """TLDF / TTLD: fixed 44-byte tool-definition record.
    Observed field split: u32 tool_id, u32 base_id(0x0001d010 in every TLDF sampled),
    u32 related_id, u32, u32, u32, u32, u16, u32, u32, u32, u16."""
    if len(d) != 44:
        return False, 0, "not 44 bytes"
    w = struct.unpack_from("<6I", d, 0)
    tail = struct.unpack_from("<IHIIIH", d, 24)
    return True, 1, "u32[0:6]=%s tail=%s" % ([hex(x) for x in w], [hex(x) for x in tail])


def dec_flat_u32(d):
    if len(d) % 4 or not d:
        return False, 0, "size %d not a positive multiple of 4" % len(d)
    n = len(d) // 4
    return True, n, "u32_words=%d values=%s" % (n, [hex(u32(d, i * 4)) for i in range(min(n, 6))])


DECODERS = {
    "PLUG": dec_plug, "FTTB": dec_fttb, "CNTL": dec_cntl, "Colr": dec_colr,
    "EVE_": dec_eve, "ADAM": dec_eve, "rulr": dec_rulr, "HOTC": dec_hotc,
    "TIPS": dec_tips, "ACCF": dec_accf, "TOCL": dec_tocl, "PLST": dec_plst,
    "KLST": dec_klst, "CLAS": dec_clas, "ACLS": dec_clas, "ISui": dec_isui,
    "IALS": dec_ials, "ILST": dec_ilst, "ILTP": dec_iltp, "VRLS": dec_vrls,
    "CTAG": dec_ctag, "ITAG": dec_ctag, "SERV": dec_serv, "petb": dec_petb,
    "uetb": dec_uetb, "PMST": dec_pmst, "LOCR": dec_locr, "FACT": dec_fact,
    "CRCT": dec_crct, "TLDF": dec_tldf, "TTLD": dec_tldf,
    "MENR": dec_menr, "ACTD": dec_actd, "GUID": dec_guid, "PVER": dec_pver,
    "SCE2": dec_sce2, "CLST": dec_clst,
    "PNGA": dec_raw_png, "PNGC": dec_raw_png, "PNGD": dec_raw_png,
    "PNGK": dec_raw_png, "PNGR": dec_raw_png,
    "SVGA": dec_raw_svg, "SVGC": dec_raw_svg, "SVGD": dec_raw_svg,
    "APFT": dec_flat_u32, "ACTP": dec_flat_u32, "FEAT": dec_flat_u32,
    "PROD": dec_flat_u32, "IFEQ": dec_flat_u32,
}

# --------------------------------------------------------------------------------------
# structure probes for codes with no verified decoder
# --------------------------------------------------------------------------------------

def probe_fixed_record(samples):
    """Test: file is a flat array of fixed-size records with no header.
    Held only if one stride in 2..64 divides every sampled file size."""
    sizes = sorted({len(d) for _, d in samples})
    if not sizes:
        return {"hypothesis": "flat fixed-size record array (no header)", "held": False,
                "detail": "no samples"}
    if len(sizes) == 1:
        return {"hypothesis": "flat fixed-size record array (no header)", "held": True,
                "detail": "every sampled file is exactly %d bytes" % sizes[0]}
    for stride in range(2, 65):
        if all(s % stride == 0 for s in sizes) and stride > 1:
            return {"hypothesis": "flat fixed-size record array (no header)", "held": True,
                    "detail": "stride %d divides all %d distinct sampled sizes" % (stride, len(sizes))}
    return {"hypothesis": "flat fixed-size record array (no header)", "held": False,
            "detail": "no stride in 2..64 divides all %d distinct sampled sizes (e.g. %s)" % (
                len(sizes), sizes[:6])}


def probe_count_header(samples):
    """Test: a u16 or u32 count in the first 16 bytes, followed by fixed-size records.
    Held when one (offset, width) pair yields an integral record size across every sample."""
    best = None
    for width, rd in ((2, u16), (4, u32)):
        for off in range(0, 13, 2):
            hits, sizes = 0, set()
            ok = True
            for _, d in samples:
                if len(d) < off + width + 2:
                    ok = False
                    break
                try:
                    n = rd(d, off)
                except Eof:
                    ok = False
                    break
                body = len(d) - (off + width)
                if n <= 0 or n > 200000 or body <= 0 or body % n:
                    ok = False
                    break
                sizes.add(body // n)
                hits += 1
            if ok and hits == len(samples) and len(sizes) == 1:
                cand = {"hypothesis": "u%d count at offset %d + fixed records" % (width * 8, off),
                        "held": True,
                        "detail": "(filesize-%d)/count == %d for all %d sampled files" % (
                            off + width, sizes.pop(), hits)}
                if best is None:
                    best = cand
    if best:
        return best
    return {"hypothesis": "u16/u32 count in first 16 bytes + fixed-size records", "held": False,
            "detail": "no (offset<=12, width in {2,4}) pair gave an integral, constant record size "
                      "across the sampled files"}


def probe_length_prefixed_strings(samples):
    """Test: u8 and u16-LE length-prefixed ASCII runs somewhere in the payload."""
    u8_hits = u16_hits = 0
    example = None
    for _, d in samples:
        n = len(d)
        for o in range(0, min(n, 4096)):
            ln = d[o]
            if 4 <= ln <= 80 and o + 1 + ln <= n and all(32 <= b < 127 for b in d[o + 1:o + 1 + ln]):
                u8_hits += 1
                break
        for o in range(0, min(n, 4096) - 1):
            ln = struct.unpack_from("<H", d, o)[0]
            if 4 <= ln <= 400 and o + 2 + ln <= n and all(32 <= b < 127 for b in d[o + 2:o + 2 + ln]):
                u16_hits += 1
                if example is None:
                    example = d[o + 2:o + 2 + min(ln, 48)].decode("latin-1")
                break
    return {"hypothesis": "length-prefixed ASCII strings (u8 and u16-LE)", "held": bool(u16_hits or u8_hits),
            "detail": "u8-prefixed candidate in %d/%d samples, u16-prefixed in %d/%d%s" % (
                u8_hits, len(samples), u16_hits, len(samples),
                (" e.g. %r" % example) if example else "")}


def probe_bosd(samples):
    """BOSD-specific: u16 count header + u16-kind tagged records with a fixed size per kind."""
    tbl = {2: 28, 6: 28, 9: 16, 11: 12}
    ok = 0
    kinds = collections.Counter()
    for _, d in samples:
        try:
            cnt = u16(d, 0)
            kinds[u16(d, 2)] += 1
        except Eof:
            continue
        o, recs = 2, 0
        good = True
        while o < len(d):
            if o + 2 > len(d):
                good = False
                break
            k = u16(d, o)
            if k not in tbl:
                good = False
                break
            o += 2 + tbl[k]
            recs += 1
        if good and o == len(d) and recs == cnt:
            ok += 1
    return {"hypothesis": "u16 count + u16-kind tagged records, fixed size per kind "
                          "{2:28, 6:28, 9:16, 11:12}",
            "held": ok == len(samples),
            "detail": "byte-exact for %d/%d sampled files; leading kinds seen %s. A free-size "
                      "depth-first search over candidate sizes 4..48 solved 89/113 files but "
                      "returned mutually inconsistent sizes for the SAME kind across files "
                      "(e.g. kind 5 solved as 12 in 24 files and 20 in 21 files), so the "
                      "fixed-size-per-kind model is REJECTED, not merely unproven"
                      % (ok, len(samples), dict(kinds.most_common(8)))}


def probe_bosd_clst(samples):
    """BOSD: does it use the CLST record grammar (kind + 3 u32 + pair list) with a u16 count?"""
    ok = 0
    for _, d in samples:
        try:
            cnt = u16(d, 0)
        except Eof:
            continue
        o, recs = 2, 0
        good = True
        while o < len(d):
            if o + 14 > len(d):
                good = False
                break
            m = u32(d, o + 10)
            if m > 100000 or o + 14 + m * 8 > len(d):
                good = False
                break
            o += 14 + m * 8
            recs += 1
        if good and o == len(d) and recs == cnt:
            ok += 1
    return {"hypothesis": "BOSD reuses the CLST record grammar: u16 count then "
                          "{u16 kind, u32 class_id, u32 base_id, u32 pair_count, pair_count*8B}",
            "held": ok == len(samples),
            "detail": "byte-exact with matching declared count for %d/%d sampled files -- the "
                      "grammar fits the kind-2 and kind-6 records but not kinds 1/5/8/9/10/11, so "
                      "BOSD is a superset container that is NOT decoded here" % (ok, len(samples))}


SCK_TYPES = {0: 2, 1: 2, 2: 2, 3: 2, 4: 2, 5: 2, 6: 2, 7: 4, 8: 4, 9: 8, 0xA: 4, 0xB: 2,
             0xC: 2, 0x12: 4, 0x13: 4, 0x14: 2, 0x15: 2, 0x8001: 2, 0x8003: 32,
             0x8004: 48, 0xC001: 8}

# The confirmed SCML/SCMA entry value-type sizes (type -> value bytes).
SC_ENTRY_TYPES = {2: 2, 3: 4, 7: 4, 9: 8}
SC_ENTRY_TYPES_EXT = {2: 2, 3: 4, 7: 4, 9: 8, 1: 1, 4: 8, 5: 4}
SC_FILE_HEADER = 10   # u16, u32, u32
SC_BLOCK_HEADER = 14  # u32 block_id, u32 a, u32 b, u16 entry_count


def _sc_walk(d, types, trailer):
    """Walk the block-framed SCML/SCMA grammar. Returns ((blocks, entries), 'ok') or (None, why)."""
    n = len(d)
    if n < SC_FILE_HEADER + SC_BLOCK_HEADER:
        return None, "shorter than file header + one block header"
    o = SC_FILE_HEADER
    blocks = entries = 0
    while o < n:
        if o + SC_BLOCK_HEADER > n:
            return None, "eof in block header"
        cnt = struct.unpack_from("<H", d, o + 12)[0]
        o += SC_BLOCK_HEADER
        for _ in range(cnt):
            if o + 4 > n:
                return None, "eof in entry header"
            t = struct.unpack_from("<H", d, o)[0]
            sz = types.get(t)
            if sz is None:
                return None, "unknown value_type %d" % t
            if o + 4 + sz > n:
                return None, "eof in value"
            o += 4 + sz
            entries += 1
        o += trailer
        if o > n:
            return None, "eof in block trailer"
        blocks += 1
    return (blocks, entries), "ok"


def _sc_ladder_rung(samples, types, trailer, label):
    ok = blocks = entries = 0
    fails = collections.Counter()
    for _e, d in samples:
        r, msg = _sc_walk(d, types, trailer)
        if r:
            ok += 1
            blocks += r[0]
            entries += r[1]
        else:
            fails[msg] += 1
    return {"hypothesis": label,
            "held": ok == len(samples),
            "detail": "consumed byte-exactly for %d/%d sampled files (%d blocks, %d entries); "
                      "failures: %s" % (ok, len(samples), blocks, entries,
                                        dict(fails.most_common(4)))}, ok


def probe_sc_block_no_trailer(samples):
    """Rung 1: file header + blocks with NO trailer after the entry list."""
    r, _ = _sc_ladder_rung(samples, SC_ENTRY_TYPES, 0,
                           "10-byte file header {u16,u32,u32} then blocks "
                           "{u32 block_id, u32 a, u32 b, u16 entry_count, entries} with NO trailer; "
                           "entry = {u16 value_type, u16 key, value}, sizes 2->2B 3->4B "
                           "7->4B signed int 9->8B IEEE double")
    return r


def probe_sc_block_trailer8(samples):
    """Rung 2: same, with a fixed 8-byte trailer after each block's entry list."""
    r, _ = _sc_ladder_rung(samples, SC_ENTRY_TYPES, 8,
                           "same block grammar WITH a fixed 8-byte trailer after each block's "
                           "entries")
    return r


def probe_sc_block_ext_types(samples):
    """Rung 3: 8-byte trailer plus extra value types 1 (1B), 4 (8B), 5 (4B)."""
    r, _ = _sc_ladder_rung(samples, SC_ENTRY_TYPES_EXT, 8,
                           "same block grammar with an 8-byte trailer AND extra value types "
                           "1 (1 byte), 4 (8 bytes), 5 (4 bytes)")
    zero_mid = 0
    for _e, d in samples:
        _r, msg = _sc_walk(d, SC_ENTRY_TYPES_EXT, 8)
        if msg == "unknown value_type 0":
            zero_mid += 1
    r["detail"] += ("; value_type 0 appears mid-stream in %d sampled files, which points at wrong "
                    "BLOCK framing rather than a missing value type" % zero_mid)
    return r


def probe_sc_entry_grammar(samples):
    """Does block 0's declared entry list parse with sequential keys 1..n? This isolates the
    entry grammar from the block framing."""
    ok = tries = keys_seq = 0
    entries = 0
    example = None
    for _e, d in samples:
        if len(d) < SC_FILE_HEADER + SC_BLOCK_HEADER:
            continue
        tries += 1
        bid = struct.unpack_from("<I", d, SC_FILE_HEADER)[0]
        cnt = struct.unpack_from("<H", d, SC_FILE_HEADER + 12)[0]
        o = SC_FILE_HEADER + SC_BLOCK_HEADER
        good = cnt > 0
        seq = True
        vals = []
        for i in range(cnt):
            if o + 4 > len(d):
                good = False
                break
            t, k = struct.unpack_from("<HH", d, o)
            sz = SC_ENTRY_TYPES.get(t)
            if sz is None or o + 4 + sz > len(d):
                good = False
                break
            if k != i + 1:
                seq = False
            raw = d[o + 4:o + 4 + sz]
            v = (struct.unpack("<d", raw)[0] if sz == 8
                 else struct.unpack("<i", raw)[0] if sz == 4
                 else struct.unpack("<h", raw)[0])
            if len(vals) < 4:
                vals.append((t, k, v))
            o += 4 + sz
        if good:
            ok += 1
            entries += cnt
            if seq:
                keys_seq += 1
            if example is None:
                example = (hex(bid), cnt, vals)
    return {"hypothesis": "block 0's declared entry_count entries all parse as "
                          "{u16 value_type, u16 key, value} with the confirmed size table",
            "held": tries > 0 and ok == tries,
            "detail": "block 0 fully parsed in %d/%d sampled files (%d entries); keys ran "
                      "sequentially 1..n in %d of those. Example: block_id=%s entry_count=%s "
                      "first_entries=%s" % (ok, tries, entries, keys_seq,
                                            example[0] if example else None,
                                            example[1] if example else None,
                                            example[2] if example else None)}


def _scan_kv(d, hdr):
    o, c = hdr, 0
    n = len(d)
    while o + 4 <= n:
        t = struct.unpack_from("<H", d, o)[0]
        sz = SCK_TYPES.get(t)
        if sz is None or o + 4 + sz > n:
            return None
        o += 4 + sz
        c += 1
    return c if o == n else None


def probe_typed_kv(samples):
    """SCML/SCMA: {u16 value_type, u16 key, value} stream after a variable-length header."""
    walked, total, offs, recs = 0, 0, collections.Counter(), 0
    for _, d in samples:
        total += 1
        for h in range(0, 65):
            r = _scan_kv(d, h)
            if r and r >= 2:
                walked += 1
                offs[h] += 1
                recs += r
                break
    return {"hypothesis": "{u16 value_type, u16 key, value} stream with type sizes "
                          "{7:u32, 9:double, 0x8001:u16, 0x8003:32B, 0xc001:8B} after a header",
            "held": walked == total,
            "detail": "byte-exact to EOF for %d/%d sampled files (%d key/value records); the "
                      "required header length is NOT constant (offsets seen: %s), so the container "
                      "is block-structured and the block header is not decoded" % (
                          walked, total, recs, dict(offs.most_common(5)))}


VIEW_SRC = re.compile(rb"D::[A-Za-z0-9_:.\-]{8,220}")


def probe_view(samples):
    """VIEW: u32 root id, then a node stream whose nodes start 0x33 0x33 + u32 boss class id."""
    at4 = sum(1 for _, d in samples if d[4:6] == b"\x33\x33")
    at0 = sum(1 for _, d in samples if d[0:2] == b"\x33\x33")
    paths = 0
    locales = collections.Counter()
    example = None
    nodes = 0
    for _, d in samples:
        m = VIEW_SRC.search(d)
        if m:
            paths += 1
            p = m.group().decode("latin-1")
            if example is None:
                example = p
            lm = re.search(r"_([a-z]{2}[A-Z]{2})\.fr$", p)
            if lm:
                locales[lm.group(1)] += 1
        nodes += d.count(b"\x33\x33")
    return {"hypothesis": "u32 root id at offset 0, then nodes each starting with magic 0x3333 "
                          "followed by a u32 boss class id (prior work)",
            "held": at4 == len(samples),
            "detail": "0x3333 at offset 4 in %d/%d samples (offset 0 in %d/%d, so the prior-work "
                      "note that files START with 0x3333 is off by the 4-byte root id); embedded "
                      "D:: source path in %d/%d samples; locales seen %s; raw 0x3333 occurrences "
                      "across samples = %d (HEURISTIC upper bound on node count - 0x3333 can also "
                      "occur inside node payload bytes, so this is not a verified record count)%s"
                      % (at4, len(samples), at0, len(samples), paths, len(samples),
                         dict(locales.most_common(6)), nodes,
                         (" e.g. %s" % example) if example else "")}


def probe_view_locale(samples):
    """VIEW: does every file that carries a source path also name a locale?"""
    with_path = with_locale = 0
    for _, d in samples:
        m = VIEW_SRC.search(d)
        if m:
            with_path += 1
            if re.search(r"_[a-z]{2}[A-Z]{2}\.fr$", m.group().decode("latin-1")):
                with_locale += 1
    return {"hypothesis": "every VIEW source path ends _<locale>.fr naming the dialog's locale",
            "held": with_path > 0 and with_path == with_locale,
            "detail": "%d/%d sampled files carry a D:: path; %d of those end in _<locale>.fr" % (
                with_path, len(samples), with_locale)}


def probe_clst(samples):
    """CLST: prior work calls these 12-byte-ish numeric records. Test 4/8/12/16-byte strides."""
    sizes = sorted({len(d) for _, d in samples})
    res = []
    for stride in (4, 8, 12, 16, 20, 24):
        if all(s % stride == 0 for s in sizes):
            res.append(stride)
    return {"hypothesis": "flat numeric record array with a 4/8/12/16/20/24-byte stride",
            "held": bool(res),
            "detail": "strides dividing every one of the %d distinct sampled sizes: %s "
                      "(sizes e.g. %s)" % (len(sizes), res or "none", sizes[:8])}


def census_png(ents):
    """Full-population probe (every file, first 16 bytes only): is the payload really a PNG,
    and does the signature match the canonical 89 50 4E 47 0D 0A 1A 0A?"""
    good = mangled = other = 0
    bad_examples = []
    for e in ents:
        try:
            with open(e[2], "rb") as fh:
                head = fh.read(16)
        except OSError:
            continue
        if head[:8] == PNG_SIG:
            good += 1
        elif head[:10] == b"\x89PNG\r\r\n\x1a\r\n":
            mangled += 1
            if len(bad_examples) < 8:
                bad_examples.append(e[2])
        else:
            other += 1
            if len(bad_examples) < 8:
                bad_examples.append(e[2])
    return {"hypothesis": "every file in this code is a real PNG with the canonical 8-byte signature "
                          "(checked over ALL files, not just the sample)",
            "held": mangled == 0 and other == 0,
            "detail": "%d/%d canonical PNG; %d carry the CRLF-mangled signature "
                      "89 50 4E 47 0D 0D 0A 1A 0D 0A (a text-mode LF->CRLF conversion applied to a "
                      "binary file, a real defect in the shipped resources); %d are neither. "
                      "Examples: %s" % (good, len(ents), mangled, other, bad_examples[:4])}


def census_svg(ents):
    """Full-population probe (every file, first 512 bytes): does the payload start with <svg,
    optionally after an XML prolog, with no binary wrapper?"""
    at0 = after_prolog = other = 0
    bad = []
    for e in ents:
        try:
            with open(e[2], "rb") as fh:
                head = fh.read(512)
        except OSError:
            continue
        if head.startswith(b"<svg"):
            at0 += 1
        elif head.lstrip().startswith(b"<?xml") and b"<svg" in head:
            after_prolog += 1
        else:
            other += 1
            if len(bad) < 6:
                bad.append(e[2])
    return {"hypothesis": "every file in this code is raw SVG text with no binary wrapper "
                          "(checked over ALL files, not just the sample)",
            "held": other == 0,
            "detail": "%d/%d start with <svg at offset 0, %d start with an XML prolog then <svg, "
                      "%d neither%s" % (at0, len(ents), after_prolog, other,
                                        (" e.g. %s" % bad[:3]) if bad else "")}


FULL_PROBES = {
    "PNGA": census_png, "PNGC": census_png, "PNGD": census_png,
    "PNGK": census_png, "PNGR": census_png,
    "SVGA": census_svg, "SVGC": census_svg, "SVGD": census_svg,
}

PROBES = {
    "BOSD": [probe_bosd, probe_bosd_clst, probe_count_header, probe_length_prefixed_strings],
    "SCML": [probe_sc_entry_grammar, probe_sc_block_no_trailer, probe_sc_block_trailer8,
             probe_sc_block_ext_types, probe_typed_kv],
    "SCMA": [probe_sc_entry_grammar, probe_sc_block_no_trailer, probe_sc_block_trailer8,
             probe_sc_block_ext_types, probe_typed_kv],
    "VIEW": [probe_view, probe_view_locale, probe_length_prefixed_strings],
    "CLST": [probe_clst, probe_count_header],
    "BBlb": [probe_fixed_record, probe_count_header, probe_length_prefixed_strings],
    "FNPA": [probe_fixed_record, probe_count_header],
}
DEFAULT_PROBES = [probe_fixed_record, probe_count_header, probe_length_prefixed_strings]

# --------------------------------------------------------------------------------------
# per-code classification + evidence (only filled in from what the run observed)
# --------------------------------------------------------------------------------------

CLASSIFY = {
    "SCE2": ("scripting_model", "Scripting object model: suites, classes, methods, properties, "
                                "enumerations. Fully parsed by the sibling tool "
                                "indesign-sce2-parse.py (prior work, not re-derived here)."),
    "uetb": ("error_catalog", "Error-code catalog. High 16 bits of the code are the plug-in/service "
                              "id, low 16 bits the ordinal (prior work, re-verified here)."),
    "petb": ("error_catalog", "Presentable (user-facing) error message table -- the string half of "
                              "uetb's symbol catalog."),
    "PMST": ("localized_strings", "Localized string tables keyed by ZString-style key, one file per "
                                  "locale per plug-in (prior work, re-verified here)."),
    "LOCR": ("locale_index", "Locale -> resource-id index. CORRECTION to prior work: the leading "
                             "4 bytes are not always 'TSMP'; they are the byte-reversed type code "
                             "of the resource family the index points at (WEIV=VIEW 1263 files, "
                             "TSMP=PMST 539, RNEM=MENR 107, DTCA=ACTD 96, TSLK=KLST, FDLT=TLDF, "
                             "_EVE=EVE_, TSLP=PLST). So LOCR is the generic 'which resource id "
                             "serves which locale' table for every localisable code."),
    "VIEW": ("ui_layout", "Serialized widget/dialog trees. Nodes start 0x3333 + u32 boss class id; "
                          "many embed the original D::RESS:ID:InDesign:source:... path naming the "
                          "dialog and locale."),
    "EVE_": ("ui_layout", "Adobe Eve declarative UI layout source, stored as plain text. This is the "
                          "layout half of the Adobe Source Libraries Adam/Eve pair: view/column/row/"
                          "button_view nodes with margin, spacing, alignment and action bindings."),
    "ADAM": ("ui_layout", "Adobe Adam property-model (sheet) source, stored as plain text: interface/"
                          "logic/output cells that drive the matching Eve layout."),
    "MENR": ("ui_layout", "Menu resource table: for each menu item an action id, the full menu path "
                          "(e.g. 'Main:&Edit', 'TablePanelPopup:kTablesMenuTable Options_&') and a "
                          "double sort position that fixes the item's order inside its menu."),
    "ACTD": ("keyboard_shortcuts", "Action definition table: action id, command id, the action's "
                                   "display name (e.g. 'Find &Next', 'To Uppercase') and the "
                                   "shortcut area it belongs to (e.g. 'KBSCE Edit menu', "
                                   "'KBSCE Type menu: Change Case: '). This is the table the "
                                   "keyboard-shortcut editor groups by."),
    "TIPS": ("localized_strings", "Tooltip table: widget id -> tooltip key or literal tooltip text."),
    "ACCF": ("localized_strings", "Accessibility caption table: widget id -> caption/label text "
                                  "(the 'caption for' relationship used by screen readers and by "
                                  "the dialog layout engine)."),
    "TOCL": ("ui_layout", "Single panel/workspace ordering record: widget id, display name, related "
                          "id and a double sort key."),
    "PLST": ("ui_layout", "Panel registration: menu path (e.g. 'UtilitiesSubmenu:...'), parent menu "
                          "path, and a double menu position, terminated by the literal 'SNPC'."),
    "KLST": ("ui_layout", "Same record grammar as PLST with a u16 version + u16 count header; used "
                          "for submenu registrations (styles, transform, track changes)."),
    "CNTL": ("ui_layout", "Native Win32 control class registry: control class name plus dwStyle and "
                          "dwExStyle used when the widget is realised."),
    "HOTC": ("ui_layout", "Cursor hotspot (x, y) for the cursor bitmap with the same id in PNGC/SVGC."),
    "rulr": ("settings_table", "Ruler subdivision table: label font and size, then per zoom level a "
                               "tick spacing and a list of (divisions, unit) subdivision pairs."),
    "Colr": ("settings_table", "UI colour palette: a flat list of RGB triples as IEEE doubles in 0..1."),
    "SCML": ("settings_table", "Block-framed typed key/value tables carrying DEFAULT PREFERENCE "
                               "VALUES per feature -- the (PDF Resources) instance holds the PDF "
                               "export defaults, so these are the shipped factory settings a "
                               "reimplementation would need to match. The entry grammar is "
                               "confirmed byte-level; the inter-block framing is not reversed."),
    "SCMA": ("settings_table", "Schema companion to SCML in the same container: the same typed "
                               "key/value grammar with keys running sequentially from 1, i.e. the "
                               "ordered field list plus its defaults. Entry grammar confirmed, "
                               "block framing not reversed."),
    "FTTB": ("settings_table", "File-type table: OSType/creator 4CC, file extension and MIME type, "
                               "grouped by format id."),
    "PLUG": ("version_info", "Plug-in prerequisite table: for each required plug-in its id, name and "
                             "required major.minor version."),
    "PVER": ("version_info", "Version records; the largest ends with a length-prefixed ASCII version "
                             "string (prior work)."),
    "GUID": ("version_info", "Per-release GUID table: release name plus three 16-byte GUIDs."),
    "VRLS": ("version_info", "Version list: 12-byte records holding a monotonically increasing "
                             "(major, minor) pair -- the document/format versions the plug-in reads."),
    "CLST": ("class_registry", "Boss class list: variable-length records each declaring a class id, "
                               "a base id and a list of (interface id, implementation id) pairs. "
                               "This is the largest machine-readable description of InDesign's "
                               "boss-class/interface object graph in the install."),
    "FACT": ("class_registry", "Flat array of u32 class ids, no strings (prior work, re-verified)."),
    "CLAS": ("class_registry", "Boss class definition: class id, base id, and (interface id, "
                               "implementation id) pairs."),
    "ACLS": ("class_registry", "Class addition: extends an existing boss class with extra "
                               "(interface, implementation) pairs."),
    "ISui": ("class_registry", "Owner id plus a list of (id, id) pairs; same 8-byte pair shape as "
                               "CLAS/IALS."),
    "IALS": ("class_registry", "Interface -> implementation pair list (aggregate registration)."),
    "ILST": ("class_registry", "Flat u32 id list with a u32 count."),
    "ILTP": ("class_registry", "12-byte id triples with a u32 count."),
    "CTAG": ("class_registry", "Owner id plus a list of u32 class ids."),
    "ITAG": ("class_registry", "Owner id plus a list of u32 class ids (same grammar as CTAG)."),
    "BOSD": ("class_registry", "Boss-class definition stream: u16 record count then u16-kind tagged "
                               "records whose length varies by kind. Kinds 1,2,5,6,8,9,10,11 observed."),
    "SERV": ("class_registry", "Service registry entry: service id, provider id and service name."),
    "TLDF": ("ui_layout", "Fixed 44-byte tool definition record; field 2 is the constant id "
                          "0x0001d010 in every sampled file (a shared tool base class)."),
    "TTLD": ("ui_layout", "Same 44-byte record as TLDF; the sampled TTLD files duplicate the TLDF "
                          "record with id+1 (a paired/alternate tool definition)."),
    "CRCT": ("other", "8-byte all-zero marker present once per plug-in; carries no records."),
    "BBlb": ("other", "Heterogeneous binary blob container with NO common header. Sampled payloads "
                      "are an ICC profile, compiled ActionScript/SWF data, an HTML template and a "
                      "Flash publish-profile XML."),
    "PNGA": ("raster_asset", "Raw PNG, no wrapper (application/panel artwork)."),
    "PNGC": ("raster_asset", "Raw PNG, no wrapper (cursor bitmaps; paired with HOTC hotspots)."),
    "PNGD": ("raster_asset", "Raw PNG, no wrapper (dialog / document artwork)."),
    "PNGK": ("raster_asset", "Raw PNG, no wrapper."),
    "PNGR": ("raster_asset", "Raw PNG, no wrapper."),
    "SVGA": ("vector_asset", "Raw SVG text, no wrapper (application/panel icons)."),
    "SVGC": ("vector_asset", "Raw SVG text, no wrapper (cursor icons; paired with HOTC hotspots)."),
    "SVGD": ("vector_asset", "Raw SVG text, no wrapper; data-name attribute carries the Adobe Spectrum "
                             "asset name (e.g. S_LABColor_Xs_N@2x)."),
    "APFT": ("unknown_numeric", "Two u32 words."),
    "ACTP": ("unknown_numeric", "Two u32 words that look like an action-id range mask."),
    "FEAT": ("unknown_numeric", "One u32 word."),
    "PROD": ("unknown_numeric", "One u32 word."),
    "IFEQ": ("unknown_numeric", "One u32 word (zero)."),
    "FNPA": ("unknown_numeric", "26 bytes of u16/u32 words ending 0xffffffff."),
}

# --------------------------------------------------------------------------------------
# scan
# --------------------------------------------------------------------------------------

def owner_of(dirpath: str) -> str:
    for part in dirpath.split(os.sep):
        if part.startswith("(") and part.endswith("Resources)"):
            return part.strip("()").replace(" Resources", "")
    return "APP_ROOT"


def scan(root: str):
    out = []
    for dirpath, _dirnames, filenames in os.walk(root):
        base = os.path.basename(dirpath)
        if not base.startswith("idrc_"):
            continue
        code = base[5:] or "?"
        owner = owner_of(dirpath)
        for fn in filenames:
            if not fn.lower().endswith(".idrc"):
                continue
            full = os.path.join(dirpath, fn)
            try:
                size = os.path.getsize(full)
            except OSError:
                continue
            out.append((code, owner, full, fn, size))
    return out


def pick_samples(entries, min_n, cap_bytes, max_file):
    """Sample = the few largest files (so the header-hex and the hardest cases are covered) plus a
    uniform stride across the whole size-ordered list, bounded by a per-code byte cap."""
    ordered = [e for e in sorted(entries, key=lambda e: -e[4]) if e[4] <= max_file]
    chosen, used, seen = [], 0, set()

    def take(e):
        nonlocal used
        key = e[2]
        if key in seen:
            return False
        if used + e[4] > cap_bytes and len(chosen) >= min_n:
            return False
        seen.add(key)
        chosen.append(e)
        used += e[4]
        return True

    for e in ordered[:4]:
        take(e)
    target = min(len(ordered), 600)
    stride = max(1, len(ordered) // target)
    for e in ordered[::stride]:
        if not take(e) and used > cap_bytes:
            break
    for e in ordered:  # top up to the floor if the cap bit early
        if len(chosen) >= min_n:
            break
        take(e)
    return chosen, used


_SIB_CACHE: dict = {}


def sibling_totals(path):
    """Read the 'totals' object out of a sibling green-room artifact, cached. Returns None when
    the artifact is absent or unreadable -- the caller records that rather than guessing."""
    if path in _SIB_CACHE:
        return _SIB_CACHE[path]
    val = None
    try:
        with open(path, "r", encoding="utf-8") as fh:
            val = json.load(fh).get("totals")
    except (OSError, ValueError):
        val = None
    _SIB_CACHE[path] = val
    return val


def content_shape(datas):
    """Heuristic byte-composition label."""
    tot = ascii_b = zero_b = high_b = 0
    utf16 = 0
    for d in datas:
        tot += len(d)
        for b in d[:65536]:
            if b == 0:
                zero_b += 1
            elif 32 <= b < 127:
                ascii_b += 1
            elif b >= 128:
                high_b += 1
        utf16 += len(UTF16_RUN.findall(d[:65536]))
    seen = min(tot, 65536 * len(datas)) or 1
    return {"ascii_pct": round(100.0 * ascii_b / seen, 1),
            "zero_pct": round(100.0 * zero_b / seen, 1),
            "high_byte_pct": round(100.0 * high_b / seen, 1),
            "utf16le_runs": utf16}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=r"C:\Program Files\Adobe\Adobe InDesign 2026")
    ap.add_argument("--out", required=True)
    ap.add_argument("--cap-bytes-per-code", type=int, default=DEFAULT_CAP)
    ap.add_argument("--max-file-bytes", type=int, default=4 * 1024 * 1024)
    args = ap.parse_args()

    if not os.path.isdir(args.root):
        print("[fatal] install root not found: %s" % args.root, file=sys.stderr)
        return 2

    out_path_abs = os.path.abspath(args.out)
    entries = scan(args.root)
    by_code = collections.defaultdict(list)
    for e in entries:
        by_code[e[0]].append(e)

    total_bytes = sum(e[4] for e in entries)
    codes_out = []
    n_parsed = n_partial = n_none = 0

    for code in sorted(by_code, key=lambda c: -sum(e[4] for e in by_code[c])):
        ents = by_code[code]
        owners = {e[1] for e in ents}
        ids = []
        for e in ents:
            stem = e[3][:-5]
            try:
                ids.append(int(stem))
            except ValueError:
                pass
        code_bytes = sum(e[4] for e in ents)

        samples, read_bytes = pick_samples(ents, MIN_SAMPLE, args.cap_bytes_per_code,
                                           args.max_file_bytes)
        loaded = []
        for e in samples:
            try:
                with open(e[2], "rb") as fh:
                    loaded.append((e, fh.read()))
            except OSError:
                continue

        # magics
        magic_hits = collections.Counter()
        for _e, d in loaded:
            for name, sig in MAGICS.items():
                i = d.find(sig, 0, 4096)
                if i >= 0:
                    magic_hits["%s@%d" % (name, i) if i else name] += 1

        # decoder
        decoded = "not_decoded"
        record_structure = None
        dec_ok = dec_try = 0
        dec_records = 0
        dec_note = ""
        dec_fail = collections.Counter()
        fn = DECODERS.get(code)
        if fn:
            for _e, d in loaded:
                dec_try += 1
                try:
                    ok, nrec, note = fn(d)
                except Eof:
                    ok, nrec, note = False, 0, "eof/format mismatch"
                except Exception as exc:  # noqa: BLE001 - a decoder crash is a failed probe, not a stop
                    ok, nrec, note = False, 0, "exception %s" % type(exc).__name__
                if ok:
                    dec_ok += 1
                    dec_records += nrec
                    if not dec_note:
                        dec_note = note
                else:
                    dec_fail[note[:70]] += 1
            if dec_try:
                if dec_ok == dec_try:
                    decoded = "parsed"
                elif dec_ok:
                    decoded = "partially_parsed"
            if fn.__doc__:
                record_structure = " ".join(fn.__doc__.split())

        # probes
        probes = []
        for pf in PROBES.get(code, DEFAULT_PROBES):
            try:
                probes.append(pf(loaded))
            except Exception as exc:  # noqa: BLE001
                probes.append({"hypothesis": pf.__name__, "held": False,
                               "detail": "probe raised %s" % type(exc).__name__})
        cf = FULL_PROBES.get(code)
        if cf:
            try:
                probes.append(cf(ents))
            except Exception as exc:  # noqa: BLE001
                probes.append({"hypothesis": cf.__name__, "held": False,
                               "detail": "probe raised %s" % type(exc).__name__})
        if decoded == "not_decoded" and any(p["held"] for p in probes) and code not in ("CRCT",):
            decoded = "partially_parsed"
        if code in PARTIAL_RECORD_STRUCTURE:
            decoded = "partially_parsed"
            record_structure = " ".join(PARTIAL_RECORD_STRUCTURE[code].split())

        cross = None
        if code in CROSS_REFS:
            tool, artifact, keys = CROSS_REFS[code]
            apath = os.path.join(os.path.dirname(out_path_abs), artifact)
            entry = {"tool": tool, "artifact": artifact, "artifact_path": apath,
                     "note": CROSS_REF_NOTES.get(code)}
            sib = sibling_totals(apath)
            if sib is None:
                entry["artifact_totals"] = None
                entry["status"] = "artifact not found or unreadable at run time"
            else:
                entry["artifact_totals"] = {k: sib[k] for k in keys if k in sib}
                missing = [k for k in keys if k not in sib]
                entry["status"] = ("totals read from the artifact at run time"
                                   + (" (keys absent: %s)" % missing if missing else ""))
            cross = entry

        # strings
        strings, zstrings = [], []
        seen_s, seen_z = set(), set()
        for _e, d in loaded:
            for m in ASCII_RUN.finditer(d):
                s = m.group().decode("latin-1")
                for z in ZSTRING.findall(s):
                    if z not in seen_z and len(zstrings) < 25:
                        seen_z.add(z)
                        zstrings.append(z)
                if s not in seen_s and len(strings) < 25:
                    seen_s.add(s)
                    strings.append(s[:120])
            for m in UTF16_RUN.finditer(d):
                s = m.group().decode("utf-16-le", errors="ignore")
                if s not in seen_s and len(strings) < 25:
                    seen_s.add(s)
                    strings.append(s[:120])
            if len(strings) >= 25 and len(zstrings) >= 25:
                break

        largest = max(ents, key=lambda e: e[4])
        try:
            with open(largest[2], "rb") as fh:
                head = fh.read(32)
        except OSError:
            head = b""

        cls, evidence_base = CLASSIFY.get(code, ("unknown_mixed", "Not previously examined."))
        shape = content_shape([d for _e, d in loaded])

        ev = [evidence_base]
        ev.append("Sampled %d of %d files (%d bytes read)." % (len(loaded), len(ents), read_bytes))
        if dec_try:
            ev.append("Decoder consumed the file byte-exactly for %d/%d sampled files, yielding %d "
                      "records." % (dec_ok, dec_try, dec_records))
            if dec_note:
                ev.append("Example: %s" % dec_note)
            if dec_fail:
                ev.append("Decoder failures: %s" % dict(dec_fail.most_common(3)))
        ev.append("Byte composition: ascii %.1f%%, zero %.1f%%, high %.1f%%, utf16le runs %d." % (
            shape["ascii_pct"], shape["zero_pct"], shape["high_byte_pct"], shape["utf16le_runs"]))
        if magic_hits:
            ev.append("Magic hits: %s." % dict(magic_hits.most_common(6)))
        else:
            ev.append("No known file magic in the first 4 KiB of any sampled file.")
        if cross and cross.get("artifact_totals"):
            ev.append("Decoded further by the sibling tool %s -> %s; that artifact's own totals are "
                      "carried in cross_reference.artifact_totals (read from it at run time, not "
                      "restated from memory). %s" % (cross["tool"], cross["artifact"],
                                                     cross.get("note") or ""))

        if decoded == "parsed":
            n_parsed += 1
        elif decoded == "partially_parsed":
            n_partial += 1
        else:
            n_none += 1

        codes_out.append({
            "code": code,
            "files": len(ents),
            "bytes": code_bytes,
            "owning_plugins": len(owners),
            "plugins_sample": sorted(owners)[:10],
            "id_range": [min(ids), max(ids)] if ids else None,
            "content_class": cls if cls in CONTENT_CLASSES else "other",
            "decoded": decoded,
            "evidence": " ".join(ev),
            "record_structure": record_structure,
            "cross_reference": cross,
            "decoder_coverage": {"sampled_files": dec_try, "byte_exact_files": dec_ok,
                                 "records_decoded": dec_records} if dec_try else None,
            "structure_probes": probes,
            "magic_hits": dict(magic_hits.most_common(12)),
            "header_hex_largest": binascii.hexlify(head).decode(),
            "largest_file": largest[2],
            "largest_file_bytes": largest[4],
            "sampled_files": len(loaded),
            "sampled_bytes": read_bytes,
            "byte_composition": shape,
            "string_samples": strings,
            "zstring_samples": zstrings,
        })

    doc = {
        "schema_id": "handshake.reference.indesign_resource_code_map@1",
        "generated_at": now(),
        "source_root": args.root,
        "method": (
            "Offline binary survey of the installed Adobe InDesign 2026 tree. No Adobe executable "
            "was launched, no COM/ExtendScript bridge was used, and nothing in the install tree was "
            "written. The tool walks every idrc_<CODE> directory, aggregates real file/byte/plug-in/"
            "id-range counts, then for each code reads a sample (>=12 files or all files, largest "
            "first plus a stride across the rest, capped per code) and runs three passes: "
            "(1) DECODERS -- hand-written record parsers; a code is marked 'parsed' only when the "
            "parser consumes every sampled file byte-exactly and, where the container declares a "
            "count, the parsed record count equals it; "
            "(2) PROBES -- explicit structural hypotheses (flat fixed-size records; a u16/u32 count "
            "in the first 16 bytes followed by fixed records with (filesize-header)/count integral "
            "and constant; u8/u16-LE length-prefixed ASCII; plus code-specific probes) each reported "
            "with held=true/false and what was tried; "
            "(3) SURFACE -- file magics (PNG/SVG/XML/HTML/TSMP/zlib/gzip/ICC 'acsp'/UTF-8 BOM), byte "
            "composition percentages, ASCII/UTF-16LE string samples and $$$/ ZString keys. "
            "SCE2, PMST, LOCR, uetb, VIEW, CLST, FACT, PVER, SCML, MENR and ACTD grammars come from "
            "earlier passes and sibling tools; where a decoder exists here they were re-verified "
            "against this install, and three prior-work claims were corrected (see the LOCR, VIEW "
            "and CLST evidence). Codes that a sibling green-room tool decodes more deeply carry a "
            "cross_reference whose artifact_totals are read out of that artifact at run time rather "
            "than restated, so they cannot drift from it. Every field-name interpretation (what an "
            "id or flag MEANS) is heuristic and is labelled as such in the evidence text; only "
            "sizes, counts and byte-exact consumption are verified. A file count is never reported "
            "as a record count."
        ),
        "totals": {
            "idrc_files": len(entries),
            "type_codes": len(by_code),
            "bytes": total_bytes,
            "codes_parsed": n_parsed,
            "codes_partially_parsed": n_partial,
            "codes_not_decoded": n_none,
        },
        "codes": codes_out,
    }

    out_path = args.out
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
        fh.write("\n")

    print("[idrc-map] files=%d codes=%d bytes=%d parsed=%d partial=%d none=%d -> %s" % (
        len(entries), len(by_code), total_bytes, n_parsed, n_partial, n_none, out_path))
    for c in codes_out:
        print("   %-6s files=%6d MB=%7.2f owners=%3d %-18s %-18s %s" % (
            c["code"], c["files"], c["bytes"] / 1048576.0, c["owning_plugins"],
            c["content_class"], c["decoded"],
            ("recs=%d" % c["decoder_coverage"]["records_decoded"]) if c["decoder_coverage"] else ""))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
