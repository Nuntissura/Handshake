"""dw_zstrings.py -- decoder for Adobe "ZString Binary Format" .zbin resources.

Reverse-engineered offline from the bytes of
  <install>/en_US/Resources/strings.zbin
  <install>/en_US/Resources/NonLocalisedStrings.zbin
No application was launched; no Adobe library was loaded.

LAYOUT (little-endian), derived by structural probing and confirmed by
round-tripping every entry of both shipped files without decode errors:

  0x00  21 bytes   magic  b"ZString Binary Format"
  0x15  4  bytes   version quad (observed 02 01 00 00)
  0x19  u32        locale name length
  ....  bytes      locale name, ascii (observed "en_US")
  ....  u32        reserved (observed 0)
  ....  u8         section marker (observed 0x59)
  ....  u32        n            -- entry count
  ....  u32        n2           -- entry count, repeated
  ....  u32        n_groups     -- namespace/group count (not needed to read pairs)
  ....  u32        key_blob_size    (bytes)
  ....  u32        value_blob_size  (bytes)
  ....  u32[n]     key offsets   (byte offsets into key blob)
  ....  u16[n]     key lengths   (bytes)
  ....  u32[n]     value offsets (byte offsets into value blob)
  ....  u16[n]     value lengths (bytes, UTF-16LE so always even)
  ....  bytes      key blob      (UTF-8 / ascii)
  ....  bytes      value blob    (UTF-16LE)
  ....  bytes      trailing group index (not decoded; not required)

Usage:
  python dw_zstrings.py <install_root> <out.json>
  or: from dw_zstrings import load_all_strings
"""
import json
import os
import struct
import sys


def read_zbin(path):
    """Return list of (key, value). Raises ValueError on a bad file."""
    with open(path, "rb") as fh:
        d = fh.read()
    if d[:21] != b"ZString Binary Format":
        raise ValueError("not a ZString binary file: %s" % path)
    off = 21
    version = d[off:off + 4]
    off += 4
    (loc_len,) = struct.unpack_from("<I", d, off)
    off += 4
    locale = d[off:off + loc_len].decode("ascii")
    off += loc_len
    off += 4          # reserved u32
    marker = d[off]   # 0x59
    off += 1
    n, n2, n_groups, key_size, val_size = struct.unpack_from("<5I", d, off)
    off += 20
    key_off = struct.unpack_from("<%dI" % n, d, off); off += n * 4
    key_len = struct.unpack_from("<%dH" % n, d, off); off += n * 2
    val_off = struct.unpack_from("<%dI" % n, d, off); off += n * 4
    val_len = struct.unpack_from("<%dH" % n, d, off); off += n * 2
    kb = off; off += key_size
    vb = off; off += val_size
    out = []
    for i in range(n):
        k = d[kb + key_off[i]: kb + key_off[i] + key_len[i]].decode("utf-8", "replace")
        v = d[vb + val_off[i]: vb + val_off[i] + val_len[i]].decode("utf-16-le", "replace")
        out.append((k, v))
    return {
        "locale": locale,
        "version_quad": list(version),
        "marker": marker,
        "entry_count": n,
        "entry_count_repeat": n2,
        "group_count": n_groups,
        "key_blob_bytes": key_size,
        "value_blob_bytes": val_size,
        "trailing_bytes_undecoded": len(d) - off,
        "entries": out,
    }


def load_all_strings(install_root, locale="en_US"):
    """Merged {key: value} across strings.zbin and NonLocalisedStrings.zbin."""
    res = os.path.join(install_root, locale, "Resources")
    merged = {}
    meta = {}
    for name in ("strings.zbin", "NonLocalisedStrings.zbin"):
        p = os.path.join(res, name)
        if not os.path.isfile(p):
            meta[name] = {"status": "missing", "path": p}
            continue
        try:
            r = read_zbin(p)
        except Exception as exc:      # noqa: BLE001
            meta[name] = {"status": "failed", "path": p, "error": repr(exc)}
            continue
        for k, v in r["entries"]:
            merged.setdefault(k, v)
        meta[name] = {
            "status": "parsed",
            "path": p,
            "entry_count": r["entry_count"],
            "group_count": r["group_count"],
            "trailing_bytes_undecoded": r["trailing_bytes_undecoded"],
        }
    # case-insensitive alias table: menus.xml mixes "menus/..." and "Menus/..."
    lower = {}
    for k, v in merged.items():
        lower.setdefault(k.lower(), v)
    return merged, lower, meta


def resolve(key, exact, lower):
    if key is None:
        return None, "none"
    if key in exact:
        return exact[key], "exact"
    lk = key.lower()
    if lk in lower:
        return lower[lk], "case_insensitive"
    return None, "unresolved"


if __name__ == "__main__":
    root = sys.argv[1]
    out = sys.argv[2]
    exact, lower, meta = load_all_strings(root)
    with open(out, "w", encoding="utf-8") as fh:
        json.dump({"meta": meta, "strings": exact}, fh, ensure_ascii=False, indent=1)
    print("wrote", out, len(exact), "unique keys")
    print(json.dumps(meta, indent=1))
