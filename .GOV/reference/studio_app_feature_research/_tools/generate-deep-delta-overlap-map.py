#!/usr/bin/env python3
"""Generate 57-deep-delta-cross-app-overlap-map.md from the deep-delta files 51-55.

Groups deep-delta rows that share a normalized capability label across two or more
source apps, so shared behavior maps to ONE Handshake-native Studio primitive
(same policy as 44-cross-app-overlap-and-affinity-dedupe-map.md). Source rows are
never deleted; they remain app-specific provenance variants.
"""
import re
import sys
import io
from collections import defaultdict
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DELTA_FILES = {
    "photoshop": "51-photoshop-deep-feature-delta.md",
    "illustrator": "52-illustrator-deep-feature-delta.md",
    "indesign": "53-indesign-deep-feature-delta.md",
    "affinity": "54-affinity-deep-feature-delta.md",
    "figma": "55-figma-deep-feature-delta.md",
}
OUT = ROOT / "57-deep-delta-cross-app-overlap-map.md"

STOPWORDS = {"the", "a", "an", "and", "or", "of", "with", "for", "to", "in", "on"}
# menu-path prefixes and generic decorations to strip before comparing labels
PREFIX_RE = re.compile(
    r"^(effect|filter|object|layer|type|select|view|window|image|edit|file|table)\s*>\s*", re.I
)


def normalize(name: str) -> str:
    n = name.strip().lower()
    while True:
        stripped = PREFIX_RE.sub("", n)
        # also strip second-level menu segments like "pathfinder > add"
        if ">" in stripped:
            stripped = stripped.split(">")[-1].strip()
        # strip short decoration prefixes like "live filter: gaussian blur"
        if ":" in stripped:
            head, _, tail = stripped.partition(":")
            if tail.strip() and len(head.split()) <= 3:
                stripped = tail.strip()
        if stripped == n:
            break
        n = stripped
    n = re.sub(r"\(.*?\)", " ", n)
    n = re.sub(r"[^a-z0-9]+", " ", n)
    toks = [t for t in n.split() if t not in STOPWORDS]
    # singularize trivial plurals for grouping stability
    toks = [t[:-1] if len(t) > 3 and t.endswith("s") and not t.endswith("ss") else t for t in toks]
    return " ".join(toks)


REC_RE = re.compile(
    r"^\s*-\s+id:\s*\"?(?P<id>[A-Za-z0-9_.-]+)\"?\s*\n"
    r"(?P<body>(?:[ \t]+\S.*\n?)+?)"
    r"(?=^\s*-\s+id:|\Z)",
    re.M,
)
FIELD_RE = {
    "name": re.compile(r"^\s+name:\s*\"?(.+?)\"?\s*$", re.M),
    "primitive_domain": re.compile(r"^\s+primitive_domain:\s*\"?([a-z_]+)\"?\s*$", re.M),
}


def parse_records(path: Path):
    text = path.read_text(encoding="utf-8")
    records = []
    for block in re.findall(r"```yaml\n(.*?)```", text, re.S):
        if "record_role" not in block or "feature_deep_delta" not in block:
            continue
        for m in REC_RE.finditer(block):
            body = m.group("body")
            name_m = FIELD_RE["name"].search(body)
            dom_m = FIELD_RE["primitive_domain"].search(body)
            if not name_m:
                continue
            records.append(
                {
                    "id": m.group("id"),
                    "name": name_m.group(1).strip(),
                    "primitive_domain": dom_m.group(1).strip() if dom_m else "unclassified",
                }
            )
    return records


