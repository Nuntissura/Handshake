#!/usr/bin/env python3
"""Re-index a promoted spec bundle so its manifest describes the files it actually contains.

The bundle's manifest carries, per module, a sha256 of the file, a sha256 of the body after the
YAML frontmatter, byte and line counts, and heading counts. It also carries a reconstruction hash
over every module concatenated in manifest order. `spec-current-lib.mjs` verifies all of them, and
rewriting a module without re-indexing makes the bundle fail its own checks.

Each module's frontmatter is machine metadata that repeats the bundle version and the body hash,
so that is refreshed too. `source_body_original_sha256` is deliberately left alone: it identifies
the ORIGINAL body this module was split from and is lineage, not a description of current content.

Reference tooling. Writes only inside the bundle it is pointed at.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
from pathlib import Path

FRONTMATTER = re.compile(rb"\A---\r?\n.*?\r?\n---\r?\n", re.S)
HEADING = re.compile(r"^(#{1,6})\s+\S", re.M)


def sha256(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def split_body(buf: bytes) -> tuple[bytes, bytes]:
    m = FRONTMATTER.match(buf)
    return (buf[: m.end()], buf[m.end():]) if m else (b"", buf)


def set_fm(fm: bytes, key: str, value: str) -> bytes:
    """Replace a scalar in the frontmatter, leaving everything else byte-identical."""
    pat = re.compile(rb"^(" + re.escape(key.encode()) + rb":\s*)(.*)$", re.M)
    if not pat.search(fm):
        return fm
    quoted = f'"{value}"'.encode()
    return pat.sub(lambda m: m.group(1) + quoted, fm, count=1)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bundle", type=Path, required=True)
    ap.add_argument("--version", required=True)
    args = ap.parse_args()

    man_path = args.bundle / "indexed-spec-manifest.json"
    manifest = json.loads(man_path.read_text(encoding="utf-8"))
    bundle_id = args.bundle.name

    buffers, changed = [], []
    for mod in manifest.get("modules", []):
        p = args.bundle / mod["path"]
        buf = p.read_bytes()
        fm, body = split_body(buf)

        if fm:
            fm2 = set_fm(fm, "spec_version", args.version)
            fm2 = set_fm(fm2, "bundle_id", bundle_id)
            fm2 = set_fm(fm2, "body_sha256", sha256(body))
            if fm2 != fm:
                buf = fm2 + body
                p.write_bytes(buf)
                fm, body = fm2, body

        before = mod.get("sha256")
        mod["sha256"] = sha256(buf)
        mod["byte_count"] = len(buf)
        mod["line_count"] = buf.count(b"\n") + (0 if buf.endswith(b"\n") else 1)
        mod["file_byte_count"] = mod["byte_count"]
        mod["file_line_count"] = mod["line_count"]
        mod["body_sha256"] = sha256(body)
        mod["body_byte_count"] = len(body)
        mod["body_line_count"] = body.count(b"\n") + (0 if body.endswith(b"\n") else 1)
        text = body.decode("utf-8", errors="replace")
        levels = collections.Counter(len(h) for h in HEADING.findall(text))
        mod["heading_count"] = sum(levels.values())
        mod["heading_level_counts"] = {str(k): levels[k] for k in sorted(levels)}
        mod["spec_version"] = args.version
        mod["bundle_id"] = bundle_id
        if before != mod["sha256"]:
            changed.append(mod["path"])
        buffers.append(buf)

    recon = manifest.setdefault("reconstruction", {})
    recon["reconstructed_sha256"] = sha256(b"".join(buffers))
    recon["matches_source_sha256"] = recon["reconstructed_sha256"] == (
        manifest.get("source", {}) or {}).get("sha256")
    recon["note"] = ("The bundle no longer reconstructs the v02.182 baseline byte-for-byte, and has "
                     "not since the section modules began diverging from it. The hash is kept so "
                     "the bundle can verify itself against its own modules.")
    manifest["spec_version"] = args.version
    manifest["bundle_id"] = bundle_id
    man_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
                        encoding="utf-8", newline="\n")

    idx_path = args.bundle / "INDEX.json"
    if idx_path.exists():
        idx = json.loads(idx_path.read_text(encoding="utf-8"))
        by_path = {m["path"]: m for m in manifest["modules"]}

        def refresh(node):
            if isinstance(node, dict):
                p = node.get("path")
                if p in by_path:
                    for k in ("sha256", "byte_count", "line_count", "heading_count"):
                        if k in node:
                            node[k] = by_path[p][k]
                for v in node.values():
                    refresh(v)
            elif isinstance(node, list):
                for v in node:
                    refresh(v)

        refresh(idx)
        idx_path.write_text(json.dumps(idx, indent=2, ensure_ascii=False) + "\n",
                            encoding="utf-8", newline="\n")

    print(f"[reindex] {bundle_id}: {len(manifest['modules'])} modules, {len(changed)} rehashed")
    for c in changed:
        print(f"[reindex]   {c}")
    print(f"[reindex] reconstruction sha256 = {recon['reconstructed_sha256'][:16]}...")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
