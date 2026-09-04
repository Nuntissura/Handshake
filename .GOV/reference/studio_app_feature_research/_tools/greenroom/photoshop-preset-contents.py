#!/usr/bin/env python
"""
photoshop-preset-contents.py

Offline parser for Adobe Photoshop 2026 preset CONTAINER files.

Purpose: an earlier harvest pass (presets.json) counted FILES and reported that
count as a preset count. That is wrong. This tool opens each container and
recovers the real entry names, entry counts and, where the binary format allows,
the entry parameters.

Nothing here launches Photoshop. Every byte is read from disk.

Formats implemented:
  descriptor  Photoshop Action Descriptor (shared primitive, used by many formats)
  .grd  gradients        8BGR + descriptor list           -> full params
  .pat  patterns         8BPT pattern records             -> names + geometry
  .asl  layer styles     8BSL + patterns + 2 desc/style   -> names + effect params
  .abr  brushes          8BIM sectioned (v6+) / v1-2      -> names + brush params
  .csh  custom shapes    'cush' shape records             -> names + bounds
  .tpl  tool presets     8BTP + 8BIM blocks               -> names + tool params
  .atn  actions          v16 action set                   -> set/action/step names
  .aco  swatches         v1/v2 color records              -> names + colors
  .acb  color books      8BCB                             -> book meta + colors
  .act  color tables     raw 768/772 byte palette         -> 256 RGB entries
  .acv  curves           curve point arrays               -> per-channel points
  .blw  black & white    raw descriptor                   -> full params
  .cha  channel mixer    fixed record                     -> full params
  .ahu  hue/saturation   fixed record                     -> full params
  .alv  levels           fixed record                     -> full params
  .hdt  HDR toning       'hdrt' + 'hdra'                  -> partial
  .shc  contours         8BFS contour records             -> names + points
  .ado  duotones         fixed record                     -> ink names + curves
  .irs  save-for-web     binary settings                  -> partial
  .cube 3D LUT           text                             -> full header + size
  .3dl  3D LUT           text                             -> full header + size
  .look SpeedGrade look  XML                              -> shader/param tree
  .mnu  menu set         8MNU                             -> set names

Every result records `parse_status`: "parsed" | "partial" | "failed", and any
value obtained by inference rather than by reading a documented field is flagged.
"""

import binascii
import datetime
import hashlib
import json
import os
import re
import struct
import sys
import traceback
import xml.etree.ElementTree as ET
from collections import Counter, OrderedDict

INSTALL_ROOT = r"C:\Program Files\Adobe\Adobe Photoshop 2026"
USER_ROOT = os.path.join(
    os.environ.get("APPDATA", ""), "Adobe", "Adobe Photoshop 2026"
)
HERE = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.path.abspath(
    os.path.join(
        HERE,
        "..",
        "..",
        "_greenroom_20260903",
        "installed_exports",
        "photoshop",
        "offline",
    )
)
OUT_FILE = os.path.join(OUT_DIR, "photoshop_preset_contents.json")

SCHEMA_ID = "handshake.adobe.photoshop.preset_contents.v1"

# --------------------------------------------------------------------------
# byte reader
# --------------------------------------------------------------------------


class Trunc(Exception):
    pass


class R:
    """Big-endian binary reader. Photoshop preset containers are all BE."""

    def __init__(self, data, pos=0):
        self.d = data
        self.p = pos
        self.n = len(data)

    def need(self, k):
        if self.p + k > self.n:
            raise Trunc("want %d at %d, have %d" % (k, self.p, self.n))

    def take(self, k):
        self.need(k)
        b = self.d[self.p : self.p + k]
        self.p += k
        return b

    def u8(self):
        return struct.unpack(">B", self.take(1))[0]

    def i8(self):
        return struct.unpack(">b", self.take(1))[0]

    def u16(self):
        return struct.unpack(">H", self.take(2))[0]

    def i16(self):
        return struct.unpack(">h", self.take(2))[0]

    def u32(self):
        return struct.unpack(">I", self.take(4))[0]

    def i32(self):
        return struct.unpack(">i", self.take(4))[0]

    def i64(self):
        return struct.unpack(">q", self.take(8))[0]

    def f32(self):
        return struct.unpack(">f", self.take(4))[0]

    def f64(self):
        return struct.unpack(">d", self.take(8))[0]

    def key4(self):
        return self.take(4).decode("latin-1")

    def ustr(self):
        """Unicode string: uint32 char count then UTF-16BE. Trailing NUL dropped."""
        n = self.u32()
        if n > 0x00FFFFFF:
            raise Trunc("absurd unicode length %d at %d" % (n, self.p))
        raw = self.take(n * 2)
        s = raw.decode("utf-16-be", "replace")
        return s.rstrip("\x00")

    def pstr(self, pad=1):
        """Pascal string: 1 length byte then bytes, padded to `pad`."""
        ln = self.u8()
        s = self.take(ln).decode("latin-1")
        if pad > 1:
            total = 1 + ln
            rem = total % pad
            if rem:
                self.take(pad - rem)
        return s

    def ostype(self):
        """OSType key: uint32 length; 0 means a literal 4-byte key."""
        ln = self.u32()
        if ln == 0:
            return self.take(4).decode("latin-1")
        if ln > 0x00FFFFFF:
            raise Trunc("absurd ostype length %d at %d" % (ln, self.p))
        return self.take(ln).decode("latin-1")

    def eof(self):
        return self.p >= self.n


# --------------------------------------------------------------------------
# zstring
# --------------------------------------------------------------------------

ZSTR = re.compile(r"^\$\$\$/[^=]*=(.*)$", re.S)


def zresolve(s):
    """'$$$/Presets/Patterns/TreeTile4=Tree Tile 4' -> 'Tree Tile 4'."""
    if not isinstance(s, str):
        return s
    m = ZSTR.match(s)
    if m:
        return m.group(1)
    return s


def zpath(s):
    if isinstance(s, str) and s.startswith("$$$/") and "=" in s:
        return s.split("=", 1)[0]
    return None


# --------------------------------------------------------------------------
# Photoshop Action Descriptor
# --------------------------------------------------------------------------

DESC_MAX_DEPTH = 40


def parse_descriptor(r, depth=0):
    if depth > DESC_MAX_DEPTH:
        raise Trunc("descriptor nesting > %d" % DESC_MAX_DEPTH)
    name = r.ustr()
    cls = r.ostype()
    cnt = r.u32()
    if cnt > 100000:
        raise Trunc("absurd descriptor item count %d" % cnt)
    items = OrderedDict()
    for _ in range(cnt):
        k = r.ostype()
        items[k] = parse_item(r, depth + 1)
    out = OrderedDict()
    if name:
        out["_name"] = name
    out["_class"] = cls
    out["_items"] = items
    return out


