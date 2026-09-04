"""ai_private.py -- extract the Adobe Illustrator private data stream from a .ai file.

A modern .ai file is a PDF container.  Illustrator's own document model lives in
a private side-stream referenced from a `/Private` dictionary:

    24 0 obj <</AIMetaData 25 0 R
               /AIPrivateData1 26 0 R
               /AIPrivateData2 27 0 R
               /AIPrivateData3 28 0 R
               /NumBlock 3
               /ContainerVersion 11 /CreatorVersion 24
               /RoundtripStreamType 2 /RoundtripVersion 24>>

The AIPrivateDataN objects are raw (unfiltered) PDF streams that must be
concatenated in index order.  The concatenation begins with a compression
marker:

    %AI24_ZStandard_Data<zstd frame>       Illustrator 24+ (2020 and later)
    %AI12_CompressedData<zlib stream>      Illustrator 12..23
    %!PS-Adobe-3.0 ...                     uncompressed legacy

The decompressed payload is the legacy PostScript-flavoured AI stream, which
carries brushes, symbols, graphic styles, swatches, gradients and patterns.

Reads files only.  Never launches Illustrator.
"""
from __future__ import annotations

import re
import zlib

ZSTD_MARK = b"%AI24_ZStandard_Data"
FLATE_MARK = b"%AI12_CompressedData"

_zstd_decompress = None


def _get_zstd():
    """Return a zstd decompress callable, or None.

    Python 3.14 ships `compression.zstd`.  Older interpreters need the
    third-party `zstandard` package.
    """
    global _zstd_decompress
    if _zstd_decompress is not None:
        return _zstd_decompress
    try:
        from compression import zstd as _z  # Python 3.14+
        _zstd_decompress = _z.decompress
        return _zstd_decompress
    except Exception:
        pass
    try:
        import zstandard as _z  # third party
        _zstd_decompress = lambda b: _z.ZstdDecompressor().decompress(
            b, max_output_size=64 * 1024 * 1024)
        return _zstd_decompress
    except Exception:
        return None


# An object header always starts a line.  Anchoring on the preceding newline
# keeps random `N 0 obj` byte sequences inside compressed streams from
# poisoning the offset map.
_RE_OBJ = re.compile(rb"(?:^|[\r\n])[ \t]*(\d+)[ \t]+(\d+)[ \t]*obj\b")
_RE_PRIVATE = re.compile(
    rb"<<((?:[^<>]|<<[^>]*>>)*?/AIPrivateData1\s+\d+\s+0\s+R(?:[^<>]|<<[^>]*>>)*?)>>")


def _object_offsets(data: bytes) -> dict[int, int]:
    """Map object number -> byte offset of the `N 0 obj` token."""
    out = {}
    for m in _RE_OBJ.finditer(data):
        try:
            num = int(m.group(1))
        except ValueError:
            continue
        # Keep the FIRST definition; PDF writes objects in ascending file order
        # and a later "match" is far more likely to be stream noise.
        out.setdefault(num, m.start(1))
    return out


def _stream_body(data: bytes, obj_off: int) -> bytes | None:
    """Read the raw stream body of the object starting at obj_off using /Length."""
    head_end = data.find(b"stream", obj_off)
    if head_end < 0:
        return None
    head = data[obj_off:head_end]
    m = re.search(rb"/Length\s+(\d+)", head)
    body = head_end + len(b"stream")
    # PDF 32000-1 7.3.8.1: `stream` is followed by exactly one CRLF or one LF.
    # Consuming a greedy run of newlines corrupts binary payloads whose first
    # byte is itself 0x0D or 0x0A.
    if data[body:body + 2] == b"\r\n":
        body += 2
    elif data[body:body + 1] == b"\n":
        body += 1
    if m:
        n = int(m.group(1))
        return data[body:body + n]
    e = data.find(b"endstream", body)
    return data[body:e] if e > 0 else None


class AIPrivateResult:
    __slots__ = ("payload", "compression", "num_block", "creator_version",
                 "container_version", "metadata_header", "error")

    def __init__(self):
        self.payload = b""
        self.compression = None
        self.num_block = 0
        self.creator_version = None
        self.container_version = None
        self.metadata_header = b""
        self.error = None


def extract(path: str) -> AIPrivateResult:
    res = AIPrivateResult()
    with open(path, "rb") as fh:
        data = fh.read()

    if not data.startswith(b"%PDF"):
        res.error = "not_a_pdf_container"
        return res

    m = _RE_PRIVATE.search(data)
    if not m:
        res.error = "no_AIPrivateData_dict"
        return res
    dct = m.group(1)

    for key, attr in (("NumBlock", "num_block"),
                      ("CreatorVersion", "creator_version"),
                      ("ContainerVersion", "container_version")):
        mm = re.search(rb"/" + key.encode() + rb"\s+(\d+)", dct)
        if mm:
            setattr(res, attr, int(mm.group(1)))

    offsets = _object_offsets(data)

    mm = re.search(rb"/AIMetaData\s+(\d+)\s+0\s+R", dct)
    if mm:
        off = offsets.get(int(mm.group(1)))
        if off is not None:
            res.metadata_header = _stream_body(data, off) or b""

    refs = {}
    for mm in re.finditer(rb"/AIPrivateData(\d+)\s+(\d+)\s+0\s+R", dct):
        refs[int(mm.group(1))] = int(mm.group(2))
    if not refs:
        res.error = "no_AIPrivateData_refs"
        return res

    parts = []
    for idx in sorted(refs):
        off = offsets.get(refs[idx])
        if off is None:
            res.error = f"missing_object_{refs[idx]}"
            return res
        body = _stream_body(data, off)
        if body is None:
            res.error = f"unreadable_stream_object_{refs[idx]}"
            return res
        parts.append(body)
    blob = b"".join(parts)

    if blob.startswith(ZSTD_MARK):
        dec = _get_zstd()
        if dec is None:
            res.error = "zstd_decoder_unavailable"
            res.compression = "zstd"
            return res
        try:
            res.payload = dec(blob[len(ZSTD_MARK):])
            res.compression = "zstd"
        except Exception as exc:
            res.error = f"zstd_decompress_failed:{exc}"
            res.compression = "zstd"
    elif blob.startswith(FLATE_MARK):
        try:
            res.payload = zlib.decompress(blob[len(FLATE_MARK):].lstrip(b"\r\n"))
            res.compression = "flate"
        except Exception as exc:
            res.error = f"flate_decompress_failed:{exc}"
            res.compression = "flate"
    elif blob.startswith(b"%!PS-Adobe") or blob.startswith(b"%%AI"):
        res.payload = blob
        res.compression = "none"
    else:
        res.compression = "unknown"
        res.error = "unrecognised_private_data_marker:" + repr(blob[:24])
    return res
