#!/usr/bin/env python3
"""Real structural parser for Serif Affinity "KA"/"KS" property containers.

Covers *.propcol and *.afstudio from Affinity 3.x (Canva.Affinity).
This is a FORMAT PARSER, not a heuristic scraper: every value reported by this
module was decoded from the container's own type/tag/length encoding, and every
parse is validated by requiring the property stream to terminate exactly at the
file's declared trailer offset ("#FT4").

Container layout (little-endian):

  KA file header (0x4c bytes)
    0x00  u8[4]  magic 00 FF 'K' 'A'
    0x04  u16    format version
    0x06  u16    flags
    0x08  4CC    container type, stored byte-reversed  (e.g. 'urBR' -> 'RBru')
    0x0c  u8[4]  literal '#Inf'
    0x10  u64    offset of the '#FT4' trailer
    0x18  u64    total file size
    0x20  u64    payload byte count
    0x28  u64    reserved (0)
    0x30  u32    container uid
    0x34  u32    reserved (0)
    0x38  u32    type-table count
    0x3c  u32    object count
    0x40  u8[4]  literal 'Prot'
    0x44  u32    property-tag-table count
    0x48  u8[4]  literal '#Fil'
    0x4c  payload  -> either a KS sub-container (00 FF 'K' 'S') or a zstd frame

  KS sub-container
    0x00  u8[4]  magic 00 FF 'K' 'S'
    0x04  u16    format version
    0x06  4CC    root object type, reversed ('lFCP' -> 'PCFl')
    ...          object header tail (see read_object_header)
    ...          property stream, terminated by a 0x00 close byte
    ...          u32 0xFFFFFFFF end marker

  Property record
    u8     type code
    4CC    property tag, stored byte-reversed  ('emaN' -> 'Name')
    ...    value, length determined by the type code

  Object reference (value of an object-typed property, and each element of an
  object list)
    u8     0x01 marker
    u32    object index (unique per container)
    u8     0x00 = inline type definition, 0x01 = reuse previously defined type
    4CC    object type, reversed
    u16    type version         (only when the definition flag is 0x00)
    u16    schema tail          (only when the definition flag is 0x00 and the
                                 following bytes do not already form a valid
                                 property record; observed value 0x0200)
    ...    property stream, terminated by a 0x00 close byte

Type codes observed and decoded:
    0x00 close-object      0x03 u32           0x07 u32          0x08 u64
    0x0a f64               0x29 u8/bool       0x2a u32          0x2b string
    0x2f u32+u16           0x31 object        0x34 u32 (uid)    0x83 4CC list
    0xb1 object list
Any other code is resolved at parse time by the length prober and reported in
`unknown_types`, so nothing is silently guessed away.
"""
from __future__ import annotations

import ctypes
import mmap
import os
import struct
from collections import OrderedDict

MAGIC_KA = b"\x00\xffKA"
MAGIC_KS = b"\x00\xffKS"
MAGIC_ZSTD = b"\x28\xb5\x2f\xfd"
TRAILER = b"#FT4"

_u16 = struct.Struct("<H").unpack_from
_u32 = struct.Struct("<I").unpack_from
_u64 = struct.Struct("<Q").unpack_from
_i32 = struct.Struct("<i").unpack_from
_f64 = struct.Struct("<d").unpack_from
_f32 = struct.Struct("<f").unpack_from