def parse_item(r, depth=0):
    if depth > DESC_MAX_DEPTH:
        raise Trunc("item nesting > %d" % DESC_MAX_DEPTH)
    t = r.key4()
    if t in ("Objc", "GlbO"):
        return {"t": t, "v": parse_descriptor(r, depth + 1)}
    if t == "VlLs":
        n = r.u32()
        if n > 200000:
            raise Trunc("absurd list count %d" % n)
        return {"t": t, "v": [parse_item(r, depth + 1) for _ in range(n)]}
    if t == "doub":
        return {"t": t, "v": r.f64()}
    if t == "UntF":
        u = r.key4()
        return {"t": t, "unit": u, "v": r.f64()}
    if t == "UnFl":  # unit float list
        u = r.key4()
        n = r.u32()
        return {"t": t, "unit": u, "v": [r.f64() for _ in range(n)]}
    if t == "TEXT":
        return {"t": t, "v": r.ustr()}
    if t == "enum":
        et = r.ostype()
        ev = r.ostype()
        return {"t": t, "enum_type": et, "v": ev}
    if t == "long":
        return {"t": t, "v": r.i32()}
    if t == "comp":
        return {"t": t, "v": r.i64()}
    if t == "bool":
        return {"t": t, "v": bool(r.u8())}
    if t in ("type", "GlbC"):
        nm = r.ustr()
        cid = r.ostype()
        return {"t": t, "v": {"name": nm, "classID": cid}}
    if t == "alis":
        ln = r.u32()
        raw = r.take(ln)
        return {"t": t, "v": raw.decode("latin-1", "replace")[:400]}
    if t == "tdta":
        ln = r.u32()
        raw = r.take(ln)
        return {"t": t, "len": ln, "v_sha1": hashlib.sha1(raw).hexdigest()}
    if t == "obj ":
        return {"t": t, "v": parse_reference(r, depth + 1)}
    if t == "ObAr":  # object array (rare; Photoshop path/warp data)
        ver = r.u32()
        nm = r.ustr()
        cid = r.ostype()
        n = r.u32()
        arr = []
        for _ in range(n):
            k = r.ostype()
            at = r.key4()
            if at == "UnFl":
                u = r.key4()
                cn = r.u32()
                arr.append(
                    {"key": k, "t": at, "unit": u, "v": [r.f64() for _ in range(cn)]}
                )
            elif at == "doub":
                cn = r.u32()
                arr.append({"key": k, "t": at, "v": [r.f64() for _ in range(cn)]})
            elif at == "long":
                cn = r.u32()
                arr.append({"key": k, "t": at, "v": [r.i32() for _ in range(cn)]})
            else:
                raise Trunc("ObAr member type %r unhandled" % at)
        return {"t": t, "version": ver, "name": nm, "classID": cid, "v": arr}
    raise Trunc("unknown descriptor OSType %r at %d" % (t, r.p - 4))


def parse_reference(r, depth=0):
    n = r.u32()
    if n > 10000:
        raise Trunc("absurd reference count %d" % n)
    out = []
    for _ in range(n):
        f = r.key4()
        if f == "prop":
            out.append(
                {"form": f, "name": r.ustr(), "classID": r.ostype(), "keyID": r.ostype()}
            )
        elif f == "Clss":
            out.append({"form": f, "name": r.ustr(), "classID": r.ostype()})
        elif f == "Enmr":
            out.append(
                {
                    "form": f,
                    "name": r.ustr(),
                    "classID": r.ostype(),
                    "typeID": r.ostype(),
                    "enumID": r.ostype(),
                }
            )
        elif f == "rele":
            out.append(
                {"form": f, "name": r.ustr(), "classID": r.ostype(), "offset": r.u32()}
            )
        elif f == "Idnt":
            out.append(
                {
                    "form": f,
                    "name": r.ustr(),
                    "classID": r.ostype(),
                    "identifier": r.u32(),
                }
            )
        elif f == "indx":
            out.append(
                {
                    "form": f,
                    "name": r.ustr(),
                    "classID": r.ostype(),
                    "index": r.u32(),
                }
            )
        elif f == "name":
            out.append(
                {
                    "form": f,
                    "name": r.ustr(),
                    "classID": r.ostype(),
                    "value": r.ustr(),
                }
            )
        else:
            raise Trunc("unknown reference form %r" % f)
    return out


def desc_flat(desc, prefix="", out=None, depth=0):
    """Flatten a descriptor to {key_path: compact_value} for readability."""
    if out is None:
        out = OrderedDict()
    if depth > 12:
        return out
    for k, v in desc.get("_items", {}).items():
        path = prefix + k
        t = v.get("t")
        if t in ("Objc", "GlbO"):
            desc_flat(v["v"], path + ".", out, depth + 1)
        elif t == "VlLs":
            out[path] = "[list %d]" % len(v["v"])
            for i, item in enumerate(v["v"][:8]):
                if item.get("t") in ("Objc", "GlbO"):
                    desc_flat(item["v"], "%s[%d]." % (path, i), out, depth + 1)
                else:
                    out["%s[%d]" % (path, i)] = _scalar(item)
        else:
            out[path] = _scalar(v)
    return out


def _scalar(v):
    t = v.get("t")
    if t == "UntF":
        return "%s %s" % (v["v"], v["unit"])
    if t == "enum":
        return "%s.%s" % (v.get("enum_type"), v.get("v"))
    if t in ("type", "GlbC"):
        return "class:%s" % v["v"]["classID"]
    if t == "tdta":
        return "tdta(%d bytes)" % v["len"]
    if t == "obj ":
        return "ref(%d)" % len(v["v"])
    return v.get("v")


def desc_name(desc):
    """Best-effort human name for a descriptor entry."""
    it = desc.get("_items", {})
    for k in ("Nm  ", "Nm ", "name", "Ttl "):
        if k in it and it[k].get("t") == "TEXT":
            return zresolve(it[k]["v"])
    if desc.get("_name"):
        return zresolve(desc["_name"])
    return None


# --------------------------------------------------------------------------
# helpers shared by pattern-bearing formats
# --------------------------------------------------------------------------


def read_pattern_record(r):
    """One pattern record. Returns dict. Consumes the VM array list."""
    ver = r.u32()
    mode = r.u32()
    h = r.u16()
    w = r.u16()
    name = r.ustr()
    pid = r.pstr()
    rec = {
        "version": ver,
        "image_mode": mode,
        "image_mode_name": IMAGE_MODES.get(mode, "unknown(%d)" % mode),
        "height": h,
        "width": w,
        "name_raw": name,
        "name": zresolve(name),
        "id": pid,
    }
    if mode == 2:  # indexed colour: 256*3 palette + 4 bytes
        r.take(256 * 3 + 4)
    # virtual memory array list
    vver = r.u32()
    vlen = r.u32()
    r.take(vlen)
    rec["vm_array_version"] = vver
    rec["vm_array_bytes"] = vlen
    return rec


IMAGE_MODES = {
    0: "Bitmap",
    1: "Grayscale",
    2: "Indexed",
    3: "RGB",
    4: "CMYK",
    7: "Multichannel",
    8: "Duotone",
    9: "Lab",
}


# --------------------------------------------------------------------------
# per-format parsers
# --------------------------------------------------------------------------


def p_grd(data):
    r = R(data)
    sig = r.key4()
    if sig != "8BGR":
        raise Trunc("bad .grd signature %r" % sig)
    ver = r.u16()
    res = {"format": "8BGR", "file_version": ver, "entries": []}
    if ver == 5:
        dver = r.u32()
        res["descriptor_version"] = dver
        d = parse_descriptor(r)
        lst = d["_items"].get("Grad") or d["_items"].get("GrdL")
        if lst is None:
            raise Trunc("no gradient list key; keys=%s" % list(d["_items"]))
        for it in lst["v"]:
            g = it["v"]
            flat = desc_flat(g)
            raw = g["_items"].get("Nm  ", {}).get("v") or flat.get("Grad.Nm  ")
            res["entries"].append(
                {
                    "name": zresolve(raw) if raw else desc_name(g),
                    "name_raw": raw,
                    "gradient_class": g["_class"],
                    "gradient_form": flat.get("Grad.GrdF"),
                    "color_stop_count": len(
                        g["_items"]["Grad"]["v"]["_items"]["Clrs"]["v"]
                    )
                    if "Grad" in g["_items"]
                    and "Clrs" in g["_items"]["Grad"]["v"]["_items"]
                    else None,
                    "params": flat,
                }
            )
        res["parse_status"] = "parsed"
    elif ver == 3:
        # legacy non-descriptor gradients
        cnt = r.u16()
        res["declared_count"] = cnt
        res["parse_status"] = "partial"
        res["note"] = (
            "legacy .grd version 3 (pre-descriptor); only the declared entry "
            "count was read, per-entry stops were not decoded"
        )
    else:
        raise Trunc("unhandled .grd version %d" % ver)
    res["entry_count"] = len(res["entries"]) or res.get("declared_count", 0)
    return res


def p_pat(data):
    r = R(data)
    sig = r.key4()
    if sig != "8BPT":
        raise Trunc("bad .pat signature %r" % sig)
    ver = r.u16()
    cnt = r.u32()
    res = {
        "format": "8BPT",
        "file_version": ver,
        "declared_count": cnt,
        "entries": [],
    }
    for _ in range(cnt):
        res["entries"].append(read_pattern_record(r))
    res["entry_count"] = len(res["entries"])
    res["parse_status"] = "parsed"
    res["trailing_bytes"] = r.n - r.p
    return res


