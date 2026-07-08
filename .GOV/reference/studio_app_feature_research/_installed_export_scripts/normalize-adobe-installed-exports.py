#!/usr/bin/env python3
"""Normalize Adobe installed-app inventory JSON files into corpus JSONL rows."""
from __future__ import annotations

import argparse
import json
from pathlib import Path


def normalize(path: Path) -> list[dict]:
    data = json.loads(path.read_text(encoding="utf-8"))
    rows = []
    for index, row in enumerate(data.get("rows", [])):
        source_surface = row.get("source_surface", "unknown")
        name = row.get("name") or row.get("id") or f"row-{index}"
        rows.append(
            {
                "source_id": f"{data.get('app','adobe')}.{source_surface}.{index:06d}",
                "app": data.get("app"),
                "app_name": data.get("app_name"),
                "app_version": data.get("app_version"),
                "platform": data.get("platform"),
                "locale": data.get("locale"),
                "source_surface": source_surface,
                "display_name": name,
                "raw": row,
            }
        )
    return rows


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    all_rows = []
    for path in args.inputs:
        all_rows.extend(normalize(path))

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8", newline="\n") as fh:
        for row in all_rows:
            fh.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")
    print(f"rows={len(all_rows)} out={args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