def main() -> int:
    per_app = {}
    for app, fname in DELTA_FILES.items():
        path = ROOT / fname
        if not path.exists():
            print(f"MISSING delta file: {fname}", file=sys.stderr)
            return 1
        per_app[app] = parse_records(path)

    groups = defaultdict(list)
    for app, records in per_app.items():
        for rec in records:
            key = normalize(rec["name"])
            if not key:
                continue
            groups[key].append((app, rec))

    overlap_groups = {
        k: v for k, v in groups.items() if len({app for app, _ in v}) >= 2
    }

    domain_counts = defaultdict(lambda: defaultdict(int))
    for app, records in per_app.items():
        for rec in records:
            domain_counts[rec["primitive_domain"]][app] += 1

    today = date.today().isoformat()
    out = io.StringIO()
    total_rows = sum(len(v) for v in per_app.values())
    out.write("---\n")
    out.write("file_id: 57-deep-delta-cross-app-overlap-map\n")
    out.write("file_kind: generated_overlap_map\n")
    out.write("topic_id: SFR-DEEP-DELTA-OVERLAP\n")
    out.write('title: "Deep-Delta Cross-App Overlap Map"\n')
    out.write("status: generated\n")
    out.write(f"updated_at: \"{today}\"\n")
    out.write(f"generator: _tools/generate-deep-delta-overlap-map.py\n")
    out.write(f"source_files: [51, 52, 53, 54, 55]\n")
    out.write(f"deep_delta_row_count: {total_rows}\n")
    out.write(f"overlap_group_count: {len(overlap_groups)}\n")
    out.write("---\n\n")
    out.write("## [SFR-DEEP-DELTA-OVERLAP] Deep-Delta Cross-App Overlap Map\n\n")
    out.write(
        "> GENERATED FILE - regenerate with `python _tools/generate-deep-delta-overlap-map.py` "
        "after any 51-55 change. Policy is identical to file 44: shared capability across "
        "source apps maps to ONE Handshake-native Studio primitive; app rows stay as "
        "source-specific provenance variants and are never deleted.\n\n"
    )

    out.write("### [SFR-DEEP-DELTA-OVERLAP.coverage] Coverage\n\n```yaml\ncoverage:\n")
    out.write(f"  deep_delta_row_count: {total_rows}\n")
    for app, records in sorted(per_app.items()):
        out.write(f"  {app}_rows: {len(records)}\n")
    out.write(f"  overlap_group_count: {len(overlap_groups)}\n")
    out.write(
        "  policy: shared_behavior_maps_to_one_studio_primitive_source_rows_stay_as_variants\n"
    )
    out.write("```\n\n")

    out.write(
        "### [SFR-DEEP-DELTA-OVERLAP.domain-counts] Per-Domain Row Counts\n\n```yaml\ndomain_counts:\n"
    )
    for dom in sorted(domain_counts):
        out.write(f"  {dom}:\n")
        for app in sorted(domain_counts[dom]):
            out.write(f"    {app}: {domain_counts[dom][app]}\n")
    out.write("```\n\n")

    out.write(
        "### [SFR-DEEP-DELTA-OVERLAP.groups] Cross-App Overlap Groups\n\n```yaml\noverlap_groups:\n"
    )
    for key in sorted(overlap_groups):
        members = overlap_groups[key]
        apps = sorted({app for app, _ in members})
        doms = sorted({rec["primitive_domain"] for _, rec in members})
        out.write(f"- overlap_key: \"{key}\"\n")
        out.write(f"  apps: [{', '.join(apps)}]\n")
        out.write(f"  primitive_domains: [{', '.join(doms)}]\n")
        out.write(f"  studio_primitive_rule: one_studio_primitive_multiple_source_variants\n")
        out.write("  member_ids:\n")
        for app, rec in sorted(members, key=lambda x: x[1]["id"]):
            out.write(f"  - {rec['id']}\n")
    out.write("```\n\n")

    out.write("### [SFR-DEEP-DELTA-OVERLAP.sources] Sources\n\n```yaml\nsources:\n")
    for i, (app, fname) in enumerate(sorted(DELTA_FILES.items()), 1):
        out.write(f"  - {{ id: DDO-S{i:02d}, path: \"{fname}\", note: \"{app} deep-delta rows.\" }}\n")
    out.write("```\n")

    OUT.write_text(out.getvalue(), encoding="utf-8")
    print(f"wrote {OUT.name}: rows={total_rows} overlap_groups={len(overlap_groups)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
