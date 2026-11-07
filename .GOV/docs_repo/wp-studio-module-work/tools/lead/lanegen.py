"""Lead's lane generator: writes brief, lane file and ledger row for one wave. Scratch only."""
import json, os, subprocess

S = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
COMMON_HEAD = "binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)"


def utc():
    return subprocess.check_output(["date", "-u", "+%Y-%m-%dT%H:%M:%SZ"], text=True).strip()


def dump(path, obj, indent):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(obj, indent=indent, ensure_ascii=False) + "\n")


def write_wave(lanes, base, log_line, drop_prefixes=(), add_candidates=()):
    now = utc()
    led = json.load(open(S + "/ledger.json", encoding="utf-8"))
    known = {e.get("lane_id") for e in led["lanes"]}
    for L in lanes:
        lid = L["id"]
        assert lid not in known, lid
        low = lid.lower()
        # HSK-ART-005: a lane may take over a completed lane's worktree and target together. `tree` is that worktree's
        # dir and stays the --owner, because the target is bound to one tree. Default: the lane's own new worktree.
        tree = L.get("tree", low)
        disk = L.get("disk", "d")
        hi = {
            "schema_id": "hsk.studio_mt_host_integration@1",
            "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030",
            "spec_version": "v02.229",
            "gui_host": "none",
            "gpu": "none",
            "diagnostics": {"internal_diagnostics": L["diag"], "flight_recorder": "none", "palmistry": "none"},
            "control_descriptors": "none",
            "attribution": L["attr"],
            "jobs": "none",
            "settings": "none",
            "refactor_exposure": L.get("exposure", "low"),
        }
        disk_arg = f" --disk {disk}" if disk == "d" else ""
        disk_word = "D:" if disk == "d" else "C:"
        brief = f"""---
file_id: LANE-{lid}
file_kind: lane_brief
updated_at: {now}
wp: WP-KERNEL-STUDIO
lane: {lid}
lane_file: builders/lanes/{L['file']}.json
crates: {L['crates_head']}
model: opus
effort: high
{COMMON_HEAD}
---

<topic id="target" status="active" summary="{L['summary']}">

## Target

{L['target']}

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{json.dumps(hi, ensure_ascii=False)}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/{tree}`, branch `lane/{low}`, base {base})

{L['paths']}

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on {disk_word} (`lane-check.sh --owner {tree}{disk_arg}`): step 0; {L['suites']}; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner {tree}{disk_arg} --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
"""
        with open(f"{S}/builders/briefs/{L['brief']}.md", "w", encoding="utf-8", newline="\n") as f:
            f.write(brief)
        lane = {
            "crate": L["crate"],
            "lane": lid,
            "status": None,
            "phase": "build",
            "worktree": f"wt-studio-lanes/{tree}",
            "base": base,
            "commits": [],
            "capabilities_done": [],
            "tests": {},
            "verdicts": [],
            "blockers": [],
            "lock_wait_s": 0,
            "bg_pids": [],
            "next": f"builder: red first, one bundled run on {disk_word}, push when green",
            "updated_at": now,
            "host_integration": hi,
            "notes": dict({"dispatched_at": now, "disk": disk_word[0], "closes": L["closes"], "closes_candidates": L["mts"]}, **({"reuses_tree_and_target": tree} if tree != low else {})),
        }
        dump(f"{S}/builders/lanes/{L['file']}.json", lane, 2)
        led["lanes"].append({
            "lane_id": lid,
            "status": None,
            "phase": "build",
            "crate": L["crate"],
            "lane_file": f"builders/lanes/{L['file']}.json",
            "brief": f"builders/briefs/{L['brief']}.md",
            "worktree": lane["worktree"],
            "base": base,
            "disk_planned": disk_word[0],
            "host_integration": "see lane file",
            "updated_at": now,
            "closes": L["closes"] + L["mts"],
        })
    d = led["direction"]
    d["next_lane_candidates"] = [
        c for c in d["next_lane_candidates"]
        if not any(c.get("item", "").startswith(p) for p in drop_prefixes)
    ]
    for item, owner in add_candidates:
        d["next_lane_candidates"].append({"added_at": now, "item": item, "owner": owner})
    led["updated_at"] = now
    dump(S + "/ledger.json", led, 1)
    with open(S + "/START_LOG.md", "a", encoding="utf-8", newline="\n") as f:
        f.write(f"- {now} lead: {log_line}\n")
    return now
