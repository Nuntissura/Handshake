import json, os, subprocess, sys, datetime, re
# Lead helper: write snapshots/SNAPSHOT-<UTC>.json from ledger/team/register + repo heads.
# Usage: python tools/lead-snapshot.py "<usage note>"   (product worktree: env STUDIO_PRODUCT_WORKTREE, default ../wt-studio-assets next to the kernel)
W = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
K = os.path.abspath(os.path.join(W, "..", "..", ".."))
P = os.environ.get("STUDIO_PRODUCT_WORKTREE", os.path.abspath(os.path.join(K, "..", "wt-studio-assets")))
os.chdir(W)
usage = sys.argv[1]
now = datetime.datetime.now(datetime.timezone.utc)
stamp = now.strftime("%Y%m%dT%H%M%SZ"); iso = now.strftime("%Y-%m-%dT%H:%M:%SZ")
def git(d, *a):
    return subprocess.run(["git", "-C", d, *a], capture_output=True, text=True).stdout.strip()
l = json.load(open("ledger.json", encoding="utf-8"))
t = json.load(open("team.json", encoding="utf-8"))
r = json.load(open("workflow/plausible-register.json", encoding="utf-8"))
def cat(s):
    s = str(s)
    if s.startswith("PASS"): return "PASS"
    if s == "READY_FOR_VALIDATION": return "READY"
    if s == "BLOCKED": return "BLOCKED"
    if s == "done": return "DONE"
    return "QUEUED_OR_HELD"
by = {}
for x in l["lanes"]:
    by[cat(x.get("status"))] = by.get(cat(x.get("status")), 0) + 1
non_pass = [{"lane": x["lane_id"], "crate": x.get("crate"), "status": x.get("status"), "phase": str(x.get("phase"))[:200]} for x in l["lanes"] if cat(x.get("status")) != "PASS"]
crates = {}
for x in l["lanes"]:
    for c in re.split(r"\s*\+\s*", str(x.get("crate"))):
        if c.startswith("hsk-studio-"):
            crates.setdefault(c, []).append(cat(x.get("status")))
all_pass = sorted(c for c, v in crates.items() if all(s == "PASS" for s in v))
not_all = {c: sorted(set(v)) for c, v in sorted(crates.items()) if c not in all_pass}
rounds = {k: {kk: l[k].get(kk) for kk in ("candidate", "status", "verdicts", "fails", "not_judged", "canary")} for k in sorted(l) if k.startswith("round_R")}
snap = {
 "file_id": f"WP-STUDIO-MODULE-WORK-SNAPSHOT-{stamp}", "file_kind": "work_snapshot", "schema": "studio-snapshot.v1", "taken_at": iso,
 "authority_note": "Point-in-time copy for handoff. Live authority: ledger.json, team.json, workflow/plausible-register.json, validation/*, START_LOG.md. If this file disagrees with them, they win.",
 "repos": {
  "governance_kernel": {"repo": "https://github.com/Nuntissura/Handshake", "branch": "gov_kernel", "worktree": "wt-gov-kernel", "head": git(K, "rev-parse", "HEAD"), "origin_head": git(K, "rev-parse", "origin/gov_kernel"), "work_folder": ".GOV/docs_repo/wp-studio-module-work"},
  "product": {"repo": "https://github.com/Nuntissura/assets", "branch": "codex/handshake-studio-assets", "subfolder": "handshake-studio-assets", "worktree": "wt-studio-assets", "head": git(P, "rev-parse", "HEAD"), "origin_head": git(P, "rev-parse", "origin/codex/handshake-studio-assets"), "uncommitted": git(P, "status", "--short").splitlines()[:20]},
  "artifacts": "Handshake_Artifacts/WP-KERNEL-STUDIO (sibling of the worktrees; never build on C:)"},
 "lanes": {"total": len(l["lanes"]), "by_category": by, "non_pass": non_pass},
 "crates": {"count": len(crates), "all_pass": all_pass, "not_all_pass": not_all},
 "rounds": rounds,
 "running_agents": [{"name": m["name"], "model": m.get("model"), "role": m.get("role"), "agent_id": m.get("agent_id")} for m in t["members"] if str(m.get("status")) == "running"],
 "open_plausible_items": [{"id": i["id"], "lane": i.get("lane"), "disposition": i.get("disposition"), "note": (i.get("disposition_note") or i.get("finding", ""))[:300]} for i in r["open"] if i.get("status") == "OPEN"],
 "resume_plan": l.get("resume_plan"),
 "operator_decisions_pending": ["D20 render-gpu + round entrypoint 4th verdict (lead recommends allow)", "D22-D26 ProRes/codec licensing, STU-VID-051 spec conflict, codec selection, DNx spec download, ISO 14496-15 access", "D9 interop-media -> pulse edge", "D17 Figma-authored .fig fixtures", "D5 Photoshop black-box oracle session", "D13/STU-CON-044", "D21 optional AGL download", "repo-local 3.8 GB handshake-studio-assets/target (delete or keep)", "empty folder x at the C: root (delete by hand)", "STUDIO_MODULES.json status vocabulary (only PLANNED_NOT_IMPLEMENTED defined; ledger is live)"],
 "usage_at_snapshot": usage}
out = f"snapshots/SNAPSHOT-{stamp}.json"
json.dump(snap, open(out, "w", encoding="utf-8"), indent=1)
print(out); print(json.dumps(by)); print("all_pass", len(all_pass), "not_all", json.dumps(not_all))
print("open PLR", len(snap["open_plausible_items"]), "running", len(snap["running_agents"]))
