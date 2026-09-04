#!/usr/bin/env python3
"""Heuristic name/tag scanner for Serif Affinity "KA" preset containers (*.propcol, *.afstudio).

Read-only. Streams each file once (chunked, with a small carry-over window) and reports:
  - header fields (magic 00 ff 4b 41, version word, 8-byte container type tag, size words)
  - printable UTF-8 and UTF-16LE string counts / top strings
  - 4CC tag candidates (raw + per-4-byte reversed forms)
  - candidate preset names from string properties (type byte 0x2b + reversed tag + u32 len + utf8):
      Name ('emaN'), Desc ('cseD'), name ('eman'), LStr ('rtSL')
  - node kinds from the Name -> Levs -> Chld layout:
      Levs>0            = category holding N entries
      Levs==0, Chld>0   = tree grouping (children nodes)
      no Levs (-> _UID) = preset entry
  - 4CC id lists (type byte 0x83 + tag + u32 count + n*4CC), e.g. studio tool/panel ids
  - zstd frames (magic 28 b5 2f fd); inflated and re-scanned when a zstd backend is available
    (python `zstandard` module, or libzstd shared library located via AFFINITY_SCAN_LIBZSTD
    env var / common Git-for-Windows path). Compressed spans are masked out of the raw scan so
    they do not pollute string/tag statistics. Without a backend, frames are only counted.

Everything here is HEURISTIC (see the "method" fields in the JSON). It is not a format parser.

Usage:
  python affinity-propcol-scan.py --dir <Affinity resources dir> --out <json path>
"""
from __future__ import annotations

import argparse
import ctypes
import datetime as _dt
import fnmatch
import json
import os
import re
import sys
from collections import Counter, OrderedDict

SCANNER_ID = "handshake.affinity.propcol_scan.v1"
MAGIC_KA = b"\x00\xffKA"
MAGIC_KS = b"\x00\xffKS"
ZSTD_MAGIC = b"\x28\xb5\x2f\xfd"