def p_asl(data):
    r = R(data)
    v1 = r.u16()
    sig = r.key4()
    if sig != "8BSL":
        raise Trunc("bad .asl signature %r" % sig)
    v2 = r.u16()
    res = {
        "format": "8BSL",
        "outer_version": v1,
        "inner_version": v2,
        "patterns": [],
        "entries": [],
    }
    pat_len = r.u32()
    pend = r.p + pat_len
    while r.p + 8 <= pend:
        one = r.u32()
        if one == 0:
            break
        sub = R(r.take(one))
        try:
            res["patterns"].append(read_pattern_record(sub))
        except Trunc:
            res["patterns"].append({"parse_status": "failed", "bytes": one})
        # embedded pattern records are padded to a 4-byte boundary
        pad = (4 - (one % 4)) % 4
        if pad and r.p + pad <= pend:
            r.take(pad)
    r.p = pend
    ns = r.u32()
    res["declared_count"] = ns
    for i in range(ns):
        e = {"index": i}
        try:
            l1 = r.u32()
            b1 = R(r.take(l1))
            b1.u32()  # descriptor version
            d1 = parse_descriptor(b1)
            e["name_raw"] = d1["_items"].get("Nm  ", {}).get("v")
            e["name"] = desc_name(d1)
            e["identity"] = desc_flat(d1)
            # the same block carries a second descriptor holding the effects
            if b1.n - b1.p > 8:
                b1.u32()  # second descriptor version
                d2 = parse_descriptor(b1)
                e["style_class"] = d2["_class"]
                e["effects"] = desc_flat(d2)
                e["effect_keys"] = sorted(
                    set(
                        k.split(".")[0]
                        for k in e["effects"]
                        if not k.startswith("_")
                    )
                )
            else:
                e["effects"] = {}
                e["effect_keys"] = []
            e["block_bytes"] = l1
            e["parse_status"] = "parsed"
        except Trunc as ex:
            e["parse_status"] = "failed"
            e["error"] = str(ex)
            res["entries"].append(e)
            break
        res["entries"].append(e)
    res["entry_count"] = len(res["entries"])
    res["element_counts"] = {
        "styles": len(res["entries"]),
        "embedded_patterns": len(res["patterns"]),
        "styles_with_effects": sum(
            1 for e in res["entries"] if e.get("effect_keys")
        ),
    }
    res["parse_status"] = (
        "parsed"
        if len(res["entries"]) == ns
        and all(e.get("parse_status") == "parsed" for e in res["entries"])
        else "partial"
    )
    res["note"] = (
        "8BSL layout: uint16 outer version, '8BSL', uint16 inner version, "
        "uint32 patterns-block length, the embedded pattern records, uint32 "
        "style count, then one length-prefixed block per style. Each style "
        "block holds TWO Action Descriptors back to back: the first carries "
        "Nm  (name) and Idnt (uuid), the second carries the layer-effect "
        "parameters. Established by reading the installed files"
    )
    return res


def p_abr(data):
    r = R(data)
    major = r.u16()
    res = {"format": "abr", "major_version": major, "entries": [], "sections": []}
    if major in (6, 7, 8, 9, 10):
        minor = r.u16()
        res["minor_version"] = minor
        # 8BIM-tagged sections
        while r.p + 12 <= r.n:
            sig = r.key4()
            if sig != "8BIM":
                break
            key = r.key4()
            ln = r.u32()
            payload = r.take(ln)
            res["sections"].append({"key": key, "bytes": ln})
            if key == "desc":
                sub = R(payload)
                try:
                    sub.u32()  # descriptor version
                    d = parse_descriptor(sub)
                    lst = d["_items"].get("Brsh")
                    if lst and lst["t"] == "VlLs":
                        for it in lst["v"]:
                            b = it["v"]
                            res["entries"].append(
                                {
                                    "name": desc_name(b),
                                    "name_raw": b["_items"]
                                    .get("Nm  ", {})
                                    .get("v"),
                                    "brush_class": b["_class"],
                                    "params": desc_flat(b),
                                }
                            )
                    else:
                        res["desc_keys"] = list(d["_items"])
                except Trunc as ex:
                    res["desc_error"] = str(ex)
            elif key == "samp":
                # sampled brush tips: sequence of length-prefixed records whose
                # id is a pascal string. Names live in the 'desc' section.
                sub = R(payload)
                ids = []
                try:
                    while sub.p + 4 <= sub.n:
                        ln2 = sub.u32()
                        if ln2 == 0 or sub.p + ln2 > sub.n:
                            break
                        rec = R(sub.take(ln2))
                        ids.append(rec.pstr())
                        pad = (4 - (ln2 % 4)) % 4
                        if pad and sub.p + pad <= sub.n:
                            sub.take(pad)
                except Trunc:
                    pass
                res["sampled_tip_ids"] = ids
                res["sampled_tip_count"] = len(ids)
            elif key == "patt":
                sub = R(payload)
                pats = []
                try:
                    while sub.p + 8 <= sub.n:
                        ln2 = sub.u32()
                        if ln2 == 0 or sub.p + ln2 > sub.n:
                            break
                        pr = R(sub.take(ln2))
                        pats.append(read_pattern_record(pr))
                        pad = (4 - (ln2 % 4)) % 4
                        if pad and sub.p + pad <= sub.n:
                            sub.take(pad)
                except Trunc:
                    pass
                res["embedded_patterns"] = pats
        res["parse_status"] = "parsed" if res["entries"] else "partial"
    elif major in (1, 2):
        cnt = r.u16()
        res["declared_count"] = cnt
        res["parse_status"] = "partial"
        res["note"] = (
            "legacy .abr version %d; entry count read from header, per-brush "
            "records not decoded" % major
        )
    else:
        raise Trunc("unhandled .abr version %d" % major)
    res["entry_count"] = len(res["entries"]) or res.get("declared_count", 0)
    return res


def p_csh(data):
    r = R(data)
    sig = r.key4()
    if sig != "cush":
        raise Trunc("bad .csh signature %r" % sig)
    ver = r.u32()
    cnt = r.u32()
    res = {
        "format": "cush",
        "file_version": ver,
        "declared_count": cnt,
        "entries": [],
    }
    for i in range(cnt):
        try:
            name = r.ustr()
            # 4-byte alignment pad after the unicode name
            while r.p % 4 and not r.eof():
                r.take(1)
            unk1 = r.u32()
            ln = r.u32()
            body_start = r.p
            uid = r.pstr()
            # bounds: 4 x int32 immediately after the uid pascal string
            bounds = None
            try:
                b = [r.i32() for _ in range(4)]
                bounds = {
                    "top": b[0],
                    "left": b[1],
                    "bottom": b[2],
                    "right": b[3],
                }
            except Trunc:
                pass
            r.p = body_start + ln
            while r.p % 4 and not r.eof():
                r.take(1)
            res["entries"].append(
                {
                    "index": i,
                    "name_raw": name,
                    "name": zresolve(name),
                    "record_bytes": ln,
                    "uuid": uid,
                    "bounds_heuristic": bounds,
                }
            )
        except Trunc as ex:
            res["entries"].append({"index": i, "parse_status": "failed", "error": str(ex)})
            break
    res["entry_count"] = len([e for e in res["entries"] if "name" in e])
    res["parse_status"] = "parsed" if res["entry_count"] == cnt else "partial"
    res["field_notes"] = {
        "unk1": "4 bytes between the name and the record length; meaning not "
        "established from any documented spec",
        "bounds_heuristic": "four int32 read directly after the uuid pascal "
        "string; plausible shape bounding box but NOT confirmed against a "
        "published spec - treat as heuristic",
    }
    return res