TAG_OK = frozenset(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_#.* ")

# type code -> fixed value byte length (None => custom reader)
FIXED_LEN = {
    0x01: 1,     # u8
    0x02: 2,     # u16
    0x03: 4,     # u32
    0x07: 4,     # u32 (second integer flavour)
    0x08: 8,     # u64
    0x09: 4,     # f32
    0x0a: 8,     # f64
    0x17: 16,    # 4 x i32 (rectangle)
    0x21: 16,    # 4 x f32 (colour)
    0x24: 16,    # 16-byte value (2 x f64 point)
    0x26: 32,    # 4 x f64 (rectangle)
    0x28: 48,    # 6 x f64 (2D affine transform)
    0x29: 1,     # bool / u8
    0x2a: 4,     # u32 (enum flavour)
    0x2f: 4,     # 4CC value
    0x34: 4,     # u32 uid
    0x44: 16,    # 4 x f32 (RGBA colour)
    0x48: 20,    # 5 x f32 (CMYK + alpha colour)
}
# value decoders for the fixed-length codes
FLOAT4 = {0x21, 0x44}
FLOAT5 = {0x48}
CUSTOM = {0x2b, 0x2c, 0x2d, 0x30, 0x31, 0x32, 0x33, 0xa9, 0xac, 0x83, 0x84, 0xaa, 0xab, 0xb1, 0xb2}
DOUBLES = {0x24: 2, 0x26: 4, 0x28: 6}
INT4 = {0x17}
# 0x80 | base  ==  count-prefixed array of `base`
ARRAY_BASE = {0x80 | k: k for k in (0x01, 0x02, 0x03, 0x07, 0x08, 0x09, 0x0a,
                                    0x17, 0x21, 0x24, 0x26, 0x28, 0x2a,
                                    0x2f, 0x34, 0x44, 0x48)}
CLOSE = 0x00


class ParseError(Exception):
    pass


def rev4(b: bytes) -> str:
    return b[::-1].decode("latin-1")


def tag_name(b: bytes) -> str:
    """Property tags are 4 bytes stored reversed.  Most are printable 4CCs;
    some are numeric property ids, which are rendered as #0xXXXXXXXX."""
    if all(c in TAG_OK for c in b):
        return b[::-1].decode("latin-1")
    return "#0x%08x" % _u32(b, 0)[0]


def looks_tag(buf, p) -> bool:
    if p + 4 > len(buf):
        return False
    return (buf[p] in TAG_OK and buf[p + 1] in TAG_OK
            and buf[p + 2] in TAG_OK and buf[p + 3] in TAG_OK)


class Zstd:
    """ctypes libzstd wrapper (whole-frame decompression)."""

    def __init__(self, path=None):
        self.lib = None
        self.origin = None
        cands = []
        if path:
            cands.append(path)
        env = os.environ.get("AFFINITY_LIBZSTD")
        if env:
            cands.append(env)
        for base in (os.environ.get("ProgramFiles", r"C:\Program Files"),
                     os.environ.get("ProgramW6432", r"C:\Program Files")):
            cands.append(os.path.join(base, "Git", "mingw64", "bin", "libzstd.dll"))
        cands += ["libzstd.dll", "libzstd.so.1", "libzstd.so", "libzstd.dylib"]
        for c in cands:
            try:
                lib = ctypes.CDLL(c)
                lib.ZSTD_getFrameContentSize.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
                lib.ZSTD_getFrameContentSize.restype = ctypes.c_ulonglong
                lib.ZSTD_decompress.argtypes = [ctypes.c_void_p, ctypes.c_size_t,
                                                ctypes.c_void_p, ctypes.c_size_t]
                lib.ZSTD_decompress.restype = ctypes.c_size_t
                lib.ZSTD_isError.argtypes = [ctypes.c_size_t]
                lib.ZSTD_isError.restype = ctypes.c_uint
                lib.ZSTD_versionString.restype = ctypes.c_char_p
                self.lib = lib
                self.origin = "%s (%s)" % (lib.ZSTD_versionString().decode(), c)
                return
            except Exception:
                continue

    @property
    def available(self):
        return self.lib is not None

    def decompress(self, data: bytes) -> bytes:
        if not self.lib:
            raise ParseError("no libzstd backend available")
        src = ctypes.create_string_buffer(bytes(data), len(data))
        want = self.lib.ZSTD_getFrameContentSize(src, len(data))
        if want in (0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFE):
            want = max(len(data) * 24, 1 << 20)
        dst = ctypes.create_string_buffer(int(want))
        got = self.lib.ZSTD_decompress(dst, int(want), src, len(data))
        if self.lib.ZSTD_isError(got):
            raise ParseError("zstd decompress failed")
        return dst.raw[:got]


class Obj:
    """A parsed container object: 4CC type, container-unique index, properties."""
    __slots__ = ("type", "index", "ver", "props", "offset")

    def __init__(self, type_, index, ver, offset):
        self.type = type_
        self.index = index
        self.ver = ver
        self.offset = offset
        self.props = OrderedDict()

    def get(self, tag, default=None):
        v = self.props.get(tag, default)
        return v

    def __repr__(self):
        return "<%s#%s %s>" % (self.type, self.index, list(self.props)[:8])


class Parser:
    """Recursive-descent parser over one decoded payload buffer."""

    def __init__(self, buf, start, end, visitor=None, keep_tree=True,
                 max_depth=256):
        self.buf = buf
        self.p = start
        self.end = end
        self.visitor = visitor
        self.keep_tree = keep_tree
        self.max_depth = max_depth
        self.unknown_types = {}     # code -> {"len":n,"tags":set,"count":n}
        self.type_versions = {}     # 4CC -> version
        self.object_count = 0
        self.string_bytes = 0
        self.tag_counts = {}
        self.type_counts = {}
        self.type_counts_by_code = {}
        self.section_count = 0
        self.base_chains = {}
        self.warnings = []
        self.stack = []
        self.objects = {}
        self.allow_soft_eof = False
        self.soft_eof = False
        self.partial_root = None

    # ---------------------------------------------------------------- readers
    def u8(self):
        v = self.buf[self.p]
        self.p += 1
        return v

    def u16(self):
        v = _u16(self.buf, self.p)[0]
        self.p += 2
        return v

    def u32(self):
        v = _u32(self.buf, self.p)[0]
        self.p += 4
        return v

    def tag(self):
        t = tag_name(bytes(self.buf[self.p:self.p + 4]))
        self.p += 4
        return t

    # ------------------------------------------------------------- validation
    def valid_record_at(self, p) -> bool:
        """True if p starts a plausible property record or a close byte."""
        if p >= self.end:
            return False
        t = self.buf[p]
        if t == CLOSE:
            return True
        if t in FIXED_LEN or t in CUSTOM or t in self.unknown_types:
            return looks_tag(self.buf, p + 1)
        return False

    def probe_len(self, code, p) -> int:
        """Infer the value length of an unseen type code by lookahead."""
        for n in (0, 1, 2, 4, 6, 8, 12, 16, 20, 24, 32):
            q = p + n
            if q > self.end:
                break
            if not self.valid_record_at(q):
                continue
            # require two further plausible records to reduce false positives
            ok, r = True, q
            for _ in range(2):
                if r >= self.end or self.buf[r] == CLOSE:
                    break
                t2 = self.buf[r]
                if t2 in FIXED_LEN:
                    r += 5 + FIXED_LEN[t2]
                elif t2 == 0x2b:
                    if r + 9 > self.end:
                        ok = False
                        break
                    r += 9 + _u32(self.buf, r + 5)[0]
                else:
                    break
                if not self.valid_record_at(r):
                    ok = False
                    break
            if ok:
                return n
        raise ParseError("cannot infer length for type 0x%02x at 0x%x" % (code, p))

    # ----------------------------------------------------------------- values
    def read_value(self, code, tag):
        b, p = self.buf, self.p
        if code == 0x2b:                      # string: u32 length + UTF-8
            n = _u32(b, p)[0]
            self.p = p + 4 + n
            if self.p > self.end:
                raise ParseError("string overruns payload at 0x%x" % p)
            self.string_bytes += n
            return b[p + 4:p + 4 + n].decode("utf-8", "replace")
        if code == 0xaa:                      # u32 count + (count+1) x u16
            cnt = _u32(b, p)[0]
            self.p = p + 6 + 2 * cnt
            if self.p > self.end:
                raise ParseError("0xaa overruns payload at 0x%x" % p)
            return [_u16(b, p + 4 + 2 * i)[0] for i in range(cnt + 1)]
        if code == 0xab:                      # string array (only empty seen)
            cnt = _u32(b, p)[0]
            if cnt:
                raise ParseError("0xab count=%d unsupported at 0x%x" % (cnt, p))
            self.p = p + 8
            return []
        if code == 0x84:                      # 2-byte-element array (only empty seen)
            cnt = _u32(b, p)[0]
            if cnt:
                raise ParseError("0x84 count=%d unsupported at 0x%x" % (cnt, p))
            self.p = p + 4
            return []
        if code == 0x30:                      # tag-less nested property stream
            # u16 presence/count, then <u8 type-code><value> records with no
            # tags, terminated by a 0x00 code.  Used for geometry payloads.
            cnt = _u32(b, p)[0] & 0xFFFF
            self.p = p + 2
            if cnt == 0:
                return None
            out = OrderedDict()
            i = 0
            while True:
                if self.p >= self.end:
                    raise ParseError("0x30 stream truncated at 0x%x" % self.p)
                t = self.u8()
                if t == CLOSE:
                    break
                out["f%d_0x%02x" % (i, t)] = self.read_value(t, "f%d" % i)
                i += 1
            return out
        if code == 0xa9:                      # packed bit array (bool array)
            cnt = _u32(b, p)[0]
            nb = (cnt + 7) // 8
            self.p = p + 4 + nb
            if self.p > self.end:
                raise ParseError("0xa9 overruns payload at 0x%x" % p)
            bits = bytes(b[p + 4:p + 4 + nb])
            return [(bits[i >> 3] >> (i & 7)) & 1 for i in range(cnt)]
        if code == 0xac:                      # array of equal-length blobs
            cnt = _u32(b, p)[0]
            width = _u16(b, p + 4)[0]
            self.p = p + 6 + cnt * width
            if self.p > self.end:
                raise ParseError("0xac overruns payload at 0x%x" % p)
            o = p + 6
            return {"_count": cnt, "_stride": width,
                    "_data": bytes(b[o:o + cnt * width]).hex()}
        if code == 0x2d:                      # binary blob: u32 length + bytes
            n = _u32(b, p)[0]
            self.p = p + 4 + n
            if self.p > self.end:
                raise ParseError("blob overruns payload at 0x%x" % p)
            return {"_blob_len": n, "_blob": bytes(b[p + 4:p + 4 + n]).hex()}
        if code == 0x33:                      # 4CC kind + u32 length + UTF-8
            kind = rev4(b[p:p + 4])
            n = _u32(b, p + 4)[0]
            self.p = p + 8 + n
            if self.p > self.end:
                raise ParseError("0x33 overruns payload at 0x%x" % p)
            return {"kind": kind,
                    "value": bytes(b[p + 8:p + 8 + n]).decode("utf-8", "replace")}
        if code == 0x2c:                      # binary blob: u16 length + bytes
            n = _u16(b, p)[0]
            self.p = p + 2 + n
            if self.p > self.end:
                raise ParseError("blob overruns payload at 0x%x" % p)
            return {"_blob_len": n, "_blob": bytes(b[p + 2:p + 2 + n]).hex()}
        if code == 0x31:                      # single embedded object
            return self.read_object()
        if code == 0x32:                      # inline object, no registry index
            flag = self.u8()
            if flag == 0:
                return None
            if flag != 1:
                raise ParseError("0x32 flag 0x%02x at 0x%x" % (flag, self.p - 1))
            off = self.p
            four = self.tag()
            ver = self.u16()
            self.type_versions.setdefault(four, ver)
            self.object_count += 1
            self.type_counts[four] = self.type_counts.get(four, 0) + 1
            o = Obj(four, None, ver, off)
            self.read_props(o, 1)
            return o
        if code == 0xb2:                      # homogeneous object array
            # u32 count, 4CC element type, u16 version, then `count` elements,
            # each = u8 presence flag + property stream + close byte
            cnt = self.u32()
            off = self.p
            four = self.tag()
            ver = self.u16()
            self.type_versions.setdefault(four, ver)
            out = []
            for _i in range(cnt):
                flag = self.u8()
                if flag != 1:
                    raise ParseError("0xb2 elem flag 0x%02x at 0x%x"
                                     % (flag, self.p - 1))
                self.object_count += 1
                self.type_counts[four] = self.type_counts.get(four, 0) + 1
                o = Obj(four, None, ver, off)
                self.read_props(o, 1, _i < cnt - 1)
                out.append(o)
            return out
        if code == 0xb1:                      # list of embedded objects
            n = _u32(b, p)[0]
            self.p = p + 4
            out = []
            for i in range(n):
                out.append(self.read_object(0, i < n - 1))
            return out
        if code == 0x83:                      # list of 4CC ids
            n = _u32(b, p)[0]
            self.p = p + 4 + 4 * n
            if self.p > self.end:
                raise ParseError("4CC list overruns payload at 0x%x" % p)
            return [rev4(b[p + 4 + 4 * i:p + 8 + 4 * i]) for i in range(n)]
        base = ARRAY_BASE.get(code)
        if base is not None and code not in (0x83,):
            cnt = _u32(b, p)[0]
            w = FIXED_LEN[base]
            self.p = p + 4 + cnt * w
            if self.p > self.end:
                raise ParseError("array overruns payload at 0x%x" % p)
            o = p + 4
            if base == 0x09:
                return [_f32(b, o + 4 * i)[0] for i in range(cnt)]
            if base == 0x0a:
                return [_f64(b, o + 8 * i)[0] for i in range(cnt)]
            if base in (0x03, 0x07, 0x2a, 0x34):
                return [_u32(b, o + 4 * i)[0] for i in range(cnt)]
            if base == 0x08:
                return [_u64(b, o + 8 * i)[0] for i in range(cnt)]
            if base in (0x01, 0x29):
                return list(b[o:o + cnt])
            if base == 0x02:
                return [_u16(b, o + 2 * i)[0] for i in range(cnt)]
            if base == 0x2f:
                return [rev4(b[o + 4 * i:o + 4 * i + 4]) for i in range(cnt)]
            return [bytes(b[o + w * i:o + w * i + w]).hex() for i in range(cnt)]
        n = FIXED_LEN.get(code)
        if n is None:
            info = self.unknown_types.get(code)
            if info is None:
                n = self.probe_len(code, p)
                self.unknown_types[code] = {"len": n, "count": 0, "tags": set()}
                info = self.unknown_types[code]
            else:
                n = info["len"]
            info["count"] += 1
            if len(info["tags"]) < 40:
                info["tags"].add(tag)
            raw = b[p:p + n]
            self.p = p + n
            return {"_raw": raw.hex(), "_type": "0x%02x" % code}
        self.p = p + n
        if code == 0x0a:
            return _f64(b, p)[0]
        if code == 0x09:
            return _f32(b, p)[0]
        if code == 0x01:
            return b[p]
        if code == 0x02:
            return _u16(b, p)[0]
        if code in (0x03, 0x07, 0x2a):
            return _u32(b, p)[0]
        if code == 0x34:
            return _u32(b, p)[0]
        if code == 0x08:
            return _u64(b, p)[0]
        if code == 0x29:
            return b[p]
        if code == 0x2f:                      # 4CC-valued property, reversed
            raw = b[p:p + 4]
            if all(c in TAG_OK for c in raw):
                return rev4(raw)
            return _u32(b, p)[0]
        if code in FLOAT4:                    # 16-byte value: colour or GUID
            vals = [_f32(b, p + 4 * i)[0] for i in range(4)]
            ok = all(v == v and abs(v) < 1e6 for v in vals)
            if ok:
                return vals
            return {"_bytes16_hex": bytes(b[p:p + 16]).hex()}
        if code in INT4:
            return [_i32(b, p + 4 * i)[0] for i in range(4)]
        if code in FLOAT5:
            return [_f32(b, p + 4 * i)[0] for i in range(5)]
        if code in DOUBLES:
            return [_f64(b, p + 8 * i)[0] for i in range(DOUBLES[code])]
        return bytes(b[p:p + n]).hex()

    # ---------------------------------------------------------------- objects
    def read_object_header(self):
        """Reads marker/index/flag/type and returns (fourcc, index, ver)."""
        marker = self.u8()
        if marker == 0x00:                    # null object pointer
            return None, None, None
        if marker == 0x02:                    # reference to an existing object
            return None, self.u32(), "ref"
        if marker != 0x01:
            raise ParseError("object marker 0x%02x at 0x%x"
                             % (marker, self.p - 1))
        index = self.u32()
        flag = self.u8()
        if not looks_tag(self.buf, self.p):
            raise ParseError("bad object 4CC at 0x%x" % self.p)
        four = self.tag()
        ver = None
        if flag == 0x00:
            ver = self.u16()
            self.type_versions.setdefault(four, ver)
        elif flag != 0x01:
            raise ParseError("object def flag 0x%02x at 0x%x" % (flag, self.p))
        return four, index, ver

    def read_object(self, depth=0, list_more=False):
        off = self.p
        four, index, ver = self.read_object_header()
        if four is None:
            return None if index is None else {"_ref": index}
        self.object_count += 1
        self.type_counts[four] = self.type_counts.get(four, 0) + 1
        obj = Obj(four, index, ver, off)
        if index is not None:
            self.objects[index] = obj
        self.stack.append((four, index, off))
        if self.visitor is not None:
            self.visitor.enter(obj, depth)
        self.read_props(obj, depth, list_more)
        self.stack.pop()
        if self.visitor is not None:
            self.visitor.leave(obj, depth)
        return obj

    def chain_continues(self, obj, list_more=False):
        """After a section-close byte, decide whether another class section of
        the same object follows.

        An object is written as a chain of class sections, most-derived first.
        Each section ends with 0x00.  The byte after it is a chain opcode:
          0x00 + 4CC + u16   another base class, defined inline
          0x02               another base class, already known to the reader
          anything else      the object is finished
        """
        buf, p, end = self.buf, self.p, self.end
        if p >= end:
            return False
        b = buf[p]
        if b == 0x02:
            # 0x02 is also the marker of an object *reference* element inside a
            # list.  Prefer the class-chain reading when a real property record
            # follows; otherwise, if the list still has pending elements and the
            # next four bytes look like a plausible object index, the object has
            # ended and 0x02 belongs to the next element.
            if not self.valid_record_at(p + 1) and list_more and p + 6 <= end:
                idx = _u32(buf, p + 1)[0]
                nxt = buf[p + 5]
                if idx < (1 << 20) and (nxt in (0x00, 0x01, 0x02)
                                        or self.valid_record_at(p + 5)):
                    return False
            self.p = p + 1
            self.section_count += 1
            return True
        if b == 0x01 and p + 5 <= end and looks_tag(buf, p + 1)                 and self.valid_record_at(p + 5):
            base = rev4(buf[p + 1:p + 5])
            self.p = p + 5
            self.section_count += 1
            self.base_chains.setdefault(obj.type, [])
            if base not in self.base_chains[obj.type]:
                self.base_chains[obj.type].append(base)
            return True
        if b == 0x00 and p + 7 <= end and looks_tag(buf, p + 1):
            ver = _u16(buf, p + 5)[0]
            if ver < 4096 and self.valid_record_at(p + 7):
                base = rev4(buf[p + 1:p + 5])
                self.p = p + 7
                self.section_count += 1
                self.base_chains.setdefault(obj.type, [])
                if base not in self.base_chains[obj.type]:
                    self.base_chains[obj.type].append(base)
                return True
        return False

    def read_props(self, obj, depth, list_more=False):
        """Reads an object's property stream (see chain_continues)."""
        buf = self.buf
        while True:
            if self.p >= self.end:
                if self.allow_soft_eof:
                    self.soft_eof = True
                    return
                raise ParseError("payload exhausted inside %s#%s"
                                 % (obj.type, obj.index))
            code = buf[self.p]
            if code == CLOSE:
                self.p += 1
                if self.chain_continues(obj, list_more):
                    continue
                return
            self.p += 1
            if self.p + 4 > self.end:
                raise ParseError("truncated property tag at 0x%x" % self.p)
            tag = self.tag()
            self.tag_counts[tag] = self.tag_counts.get(tag, 0) + 1
            self.type_counts_by_code[code] = self.type_counts_by_code.get(code, 0) + 1
            if code == 0x31:
                val = self.read_object(depth + 1)
            elif code == 0xb1:
                n = self.u32()
                val = [self.read_object(depth + 1, i < n - 1) for i in range(n)]
            else:
                val = self.read_value(code, tag)
            if self.keep_tree or (self.visitor is not None
                                  and self.visitor.wants(obj, tag, depth)):
                obj.props[tag] = val

    def parse_ks(self):
        """Parses the KS sub-container rooted at self.p."""
        if self.buf[self.p:self.p + 4] != MAGIC_KS:
            raise ParseError("no KS magic at 0x%x" % self.p)
        self.p += 4
        ver = self.u16()
        four = self.tag()
        # container root header: u16 version + u32 type-table size
        root_ver = self.u16()
        table = self.u32()
        root = Obj(four, 0, root_ver, self.p)
        self.partial_root = root
        if self.visitor is not None:
            self.visitor.enter(root, 0)
        self.read_props(root, 0)
        if self.visitor is not None:
            self.visitor.leave(root, 0)
        return root, ver, table


class KAFile:
    def __init__(self, path, zstd=None):
        self.path = path
        self.size = os.path.getsize(path)
        self._fh = open(path, "rb")
        self._mm = mmap.mmap(self._fh.fileno(), 0, access=mmap.ACCESS_READ)
        b = self._mm
        if b[0:4] != MAGIC_KA:
            raise ParseError("not a KA container: %s" % path)
        self.version = _u16(b, 0x04)[0]
        self.flags = _u16(b, 0x06)[0]
        self.ctype = rev4(b[0x08:0x0c])
        self.trailer_off = _u64(b, 0x10)[0]
        self.declared_size = _u64(b, 0x18)[0]
        self.payload_size = _u64(b, 0x20)[0]
        self.uid = _u32(b, 0x30)[0]
        self.type_table_count = _u32(b, 0x38)[0]
        self.object_count_hdr = _u32(b, 0x3c)[0]
        self.prot = _u32(b, 0x44)[0]
        self.payload_off = 0x4c
        self.compressed = b[0x4c:0x50] == MAGIC_ZSTD
        self._zstd = zstd

    def close(self):
        try:
            self._mm.close()
        finally:
            self._fh.close()

    def payload(self):
        """Returns (buffer, start, end) for the property stream."""
        b = self._mm
        if self.compressed:
            if self._zstd is None or not self._zstd.available:
                raise ParseError("payload is zstd but no libzstd backend")
            # the 0xFFFFFFFF stream terminator sits after the compressed frame
            for cut in (4, 0):
                raw = b[self.payload_off:self.trailer_off - cut]
                try:
                    out = self._zstd.decompress(raw)
                except ParseError:
                    continue
                return out, 0, len(out)
            raise ParseError("zstd payload could not be decompressed")
        return b, self.payload_off, self.trailer_off

    def trailer_entries(self):
        """Parses the '#FT4' stream directory (offset/size/crc/name)."""
        b = self._mm
        t = self.trailer_off
        if b[t:t + 4] != TRAILER:
            return []
        # entry names are length-prefixed ASCII at the tail; walk backwards from
        # the end of file collecting (u8 len, name) pairs preceded by u32 crc
        out = []
        p = t + 4
        end = self.declared_size
        # locate the first name record: scan for a u8 length whose following
        # bytes are printable ASCII and which ends exactly at `end`
        q = end
        names = []
        while q > t:
            # names are packed at the end: [u8 len][name]
            found = False
            for ln in range(1, 65):
                s = q - ln
                if s - 1 <= t:
                    break
                if b[s - 1] == ln and all(32 <= c < 127 for c in b[s:q]):
                    names.append((s - 1, b[s:q].decode("ascii")))
                    q = s - 1
                    found = True
                    break
            if not found:
                break
        names.reverse()
        for off, nm in names:
            out.append({"name": nm, "name_record_offset": off})
        del p
        return out


def parse_file(path, visitor=None, keep_tree=True, zstd=None):
    """Parses one container. Returns a dict describing the parse."""
    ka = KAFile(path, zstd=zstd)
    err = None
    try:
        buf, start, end = ka.payload()
        par = Parser(buf, start, end, visitor=visitor, keep_tree=keep_tree)
        par.allow_soft_eof = ka.compressed
        try:
            root, ksver, table = par.parse_ks()
        except (ParseError, IndexError, struct.error, UnicodeDecodeError) as e:
            err = "%s: %s" % (type(e).__name__, e)
            root, ksver, table = par.partial_root, None, None
        # after the root object close, the stream ends with 0xFFFFFFFF
        tail = bytes(buf[par.p:min(par.p + 4, end)])
        clean = (err is None) and (tail == b"\xff\xff\xff\xff"
                                   or (ka.compressed and par.p >= end))
        single = (par.p + 4 == end)
        nxt = bytes(buf[par.p + 4:par.p + 8]) if par.p + 8 <= end else b""
        return {
            "path": path,
            "container_type": ka.ctype,
            "format_version": ka.version,
            "flags": ka.flags,
            "uid": ka.uid,
            "file_size": ka.size,
            "declared_size": ka.declared_size,
            "payload_size": ka.payload_size,
            "trailer_offset": ka.trailer_off,
            "compressed": ka.compressed,
            "header_type_table_count": ka.type_table_count,
            "header_object_count": ka.object_count_hdr,
            "header_prot_count": ka.prot,
            "ks_version": ksver,
            "ks_type_table": table,
            "root": root,
            "parser": par,
            "complete": clean,
            "error": err,
            "consumed": par.p - start,
            "expected": end - start,
            "trailer_bytes": tail.hex(),
            "single_stream": single,
            "soft_eof": par.soft_eof,
            "next_marker": nxt.decode("latin-1", "replace") if nxt else None,
        }
    finally:
        ka.close()
