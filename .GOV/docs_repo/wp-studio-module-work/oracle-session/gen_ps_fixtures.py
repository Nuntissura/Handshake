#!/usr/bin/env python3
"""gen_ps_fixtures.py - write the Photoshop oracle fixture inputs (stdlib only).

Why: hsk_oracle_photoshop.jsx draws each fixture with one-pixel selection fills
(7473 fills for the six fixtures; measured about 0.5 s per fill in PS 27.11, and far
slower in a freshly launched instance). The script opens inputs/<name>.tif when it
already exists, so pre-generating the inputs removes that cost.

Pixel definitions are copied from hsk_oracle_photoshop.jsx (drawRamp, drawRampRev,
drawBands, drawLattice, drawH256, drawV256). Output format matches what Photoshop
writes for saveInput(): little-endian TIFF, RGB 8-bit, 3 samples (the fixtures are
fully opaque), uncompressed, one strip, 72 ppi, embedded ICC profile.

ICC: copied byte-for-byte from --icc-from (a Photoshop-saved sRGB TIFF, default the
slow-drawn inputs/F_GRAY_RAMP_256x64.tif when present), else the Windows sRGB profile.

Usage:
  python gen_ps_fixtures.py --out <adobe-session>/photoshop/inputs [--overwrite]
  python gen_ps_fixtures.py --check <tif> --name F_GRAY_RAMP_256x64   (pixel equality)
Writes <out>/FIXTURE_PROVENANCE.json listing every file it wrote (sha256, generator).
"""
import argparse, hashlib, json, os, struct, sys, datetime

FIXTURES = {}


def fixture(name, w, h):
    def deco(fn):
        FIXTURES[name] = (w, h, fn)
        return fn
    return deco


def js_round(v):  # JavaScript Math.round: half rounds up
    import math
    return int(math.floor(v + 0.5))


@fixture("F_GRAY_RAMP_256x64", 256, 64)
def ramp(x, y):
    return (x, x, x)


@fixture("F_GRAY_RAMP_REV_256x64", 256, 64)
def ramp_rev(x, y):
    return (255 - x, 255 - x, 255 - x)


_BANDS = [(1, 0, 0), (0, 1, 0), (0, 0, 1), (0, 1, 1), (1, 0, 1), (1, 1, 0)]


@fixture("F_RGBCMY_BANDS_256x48", 256, 48)
def bands(x, y):
    b = _BANDS[y // 8]
    return (b[0] * x, b[1] * x, b[2] * x)


@fixture("F_RGB_LATTICE17_289x17", 289, 17)
def lattice(x, y):
    ri, gi, bi = x // 17, x % 17, y
    v = lambda k: js_round(k * 255 / 16)
    return (v(ri), v(gi), v(bi))


@fixture("F_GRID_HRAMP_256", 256, 256)
def hramp(x, y):
    return (x, x, x)


@fixture("F_GRID_VRAMP_256", 256, 256)
def vramp(x, y):
    return (y, y, y)


def pixels(name):
    w, h, fn = FIXTURES[name]
    out = bytearray()
    for y in range(h):
        for x in range(w):
            out.extend(fn(x, y))
    return w, h, bytes(out)


def read_tiff(path):
    """Return (endian, tags{tag:(type,count,raw_value_bytes_or_offset)}, data) for a baseline TIFF."""
    b = open(path, "rb").read()
    e = "<" if b[:2] == b"II" else ">"
    off = struct.unpack(e + "I", b[4:8])[0]
    n = struct.unpack(e + "H", b[off:off + 2])[0]
    tags = {}
    size = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1}
    for i in range(n):
        t, ty, c, v = struct.unpack(e + "HHI4s", b[off + 2 + 12 * i:off + 14 + 12 * i])
        sz = size.get(ty, 1) * c
        if sz <= 4:
            raw = v[:sz]
        else:
            raw = b[struct.unpack(e + "I", v)[0]:][:sz]
        tags[t] = (ty, c, raw)
    return e, tags, b


def tag_ints(e, tag):
    ty, c, raw = tag
    f = {3: "H", 4: "I"}[ty]
    return list(struct.unpack(e + f * c, raw))


def tiff_rgb8(path):
    """Decode an uncompressed chunky 8-bit RGB(A) TIFF into (w, h, spp, bytes)."""
    e, tags, b = read_tiff(path)
    w = tag_ints(e, tags[256])[0]
    h = tag_ints(e, tags[257])[0]
    spp = tag_ints(e, tags[277])[0] if 277 in tags else 1
    if tag_ints(e, tags[259])[0] != 1:
        raise SystemExit("compressed TIFF not supported: " + path)
    offs = tag_ints(e, tags[273])
    cnts = tag_ints(e, tags[279])
    data = b"".join(b[o:o + c] for o, c in zip(offs, cnts))
    return w, h, spp, data