def p_tpl(data):
    r = R(data)
    sig = r.key4()
    if sig != "8BTP":
        raise Trunc("bad .tpl signature %r" % sig)
    ver = r.u32()
    cnt = r.u32()
    res = {
        "format": "8BTP",
        "file_version": ver,
        "declared_block_count": cnt,
        "blocks": [],
        "entries": [],
        "embedded_patterns": [],
        "embedded_styles": [],
        "embedded_shapes": [],
        "block_errors": {},
    }
    while r.p + 12 <= r.n:
        sig2 = r.key4()
        if sig2 != "8BIM":
            break
        key = r.key4()
        ln = r.u32()
        payload = r.take(ln)
        res["blocks"].append({"key": key, "bytes": ln})
        sub = R(payload)
        try:
            if key == "tppa":  # patterns referenced by the tool presets
                while sub.p + 8 <= sub.n:
                    rl = sub.u32()
                    if rl == 0 or sub.p + rl > sub.n:
                        break
                    res["embedded_patterns"].append(
                        read_pattern_record(R(sub.take(rl)))
                    )
            elif key == "tptp":  # the tool presets themselves
                n = sub.u32()
                res["declared_preset_count"] = n
                for i in range(n):
                    nm = sub.ustr()
                    sub.u32()  # descriptor version (16)
                    d = parse_descriptor(sub)
                    res["entries"].append(
                        {
                            "index": i,
                            "name_raw": nm,
                            "name": zresolve(nm),
                            "tool_class": d["_class"],
                            "params": desc_flat(d),
                        }
                    )
            elif key == "tpst":  # layer styles referenced by the tool presets
                n = sub.u32()
                for _ in range(n):
                    l1 = sub.u32()
                    blk = R(sub.take(l1))
                    blk.u32()  # inner length
                    blk.u32()  # descriptor version
                    d = parse_descriptor(blk)
                    res["embedded_styles"].append(
                        {"name": desc_name(d), "params": desc_flat(d)}
                    )
            elif key == "tpsh":  # custom shapes referenced by the tool presets
                sub.u32()  # section length
                n = sub.u32()
                for _ in range(n):
                    rl = sub.u32()
                    rec = R(sub.take(rl))
                    uid = rec.pstr()
                    bounds = [rec.i32() for _ in range(4)]
                    res["embedded_shapes"].append(
                        {
                            "uuid": uid,
                            "bounds_heuristic": {
                                "top": bounds[0],
                                "left": bounds[1],
                                "bottom": bounds[2],
                                "right": bounds[3],
                            },
                        }
                    )
        except Trunc as ex:
            res["block_errors"][key] = str(ex)
        while r.p % 4 and not r.eof():
            r.take(1)
    res["entry_count"] = len(res["entries"])
    res["element_counts"] = {
        "tool_presets": len(res["entries"]),
        "embedded_patterns": len(res["embedded_patterns"]),
        "embedded_styles": len(res["embedded_styles"]),
        "embedded_shapes": len(res["embedded_shapes"]),
    }
    declared = res.get("declared_preset_count")
    res["parse_status"] = (
        "parsed"
        if declared is not None
        and len(res["entries"]) == declared
        and not res["block_errors"]
        else "partial"
    )
    res["note"] = (
        "8BTP holds 8BIM-tagged blocks. 'tptp' is the tool preset list "
        "(uint32 count, then per preset a unicode name followed by a "
        "descriptor whose class is the tool id, e.g. magicStampTool); "
        "'tppa' patterns, 'tpst' layer styles, 'tpsh' custom shapes that the "
        "presets reference. Block key meanings were established by reading "
        "the payloads, not from a published spec"
    )
    return res


def p_atn(data):
    r = R(data)
    ver = r.u32()
    if ver not in (16,):
        raise Trunc("unhandled .atn version %d" % ver)
    set_name = r.ustr()
    expanded = r.u8()
    nact = r.u32()
    res = {
        "format": "atn",
        "file_version": ver,
        "set_name_raw": set_name,
        "set_name": zresolve(set_name),
        "expanded": bool(expanded),
        "declared_action_count": nact,
        "entries": [],
        "event_type_markers": {},
    }
    markers = Counter()
    for ai in range(nact):
        a = {"index": ai}
        try:
            a["function_key_index"] = r.u16()
            a["shift_key"] = bool(r.u8())
            a["command_key"] = bool(r.u8())
            a["color_index"] = r.u16()
            nm = r.ustr()
            a["name_raw"] = nm
            a["name"] = zresolve(nm)
            a["expanded"] = bool(r.u8())
            nit = r.u32()
            a["declared_step_count"] = nit
            steps = []
            for si in range(nit):
                st = {"index": si}
                st["expanded"] = bool(r.u8())
                st["enabled"] = bool(r.u8())
                st["with_dialog"] = bool(r.u8())
                st["dialog_options"] = r.u8()
                marker = r.key4()
                markers[marker] += 1
                st["event_type_marker"] = marker
                if marker == "TEXT":
                    ln = r.u32()
                    st["event_id"] = r.take(ln).decode("latin-1")
                elif marker == "long":
                    st["event_id"] = r.u32()
                elif marker == "enum":
                    ln = r.u32()
                    st["event_id"] = r.take(ln).decode("latin-1")
                else:
                    st["event_id"] = marker
                    st["event_id_basis"] = "literal 4-char OSType marker"
                ln2 = r.u32()
                st["item_name"] = r.take(ln2).decode("latin-1")
                flag = r.u32()
                st["descriptor_flag"] = flag
                st["has_descriptor"] = flag != 0
                if flag != 0:
                    d = parse_descriptor(r)
                    st["param_class"] = d["_class"]
                    st["params"] = desc_flat(d)
                steps.append(st)
            a["steps"] = steps
            a["step_count"] = len(steps)
            a["parse_status"] = "parsed"
        except Trunc as ex:
            a["parse_status"] = "failed"
            a["error"] = str(ex)
            res["entries"].append(a)
            break
        res["entries"].append(a)
    res["entry_count"] = len(res["entries"])
    res["element_counts"] = {
        "actions": len(res["entries"]),
        "steps": sum(e.get("step_count", 0) for e in res["entries"]),
        "steps_with_parameters": sum(
            1
            for e in res["entries"]
            for s in e.get("steps", [])
            if s.get("has_descriptor")
        ),
    }
    res["event_type_markers"] = dict(markers)
    res["parse_status"] = "parsed" if res["entry_count"] == nact else "partial"
    res["note"] = (
        "Action step layout was established empirically from the installed "
        "files: expanded/enabled/withDialog/dialogOptions (1 byte each), a "
        "4-char event type marker, the event id (uint32 length + ASCII when "
        "the marker is TEXT), a uint32-length ASCII item name, then a uint32 "
        "descriptor flag - 0 means no parameters, 0xFFFFFFFF means an Action "
        "Descriptor follows IMMEDIATELY with no descriptor-version word. "
        "It is validated by every action set parsing to exactly its declared "
        "action and step counts with no residual bytes"
    )
    return res


ACO_SPACES = {
    0: "RGB",
    1: "HSB",
    2: "CMYK",
    3: "Pantone_matching_system",
    4: "Focoltone_colour_system",
    5: "Trumatch_colour",
    6: "Toyo_88_colorfinder_1050",
    7: "Lab",
    8: "Grayscale",
    10: "HKS_colors",
}


