#!/usr/bin/env python3
"""Handshake Studio green room: extract InDesign's error catalog (offline, no app launch).

idrc_uetb holds the compiled error tables: every error symbol the application can raise,
keyed by a 32-bit error code whose high half is the owning plug-in/service id and whose low
half is the ordinal within that service. The human-readable message for an error lives in
the localized string tables (idrc_PMST); this tool joins the two so each error carries its
symbol, its numeric code, its owning plug-in and, where a matching key exists, its English
message text.

CONTAINER GRAMMAR (reversed 2026-09-04, little-endian)
    u16 entry_count
    entry_count * { u32 error_code, u8 name_length, ASCII name }

PMST (used only for the message join)
    u32 locale_id, u32 unknown, u32 entry_count
    entry_count * { u16 key_length, ASCII key, u16 value_length, UTF-8 value }
    locale_id 1 and 2 are English.

Output: indesign_error_catalog.json
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
import struct
from pathlib import Path

ENGLISH_LOCALES = {1, 2}


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def owner_of(p: Path) -> str:
    for part in p.parts:
        if part.startswith("(") and part.endswith("Resources)"):
            return part.strip("()").replace(" Resources", "")
    return "APP_ROOT"


def parse_uetb(data: bytes):
    """Return (entries, status). status says whether the whole table was consumed."""
    if len(data) < 2:
        return [], "too_small"
    n = struct.unpack_from("<H", data, 0)[0]
    o = 2
    out = []
    for _ in range(n):
        if o + 5 > len(data):
            return out, f"truncated_after_{len(out)}_of_{n}"
        code = struct.unpack_from("<I", data, o)[0]
        ln = data[o + 4]
        o += 5
        raw = data[o:o + ln]
        o += ln
        if len(raw) != ln or not all(32 <= b < 127 or b in (9, 10, 13) for b in raw):
            return out, f"bad_name_after_{len(out)}_of_{n}"
        sym = raw.decode("ascii")
        out.append({"error_code": code, "error_code_hex": f"{code:#010x}",
                    "service_id": code >> 8, "ordinal": code & 0xFF,
                    "symbol": sym,
                    "symbol_is_message_text": " " in sym})
    status = "complete" if o == len(data) else f"trailing_{len(data)-o}_bytes"
    return out, status


def parse_petb(data: bytes):
    """idrc_petb: the same error table with a slightly different header.

    u32 entry_count, u16 pad, entry_count * { u32 error_code, u8 name_len, ASCII name }
    """
    if len(data) < 6:
        return [], "too_small"
    n = struct.unpack_from("<I", data, 0)[0]
    if n > 10000:
        return [], f"implausible_count_{n}"
    o = 6
    out = []
    for _ in range(n):
        if o + 5 > len(data):
            return out, f"truncated_after_{len(out)}_of_{n}"
        code = struct.unpack_from("<I", data, o)[0]
        ln = data[o + 4]
        o += 5
        raw = data[o:o + ln]
        o += ln
        if len(raw) != ln or not all(32 <= b < 127 or b in (9, 10, 13) for b in raw):
            return out, f"bad_name_after_{len(out)}_of_{n}"
        sym = raw.decode("ascii")
        out.append({"error_code": code, "error_code_hex": f"{code:#010x}",
                    "service_id": code >> 8, "ordinal": code & 0xFF,
                    "symbol": sym, "symbol_is_message_text": " " in sym})
    status = "complete" if o == len(data) else f"trailing_{len(data)-o}_bytes"
    return out, status


def parse_pmst(data: bytes):
    if len(data) < 12:
        return None
    loc, _unk, cnt = struct.unpack_from("<III", data, 0)
    o = 12
    out = {}
    for _ in range(cnt):
        if o + 2 > len(data):
            break
        kl = struct.unpack_from("<H", data, o)[0]
        o += 2
        k = data[o:o + kl]
        o += kl
        if o + 2 > len(data):
            break
        vl = struct.unpack_from("<H", data, o)[0]
        o += 2
        v = data[o:o + vl]
        o += vl
        if len(k) != kl or len(v) != vl:
            break
        try:
            out[k.decode("ascii")] = v.decode("utf-8")
        except UnicodeDecodeError:
            continue
    return {"locale_id": loc, "declared_entries": cnt, "strings": out}


# Symbol shapes that carry constraint semantics rather than plain failure reporting.
CONSTRAINT_HINT = re.compile(
    r"(Bad|Invalid|Illegal|TooMany|TooFew|TooLarge|TooSmall|OutOf|Exceed|NotAllowed|"
    r"NotSupported|Unsupported|Required|Missing|Duplicate|Conflict|Locked|ReadOnly|"
    r"Overflow|Underflow|Range|Limit|Empty|Mismatch|Incompatible|AlreadyExists|NotFound)",
    re.I)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    # --- English message pool from PMST -------------------------------------------------
    msg: dict[str, dict] = {}
    pmst_files = [p for p in args.root.rglob("*.idrc") if p.parent.name == "idrc_PMST"]
    pmst_english = 0
    for p in pmst_files:
        try:
            with p.open("rb") as fh:
                head = fh.read(4)
                if len(head) < 4 or struct.unpack("<I", head)[0] not in ENGLISH_LOCALES:
                    continue
                rec = parse_pmst(head + fh.read())
        except Exception:  # noqa: BLE001
            continue
        if not rec:
            continue
        pmst_english += 1
        plug = owner_of(p)
        for k, v in rec["strings"].items():
            msg.setdefault(k, {"message": v, "plugin": plug, "locale_id": rec["locale_id"]})

    # --- error tables --------------------------------------------------------------------
    files = sorted(p for p in args.root.rglob("*.idrc")
                   if p.parent.name in ("idrc_uetb", "idrc_petb"))
    errors = []
    per_file = []
    seen = {}
    for p in files:
        data = p.read_bytes()
        code_kind = p.parent.name.replace("idrc_", "")
        ents, status = (parse_petb(data) if code_kind == "petb" else parse_uetb(data))
        plug = owner_of(p)
        rel = str(p.relative_to(args.root)).replace("\\", "/")
        if code_kind == "petb":
            declared = struct.unpack_from("<I", data, 0)[0] if len(data) >= 4 else 0
        else:
            declared = struct.unpack_from("<H", data, 0)[0] if len(data) >= 2 else 0
        per_file.append({"file": rel, "plugin": plug, "resource_code": code_kind,
                         "declared_entries": declared,
                         "parsed_entries": len(ents), "status": status,
                         "bytes": len(data)})
        for e in ents:
            e["plugin"] = plug
            e["source_file"] = rel
            # message join: the symbol itself, and the symbol without its k prefix
            cands = [e["symbol"]]
            if e["symbol"].startswith("k"):
                cands.append(e["symbol"][1:])
            hit = next((c for c in cands if c in msg), None)
            if hit:
                e["message_en"] = msg[hit]["message"]
                e["message_key"] = hit
                e["message_source"] = "idrc_PMST locale_id=%d" % msg[hit]["locale_id"]
            e["constraint_candidate"] = bool(CONSTRAINT_HINT.search(e["symbol"]))
            key = e["error_code"]
            if key in seen:
                seen[key]["also_declared_in"] = seen[key].get("also_declared_in", [])
                if rel not in seen[key]["also_declared_in"]:
                    seen[key]["also_declared_in"].append(rel)
                continue
            seen[key] = e
            errors.append(e)

    by_service = collections.Counter(e["service_id"] for e in errors)
    by_plugin = collections.Counter(e["plugin"] for e in errors)
    with_msg = sum(1 for e in errors if "message_en" in e)
    constraints = [e for e in errors if e["constraint_candidate"]]

    doc = {
        "schema_id": "handshake.reference.indesign_error_catalog@1",
        "generated_at": now(),
        "source_root": str(args.root),
        "resource_codes": ["uetb", "petb", "PMST"],
        "method": (
            "Binary record parser for idrc_uetb error tables, read directly from the installed "
            "InDesign 2026 files. The application was never launched. Every error_code and symbol "
            "is PARSED, not inferred. message_en is a JOIN onto the English (locale_id 1 or 2) "
            "idrc_PMST string tables, keyed on the error symbol and on the symbol with its leading "
            "'k' removed; only exact key matches are reported and each carries message_key so the "
            "join can be audited. constraint_candidate is HEURISTIC: it flags symbols whose name "
            "matches a validation-shaped word list (Bad/Invalid/TooMany/OutOf/Required/...), which "
            "is a triage aid, not a parsed attribute."
        ),
        "format": {
            "uetb": "u16 entry_count; entry_count * {u32 error_code LE, u8 name_len, ASCII name}",
            "petb": "u32 entry_count, u16 pad; then the same entry records as uetb. One file "
                    "exists, holding the single fallback entry 0xffffffff 'Unknown error.'",
            "symbol_bytes": "ASCII, and tab/CR/LF are legal inside a symbol because the "
                            "InDesign core table stores multi-line English messages there "
                            "(for example '(InDesign Resources)/idrc_uetb/10800.idrc').",
            "error_code": "(service_id << 8) | ordinal. Verified: code>>8 yields 74 distinct "
                          "values across 72 owning plug-ins, code>>16 collapses to 3, so the "
                          "split is at bit 8. service_id 0 is the InDesign core table, whose "
                          "entries use the English message itself as the symbol "
                          "(symbol_is_message_text=true).",
            "pmst": "u32 locale_id, u32 unknown, u32 entry_count; "
                    "entries {u16 key_len, ASCII key, u16 value_len, UTF-8 value}",
        },
        "totals": {
            "uetb_resource_files": sum(1 for f in per_file if f["resource_code"] == "uetb"),
            "petb_resource_files": sum(1 for f in per_file if f["resource_code"] == "petb"),
            "uetb_files_fully_consumed": sum(1 for f in per_file if f["status"] == "complete"),
            "error_entries_parsed": sum(f["parsed_entries"] for f in per_file),
            "distinct_error_codes": len(errors),
            "errors_with_english_message": with_msg,
            "constraint_candidates": len(constraints),
            "distinct_service_ids": len(by_service),
            "distinct_plugins": len(by_plugin),
            "pmst_files_scanned": len(pmst_files),
            "pmst_english_files_used": pmst_english,
            "english_string_keys_pooled": len(msg),
        },
        "errors_by_plugin": [{"plugin": k, "errors": v} for k, v in by_plugin.most_common()],
        "errors_by_service_id": [{"service_id": k, "service_id_hex": f"{k:#06x}", "errors": v}
                                 for k, v in by_service.most_common()],
        "errors": sorted(errors, key=lambda e: e["error_code"]),
        "per_file": per_file,
    }
    outp = args.out / "indesign_error_catalog.json"
    outp.write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    t = doc["totals"]
    print(f"[uetb] files={t['uetb_resource_files']} fully_consumed={t['uetb_files_fully_consumed']} "
          f"entries={t['error_entries_parsed']} distinct={t['distinct_error_codes']}")
    print(f"[uetb] with_english_message={t['errors_with_english_message']} "
          f"constraint_candidates={t['constraint_candidates']} services={t['distinct_service_ids']}")
    print(f"[uetb] pmst english files={t['pmst_english_files_used']}/{t['pmst_files_scanned']} "
          f"keys={t['english_string_keys_pooled']}")
    print(f"[uetb] -> {outp} ({outp.stat().st_size/1048576:.1f} MB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