def icc_from(path):
    if path and os.path.exists(path):
        _, tags, _ = read_tiff(path)
        if 34675 in tags:
            return tags[34675][2], "embedded ICC of " + os.path.basename(path)
    win = os.path.join(os.environ.get("SystemRoot", "C:/Windows"), "System32/spool/drivers/color/sRGB Color Space Profile.icm")
    if os.path.exists(win):
        return open(win, "rb").read(), "Windows sRGB Color Space Profile.icm"
    return None, "none"


def write_tiff(path, w, h, data, icc):
    entries = []  # (tag, type, count, payload bytes)
    def short(t, *v): entries.append((t, 3, len(v), struct.pack("<" + "H" * len(v), *v)))
    def long_(t, v): entries.append((t, 4, 1, struct.pack("<I", v)))
    def rational(t, n, d): entries.append((t, 5, 1, struct.pack("<II", n, d)))
    long_(254, 0)
    short(256, w)
    short(257, h)
    short(258, 8, 8, 8)
    short(259, 1)
    short(262, 2)
    long_(273, 0)          # patched below
    short(274, 1)
    short(277, 3)
    short(278, h)
    long_(279, len(data))
    rational(282, 72, 1)
    rational(283, 72, 1)
    short(284, 1)
    short(296, 2)
    if icc:
        entries.append((34675, 7, len(icc), icc))
    entries.sort(key=lambda t: t[0])
    ifd_off = 8
    ifd_size = 2 + 12 * len(entries) + 4
    extra_off = ifd_off + ifd_size
    blobs = bytearray()
    payload_off = {}
    for t, ty, c, p in entries:
        if len(p) > 4:
            if (extra_off + len(blobs)) % 2:
                blobs.append(0)
            payload_off[t] = extra_off + len(blobs)
            blobs.extend(p)
    data_off = extra_off + len(blobs)
    if data_off % 2:
        blobs.append(0)
        data_off += 1
    ifd = bytearray(struct.pack("<H", len(entries)))
    for t, ty, c, p in entries:
        if t == 273:
            p = struct.pack("<I", data_off)
        if len(p) > 4:
            ifd.extend(struct.pack("<HHII", t, ty, c, payload_off[t]))
        else:
            ifd.extend(struct.pack("<HHI", t, ty, c) + p.ljust(4, b"\0"))
    ifd.extend(struct.pack("<I", 0))
    with open(path, "wb") as f:
        f.write(b"II*\0" + struct.pack("<I", ifd_off) + bytes(ifd) + bytes(blobs) + data)


def check(path, name):
    w, h, spp, data = tiff_rgb8(path)
    gw, gh, gdata = pixels(name)
    if (w, h) != (gw, gh):
        print("SIZE MISMATCH", (w, h), (gw, gh)); return 1
    rgb = data if spp == 3 else b"".join(data[i:i + 3] for i in range(0, len(data), spp))
    alpha_ok = True
    if spp == 4:
        alpha_ok = all(data[i + 3] == 255 for i in range(0, len(data), 4))
    diff = sum(1 for i in range(len(gdata)) if rgb[i] != gdata[i])
    print(f"{name} vs {path}: {w}x{h} spp={spp} differing_bytes={diff} alpha_all_255={alpha_ok}")
    return 0 if diff == 0 and alpha_ok else 1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out")
    ap.add_argument("--overwrite", action="store_true")
    ap.add_argument("--icc-from")
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--check")
    ap.add_argument("--name")
    a = ap.parse_args()
    if a.check:
        return check(a.check, a.name)
    if not a.out:
        ap.error("--out required")
    os.makedirs(a.out, exist_ok=True)
    ref = a.icc_from or os.path.join(a.out, "F_GRAY_RAMP_256x64.tif")
    icc, icc_src = icc_from(ref)
    prov_path = os.path.join(a.out, "FIXTURE_PROVENANCE.json")
    prov = json.load(open(prov_path)) if os.path.exists(prov_path) else {"schema": "hsk.oracle.fixture_provenance@1", "files": {}}
    for name in FIXTURES:
        if a.only and name not in a.only:
            continue
        p = os.path.join(a.out, name + ".tif")
        if os.path.exists(p) and not a.overwrite:
            print("keep existing", p)
            prov["files"].setdefault(name, {"source": "pre-existing (not written by this generator)", "sha256": hashlib.sha256(open(p, "rb").read()).hexdigest()})
            continue
        w, h, data = pixels(name)
        write_tiff(p, w, h, data, icc)
        prov["files"][name] = {"source": "generated externally by oracle-session/gen_ps_fixtures.py (pixel definitions copied from hsk_oracle_photoshop.jsx draw functions)",
                               "icc": icc_src, "w": w, "h": h, "sha256": hashlib.sha256(open(p, "rb").read()).hexdigest(),
                               "written_utc": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")}
        print("wrote", p)
    json.dump(prov, open(prov_path, "w"), indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