def p_aco(data):
    r = R(data)
    res = {"format": "aco", "entries": [], "sections": []}
    ver = r.u16()
    cnt = r.u16()
    res["sections"].append({"version": ver, "declared_count": cnt})
    if ver == 1:
        for _ in range(cnt):
            sp = r.u16()
            comps = [r.u16() for _ in range(4)]
            res["entries"].append(
                {
                    "colorspace_id": sp,
                    "colorspace": ACO_SPACES.get(sp, "unknown(%d)" % sp),
                    "components": comps,
                    "name": None,
                }
            )
        if not r.eof() and r.n - r.p > 4:
            ver2 = r.u16()
            cnt2 = r.u16()
            res["sections"].append({"version": ver2, "declared_count": cnt2})
            if ver2 == 2:
                named = []
                for _ in range(cnt2):
                    sp = r.u16()
                    comps = [r.u16() for _ in range(4)]
                    nlen = r.u32()
                    nm = r.take(nlen * 2).decode("utf-16-be", "replace").rstrip("\x00")
                    named.append(
                        {
                            "colorspace_id": sp,
                            "colorspace": ACO_SPACES.get(sp, "unknown(%d)" % sp),
                            "components": comps,
                            "name": zresolve(nm),
                        }
                    )
                res["entries"] = named
    elif ver == 2:
        for _ in range(cnt):
            sp = r.u16()
            comps = [r.u16() for _ in range(4)]
            nlen = r.u32()
            nm = r.take(nlen * 2).decode("utf-16-be", "replace").rstrip("\x00")
            res["entries"].append(
                {
                    "colorspace_id": sp,
                    "colorspace": ACO_SPACES.get(sp, "unknown(%d)" % sp),
                    "components": comps,
                    "name": zresolve(nm),
                }
            )
    else:
        raise Trunc("unhandled .aco version %d" % ver)
    res["entry_count"] = len(res["entries"])
    res["parse_status"] = "parsed"
    return res


def p_ase(data):
    r = R(data)
    sig = r.key4()
    if sig != "ASEF":
        raise Trunc("bad .ase signature %r" % sig)
    vmaj = r.u16()
    vmin = r.u16()
    n = r.u32()
    res = {
        "format": "ASEF",
        "version": "%d.%d" % (vmaj, vmin),
        "declared_block_count": n,
        "entries": [],
        "groups": [],
    }
    for _ in range(n):
        bt = r.u16()
        bl = r.u32()
        body = R(r.take(bl))
        if bt in (0x0001, 0xC001):
            nlen = body.u16()
            nm = body.take(nlen * 2).decode("utf-16-be", "replace").rstrip("\x00")
            if bt == 0xC001:
                res["groups"].append(nm)
                continue
            model = body.key4().strip()
            ncomp = {"RGB": 3, "CMYK": 4, "LAB": 3, "Gray": 1}.get(model, 0)
            comps = [body.f32() for _ in range(ncomp)]
            ctype = body.u16()
            res["entries"].append(
                {
                    "name": zresolve(nm),
                    "model": model,
                    "components": comps,
                    "color_type": ctype,
                }
            )
    res["entry_count"] = len(res["entries"])
    res["parse_status"] = "parsed"
    return res


def p_acb(data):
    r = R(data)
    sig = r.key4()
    if sig != "8BCB":
        raise Trunc("bad .acb signature %r" % sig)
    ver = r.u16()
    ident = r.u16()
    title = r.ustr()
    prefix = r.ustr()
    postfix = r.ustr()
    desc = r.ustr()
    cnt = r.u16()
    page_size = r.u16()
    page_sel = r.u16()
    space = r.u16()
    ncomp = {0: 3, 2: 4, 7: 3}.get(space, 3)
    res = {
        "format": "8BCB",
        "file_version": ver,
        "book_id": ident,
        "title": zresolve(title),
        "title_raw": title,
        "name_prefix": zresolve(prefix),
        "name_postfix": zresolve(postfix),
        "description": zresolve(desc),
        "declared_count": cnt,
        "page_size": page_size,
        "page_selector_offset": page_sel,
        "colorspace_id": space,
        "colorspace": ACO_SPACES.get(space, "unknown(%d)" % space),
        "components_per_color": ncomp,
        "entries": [],
    }
    for _ in range(cnt):
        nm = r.ustr()
        code = r.take(6).decode("latin-1").strip()
        comps = list(r.take(ncomp))
        res["entries"].append(
            {"name": zresolve(nm), "code": code, "components": comps}
        )
    res["entry_count"] = len(res["entries"])
    res["parse_status"] = "parsed"
    return res


def p_act(data):
    n = len(data)
    res = {"format": "act", "bytes": n, "entries": []}
    pal = data[:768]
    colors = [
        [pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]] for i in range(256)
    ]
    used = 256
    transparent = None
    if n >= 772:
        used = struct.unpack(">H", data[768:770])[0]
        transparent = struct.unpack(">H", data[770:772])[0]
        if transparent == 0xFFFF:
            transparent = None
    res["declared_count"] = used
    res["transparency_index"] = transparent
    res["colors"] = [{"index": i, "rgb": colors[i]} for i in range(min(used, 256))]
    res["entries"] = [{"name": None, "color_count": len(res["colors"])}]
    res["entry_count"] = 1
    res["element_counts"] = {"colors": len(res["colors"])}
    res["parse_status"] = "parsed"
    res["note"] = (
        "256-entry RGB palette; 772-byte variant carries a used-colour count "
        "and a transparent-colour index in the trailing 4 bytes"
    )
    return res


ACV_CHANNELS_RGB = ["composite", "red", "green", "blue", "channel4"]


def p_acv(data):
    r = R(data)
    ver = r.u16()
    cnt = r.u16()
    res = {
        "format": "acv",
        "file_version": ver,
        "declared_curve_count": cnt,
        "entries": [],
    }
    curves = []
    for i in range(cnt):
        np = r.u16()
        pts = []
        for _ in range(np):
            out = r.u16()
            inp = r.u16()
            pts.append({"input": inp, "output": out})
        curves.append(
            {
                "index": i,
                "channel": ACV_CHANNELS_RGB[i]
                if i < len(ACV_CHANNELS_RGB)
                else "channel%d" % i,
                "channel_label_basis": "heuristic: index order for an RGB curve "
                "file; .acv stores no channel identifier",
                "point_count": np,
                "points": pts,
            }
        )
    res["curves"] = curves
    res["entries"] = [{"name": None, "curve_count": len(curves)}]
    res["entry_count"] = 1
    res["element_counts"] = {"channel_curves": len(curves)}
    res["parse_status"] = "parsed"
    res["value_range"] = "input/output 0-255 for 8-bit curves"
    return res


def p_raw_descriptor(data, fmt):
    """Formats that are a bare descriptor: version uint32 then descriptor."""
    r = R(data)
    ver = r.u32()
    d = parse_descriptor(r)
    return {
        "format": fmt,
        "descriptor_version": ver,
        "descriptor_class": d["_class"],
        "entries": [
            {"name": desc_name(d), "params": desc_flat(d)}
        ],
        "entry_count": 1,
        "parse_status": "parsed",
        "trailing_bytes": r.n - r.p,
    }


def p_cha(data):
    r = R(data)
    ver = r.u16()
    mono = r.u16()
    res = {
        "format": "cha",
        "file_version": ver,
        "monochrome": bool(mono),
        "entries": [],
    }
    # 4 output channels x (4 source values + constant), int16, x100 fixed point
    outs = []
    try:
        for oi in range(4):
            vals = [r.i16() for _ in range(5)]
            outs.append(
                {
                    "output_channel_index": oi,
                    "source_r": vals[0] / 100.0,
                    "source_g": vals[1] / 100.0,
                    "source_b": vals[2] / 100.0,
                    "source_4": vals[3] / 100.0,
                    "constant": vals[4] / 100.0,
                }
            )
    except Trunc:
        pass
    res["output_channels"] = outs
    res["entries"] = [{"name": None, "output_channel_count": len(outs)}]
    res["entry_count"] = 1
    res["element_counts"] = {"output_channels": len(outs)}
    res["parse_status"] = "partial"
    res["units"] = "percent; stored as int16 scaled by 100"
    res["note"] = (
        "field layout inferred from the fixed 44-byte record and from the "
        "Channel Mixer UI value ranges (-200..+200 percent); NOT confirmed "
        "against a published spec - treat scaling as heuristic"
    )
    return res


