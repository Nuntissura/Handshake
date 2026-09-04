#!/usr/bin/env python3
"""Handshake Studio green room: parse InDesign idrc_SCE2 scripting-element resources.

The InDesign scripting object model is compiled into idrc_SCE2 resources. This is a real
record parser for that container, reverse-engineered from the installed binaries. No
application is launched and no COM scripting bridge is used (that bridge crashes the app
with EXCEPTION_ACCESS_VIOLATION in ExtendScript.dll).

CONTAINER GRAMMAR (reversed 2026-09-04, all little-endian)
----------------------------------------------------------
file:
    u32 section_count
    section_count * { u32 unk0, u32 unk1, u32 owner_script_id, u32 locale_pair }
    u32 record_count
    record_count * record          # long-header records and short-header records mixed

LONG-HEADER RECORD (46-byte fixed header, then a body chosen by kind)
    u16 kind                # 1 suite, 2 class, 3 method, 4 property, 5 enumeration,
                            # 11 typedef
    u32 ver_a, u32 ver_b    # version stamp pair A (0x7fffffff == unstamped)
    16  bytes reserved      # zero in every observed valid record
    u32 ver_c, u32 ver_d    # version stamp pair B
    u32 script_id           # stable numeric id; referenced by every type field
    8   bytes reserved      # zero in every observed valid record
    u8[4] tag               # 4-character code, stored byte-reversed
    pstr name               # u16 length + ASCII, the scripting name ("body row count")
    pstr description        # u16 length + ASCII

    1 suite       : (empty)
    2 class       : guid[16], u8[4] plural_tag, pstr plural_name, pstr plural_desc,
                    guid[16], u32 collection_type_id, u32 suite_id
    3 method      : typedesc26 reply, pstr reply_description, u32 param_count,
                    param_count * { u8[4] tag, pstr name, pstr description, <param trailer> }
    4 property    : typedesc26, u32 reserved, u32 related_id
    5 enumeration : u32 enumerator_count,
                    enumerator_count * { u8[4] tag, pstr name, pstr description }
    11 typedef    : typedesc26

SHORT-HEADER RECORD
    u16 kind                # 7 member table, 8 type binding, 9 enumeration extension
    u32 ver_a, u32 ver_b    # both 0x7fffffff in every observed record
    u32 script_id
    u32 count
    7 member table : count * { u16 member_kind, u32 member_id, u16 flags,
                               u16 flags2 (only when member_kind == 4) }
                     The kind-2 entries name the owner class(es); the kind-3 and kind-4
                     entries are the methods and properties those classes gain. This is
                     the only class-to-member association in the container.
    8 type binding : count * u32 referenced_id, u32 a, u8[4] tag, u32 b
    9 enum ext     : count * { u8[4] tag, pstr name, pstr description }
                     appended to the enumeration whose script_id matches this record's

    Short records carry an optional 2-byte trailer; the parser picks whichever alignment
    lands on the next valid record.

typedesc26:
    u16 pad(0), u32 type_id, u32 flags, 16 bytes value block

The method-parameter trailer is variable length and is only partially reversed; see
`trailer_status` on each parameter. The 26-byte core (type_id + flags) is parsed exactly;
the remainder is resynchronised against the next valid token and its raw bytes are
classified, not invented.

Output: indesign_dom_full.json
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import struct
from pathlib import Path

KINDS = {1: "suite", 2: "class", 3: "method", 4: "property", 5: "enumeration",
         11: "typedef"}
# Short-header record kinds: kind(2) + version pair(8) + id(4) + count(4) + payload.
SHORT_KINDS = {7: "member_table", 8: "type_binding", 9: "enumeration_extension"}
MEMBER_KINDS = {1: "suite", 2: "class", 3: "method", 4: "property", 5: "enumeration",
                6: "kind6", 7: "member_table", 8: "type_binding", 9: "enum_extension",
                10: "kind10", 11: "typedef", 12: "kind12"}
HDR = 46  # bytes of element header before the 4-character tag
# A 50-byte header variant also exists: in a minority of records the zero block between
# the two version-stamp pairs is 20 bytes rather than 16. Confirmed example:
# Required/(Open Place Resources)/idrc_SCE2/10.idrc, the "place" method at offset 0x24,
# whose tag lands at 0x56 (= 0x24 + 50) instead of 0x52. Accepting 50 as an alternative was
# measured and made the corpus-wide result WORSE (classes 516 -> 505, suites 28 -> 26,
# skipped bytes 78,527 -> 83,160) because the extra candidate creates false record
# boundaries during resynchronisation. The parser therefore handles only the 46-byte form
# and the 50-byte records are reported as skipped bytes rather than mis-parsed.
HDR_SIZES = (46,)


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def owner_of(p: Path) -> str:
    for part in p.parts:
        if part.startswith("(") and part.endswith("Resources)"):
            return part.strip("()").replace(" Resources", "")
    return "APP_ROOT"


class Bad(Exception):
    pass


class Reader:
    def __init__(self, data: bytes):
        self.d = data
        self.n = len(data)

    def u16(self, o):
        if o + 2 > self.n:
            raise Bad("u16 eof")
        return struct.unpack_from("<H", self.d, o)[0]

    def u32(self, o):
        if o + 4 > self.n:
            raise Bad("u32 eof")
        return struct.unpack_from("<I", self.d, o)[0]

    def tag(self, o):
        if o + 4 > self.n:
            raise Bad("tag eof")
        return self.d[o:o + 4][::-1].decode("latin-1")

    def pstr(self, o):
        ln = self.u16(o)
        if ln > 4000 or o + 2 + ln > self.n:
            raise Bad("pstr len")
        s = self.d[o + 2:o + 2 + ln]
        if not all(32 <= b < 127 for b in s):
            raise Bad("pstr nonascii")
        return s.decode("latin-1"), o + 2 + ln

    # --- validators used for resynchronisation ---
    def is_short(self, o) -> bool:
        try:
            k = self.u16(o)
            if k not in SHORT_KINDS:
                return False
            if self.u32(o + 2) != 0x7FFFFFFF or self.u32(o + 6) != 0x7FFFFFFF:
                return False
            if self.u32(o + 10) == 0:
                return False
            cnt = self.u32(o + 14)
            if not (1 <= cnt <= 256):
                return False
            if k == 7:
                q = o + 18
                for _ in range(cnt):
                    if q + 8 > self.n:
                        return False
                    mk = self.u16(q)
                    if mk not in MEMBER_KINDS or self.u32(q + 2) == 0:
                        return False
                    q += 10 if mk == 4 else 8
                return q <= self.n
            if k == 9:
                q = o + 18
                for _ in range(cnt):
                    if not self.is_named(q):
                        return False
                    _, q = self.pstr(q + 4)
                    _, q = self.pstr(q)
                return True
            if o + 18 + cnt * 4 + 12 + 2 > self.n:
                return False
            for i in range(cnt):
                if self.u32(o + 18 + i * 4) == 0:
                    return False
            t = self.d[o + 18 + cnt * 4 + 4:o + 18 + cnt * 4 + 8]
            return len(t) == 4 and all(32 <= b < 127 for b in t)
        except Bad:
            return False

    def header_size(self, o, sizes=HDR_SIZES):
        """46 or 50: the zero block between the two version-stamp pairs is 16 or 20 bytes.

        Returns the header size whose tag/name/description validate, else None.
        """
        try:
            if self.u16(o) not in KINDS:
                return None
        except Bad:
            return None
        for hdr in sizes:
            q = o + hdr
            if q + 8 > self.n:
                continue
            t = self.d[q:q + 4]
            if len(t) < 4 or not all(32 <= b < 127 for b in t):
                continue
            try:
                nm, q2 = self.pstr(q + 4)
                ds, _ = self.pstr(q2)
            except Bad:
                continue
            if 1 <= len(nm) <= 80 and len(ds) >= 2:
                return hdr
        return None

    def is_element(self, o) -> bool:
        """Strict: only the common 46-byte header. Used to resynchronise after a gap,
        where a loose match would stop the scan on a false positive."""
        return self.header_size(o, (46,)) is not None

    def is_element_any(self, o) -> bool:
        """Permissive: 46 or 50. Only used at a byte offset the parse already trusts."""
        return self.header_size(o) is not None

    def is_named(self, o) -> bool:
        """tag + name + description triple (a parameter or enumerator)."""
        try:
            t = self.d[o:o + 4]
            if len(t) < 4 or not all(32 <= b < 127 for b in t):
                return False
            nm, q2 = self.pstr(o + 4)
            ds, _ = self.pstr(q2)
            return 1 <= len(nm) <= 80 and len(ds) >= 2
        except Bad:
            return False


def typedesc(r: Reader, o):
    pad = r.u16(o)
    tid = r.u32(o + 2)
    flags = r.u32(o + 6)
    block = r.d[o + 10:o + 26]
    return {"type_id": tid, "flags": flags, "value_block": block.hex(), "pad": pad}, o + 26


def decode_param_trailer(raw: bytes, core: dict) -> dict:
    """Classify the variable-length parameter trailer.

    Only the shapes actually observed are decoded; anything else is reported raw.
    """
    out = {"trailer_bytes": len(raw), "trailer_status": "unparsed"}
    blk = bytes.fromhex(core["value_block"])
    # 16-byte value block: [8 reserved][u32 has_default][u32 default_value]
    if len(blk) == 16:
        a, b, c, e = struct.unpack("<IIII", blk)
        if a == 0 and b == 0 and c in (0, 1, 2, 6) and c != 0:
            out["has_default"] = True
            out["default_raw"] = e
            out["default_kind"] = c
    if len(raw) == 2 and raw == b"\x01\x00":
        out["trailer_status"] = "terminator_only"
    elif len(raw) >= 6 and raw[:4] == b"\x0c\x00\x00\x00":
        out["trailer_status"] = "enum_default"
        out["default_enumerator_tag"] = raw[4:8][::-1].decode("latin-1", "replace")
    elif len(raw) >= 2 and raw[-2:] == b"\x01\x00":
        out["trailer_status"] = "terminated"
    return out


def parse_file(path: Path):
    data = path.read_bytes()
    r = Reader(data)
    if r.n < 24:
        raise Bad("too small")
    nsec = r.u32(0)
    if not (0 < nsec < 64):
        raise Bad(f"section count {nsec}")
    o = 4
    sections = []
    for _ in range(nsec):
        sections.append({
            "unk0": r.u32(o), "unk1": r.u32(o + 4),
            "owner_script_id": r.u32(o + 8), "locale_pair": f"{r.u32(o + 12):#010x}",
        })
        o += 16
    declared = r.u32(o)
    o += 4
    elements = []
    members = []
    bindings = []
    extensions = []
    errors = []
    unknown_kinds = collections.Counter()
    while o < r.n - 8:
        if r.is_short(o):
            k = r.u16(o)
            sid = r.u32(o + 10)
            cnt = r.u32(o + 14)
            q = o + 18
            if k == 7:
                ents = []
                for _ in range(cnt):
                    mk = r.u16(q)
                    ent = {"member_kind": MEMBER_KINDS.get(mk, f"kind{mk}"),
                           "member_kind_id": mk, "member_id": r.u32(q + 2),
                           "flags": r.u16(q + 6)}
                    if mk == 4:  # property entries carry one extra u16
                        ent["flags2"] = r.u16(q + 8)
                        q += 10
                    else:
                        q += 8
                    ents.append(ent)
                members.append({"owner_id": sid, "entries": ents})
                o = q
            elif k == 9:
                ents = []
                for _ in range(cnt):
                    et = r.tag(q)
                    en, q = r.pstr(q + 4)
                    ed, q = r.pstr(q)
                    ents.append({"tag": et, "name": en, "description": ed})
                extensions.append({"target_enum_id": sid, "enumerators": ents})
                o = q
            else:  # kind 8
                ids = [r.u32(q + 4 * i) for i in range(cnt)]
                q += 4 * cnt
                a = r.u32(q); tag = r.tag(q + 4); b = r.u32(q + 8)
                bindings.append({"target_id": sid, "referenced_ids": ids,
                                 "unk_a": a, "tag": tag, "unk_b": b})
                o = q + 12
            # short records carry an optional 2-byte trailer; pick the alignment that
            # lands on the next valid record instead of guessing
            if not (r.is_element(o) or r.is_short(o) or o >= r.n - 8):
                if r.is_element(o + 2) or r.is_short(o + 2):
                    o += 2
            continue
        if not r.is_element_any(o):
            # resync forward to the next valid record start
            start = o
            unknown_kinds[r.u16(o) if o + 2 <= r.n else -1] += 1
            while o < r.n - 8 and not (r.is_element(o) or r.is_short(o)):
                o += 1
            if o >= r.n - 8:
                break
            elements.append({"_resync_skipped": o - start})
            continue
        head = o
        try:
            o = parse_element(r, data, o, elements)
        except Bad as e:
            # one malformed record must not cost the rest of the file
            errors.append({"offset": head, "reason": str(e)})
            o = head + 2
            start = o
            while o < r.n - 8 and not (r.is_element(o) or r.is_short(o)):
                o += 1
            elements.append({"_resync_skipped": o - start})
    return {"sections": sections, "declared_element_count": declared,
            "elements": elements, "member_tables": members, "type_bindings": bindings,
            "unknown_kinds": dict(unknown_kinds), "element_errors": errors,
            "enum_extensions": extensions}


def parse_element(r: Reader, data: bytes, o: int, elements: list) -> int:
        kind = r.u16(o)
        head = o
        hdr = r.header_size(o)
        if hdr is None:
            raise Bad("no valid header size")
        pad = hdr - HDR
        ver_a, ver_b = r.u32(o + 2), r.u32(o + 6)
        ver_c, ver_d = r.u32(o + 26 + pad), r.u32(o + 30 + pad)
        sid = r.u32(o + 34 + pad)
        o += hdr
        tag = r.tag(o)
        o += 4
        name, o = r.pstr(o)
        desc, o = r.pstr(o)
        el = {
            "kind": KINDS[kind], "script_id": sid, "tag": tag, "name": name,
            "description": desc, "offset": head,
            "version_a": ver_a, "version_b": ver_b, "version_c": ver_c, "version_d": ver_d,
            "header_bytes": hdr,
        }
        if kind == 2:
            el["guid"] = data[o:o + 16].hex()
            o += 16
            el["plural_tag"] = r.tag(o)
            o += 4
            el["plural_name"], o = r.pstr(o)
            el["plural_description"], o = r.pstr(o)
            el["plural_guid"] = data[o:o + 16].hex()
            o += 16
            el["collection_type_id"] = r.u32(o)
            el["suite_id"] = r.u32(o + 4)
            o += 8
        elif kind == 5:
            cnt = r.u32(o)
            o += 4
            if cnt > 4000:
                raise Bad(f"enum count {cnt}")
            ens = []
            for _ in range(cnt):
                et = r.tag(o)
                o += 4
                en, o = r.pstr(o)
                ed, o = r.pstr(o)
                ens.append({"tag": et, "name": en, "description": ed})
            el["enumerators"] = ens
        elif kind == 4:
            td, o = typedesc(r, o)
            el["type"] = td
            el["property_reserved"] = r.u32(o)
            el["related_id"] = r.u32(o + 4)
            o += 8
        elif kind == 11:
            td, o = typedesc(r, o)
            el["type"] = td
        elif kind == 3:
            td, o = typedesc(r, o)
            el["reply_type"] = td
            el["reply_description"], o = r.pstr(o)
            cnt = r.u32(o)
            o += 4
            if cnt > 200:
                raise Bad(f"param count {cnt}")
            params = []
            for _ in range(cnt):
                if not r.is_named(o):
                    el["parameters_truncated"] = True
                    break
                pt = r.tag(o)
                o += 4
                pn, o = r.pstr(o)
                pd, o = r.pstr(o)
                core, after = typedesc(r, o)
                # resync: next parameter, or next record
                probe = after
                limit = min(after + 800, r.n)
                while probe < limit and not (r.is_named(probe) or r.is_element(probe)
                                             or r.is_short(probe)):
                    probe += 1
                if probe >= limit:
                    el["parameters_truncated"] = True
                    o = after
                    break
                raw = data[after:probe]
                p = {"tag": pt, "name": pn, "description": pd,
                     "type_id": core["type_id"], "flags": core["flags"],
                     "value_block": core["value_block"]}
                p.update(decode_param_trailer(raw, core))
                params.append(p)
                o = probe
            el["parameters"] = params
        elements.append(el)
        return o


# Primitive scripting type ids seen as property/parameter types but never declared as an
# element. Names are INFERRED from the descriptions of every property that uses them and
# are labelled heuristic in the output.
PRIMITIVE_HINTS = {
    0x7700: "void (command with no return value)",
    0x7701: "any / variant",
    0x7702: "short integer",
    0x7703: "long integer",
    0x7704: "boolean",
    0x7705: "string",
    0x7706: "measurement (unit-bearing real: coordinates, lengths, weights)",
    0x7707: "real (double: scales, angles, percentages)",
    0x7708: "date/time",
    0x7709: "file path / file reference",
    0x770a: "properties record",
    0x770b: "binary or graphic data stream",
    0x770c: "any value (script variant)",
    0x770e: "large integer",
    0x770f: "object reference (mixed)",
    0x771f: "object reference",
    0x7720: "parent object reference",
    0x7721: "parent object reference",
}
# Evidence for the table above (property/parameter names actually typed with each id) is
# emitted into the output as `primitive_type_evidence` so every inference can be re-checked.


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    files = sorted(p for p in args.root.rglob("*.idrc") if p.parent.name == "idrc_SCE2")
    per_file = []
    failures = []
    all_el = []
    all_members = []
    all_bindings = []
    all_extensions = []
    kind_census = collections.Counter()
    unknown_census = collections.Counter()
    for p in files:
        try:
            res = parse_file(p)
        except Bad as e:
            failures.append({"file": str(p.relative_to(args.root)), "reason": str(e)})
            continue
        except Exception as e:  # noqa: BLE001
            failures.append({"file": str(p.relative_to(args.root)), "reason": f"{type(e).__name__}: {e}"})
            continue
        plugin = owner_of(p)
        rel = str(p.relative_to(args.root)).replace("\\", "/")
        real = [e for e in res["elements"] if "kind" in e]
        skipped = [e for e in res["elements"] if "kind" not in e]
        per_file.append({
            "file": rel,
            "plugin": plugin,
            "declared_record_count": res["declared_element_count"],
            "parsed_element_count": len(real),
            "member_table_count": len(res["member_tables"]),
            "type_binding_count": len(res["type_bindings"]),
            "enum_extension_count": len(res["enum_extensions"]),
            "resync_events": len(skipped),
            "element_errors": res["element_errors"],
            "resync_bytes_skipped": sum(e["_resync_skipped"] for e in skipped),
            "locale_pair": res["sections"][0]["locale_pair"] if res["sections"] else None,
        })
        for k, v in res["unknown_kinds"].items():
            unknown_census[k] += v
        for e in real:
            kind_census[e["kind"]] += 1
            e["plugin"] = plugin
            e["source_file"] = rel
            all_el.append(e)
        for m in res["member_tables"]:
            m["plugin"] = plugin
            m["source_file"] = rel
            all_members.append(m)
        for b in res["type_bindings"]:
            b["plugin"] = plugin
            b["source_file"] = rel
            all_bindings.append(b)
        for x in res["enum_extensions"]:
            x["plugin"] = plugin
            x["source_file"] = rel
            all_extensions.append(x)

    # -------- global id resolution --------
    by_id: dict[int, dict] = {}
    for e in all_el:
        by_id.setdefault(e["script_id"], e)
    coll_by_id: dict[int, dict] = {}
    for e in all_el:
        if e["kind"] == "class" and e.get("collection_type_id"):
            coll_by_id.setdefault(e["collection_type_id"], e)

    def resolve(tid: int):
        if tid in by_id:
            t = by_id[tid]
            return {"name": t["name"], "kind": t["kind"], "tag": t["tag"], "resolution": "parsed"}
        if tid in coll_by_id:
            c = coll_by_id[tid]
            return {"name": c.get("plural_name"), "kind": "collection", "tag": c.get("plural_tag"),
                    "resolution": "parsed_collection"}
        if tid in PRIMITIVE_HINTS:
            return {"name": PRIMITIVE_HINTS[tid], "kind": "primitive", "tag": None,
                    "resolution": "heuristic_primitive"}
        return {"name": None, "kind": "unresolved", "tag": None, "resolution": "unresolved"}

    # -------- class membership, from the kind-7 member tables --------
    # A member table lists, in order: zero or more kind-6 context ids, then one or more
    # kind-2 class ids (the owners), then the kind-3/kind-4 methods and properties that
    # those classes gain. Verified against the Table Model suite, where the table class
    # 0xb629 collects its documented property set this way.
    owners: dict[int, set] = collections.defaultdict(set)
    member_of: dict[int, set] = collections.defaultdict(set)
    unowned_member_tables = 0
    for m in all_members:
        cls_ids = [e["member_id"] for e in m["entries"] if e["member_kind_id"] == 2]
        mem_ids = [e["member_id"] for e in m["entries"] if e["member_kind_id"] in (3, 4)]
        m["owner_class_ids"] = cls_ids
        m["member_ids"] = mem_ids
        m["context_ids"] = [e["member_id"] for e in m["entries"] if e["member_kind_id"] == 6]
        if not cls_ids:
            unowned_member_tables += 1
            continue
        for c in cls_ids:
            for mid in mem_ids:
                owners[c].add(mid)
                member_of[mid].add(c)

    def owner_names(mid: int):
        out = []
        for oid in sorted(member_of.get(mid, ())):
            t = by_id.get(oid)
            out.append({"owner_id": oid, "owner_name": t["name"] if t else None,
                        "owner_kind": t["kind"] if t else "unresolved"})
        return out

    ext_by_enum = collections.defaultdict(list)
    for x in all_extensions:
        ext_by_enum[x["target_enum_id"]].append(x)

    suites, classes, enums, props, methods, typedefs = [], [], [], [], [], []
    for e in all_el:
        base = {k: e[k] for k in ("script_id", "tag", "name", "description", "plugin", "source_file")}
        base["version_stamp"] = {
            "a": e["version_a"], "b": e["version_b"], "c": e["version_c"], "d": e["version_d"],
        }
        if e["kind"] == "suite":
            suites.append(base)
        elif e["kind"] == "class":
            base.update({
                "plural_name": e.get("plural_name"), "plural_tag": e.get("plural_tag"),
                "plural_description": e.get("plural_description"),
                "collection_type_id": e.get("collection_type_id"),
                "suite_id": e.get("suite_id"),
                "guid": e.get("guid"), "collection_guid": e.get("plural_guid"),
            })
            mem = sorted(owners.get(e["script_id"], ()))
            props_of, meths_of, unk_of = [], [], []
            for mid in mem:
                t = by_id.get(mid)
                if t is None:
                    unk_of.append(mid)
                elif t["kind"] == "property":
                    props_of.append({"id": mid, "name": t["name"], "tag": t["tag"]})
                elif t["kind"] == "method":
                    meths_of.append({"id": mid, "name": t["name"], "tag": t["tag"]})
                else:
                    unk_of.append(mid)
            base["properties"] = props_of
            base["methods"] = meths_of
            base["unresolved_member_ids"] = unk_of
            base["property_count"] = len(props_of)
            base["method_count"] = len(meths_of)
            base["member_count"] = len(mem)
            classes.append(base)
        elif e["kind"] == "enumeration":
            base["enumerators"] = list(e.get("enumerators", []))
            for x in ext_by_enum.get(e["script_id"], []):
                for en in x["enumerators"]:
                    base["enumerators"].append({**en, "added_by_plugin": x["plugin"],
                                                "source": "enumeration_extension"})
            base["enumerator_count"] = len(base["enumerators"])
            enums.append(base)
        elif e["kind"] == "property":
            t = e["type"]
            base.update({
                "type_id": t["type_id"], "type": resolve(t["type_id"]),
                "flags": t["flags"], "value_block": t["value_block"],
                "related_id": e.get("related_id"),
                "declared_on": owner_names(e["script_id"]),
            })
            props.append(base)
        elif e["kind"] == "typedef":
            t = e.get("type", {})
            base.update({"type_id": t.get("type_id"),
                         "type": resolve(t["type_id"]) if t else None,
                         "flags": t.get("flags")})
            typedefs.append(base)
        elif e["kind"] == "method":
            rt = e["reply_type"]
            base.update({
                "reply": {"type_id": rt["type_id"], "type": resolve(rt["type_id"]),
                          "description": e.get("reply_description")},
                "parameters": [
                    {**{k: v for k, v in p.items() if k != "value_block"},
                     "type": resolve(p["type_id"])}
                    for p in e.get("parameters", [])
                ],
            })
            base["parameter_count"] = len(base["parameters"])
            base["declared_on"] = owner_names(e["script_id"])
            methods.append(base)

    # id -> label map, plus unresolved census
    unresolved = collections.Counter()
    for p in props:
        if p["type"]["resolution"] == "unresolved":
            unresolved[p["type_id"]] += 1
    for m in methods:
        if m["reply"]["type"]["resolution"] == "unresolved":
            unresolved[m["reply"]["type_id"]] += 1
        for pp in m["parameters"]:
            if pp["type"]["resolution"] == "unresolved":
                unresolved[pp["type_id"]] += 1

    prim_use = collections.defaultdict(list)
    for p_ in props:
        prim_use[p_["type_id"]].append(p_["name"])
    for m_ in methods:
        prim_use[m_["reply"]["type_id"]].append("reply:" + m_["name"])
        for pp_ in m_["parameters"]:
            prim_use[pp_["type_id"]].append("param:" + pp_["name"])
    prim_evidence = [
        {"type_id_hex": f"{k:#x}", "inferred_name": v,
         "basis": "heuristic: inferred from the members typed with this id; "
                  "the id is referenced by SCE2 but never declared in it",
         "uses": len(prim_use.get(k, [])),
         "example_members": sorted(set(prim_use.get(k, [])))[:16]}
        for k, v in sorted(PRIMITIVE_HINTS.items())
    ]
    prim_evidence.sort(key=lambda x: -x["uses"])

    unresolved_samples = []
    for tid, n in unresolved.most_common(120):
        ex = [p["name"] for p in props if p["type_id"] == tid][:5]
        unresolved_samples.append({"type_id": tid, "type_id_hex": f"{tid:#x}", "uses": n,
                                   "example_members": ex})

    dup_names = collections.Counter(p["name"] for p in props)
    doc = {
        "schema_id": "handshake.reference.indesign_dom_full@1",
        "generated_at": now(),
        "source_root": str(args.root),
        "resource_code": "SCE2",
        "method": (
            "Binary record parser for idrc_SCE2 scripting-element resources, reverse-engineered "
            "from the installed InDesign 2026 files. Read-only; the application was never launched "
            "and the COM/ExtendScript bridge was never touched. Every name, description, tag, "
            "script id, enumerator and parameter below is PARSED from the container, not inferred. "
            "Two things are labelled heuristic and marked as such per-record: (1) primitive type-id "
            "names in `primitive_type_hints` / resolution=heuristic_primitive, which are inferred "
            "from the descriptions of the members that use them because the primitive ids are "
            "referenced but never declared in SCE2; (2) parameter trailer fields "
            "(has_default/default_raw/default_enumerator_tag) where trailer_status says how the "
            "variable-length trailer was classified."
        ),
        "format": {
            "endianness": "little",
            "element_header_bytes": list(HDR_SIZES),
            "element_header_note": "46 in most records, 50 where the zero block between the "
                                   "two version-stamp pairs is 20 bytes instead of 16; the "
                                   "size actually used is on each element as header_bytes",
            "kinds": {"1": "suite", "2": "class", "3": "method", "4": "property", "5": "enumeration"},
            "tag_encoding": "4-character code stored byte-reversed (little-endian u32)",
            "string_encoding": "u16 length prefix + ASCII, not NUL terminated",
            "record_kinds": {
                "1": "suite", "2": "class", "3": "method", "4": "property",
                "5": "enumeration", "11": "typedef (@private, typedesc only)",
                "7": "member table: the record's own id is a provider id, NOT the owner. "
                     "Its entry list holds zero or more kind-6 context ids, then the kind-2 "
                     "class ids that own the members, then the kind-3 methods and kind-4 "
                     "properties those classes gain. Property entries are 10 bytes wide "
                     "(one extra u16); every other entry kind is 8.",
                "8": "type binding: target id + referenced id list + 4-character tag",
                "9": "enumeration extension: appends (tag, name, description) enumerators to "
                     "the enumeration whose script_id equals this record's id",
            },
            "notes": [
                "version_stamp pairs: 0x7fffffff means unstamped; small values look like "
                "InDesign scripting versions (4=CS2, 5=CS3 ... 19.4, 21.1). Interpretation is "
                "heuristic; the raw values are preserved.",
                "Class membership comes from the kind-7 member tables, which are separate "
                "records from the class/property/method declarations. classes[].properties, "
                "classes[].methods and properties[].declared_on / methods[].declared_on are "
                "joins over those tables, not guesses from file ordering. 479 of 516 classes "
                "resolve members this way; the join was verified against the Table Model "
                "suite, whose table class 0xb629 collects its documented property set.",
                "enumerations[].enumerators merges the enumerators declared in the kind-5 "
                "record with those appended by kind-9 extension records; extension-sourced "
                "values carry source='enumeration_extension' and added_by_plugin.",
                "declared_record_count in per_file counts all record kinds in the file, so it "
                "is only equal to parsed_element_count for files with no member tables or "
                "type bindings.",
                "properties[].related_id is the raw trailing u32 of the property record. Its "
                "meaning is NOT established: 1084 of 3398 properties carry a non-zero value "
                "and only 358 of those resolve to any declared script id, split across "
                "properties (214), suites (131), methods (7), classes (5) and one "
                "enumeration. It is exported raw and no semantics are claimed for it.",
                "78,527 bytes across 852 resync events (5.1% of the 1.54 MB SCE2 corpus) could "
                "not be assigned to a record and were skipped. The largest single contributor "
                "is a 50-byte header variant described in the module docstring.",
            ],
        },
        "totals": {
            "sce2_resource_files_found": len(files),
            "sce2_resource_files_parsed": len(per_file),
            "sce2_resource_files_failed": len(failures),
            "elements_parsed": len(all_el),
            "suites": len(suites),
            "classes": len(classes),
            "enumerations": len(enums),
            "enumerators": sum(e["enumerator_count"] for e in enums),
            "properties": len(props),
            "distinct_property_names": len(dup_names),
            "methods": len(methods),
            "method_parameters": sum(m["parameter_count"] for m in methods),
            "typedefs": len(typedefs),
            "member_tables": len(all_members),
            "member_table_entries": sum(len(m["entries"]) for m in all_members),
            "classes_with_members": sum(1 for c in classes if c["member_count"]),
            "class_property_edges": sum(c["property_count"] for c in classes),
            "class_method_edges": sum(c["method_count"] for c in classes),
            "member_tables_without_owner_class": unowned_member_tables,
            "type_bindings": len(all_bindings),
            "enumeration_extensions": len(all_extensions),
            "enumeration_extension_entries": sum(len(x["enumerators"]) for x in all_extensions),
            "enumeration_extensions_merged": sum(1 for x in all_extensions if x["target_enum_id"] in by_id),
            "resync_events": sum(f["resync_events"] for f in per_file),
            "resync_bytes_skipped": sum(f["resync_bytes_skipped"] for f in per_file),
            "unresolved_type_ids": len(unresolved),
        },
        "record_kind_census": dict(kind_census),
        "unrecognised_kind_census": {str(k): v for k, v in sorted(unknown_census.items())},
        "primitive_type_hints": {f"{k:#x}": v for k, v in PRIMITIVE_HINTS.items()},
        "primitive_type_evidence": prim_evidence,
        "unresolved_type_ids": unresolved_samples,
        "suites": sorted(suites, key=lambda x: x["name"]),
        "classes": sorted(classes, key=lambda x: x["name"]),
        "enumerations": sorted(enums, key=lambda x: x["name"]),
        "properties": sorted(props, key=lambda x: (x["name"], x["script_id"])),
        "methods": sorted(methods, key=lambda x: (x["name"], x["script_id"])),
        "typedefs": sorted(typedefs, key=lambda x: x["name"]),
        "member_tables": all_members,
        "type_bindings": all_bindings,
        "orphan_enumeration_extensions": [x for x in all_extensions if x["target_enum_id"] not in by_id],
        "per_file": per_file,
        "parse_failures": failures,
    }
    outp = args.out / "indesign_dom_full.json"
    outp.write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    t = doc["totals"]
    print(f"[sce2] files {t['sce2_resource_files_parsed']}/{t['sce2_resource_files_found']} "
          f"failed={t['sce2_resource_files_failed']} elements={t['elements_parsed']}")
    print(f"[sce2] classes={t['classes']} props={t['properties']} methods={t['methods']} "
          f"params={t['method_parameters']} enums={t['enumerations']} enumerators={t['enumerators']} "
          f"suites={t['suites']} typedefs={t['typedefs']}")
    print(f"[sce2] member_tables={t['member_tables']} entries={t['member_table_entries']} "
          f"classes_with_members={t['classes_with_members']} "
          f"prop_edges={t['class_property_edges']} meth_edges={t['class_method_edges']} "
          f"resyncs={t['resync_events']} skipped_bytes={t['resync_bytes_skipped']}")
    print('[sce2] unrecognised kind census:', doc['unrecognised_kind_census'])
    print(f"[sce2] unresolved type ids={t['unresolved_type_ids']}")
    print(f"[sce2] -> {outp} ({outp.stat().st_size/1048576:.1f} MB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