ASCII_RE = rb"[\x20-\x7e]{%d,1024}"
UTF16_RE = rb"(?:[\x20-\x7e]\x00){%d,512}"
# 4CC candidate: 4 tag chars not embedded in a word; keep if reversed form starts with upper/#/_
# (raw last char) or raw starts with upper/# (forward-stored header tags like "Prot", "#Fil").
# zero-width + lookahead so candidates may overlap (a type byte like '1' or '4' precedes some tags)
TAG_RE = re.compile(rb"(?<![A-Za-z ])(?=([A-Za-z0-9_#]{4})(?![a-z ]))")
TAG_KEEP_LAST = set(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ#_")
TAG_KEEP_FIRST = set(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ#")
TAG_CHARS = set(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_# ")
STRTAG_RE = re.compile(rb"\x2b([A-Za-z0-9_#]{4})(.{4})", re.S)   # 0x2b string property
LISTTAG_RE = re.compile(rb"\x83([A-Za-z0-9_#]{4})(.{4})", re.S)  # 0x83 4CC-id list property
NAME_PROPS = ("Name", "Desc", "name", "LStr")
NAMELIKE_RE = re.compile(r"^[A-Z0-9][A-Za-z0-9 ,.'&()/+\-_:#]{3,47}$")
MAX_NAME_LEN = 512
MAX_LIST_ITEMS = 256


# ----------------------------------------------------------------------------- zstd backend
class ZstdBackend:
    """Streaming zstd frame inflater: python-zstandard if importable, else libzstd via ctypes."""

    def __init__(self):
        self.kind = None
        self._mod = None
        self._lib = None
        try:
            import zstandard  # type: ignore
            self._mod = zstandard
            self.kind = "python-zstandard %s" % getattr(zstandard, "__version__", "?")
            return
        except Exception:
            pass
        cands = []
        env = os.environ.get("AFFINITY_SCAN_LIBZSTD")
        if env:
            cands.append(env)
        for base in (os.environ.get("ProgramFiles", r"C:\Program Files"),
                     os.environ.get("ProgramW6432", r"C:\Program Files")):
            cands.append(os.path.join(base, "Git", "mingw64", "bin", "libzstd.dll"))
        cands += ["libzstd.dll", "libzstd.so.1", "libzstd.so", "libzstd.dylib", "libzstd.1.dylib"]
        for c in cands:
            try:
                lib = ctypes.CDLL(c)
            except OSError:
                continue
            try:
                lib.ZSTD_createDStream.restype = ctypes.c_void_p
                lib.ZSTD_freeDStream.argtypes = [ctypes.c_void_p]
                lib.ZSTD_initDStream.argtypes = [ctypes.c_void_p]
                lib.ZSTD_initDStream.restype = ctypes.c_size_t
                lib.ZSTD_decompressStream.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
                lib.ZSTD_decompressStream.restype = ctypes.c_size_t
                lib.ZSTD_isError.argtypes = [ctypes.c_size_t]
                lib.ZSTD_isError.restype = ctypes.c_uint
                lib.ZSTD_getErrorName.argtypes = [ctypes.c_size_t]
                lib.ZSTD_getErrorName.restype = ctypes.c_char_p
                lib.ZSTD_versionString.restype = ctypes.c_char_p
                self._lib = lib
                self.kind = "libzstd %s via ctypes (%s)" % (lib.ZSTD_versionString().decode(), c)
                return
            except AttributeError:
                continue

    @property
    def available(self):
        return self.kind is not None

    def new_stream(self):
        if self._mod is not None:
            return _PyZstdStream(self._mod)
        return _CtypesZstdStream(self._lib)


class _InBuf(ctypes.Structure):
    _fields_ = [("src", ctypes.c_void_p), ("size", ctypes.c_size_t), ("pos", ctypes.c_size_t)]


class _OutBuf(ctypes.Structure):
    _fields_ = [("dst", ctypes.c_void_p), ("size", ctypes.c_size_t), ("pos", ctypes.c_size_t)]


class _CtypesZstdStream:
    OUT_CHUNK = 1 << 20

    def __init__(self, lib):
        self.lib = lib
        self.ds = lib.ZSTD_createDStream()
        lib.ZSTD_initDStream(self.ds)
        self.out = ctypes.create_string_buffer(self.OUT_CHUNK)
        self.error = None

    def feed(self, data: bytes):
        """Returns (consumed_bytes, list_of_output_chunks, frame_done)."""
        src = ctypes.create_string_buffer(data, len(data))
        ib = _InBuf(ctypes.cast(src, ctypes.c_void_p), len(data), 0)
        outs = []
        done = False
        while ib.pos < ib.size:
            ob = _OutBuf(ctypes.cast(self.out, ctypes.c_void_p), self.OUT_CHUNK, 0)
            ret = self.lib.ZSTD_decompressStream(self.ds, ctypes.byref(ob), ctypes.byref(ib))
            if self.lib.ZSTD_isError(ret):
                self.error = self.lib.ZSTD_getErrorName(ret).decode()
                return ib.pos, outs, True
            if ob.pos:
                outs.append(self.out.raw[:ob.pos])
            if ret == 0:
                done = True
                break
            if ob.pos == 0 and ib.pos == ib.size:
                break
        return ib.pos, outs, done

    def close(self):
        if self.ds:
            self.lib.ZSTD_freeDStream(self.ds)
            self.ds = None


class _PyZstdStream:
    def __init__(self, mod):
        self.dobj = mod.ZstdDecompressor().decompressobj()
        self.error = None

    def feed(self, data: bytes):
        try:
            out = self.dobj.decompress(data)
        except Exception as e:  # noqa: BLE001
            self.error = str(e)
            return len(data), [], True
        if self.dobj.eof:
            consumed = len(data) - len(self.dobj.unused_data)
            return consumed, [out] if out else [], True
        return len(data), [out] if out else [], False

    def close(self):
        pass


# ----------------------------------------------------------------------------- stream scanner
class StreamScanner:
    """Accumulates heuristics over a byte stream fed in chunks. `source` labels raw vs inflated."""

    def __init__(self, source: str, min_len: int, carry: int = 8192, max_distinct: int = 300000,
                 max_strvals: int = 50000):
        self.source = source
        self.min_len = min_len
        self.carry_n = carry
        self.max_distinct = max_distinct
        self.max_strvals = max_strvals
        self.ascii_re = re.compile(ASCII_RE % min_len)
        self.utf16_re = re.compile(UTF16_RE % min_len)
        self.buf = b""
        self.base = 0            # stream offset of self.buf[0]
        self.carry_len = 0
        self.bytes_seen = 0
        self.ascii = Counter()
        self.ascii_total = 0
        self.utf16 = Counter()
        self.utf16_total = 0
        self.tags_raw = Counter()
        self.nodes = []          # Name nodes: name, uid, levs, chld, kind, offset, source
        self.name_entries_total = 0
        self.strvals = []        # (offset, tag, value) for string properties, capped
        self.strvals_dropped = 0
        self.strtag_counts = Counter()
        self.strtag_samples = {}  # tag -> Counter (capped at 40 distinct)
        self.list_counts = Counter()
        self.list_ids = {}       # tag -> Counter of reversed 4CC ids
        self.ks_count = 0
        self.ptnd_count = 0

    def _count(self, counter: Counter, key, total_attr: str):
        setattr(self, total_attr, getattr(self, total_attr) + 1)
        if key in counter or len(counter) < self.max_distinct:
            counter[key] += 1

    def feed(self, data: bytes, final: bool = False):
        if not data and not final:
            return
        self.bytes_seen += len(data)
        buf = self.buf + data
        base = self.base
        cl = self.carry_len
        end = len(buf)
        self._scan(buf, base, cl, end, final)
        if final:
            self.buf = b""
            self.base = base + end
            self.carry_len = 0
        else:
            keep = min(self.carry_n, end)
            self.buf = buf[end - keep:]
            self.base = base + end - keep
            self.carry_len = keep

    def finish(self):
        self.feed(b"", final=True)

    @staticmethod
    def _accept(stop: int, cl: int, end: int, final: bool) -> bool:
        # accept matches ending inside the new region and (unless final) not touching the buffer end
        if stop < cl:
            return False
        if not final and stop >= end:
            return False
        return True

    def _scan(self, buf: bytes, base: int, cl: int, end: int, final: bool):
        acc = self._accept
        for m in self.ascii_re.finditer(buf):
            if acc(m.end(), cl, end, final):
                self._count(self.ascii, m.group(0), "ascii_total")
        for m in self.utf16_re.finditer(buf):
            if acc(m.end(), cl, end, final):
                self._count(self.utf16, m.group(0)[::2], "utf16_total")
        for m in TAG_RE.finditer(buf):
            if not acc(m.start() + 4, cl, end, final):
                continue
            t = m.group(1)
            if t[3] in TAG_KEEP_LAST or t[0] in TAG_KEEP_FIRST:
                self.tags_raw[t] += 1
        for pat, attr in ((MAGIC_KS, "ks_count"), (b"dNTP", "ptnd_count")):
            i = buf.find(pat, max(cl - len(pat) + 1, 0))
            while i != -1:
                if i + len(pat) >= cl and (final or i + len(pat) < end):
                    setattr(self, attr, getattr(self, attr) + 1)
                i = buf.find(pat, i + 1)
        # 0x83 4CC-id lists
        for m in LISTTAG_RE.finditer(buf):
            n = int.from_bytes(m.group(2), "little")
            if n <= 0 or n > MAX_LIST_ITEMS:
                continue
            s0, s1 = m.end(), m.end() + 4 * n
            if (s1 > end and not final) or s0 < cl or s1 > end:
                continue
            body = buf[s0:s1]
            if any(b not in TAG_CHARS for b in body):
                continue
            tag = m.group(1)[::-1].decode("ascii")
            self.list_counts[tag] += 1
            ids = self.list_ids.setdefault(tag, Counter())
            for k in range(n):
                ids[body[4 * k:4 * k + 4][::-1].decode("ascii")] += 1
        # 0x2b string properties (tag + u32 len + utf8)
        for m in STRTAG_RE.finditer(buf):
            n = int.from_bytes(m.group(2), "little")
            if n > MAX_NAME_LEN:
                continue
            s0, s1 = m.end(), m.end() + n
            if (s1 + 24 > end and not final) or s0 < cl:
                continue
            tag = m.group(1)[::-1].decode("ascii")
            if tag == "Name":
                self.name_entries_total += 1
            if n == 0:
                self.strtag_counts[tag] += 1
                continue
            raw = buf[s0:s1]
            try:
                val = raw.decode("utf-8")
            except UnicodeDecodeError:
                continue
            if not val.isprintable() or not val.strip():
                continue
            self.strtag_counts[tag] += 1
            samples = self.strtag_samples.setdefault(tag, Counter())
            if val in samples or len(samples) < 40:
                samples[val] += 1
            off = base + m.start()
            if tag in NAME_PROPS:
                if len(self.strvals) < self.max_strvals:
                    self.strvals.append((off, tag, val))
                else:
                    self.strvals_dropped += 1
            if tag == "Name":
                self._node(buf, m.start(), s1, off, val)

    def _node(self, buf: bytes, mstart: int, s1: int, off: int, name: str):
        uid = None
        pre = buf[max(0, mstart - 9):mstart]
        if len(pre) == 9 and pre[:5] == b"\x03DIU_":
            uid = int.from_bytes(pre[5:9], "little")
        levs = chld = None
        p = s1
        if buf[p:p + 5] == b"\xb1sveL":
            levs = int.from_bytes(buf[p + 5:p + 9], "little")
            p += 9
            if levs > 0:
                kind = "category"           # holds N level entries
            elif buf[p:p + 5] == b"\xb1dlhC":
                chld = int.from_bytes(buf[p + 5:p + 9], "little")
                kind = "tree_group" if chld > 0 else "empty_group"
            else:
                kind = "unknown"
        elif buf[p:p + 5] == b"\x03DIU_":
            kind = "preset_entry"
        else:
            kind = "unknown"
        self.nodes.append({"name": name, "uid": uid, "levs": levs, "chld": chld,
                           "kind": kind, "offset": off, "source": self.source})


# ----------------------------------------------------------------------------- per-file driver
def rev4(b: bytes) -> str:
    return b[::-1].decode("latin-1")


def parse_header(h: bytes, size: int) -> dict:
    d = {"magic_ok": h[:4] == MAGIC_KA}
    if len(h) < 16:
        d["note"] = "file shorter than 16 bytes; not a KA container"
        return d
    u32 = lambda o: int.from_bytes(h[o:o + 4], "little")  # noqa: E731
    u64 = lambda o: int.from_bytes(h[o:o + 8], "little")  # noqa: E731
    d.update({
        "version_u16_at_4": int.from_bytes(h[4:6], "little"),
        "flags_u16_at_6": int.from_bytes(h[6:8], "little"),
        "type_tag8_raw": h[8:16].decode("latin-1"),
        "type_tag8_reversed_per4": rev4(h[8:12]) + rev4(h[12:16]),
        "container_type_guess": rev4(h[8:12]),
    })
    if len(h) >= 80:
        d.update({
            "u64_at_16": u64(16),
            "u64_at_24": u64(24),
            "u64_at_24_equals_file_size": u64(24) == size,
            "u64_at_32": u64(32),
            "u32_at_48": u32(48),
            "u32_at_52": u32(52),
            "u32_at_56": u32(56),
            "u32_at_60": u32(60),
            "tag_at_64": h[64:68].decode("latin-1"),
            "u32_at_68": u32(68),
            "tag_at_72": h[72:76].decode("latin-1"),
            "payload_magic_at_76": h[76:80].hex(),
            "payload_kind_guess": ("KS-subcontainer" if h[76:80] == MAGIC_KS else
                                   "zstd-frame" if h[76:80] == ZSTD_MAGIC else "unknown"),
        })
    return d


def scan_file(path: str, min_len: int, chunk: int, zb: ZstdBackend, zstd_max_out: int) -> dict:
    size = os.path.getsize(path)
    raw = StreamScanner("raw", min_len)
    inf = StreamScanner("zstd_inflated", min_len)
    header = b""
    zframes = zinflated = zout_bytes = zskipped_cap = 0
    zerrors: list[str] = []
    zsizes = Counter()
    zactive = None
    zcur_out = 0
    with open(path, "rb") as fh:
        first = True
        while True:
            data = fh.read(chunk)
            if not data:
                break
            if first:
                header = data[:96]
                first = False
            spans = []
            if zb.available:
                i, n = 0, len(data)
                while i < n:
                    if zactive is None:
                        j = data.find(ZSTD_MAGIC, i)
                        if j == -1:
                            break
                        zframes += 1
                        zactive = zb.new_stream()
                        zcur_out = 0
                        i = j
                    start = i
                    consumed, outs, done = zactive.feed(data[i:])
                    for o in outs:
                        zcur_out += len(o)
                        if zout_bytes < zstd_max_out:
                            zout_bytes += len(o)
                            inf.feed(o)
                    i += consumed
                    spans.append((start, i))
                    if done:
                        if zactive.error:
                            zerrors.append(zactive.error)
                        else:
                            zinflated += 1
                            zsizes[zcur_out] += 1
                            if zout_bytes >= zstd_max_out:
                                zskipped_cap += 1
                        zactive.close()
                        zactive = None
                    elif consumed == 0:
                        break
                if spans:
                    ba = bytearray(data)
                    for a, b in spans:
                        ba[a:b] = bytes(b - a)
                    data = bytes(ba)
            else:
                j = data.find(ZSTD_MAGIC)
                while j != -1:
                    zframes += 1
                    j = data.find(ZSTD_MAGIC, j + 1)
            raw.feed(data)
    if zactive is not None:
        zerrors.append("truncated frame at EOF")
        zactive.close()
    raw.finish()
    inf.finish()

    scanners = [raw] + ([inf] if inf.bytes_seen else [])
    tags = Counter()
    for s in scanners:
        tags.update(s.tags_raw)
    tags_rev_top = [{"reversed": rev4(t), "raw": t.decode("latin-1"), "count": c}
                    for t, c in tags.most_common(50)]
    tagset = set()
    for t in tags:
        tagset.add(t.decode("latin-1"))
        tagset.add(rev4(t))

    strvals = sorted(raw.strvals + inf.strvals)
    by_prop = {p: [] for p in NAME_PROPS}
    seen_prop = {p: set() for p in NAME_PROPS}
    ordered = OrderedDict()
    for _, tag, val in strvals:
        if val not in seen_prop[tag]:
            seen_prop[tag].add(val)
            by_prop[tag].append(val)
        ordered.setdefault(val, tag)
    candidate_names = list(ordered.keys())
    method = "heuristic:string properties 0x2b+reversed tag+u32len+utf8 for Name/Desc/name/LStr, ordered by first offset"
    if not candidate_names:
        pool = Counter()
        for s in scanners:
            for k, c in s.ascii.items():
                try:
                    v = k.decode("ascii")
                except UnicodeDecodeError:
                    continue
                if v in tagset or v[:4] in tagset or v[1:5] in tagset:
                    continue
                if NAMELIKE_RE.match(v) and any(ch.islower() for ch in v) and \
                        sum(ch.isalpha() for ch in v) >= max(4, len(v) // 2):
                    pool[v] += c
            for k, c in s.utf16.items():
                v = k.decode("latin-1")
                if NAMELIKE_RE.match(v) and v not in tagset:
                    pool[v] += c
        candidate_names = [v for v, _ in pool.most_common(200)]
        method = "heuristic-fallback:name-like printable ASCII/UTF-16LE strings (LOW confidence)"

    nodes = raw.nodes + inf.nodes
    node_kinds = Counter(n["kind"] for n in nodes)
    categories = [{"name": n["name"], "kind": n["kind"],
                   "entries": n["levs"] if n["kind"] == "category" else n["chld"],
                   "uid": n["uid"], "source": n["source"]}
                  for n in nodes if n["kind"] in ("category", "tree_group")]
    name_counts = Counter(v for _, t, v in strvals if t == "Name")
    strprops = {}
    for t in (raw.strtag_counts + inf.strtag_counts):
        samples = raw.strtag_samples.get(t, Counter()) + inf.strtag_samples.get(t, Counter())
        strprops[t] = {"count": raw.strtag_counts[t] + inf.strtag_counts[t],
                       "samples": [v for v, _ in samples.most_common(8)]}
    lists = {}
    for t in (raw.list_counts + inf.list_counts):
        ids = raw.list_ids.get(t, Counter()) + inf.list_ids.get(t, Counter())
        lists[t] = {"count": raw.list_counts[t] + inf.list_counts[t],
                    "distinct_ids": len(ids),
                    "ids_top": [{"id": k, "n": c} for k, c in ids.most_common(60)]}
    top_ascii = [{"s": k.decode("latin-1"), "n": c} for k, c in
                 (raw.ascii + (inf.ascii if inf.bytes_seen else Counter())).most_common(40)]
    top_utf16 = [{"s": k.decode("latin-1"), "n": c} for k, c in
                 (raw.utf16 + (inf.utf16 if inf.bytes_seen else Counter())).most_common(20)]
    notes = []
    if header[:4] != MAGIC_KA:
        notes.append("no KA magic; not a preset container")
    if zframes and not zb.available:
        notes.append("zstd frames present but no zstd backend; compressed payloads not read")
    if zframes and zb.available and zinflated == 0:
        notes.append("zstd frames present but none inflated successfully")
    if not any(t == "Name" for _, t, _ in strvals):
        notes.append("no non-empty Name properties found")
    if zskipped_cap:
        notes.append("zstd inflate cap reached; %d later frames parsed but not scanned" % zskipped_cap)
    if raw.strvals_dropped or inf.strvals_dropped:
        notes.append("string-value cap reached; %d values dropped" % (raw.strvals_dropped + inf.strvals_dropped))
    return {
        "file": os.path.basename(path),
        "bytes": size,
        "header_hex": header[:80].hex(),
        "header": parse_header(header, size),
        "method": method,
        "tag_method": "heuristic:4 chars [A-Za-z0-9_#] not embedded in a word, reversed per 4 bytes; zstd spans masked; raw+inflated merged",
        "tag_count": len(tags),
        "tags_reversed_top": tags_rev_top,
        "candidate_names": candidate_names,
        "candidate_name_count": len(candidate_names),
        "candidate_names_by_property": by_prop,
        "name_entries_total": raw.name_entries_total + inf.name_entries_total,
        "name_occurrence_top": [{"name": k, "n": c} for k, c in name_counts.most_common(15)],
        "categories_guess": categories,
        "categories_method": "heuristic:Name node followed by Levs>0 (category with N entries) or Levs==0,Chld>0 (tree group)",
        "node_kind_counts": dict(node_kinds),
        "name_nodes": nodes[:4000],
        "string_properties": dict(sorted(strprops.items(), key=lambda kv: -kv[1]["count"])),
        "fourcc_id_lists": dict(sorted(lists.items(), key=lambda kv: -kv[1]["count"])),
        "fourcc_id_lists_method": "heuristic:0x83+reversed tag+u32 count+n*4CC, ids reported reversed",
        "structure_counts": {
            "ks_subcontainers": raw.ks_count + inf.ks_count,
            "ptnd_nodes": raw.ptnd_count + inf.ptnd_count,
            "zstd_frames_seen": zframes,
            "zstd_frames_inflated": zinflated,
            "zstd_inflated_bytes_scanned": zout_bytes,
            "zstd_inflated_size_histogram_top": [{"size": s, "n": c} for s, c in zsizes.most_common(8)],
            "zstd_errors": zerrors[:10],
        },
        "strings": {
            "ascii_total": raw.ascii_total + inf.ascii_total,
            "ascii_distinct": len(raw.ascii) + len(inf.ascii),
            "utf16le_total": raw.utf16_total + inf.utf16_total,
            "utf16le_distinct": len(raw.utf16) + len(inf.utf16),
            "ascii_top": top_ascii,
            "utf16le_top": top_utf16,
        },
        "notes": notes,
    }


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dir", required=True, help="Affinity resources directory")
    ap.add_argument("--out", required=True, help="output JSON path")
    ap.add_argument("--min-len", type=int, default=3)
    ap.add_argument("--chunk", type=int, default=8 * 1024 * 1024)
    ap.add_argument("--zstd-max-out", type=int, default=768 * 1024 * 1024,
                    help="max inflated bytes scanned per file (frames beyond it are parsed for span masking only)")
    ap.add_argument("--glob", default="*.propcol,*.afstudio")
    ap.add_argument("--only", default="", help="comma list of basenames to restrict to")
    args = ap.parse_args(argv)

    pats = [p.strip() for p in args.glob.split(",") if p.strip()]
    only = {p.strip() for p in args.only.split(",") if p.strip()}
    files = sorted(f for f in os.listdir(args.dir) if any(fnmatch.fnmatch(f, p) for p in pats))
    if only:
        files = [f for f in files if f in only]
    zb = ZstdBackend()
    print("zstd backend:", zb.kind or "none", file=sys.stderr)
    results = []
    for f in files:
        p = os.path.join(args.dir, f)
        t0 = _dt.datetime.now()
        print("scanning", f, os.path.getsize(p), "bytes", file=sys.stderr, flush=True)
        try:
            r = scan_file(p, args.min_len, args.chunk, zb, args.zstd_max_out)
        except Exception as e:  # noqa: BLE001
            r = {"file": f, "bytes": os.path.getsize(p), "error": repr(e),
                 "candidate_names": [], "candidate_name_count": 0, "notes": ["scan error"]}
        r["scan_seconds"] = round((_dt.datetime.now() - t0).total_seconds(), 2)
        results.append(r)
    totals = {
        "files": len(results),
        "bytes": sum(r.get("bytes", 0) for r in results),
        "candidate_names_total": sum(r.get("candidate_name_count", 0) for r in results),
        "name_entries_total": sum(r.get("name_entries_total", 0) for r in results),
        "categories_guess_total": sum(len(r.get("categories_guess", [])) for r in results),
        "files_without_names": [r["file"] for r in results if not r.get("candidate_names")],
        "files_using_fallback_method": [r["file"] for r in results
                                        if str(r.get("method", "")).startswith("heuristic-fallback")],
        "files_with_errors": [r["file"] for r in results if r.get("error")],
        "zstd_frames_seen": sum(r.get("structure_counts", {}).get("zstd_frames_seen", 0) for r in results),
        "zstd_frames_inflated": sum(r.get("structure_counts", {}).get("zstd_frames_inflated", 0) for r in results),
    }
    out = {
        "scanner_id": SCANNER_ID,
        "scanned_at": _dt.datetime.now(_dt.timezone.utc).isoformat(timespec="seconds"),
        "source_dir": args.dir,
        "zstd_backend": zb.kind,
        "method_summary": {
            "names": "heuristic: 0x2b string properties Name/Desc/name/LStr; fallback name-like printable strings",
            "tags": "heuristic: 4CC candidates stored byte-reversed per 4 bytes; reported raw and reversed",
            "categories": "heuristic: Name node with Levs>0 (category) or Levs==0,Chld>0 (tree group)",
            "id_lists": "heuristic: 0x83 lists of 4CC ids (studio tool/panel identifiers)",
            "compression": "zstd frames inflated when a backend is available, re-scanned, and masked out of the raw scan",
        },
        "files": results,
        "totals": totals,
    }
    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1, ensure_ascii=False)
    print("wrote", args.out, file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