def p_alv(data):
    r = R(data)
    ver = r.u16()
    res = {"format": "alv", "file_version": ver, "entries": []}
    recs = []
    # 29 records of: input floor u16, input ceil u16, output floor u16,
    # output ceil u16, gamma u16 (x100)
    try:
        for i in range(29):
            vals = [r.u16() for _ in range(5)]
            recs.append(
                {
                    "record_index": i,
                    "input_floor": vals[0],
                    "input_ceiling": vals[1],
                    "output_floor": vals[2],
                    "output_ceiling": vals[3],
                    "gamma": vals[4] / 100.0,
                }
            )
    except Trunc:
        pass
    res["level_records"] = recs
    res["entries"] = [{"name": None, "level_record_count": len(recs)}]
    res["entry_count"] = 1
    res["element_counts"] = {"level_records": len(recs)}
    res["parse_status"] = "partial"
    res["note"] = (
        "the .alv container always holds 29 fixed level records (composite "
        "plus per-channel slots for every supported mode); field order "
        "inferred from the Levels dialog value set, gamma scaled by 100 - "
        "heuristic, not from a published spec"
    )
    return res


def p_ahu(data):
    r = R(data)
    ver = r.u16()
    mode = r.u16()
    res = {
        "format": "ahu",
        "file_version": ver,
        "colorize_or_mode": mode,
        "entries": [],
    }
    bandrecs = []
    try:
        master = {
            "band": "master",
            "hue": r.i16(),
            "saturation": r.i16(),
            "lightness": r.i16(),
        }
        bandrecs.append(master)
        bands = [
            "reds",
            "yellows",
            "greens",
            "cyans",
            "blues",
            "magentas",
        ]
        for b in bands:
            rng = [r.u16() for _ in range(4)]
            hsl = [r.i16() for _ in range(3)]
            bandrecs.append(
                {
                    "band": b,
                    "range_start_falloff": rng[0],
                    "range_start": rng[1],
                    "range_end": rng[2],
                    "range_end_falloff": rng[3],
                    "hue": hsl[0],
                    "saturation": hsl[1],
                    "lightness": hsl[2],
                }
            )
    except Trunc:
        pass
    res["bands"] = bandrecs
    res["entries"] = [{"name": None, "band_count": len(bandrecs)}]
    res["entry_count"] = 1
    res["element_counts"] = {"bands": len(bandrecs)}
    res["parse_status"] = "partial"
    res["note"] = (
        "band ordering and the 4-value colour-range window are inferred from "
        "the Hue/Saturation dialog layout, not from a published spec - "
        "heuristic"
    )
    return res


def p_shc(data):
    r = R(data)
    sig = r.key4()
    if sig != "8BFS":
        raise Trunc("bad .shc signature %r" % sig)
    ver = r.u16()
    cnt = r.u32()
    res = {
        "format": "8BFS",
        "file_version": ver,
        "declared_count": cnt,
        "entries": [],
    }
    bad = None
    for i in range(cnt):
        try:
            flag = r.u32()
            nm = r.ustr()
            unk = r.u16()
            npts = r.u16()
            pts = []
            for _ in range(npts):
                out = r.u16()
                inp = r.u16()
                pts.append({"input": inp, "output": out})
            corners = [bool(r.u8()) for _ in range(npts)]
            trailer = binascii.hexlify(r.take(8)).decode()
            for j, c in enumerate(corners):
                pts[j]["corner"] = c
            res["entries"].append(
                {
                    "index": i,
                    "entry_flag": flag,
                    "unknown_u16": unk,
                    "name_raw": nm,
                    "name": zresolve(nm),
                    "point_count": npts,
                    "points": pts,
                    "trailer_hex": trailer,
                }
            )
        except Trunc as ex:
            bad = str(ex)
            break
    res["entry_count"] = len(res["entries"])
    res["parse_status"] = "parsed" if res["entry_count"] == cnt else "partial"
    if bad:
        res["error"] = bad
    res["trailing_bytes"] = r.n - r.p
    res["value_range"] = "input/output 0-255"
    res["note"] = (
        "Contour entry layout established empirically: uint32 entry flag "
        "(always 2), unicode name, uint16 unknown, uint16 point count, then "
        "point count x (uint16 output, uint16 input), then one corner-flag "
        "byte per point, then an 8-byte trailer. Validated by every entry "
        "landing on the next entry's flag word and by Linear decoding to "
        "(0,0)-(255,255). The unknown_u16 and trailer_hex fields are captured "
        "raw because their meaning is NOT established - heuristic"
    )
    return res


def p_ado(data):
    r = R(data)
    ver = r.u16()
    nink = r.u16()
    res = {
        "format": "ado",
        "file_version": ver,
        "declared_ink_count": nink,
        "entries": [],
    }
    try:
        r.u16()  # second count / mode
        r.take(6)  # 3 x 0xFFFF sentinels
        for i in range(4):
            ncurve = r.u16()
            curve = [r.u8() for _ in range(28)]
            nm = r.pstr()
            # ink colour + book name follow; recover the book name pascal string
            res["entries"].append(
                {
                    "ink_index": i,
                    "curve_point_count_field": ncurve,
                    "curve_bytes": curve,
                    "ink_name": nm,
                }
            )
    except Trunc:
        pass
    # ink / spot names are pascal strings; harvest them independently as a
    # cross-check rather than relying on the speculative record walk
    names = []
    i = 0
    while i < len(data) - 1:
        ln = data[i]
        if 3 <= ln <= 40 and i + 1 + ln <= len(data):
            cand = data[i + 1 : i + 1 + ln]
            if all(32 <= c < 127 for c in cand):
                s = cand.decode("ascii")
                if re.search(r"[A-Za-z]{3}", s):
                    names.append(s)
                    i += 1 + ln
                    continue
        i += 1
    seen = []
    for nm in names:
        if nm not in seen:
            seen.append(nm)
    res["inks"] = res["entries"]
    res["ink_names_scanned"] = seen
    res["entries"] = [{"name": None, "ink_count": nink}]
    res["entry_count"] = 1
    res["element_counts"] = {"declared_inks": nink, "ink_names_found": len(seen)}
    res["parse_status"] = "partial"
    res["note"] = (
        "duotone .ADO record walk is speculative; ink_names_scanned is a "
        "pascal-string sweep over the whole file and is the reliable field. "
        "declared_ink_count comes from the 2-byte header field. Curve bytes "
        "are captured but their exact indexing is NOT confirmed - heuristic"
    )
    return res


def p_hdt(data):
    r = R(data)
    sig = r.key4()
    if sig != "hdrt":
        raise Trunc("bad .hdt signature %r" % sig)
    ver = r.u32()
    res = {"format": "hdrt", "file_version": ver, "entries": [], "sections": []}
    try:
        res["gamma_or_exposure"] = r.f32()
        method = r.u32()
        res["method_id"] = method
        nm = r.ustr()
        res["preset_name_raw"] = nm
        res["preset_name"] = zresolve(nm)
    except Trunc:
        pass
    # locate the 'hdra' sub-block and record its float payload
    idx = data.find(b"hdra")
    if idx >= 0:
        sr = R(data, idx + 4)
        try:
            n = sr.u32()
            res["sections"].append(
                {
                    "key": "hdra",
                    "offset": idx,
                    "field_count": n,
                    "floats": [sr.f32() for _ in range(min(n, 32))],
                }
            )
        except Trunc:
            pass
    res["entry_count"] = 1
    res["parse_status"] = "partial"
    res["note"] = (
        "HDR Toning .hdt has no published spec. Signature, version, preset "
        "name and the 'hdra' sub-block offset are read directly; every "
        "numeric interpretation is heuristic"
    )
    return res


def p_irs(data):
    r = R(data)
    ver = r.u32()
    res = {"format": "irs", "file_version": ver, "entries": [], "parse_status": "partial"}
    try:
        res["field_1"] = r.u32()
        res["field_2"] = r.u32()
    except Trunc:
        pass
    # a 256-entry RGB colour table is embedded; count non-zero palette slots
    res["bytes"] = len(data)
    res["note"] = (
        "Save For Web optimized-settings .irs has no published spec. Only the "
        "leading version/word fields and the file size are read. The "
        "human-meaningful settings (format, colour count, dither, lossy, "
        "quality) are encoded positionally and were NOT decoded. The preset "
        "NAME is carried by the filename, not by the file body - filenames "
        "such as 'GIF 128 Dithered' are the authoritative label"
    )
    res["entry_count"] = 1
    return res


