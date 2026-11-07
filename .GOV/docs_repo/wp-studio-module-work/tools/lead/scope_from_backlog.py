"""Lead: write research/scope-20261107-<slug>.json from a row of workflows/backlog-20261107.json, for wave_from_scope.py.

Usage: python scope_from_backlog.py <LANE-ID> [<LANE-ID> ...]
The builder reads the code and the scenarios itself, so the brief carries the verified facts, not a full design.
"""
import json
import os
import sys

S = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
back = json.load(open(os.path.join(S, "workflows", "backlog-20261107.json"), encoding="utf-8"))
rows = {r["id"]: r for r in back["top_lanes"]}
ATTR = "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor"
for lane in sys.argv[1:]:
    r = rows[lane]
    low = lane.lower()
    slug = low.replace("smb-", "", 1)
    facts = r.get("verified_facts") or []
    facts_text = "\n".join("- " + (json.dumps(f, ensure_ascii=False) if not isinstance(f, str) else f) for f in facts)
    crates = r.get("crates") or []
    paths = r.get("paths")
    target = f"""{r.get('summary', '')}

Why this lane: the eight professional project scenarios in `S/workflows/scenario-*.json` show engines that exist and cannot be reached. This row of `S/workflows/backlog-20261107.json` ranks it: steps unblocked {r.get('steps_unblocked')}, scenarios {json.dumps(r.get('scenarios_unblocked'), ensure_ascii=False)}. Read that row and the scenario steps it unblocks: they are your acceptance. A step is done when a model can run it through a command, in the JSONL loop of `harness/src/cli/drive.rs` once SMB-HARNESS-DRIVE-1 lands, and the result reaches save and export.

Facts verified at c4d06841 by the backlog agent, read-only, not run. Treat them as claims and check them:
{facts_text}

Depends on: {json.dumps(r.get('depends_on'), ensure_ascii=False)}. If a dependency is not on origin yet, build what does not need it and tell the lead what waits.

How to work:
1. Read the code first. Use the spec's names and cite clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`). Use only names already in the spec, MTs or code.
2. Red test and focused test per item. Engine, command and test: no draw code and no panel. The GUI is being rebuilt in pure Vulkan, and a panel lane will call your command.
3. Commands register in the existing catalog (`RUN_COMMANDS` in `harness-core/src/action.rs`, or a `viewer.<method>` row) with typed parameters and typed refusals. Each mutating command is one named undo step, carries the caller's attribution, and survives save and reopen.
4. V4-06: a short list of sibling sites, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. About 5 commits. Width over polish. State what you cut.
MTs: read those your slice proves and name them in your report; the lead records them. Do not edit MT files.

Out of scope: panels, drawing, RF-01, render-gpu, any Cargo.lock change without asking the lead."""
    scope = {
        "id": lane, "file": "hsk-studio-" + slug, "brief": "lane-" + low, "crate": ", ".join(crates) if isinstance(crates, list) else str(crates),
        "crates_head": ", ".join(c.replace("hsk-studio-", "") for c in crates) if isinstance(crates, list) else str(crates),
        "summary": str(r.get("summary", "")).replace('"', "'")[:200], "target": target,
        "diag": "producer_through_observe: one terminal frame per command, codes and counters only", "attr": ATTR, "exposure": "medium",
        "paths": paths if isinstance(paths, str) else "; ".join(paths or []), "suites": "the suites of the crates you edit, with named targets only for harness and harness-view unless you edit their src",
        "closes": ["workflows/backlog-20261107.json " + lane], "mts": r.get("mts") or [], "tree": r.get("tree"), "disk": r.get("disk") or "d",
    }
    out = os.path.join(S, "research", f"scope-20261107-{slug}.json")
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(scope, indent=1, ensure_ascii=False) + "\n")
    print(slug, "tree", scope["tree"], "disk", scope["disk"], "target chars", len(target))
