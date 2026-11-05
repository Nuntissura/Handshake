"""Lead: record one lane READY. Usage: record_ready.py <spec.json>
spec: {lane_id, lane_file, claim, tip, log, commit_msg, candidates:[[item,owner]], sd:[{gap,needs,lead_reading}], lead_decisions:[{on,decision}]}
Verifies the lane file says READY and that tip is an ancestor of product origin, then updates ledger, log, enrichment, commits and pushes."""
import json, os, subprocess, sys
from lanegen import S, dump, utc

KERNEL = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], cwd=S, text=True).strip()
SCOUT = os.environ["STUDIO_SCOUT_WORKTREE"]  # any clean lane worktree of the product repo
REL = ".GOV/docs_repo/wp-studio-module-work"


def run(*a, cwd=KERNEL):
    return subprocess.run(a, cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


spec = json.load(open(sys.argv[1], encoding="utf-8"))
now = utc()
lf = f"{S}/builders/lanes/{spec['lane_file']}.json"
lane = json.load(open(lf, encoding="utf-8"))
assert lane["status"] == "READY_FOR_VALIDATION", lane["status"]
assert lane["commits"], "no commits"

run("git", "fetch", "-q", "origin", "codex/handshake-studio-assets", cwd=SCOUT)
origin = run("git", "rev-parse", "origin/codex/handshake-studio-assets", cwd=SCOUT)
subprocess.run(["git", "merge-base", "--is-ancestor", spec["tip"], origin], cwd=SCOUT, check=True)
print("origin", origin[:8], "contains", spec["tip"])

if spec.get("lead_decisions"):
    lane["notes"].setdefault("lead_decisions", []).extend({"at": now, **d} for d in spec["lead_decisions"])
    dump(lf, lane, 2)

led = json.load(open(S + "/ledger.json", encoding="utf-8"))
found = False
for e in led["lanes"]:
    if e.get("lane_id") == spec["lane_id"]:
        found = True
        e["status"] = "READY_FOR_VALIDATION"
        e["phase"] = "validation"
        e["updated_at"] = now
        e["claim"] = spec["claim"]
assert found, spec["lane_id"]
for item, owner in spec.get("candidates", []):
    led["direction"]["next_lane_candidates"].append({"added_at": now, "item": item, "owner": owner})
led["updated_at"] = now
dump(S + "/ledger.json", led, 1)

paths = [f"{REL}/ledger.json", f"{REL}/START_LOG.md", f"{REL}/builders/lanes/{spec['lane_file']}.json"]
sds = []
if spec.get("sd"):
    ep = f"{S}/spec-enrichment/STUDIO-ENRICHMENT-20261030-HOST-INTEGRATION.json"
    en = json.load(open(ep, encoding="utf-8"))
    ids = [int(i["id"].split("-")[1]) for i in en["open_items_for_spec_owner"] if str(i.get("id", "")).startswith("SD-")]
    n = max(ids)
    for s in spec["sd"]:
        n += 1
        sds.append(f"SD-{n}")
        en["open_items_for_spec_owner"].append({"id": f"SD-{n}", **s, "source": f"lane {spec['lane_id']}, v02.229", "status": "PROPOSED"})
    dump(ep, en, 1)
    paths.append(f"{REL}/spec-enrichment/STUDIO-ENRICHMENT-20261030-HOST-INTEGRATION.json")

log = spec["log"] + (f" {', '.join(sds)} PROPOSED." if sds else "")
with open(S + "/START_LOG.md", "a", encoding="utf-8", newline="\n") as f:
    f.write(f"- {now} lead: {log}\n")

msg = spec["commit_msg"] + (f"; {', '.join(sds)}" if sds else "") + "\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n"
tmp = os.path.abspath(sys.argv[1] + ".msg.txt")
open(tmp, "w", encoding="utf-8", newline="\n").write(msg)
run("git", "add", "--", *paths)
run("git", "-c", "core.fscache=false", "commit", "-q", "-F", tmp, "--", *paths)
sha = run("git", "rev-parse", "HEAD")
assert sha
run("git", "push", "-q", "origin", f"{sha}:refs/heads/gov_kernel")
print("kernel", sha[:8], "pushed;", run("git", "rev-parse", "--short", "origin/gov_kernel"))
print(run("git", "status", "--short"))