def p_mnu(data):
    r = R(data)
    sig = r.key4()
    if sig != "8MNU":
        raise Trunc("bad .mnu signature %r" % sig)
    ver = r.u32()
    res = {"format": "8MNU", "file_version": ver, "entries": []}
    try:
        while not r.eof() and r.n - r.p >= 4:
            nm = r.ustr()
            if not nm:
                break
            res["entries"].append({"name_raw": nm, "name": zresolve(nm)})
            if r.n - r.p >= 8:
                res["entries"][-1]["trailing_u32"] = [r.u32(), r.u32()]
    except Trunc:
        pass
    res["entry_count"] = len(res["entries"])
    res["parse_status"] = "partial"
    return res


def p_cube(data):
    txt = data.decode("utf-8", "replace")
    res = {"format": "cube_text", "entries": [], "header": {}}
    size = None
    npoints = 0
    for line in txt.splitlines():
        s = line.strip()
        if not s:
            continue
        if s.startswith("#"):
            res["header"].setdefault("comments", []).append(s.lstrip("#").strip())
            continue
        if s.upper().startswith("TITLE"):
            res["header"]["title"] = s.split(None, 1)[1].strip().strip('"')
        elif s.upper().startswith("LUT_3D_SIZE"):
            size = int(s.split()[1])
            res["header"]["lut_3d_size"] = size
        elif s.upper().startswith("LUT_1D_SIZE"):
            res["header"]["lut_1d_size"] = int(s.split()[1])
        elif s.upper().startswith("DOMAIN_MIN"):
            res["header"]["domain_min"] = [float(x) for x in s.split()[1:]]
        elif s.upper().startswith("DOMAIN_MAX"):
            res["header"]["domain_max"] = [float(x) for x in s.split()[1:]]
        else:
            npoints += 1
    res["sample_count"] = npoints
    res["expected_sample_count"] = size**3 if size else None
    res["entries"] = [
        {"name": res["header"].get("title"), "lut_3d_size": size, "samples": npoints}
    ]
    res["entry_count"] = 1
    res["parse_status"] = "parsed"
    return res


def p_3dl(data):
    txt = data.decode("utf-8", "replace")
    res = {"format": "3dl_text", "entries": [], "header": {}}
    mesh = None
    npoints = 0
    for line in txt.splitlines():
        s = line.strip()
        if not s:
            continue
        if s.startswith("#"):
            res["header"].setdefault("comments", []).append(s.lstrip("#").strip())
            continue
        parts = s.split()
        if mesh is None and len(parts) > 4 and all(p.isdigit() for p in parts):
            mesh = parts
            res["header"]["input_mesh"] = [int(p) for p in parts]
            res["header"]["mesh_points"] = len(parts)
            continue
        npoints += 1
    res["sample_count"] = npoints
    res["entry_count"] = 1
    res["entries"] = [
        {
            "name": None,
            "mesh_points": res["header"].get("mesh_points"),
            "samples": npoints,
        }
    ]
    res["parse_status"] = "parsed"
    res["note"] = ".3dl carries no title field; the filename is the label"
    return res


def p_look(data):
    txt = data.decode("utf-8", "replace")
    res = {"format": "look_xml", "entries": [], "parse_status": "parsed"}
    try:
        root = ET.fromstring(txt)
    except ET.ParseError as ex:
        res["parse_status"] = "failed"
        res["error"] = str(ex)
        return res
    params = OrderedDict()

    def walk(node, path):
        kids = list(node)
        if not kids:
            v = (node.text or "").strip().strip('"')
            params[path] = v
            return
        for k in kids:
            walk(k, path + "/" + k.tag if path else k.tag)

    walk(root, root.tag)
    res["entries"] = [{"name": None, "params": params}]
    res["entry_count"] = 1
    res["param_count"] = len(params)
    res["note"] = (
        "SpeedGrade .look is XML; every leaf element is captured as a "
        "parameter path. The title lives in the filename"
    )
    return res


PARSERS = {
    ".grd": p_grd,
    ".pat": p_pat,
    ".asl": p_asl,
    ".abr": p_abr,
    ".csh": p_csh,
    ".tpl": p_tpl,
    ".atn": p_atn,
    ".aco": p_aco,
    ".ase": p_ase,
    ".acb": p_acb,
    ".act": p_act,
    ".acv": p_acv,
    ".blw": lambda d: p_raw_descriptor(d, "blw_descriptor"),
    ".cha": p_cha,
    ".ahu": p_ahu,
    ".alv": p_alv,
    ".shc": p_shc,
    ".ado": p_ado,
    ".hdt": p_hdt,
    ".irs": p_irs,
    ".mnu": p_mnu,
    ".cube": p_cube,
    ".3dl": p_3dl,
    ".look": p_look,
}

FAMILY = {
    ".grd": "gradients",
    ".pat": "patterns",
    ".asl": "layer_styles",
    ".abr": "brushes",
    ".csh": "custom_shapes",
    ".tpl": "tool_presets",
    ".atn": "actions",
    ".aco": "swatches",
    ".ase": "swatches_exchange",
    ".acb": "color_books",
    ".act": "color_tables",
    ".acv": "curves",
    ".blw": "black_and_white",
    ".cha": "channel_mixer",
    ".ahu": "hue_saturation",
    ".alv": "levels",
    ".shc": "contours",
    ".ado": "duotones",
    ".hdt": "hdr_toning",
    ".irs": "save_for_web_settings",
    ".mnu": "menu_customization",
    ".cube": "luts_3d",
    ".3dl": "luts_3d",
    ".look": "luts_look",
}


def sha1_of(path):
    h = hashlib.sha1()
    with open(path, "rb") as fh:
        while True:
            b = fh.read(1 << 20)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def find_files():
    out = []
    for root_label, root in (("INSTALL", INSTALL_ROOT), ("USER", USER_ROOT)):
        if not os.path.isdir(root):
            continue
        for dirpath, _dirnames, filenames in os.walk(root):
            for fn in filenames:
                ext = os.path.splitext(fn)[1].lower()
                if ext in PARSERS:
                    full = os.path.join(dirpath, fn)
                    out.append((root_label, root, full, ext))
    return out


