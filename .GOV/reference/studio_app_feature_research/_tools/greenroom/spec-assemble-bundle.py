#!/usr/bin/env python3
"""Assemble a PROPOSED Master Spec bundle carrying the rebuilt Studio section.

This writes a proposal, never the live bundle. Promoting a proposal into
`.GOV/spec/` and moving SPEC_CURRENT is the operator's decision, not this tool's.

WHAT IT DOES

Section 14 lives as one module file per the bundle's own convention. Nineteen staged sub-section
files rebuild most of it. The merge rule is chosen so nothing can be silently lost:

    A clause defined in a staged sub-section REPLACES the v02.205 clause of the same anchor.
    A clause NOT redefined by any staged sub-section is CARRIED FORWARD verbatim.

That rule needs no disposition table to be parsed correctly and it cannot drop a clause by
accident. Every carried-forward clause is listed in the assembly report, so a reviewer can see
exactly what survived from the old text and where it now sits.

Sub-sections with no staged replacement are copied through unchanged.

Reference tooling under .GOV/reference.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import shutil
from pathlib import Path

SUBSECTION = re.compile(r"^##\s+(14\.\d+)\b(.*)$", re.M)
DEF_LINE = re.compile(r"^(?:\*\*)?\[(STU-[A-Z]+-\d+[a-z]?)\]")
MODULE_NUM = re.compile(r"^14-(\d+)-")
FRONTMATTER = re.compile(r"\A---\n.*?\n---\n", re.S)


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def clause_spans(lines: list[str]) -> list[tuple[int, int, str]]:
    """Paragraph-opening anchors and the line span each one owns."""
    marks, fence = [], False
    for i, ln in enumerate(lines):
        if ln.lstrip().startswith("```"):
            fence = not fence
            continue
        if fence:
            continue
        m = DEF_LINE.match(ln)
        if m and (i == 0 or not lines[i - 1].strip()):
            marks.append((i, m.group(1)))
    spans = []
    for j, (start, anchor) in enumerate(marks):
        end = marks[j + 1][0] if j + 1 < len(marks) else len(lines)
        spans.append((start, end, anchor))
    return spans


def split_subsections(text: str) -> list[dict]:
    """Split section 14 into its ## 14.N sub-sections, keeping the preamble as index -1."""
    marks = [(m.start(), m.group(1), m.group(0)) for m in SUBSECTION.finditer(text)]
    out = []
    if not marks:
        return [{"key": None, "title": "", "body": text}]
    out.append({"key": None, "title": "", "body": text[:marks[0][0]]})
    for i, (pos, key, headline) in enumerate(marks):
        end = marks[i + 1][0] if i + 1 < len(marks) else len(text)
        out.append({"key": key, "title": headline.strip(), "body": text[pos:end]})
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--current-bundle", type=Path, required=True)
    ap.add_argument("--modules", type=Path, required=True, help="staged sub-section modules")
    ap.add_argument("--out-bundle", type=Path, required=True, help="PROPOSED bundle root")
    ap.add_argument("--new-version", default="v02.206")
    ap.add_argument("--prev-version", default="v02.205")
    args = ap.parse_args()

    src_mod_dir = args.current_bundle / "spec-modules"
    sec14 = src_mod_dir / "14-studio-creative-suite.md"
    if not sec14.exists():
        print(f"[assemble] FATAL: {sec14} not found")
        return 2

    # --- copy the whole bundle first, then rewrite only section 14 --------------------------
    if args.out_bundle.exists():
        shutil.rmtree(args.out_bundle)
    shutil.copytree(args.current_bundle, args.out_bundle)

    old_text = sec14.read_text(encoding="utf-8", errors="replace")
    subs = split_subsections(old_text)

    staged: dict[str, dict] = {}
    for p in sorted(args.modules.glob("*.md")):
        m = MODULE_NUM.match(p.name)
        if not m:
            continue
        key = f"14.{int(m.group(1))}"
        body = FRONTMATTER.sub("", p.read_text(encoding="utf-8", errors="replace"))
        staged.setdefault(key, {"files": [], "body": ""})
        staged[key]["files"].append(p.name)
        staged[key]["body"] += ("\n\n" if staged[key]["body"] else "") + body.strip()

    new_anchors: set[str] = set()
    for key, s in staged.items():
        for _, _, a in clause_spans(s["body"].split("\n")):
            new_anchors.add(a)

    report = {"replaced": [], "carried_forward": [], "copied_unchanged": [], "appended": [],
              "superseded_in_place": []}
    pieces: list[str] = []

    for sub in subs:
        key = sub["key"]
        if key is None:
            pieces.append(sub["body"].rstrip())
            continue
        if key not in staged:
            # Copied through, but the replacement rule still applies: a clause a staged module
            # redefines must not survive here, or the section carries two definitions of one
            # anchor and every citation of it becomes ambiguous.
            lines = sub["body"].split("\n")
            spans = clause_spans(lines)
            drop = [(a, b, anc) for (a, b, anc) in spans if anc in new_anchors]
            if drop:
                keep, cursor = [], 0
                for start, end, anchor in drop:
                    keep.extend(lines[cursor:start])
                    cursor = end
                    report["superseded_in_place"].append({"subsection": key, "anchor": anchor})
                keep.extend(lines[cursor:])
                body = "\n".join(keep).rstrip()
                note = ("\n\n> Clauses formerly defined in this sub-section and now redefined by a "
                        "rebuilt sub-section have been removed here so that each anchor has exactly "
                        "one definition: "
                        + ", ".join(f"`[{a}]`" for _, _, a in drop) + ".")
                pieces.append(body + note)
            else:
                pieces.append(sub["body"].rstrip())
            report["copied_unchanged"].append(key)
            continue

        lines = sub["body"].split("\n")
        carried, spans = [], clause_spans(lines)
        for start, end, anchor in spans:
            if anchor not in new_anchors:
                carried.append((anchor, "\n".join(lines[start:end]).rstrip()))

        block = [sub["title"], ""]
        block.append(staged[key]["body"].strip())
        if carried:
            block += [
                "", f"### {key}.CARRIED Clauses carried forward unchanged from {args.prev_version}",
                "",
                "These clauses were NOT redefined by the rebuilt sub-section above, so they remain "
                "in force exactly as written. They are gathered here rather than left in place so "
                "that the rebuilt text reads in one sequence and nothing is lost in the merge.",
                "",
            ]
            for anchor, body in carried:
                block.append(body)
                block.append("")
                report["carried_forward"].append({"subsection": key, "anchor": anchor})
        pieces.append("\n".join(block).rstrip())
        report["replaced"].append({"subsection": key, "from_files": staged[key]["files"],
                                   "carried_forward": len(carried)})

    # Staged sub-sections with no counterpart in v02.205 are appended in numeric order.
    existing = {s["key"] for s in subs if s["key"]}
    for key in sorted(set(staged) - existing, key=lambda k: float(k.split(".")[1])):
        pieces.append(f"## {key}\n\n" + staged[key]["body"].strip())
        report["appended"].append({"subsection": key, "from_files": staged[key]["files"]})

    merged = "\n\n---\n\n".join(p for p in pieces if p.strip()) + "\n"
    out_sec = args.out_bundle / "spec-modules" / "14-studio-creative-suite.md"
    out_sec.write_text(merged, encoding="utf-8", newline="\n")

    # --- bundle metadata ---------------------------------------------------------------------
    for name in ("INDEX.json", "indexed-spec-manifest.json"):
        p = args.out_bundle / name
        if not p.exists():
            continue
        raw = p.read_text(encoding="utf-8")
        p.write_text(raw.replace(args.prev_version, args.new_version), encoding="utf-8", newline="\n")

    changelog = args.out_bundle / "spec-changelog.jsonl"
    entry = {
        "version": args.new_version, "previous_version": args.prev_version, "at": now(),
        "section": "14 Studio",
        "change": "Studio section rebuilt from offline teardown of the installed applications rather "
                  "than from vendor help pages. Adds video timeline, motion and keyframing, "
                  "compositing, web authoring, asset-library binding, operator shell, tools and "
                  "controls, and tooltips and manual sub-sections.",
        "modules_merged": sorted({f for s in staged.values() for f in s["files"]}),
        "clauses_after_merge": len(new_anchors) + len(report["carried_forward"]),
        "clauses_carried_forward": len(report["carried_forward"]),
        "status": "PROPOSED: not promoted to SPEC_CURRENT. Operator approval required.",
    }
    with changelog.open("a", encoding="utf-8", newline="\n") as fh:
        fh.write(json.dumps(entry, ensure_ascii=False) + "\n")

    rep_path = args.out_bundle.parent / "spec-assembly-report.json"
    rep_path.write_text(json.dumps({
        "schema_id": "handshake.reference.studio_spec_assembly@1",
        "generated_at": now(),
        "status": "PROPOSED_BUNDLE_NOT_PROMOTED",
        "merge_rule": "A clause defined in a staged sub-section replaces the same anchor from the "
                      "previous version. A clause not redefined anywhere is carried forward verbatim.",
        "new_section_bytes": len(merged),
        "subsections_replaced": report["replaced"],
        "subsections_appended": report["appended"],
        "subsections_copied_unchanged": report["copied_unchanged"],
        "clauses_superseded_in_an_unreplaced_subsection": report["superseded_in_place"],
        "clauses_superseded_in_place_count": len(report["superseded_in_place"]),
        "clauses_carried_forward_count": len(report["carried_forward"]),
        "clauses_carried_forward": report["carried_forward"],
    }, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")

    print(f"[assemble] section 14: {len(old_text):,} -> {len(merged):,} bytes")
    print(f"[assemble] replaced={len(report['replaced'])} appended={len(report['appended'])} "
          f"unchanged={len(report['copied_unchanged'])} carried_forward={len(report['carried_forward'])}")
    print(f"[assemble] clauses superseded inside an unreplaced sub-section: "
          f"{len(report['superseded_in_place'])}")
    print(f"[assemble] PROPOSED bundle -> {args.out_bundle}")
    print(f"[assemble] report -> {rep_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