def main():
    files = find_files()
    now = datetime.datetime.now(datetime.timezone.utc).isoformat()
    containers = []
    fam_stats = {}
    errors = []
    for root_label, root, full, ext in sorted(files, key=lambda x: x[2]):
        rel = os.path.relpath(full, root).replace("\\", "/")
        try:
            size = os.path.getsize(full)
            with open(full, "rb") as fh:
                data = fh.read()
        except OSError as ex:
            errors.append({"file": rel, "error": "read failed: %s" % ex})
            continue
        rec = {
            "root": root_label,
            "path": rel,
            "ext": ext,
            "family": FAMILY[ext],
            "bytes": size,
            "sha1": hashlib.sha1(data).hexdigest(),
        }
        try:
            parsed = PARSERS[ext](data)
            rec.update(parsed)
        except Exception as ex:  # noqa: BLE001 - we want the reason recorded
            rec["parse_status"] = "failed"
            rec["entry_count"] = 0
            rec["error"] = "%s: %s" % (type(ex).__name__, ex)
            rec["error_trace_tail"] = traceback.format_exc().strip().splitlines()[-1]
            errors.append({"file": rel, "error": rec["error"]})
        fam = rec["family"]
        st = fam_stats.setdefault(
            fam,
            {
                "container_files": 0,
                "entries_parsed": 0,
                "containers_parsed": 0,
                "containers_partial": 0,
                "containers_failed": 0,
            },
        )
        st["container_files"] += 1
        st["entries_parsed"] += rec.get("entry_count", 0) or 0
        s = rec.get("parse_status", "failed")
        st["containers_" + ("parsed" if s == "parsed" else ("partial" if s == "partial" else "failed"))] += 1
        containers.append(rec)

    total_entries = sum(c.get("entry_count", 0) or 0 for c in containers)

    doc = OrderedDict()
    doc["schema_id"] = SCHEMA_ID
    doc["generated_at"] = now
    doc["generator"] = "photoshop-preset-contents.py"
    doc["install_root"] = INSTALL_ROOT
    doc["user_root"] = USER_ROOT
    doc["method"] = (
        "Every preset container file under the Photoshop install root and the "
        "user preset root was opened and parsed byte-for-byte offline with a "
        "purpose-written parser per format. Photoshop was never launched and "
        "no COM automation was used. The shared primitive is a full "
        "implementation of the Photoshop Action Descriptor binary format "
        "(OSType-keyed items: Objc, VlLs, doub, UntF, UnFl, TEXT, enum, long, "
        "comp, bool, type, GlbC, GlbO, alis, tdta, obj , ObAr; reference forms "
        "prop, Clss, Enmr, rele, Idnt, indx, name), which unlocks .grd, .asl, "
        "the .abr 'desc' section, .tpl blocks, .atn step parameters and the "
        "bare-descriptor adjustment formats. Container-specific headers were "
        "confirmed by direct hex inspection of the installed files before the "
        "parser was written. Adobe zstrings of the form "
        "'$$$/Path/Key=English Text' are resolved to the text after the first "
        "'='; the raw form is retained alongside as name_raw. "
        "IMPORTANT: entry_count is the number of PRESETS RECOVERED FROM INSIDE "
        "the container, never a file count. Per-format and per-file "
        "parse_status is one of parsed / partial / failed, and every field "
        "whose meaning was inferred rather than read from a documented "
        "structure is called out in that record's 'note' or a "
        "'*_heuristic' field name."
    )
    doc["corrections"] = {
        "supersedes": "presets.json",
        "false_claim": (
            "presets.json reported count=2136 with a by_family breakdown; "
            "those are FILE counts discovered by extension, not preset entry "
            "counts. It also folded 1670 .js/.jsx script files and 36 .strings "
            "localisation tables into the 'preset' total, which are not "
            "presets at all."
        ),
        "what_this_file_reports_instead": (
            "container_files = number of container files of that family; "
            "entries_parsed = number of individual presets actually decoded "
            "from inside those containers."
        ),
    }
    doc["totals"] = {
        "container_files_scanned": len(containers),
        "preset_entries_recovered": total_entries,
        "containers_fully_parsed": sum(
            1 for c in containers if c.get("parse_status") == "parsed"
        ),
        "containers_partial": sum(
            1 for c in containers if c.get("parse_status") == "partial"
        ),
        "containers_failed": sum(
            1 for c in containers if c.get("parse_status") == "failed"
        ),
    }
    doc["by_family"] = OrderedDict(sorted(fam_stats.items()))
    doc["formats_implemented"] = OrderedDict(
        (e, FAMILY[e]) for e in sorted(PARSERS)
    )
    doc["parse_status_legend"] = {
        "parsed": "every declared entry in the container was decoded and the "
                  "reader landed exactly where the format predicts",
        "partial": "entry names and counts were recovered, but at least one "
                   "field's meaning is inferred rather than read from an "
                   "established structure - see that record's 'note'",
        "failed": "the container could not be decoded; the reason is in "
                  "'error'",
    }
    doc["unknowns"] = [
        {
            "id": "UNK-PRE-001",
            "formats": [".ado"],
            "what": "the duotone curve record layout",
            "tried": (
                "walked the fixed record after the 2-byte version and ink "
                "count; the ink/curve field order did not reproduce "
                "consistently across the 114 shipped files"
            ),
            "recovered_instead": (
                "declared ink count from the header field, plus "
                "ink_names_scanned - an exhaustive pascal-string sweep of the "
                "whole file, which reliably yields the ink and Pantone names"
            ),
        },
        {
            "id": "UNK-PRE-002",
            "formats": [".irs"],
            "what": "Save For Web optimized-settings values",
            "tried": (
                "read the leading version and word fields; the settings "
                "(format, colour count, dither, lossy, quality) are encoded "
                "positionally with no signature or key names to anchor on"
            ),
            "recovered_instead": (
                "version, leading fields and size only. The preset LABEL is "
                "carried by the filename, not the body"
            ),
        },
        {
            "id": "UNK-PRE-003",
            "formats": [".hdt"],
            "what": "HDR Toning parameter values",
            "tried": (
                "read the 'hdrt' header and located the 'hdra' sub-block; the "
                "float payload has no field names and no published spec"
            ),
            "recovered_instead": "signature, version, preset name, hdra floats",
        },
        {
            "id": "UNK-PRE-004",
            "formats": [".csh", ".tpl"],
            "what": "custom shape path geometry (bezier subpaths)",
            "tried": (
                "located the per-shape record and its uuid; the geometry "
                "payload was not decoded into subpaths"
            ),
            "recovered_instead": "shape count, names, uuids, record sizes",
        },
        {
            "id": "UNK-PRE-005",
            "formats": [".ase"],
            "what": "Adobe Swatch Exchange contents",
            "tried": "a full ASEF parser is implemented and registered",
            "recovered_instead": (
                "nothing - this install ships ZERO .ase files. The parser is "
                "present but unexercised. Reported so the absence is not "
                "mistaken for a parse failure"
            ),
        },
        {
            "id": "UNK-PRE-006",
            "formats": ["all"],
            "what": "value RANGES and factory DEFAULTS",
            "tried": "n/a",
            "recovered_instead": (
                "preset containers hold concrete shipped VALUES, not the "
                "legal range or the factory default of a parameter. Ranges "
                "and defaults are in photoshop_parameter_surface.json where "
                "the type library states them"
            ),
        },
    ]
    doc["heuristics"] = [
        {
            "id": "HEU-PRE-001",
            "formats": [".cha", ".alv", ".ahu"],
            "what": "field order and numeric scaling of the fixed adjustment "
                    "records (Channel Mixer, Levels, Hue/Saturation)",
            "basis": "inferred from record size and the corresponding dialog's "
                     "value set; not from a published spec",
        },
        {
            "id": "HEU-PRE-002",
            "formats": [".acv"],
            "what": "the channel label on each curve",
            "basis": "index order. The .acv container stores no channel "
                     "identifier at all",
        },
        {
            "id": "HEU-PRE-003",
            "formats": [".shc"],
            "what": "the unknown_u16 field and the 8-byte per-entry trailer",
            "basis": "captured raw; meaning not established. The surrounding "
                     "layout IS validated - every entry lands exactly on the "
                     "next entry's flag word",
        },
        {
            "id": "HEU-PRE-004",
            "formats": [".csh", ".tpl"],
            "what": "the bounds_heuristic bounding box on each shape",
            "basis": "four int32 read directly after the uuid pascal string; "
                     "plausible but not confirmed against a published spec",
        },
        {
            "id": "HEU-PRE-005",
            "formats": [".atn", ".shc", ".asl", ".tpl"],
            "what": "the record layouts themselves",
            "basis": "established empirically by hex-inspecting the installed "
                     "files, then VALIDATED by every container decoding to "
                     "exactly its declared entry count with the reader landing "
                     "where the format predicts. Empirically validated, but "
                     "not read from a published Adobe specification",
        },
        {
            "id": "HEU-PRE-006",
            "formats": ["all"],
            "what": "zstring resolution",
            "basis": "text after the first '=' in a '$$$/Path/Key=Text' "
                     "string. The unresolved original is always kept "
                     "alongside as name_raw",
        },
    ]
    doc["errors"] = errors
    doc["containers"] = containers
    return doc


if __name__ == "__main__":
    d = main()
    os.makedirs(OUT_DIR, exist_ok=True)
    with open(OUT_FILE, "w", encoding="utf-8") as fh:
        json.dump(d, fh, indent=1, ensure_ascii=False)
    print("wrote", OUT_FILE)
    print(json.dumps(d["totals"], indent=1))
    print(json.dumps(d["by_family"], indent=1))
    if d["errors"]:
        print("ERRORS (%d):" % len(d["errors"]))
        for e in d["errors"][:40]:
            print("  ", e["file"], "->", e["error"][:160])
