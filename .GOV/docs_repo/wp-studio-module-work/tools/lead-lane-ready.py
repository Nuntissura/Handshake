# Lead helper: mark a lane READY_FOR_VALIDATION in ledger.json + team.json + journal.
# Usage: python tools/lead-lane-ready.py <utc-now> <LANE_ID> <team-member-name> <commit> "<note>"
import json, sys, os
os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
now, lane, member, commit, note = sys.argv[1:6]
l = json.load(open("ledger.json", encoding="utf-8"))
for x in l["lanes"]:
    if x["lane_id"] == lane:
        x.update(status="READY_FOR_VALIDATION", phase="validation", candidate=commit, note=note)
        x["commits"] = list(dict.fromkeys(x.get("commits",[]) + [commit]))
l.setdefault("next_round_after_R5", [])
if lane not in l["next_round_after_R5"]:
    l["next_round_after_R5"].append(lane)
l["updated_at"] = now
json.dump(l, open("ledger.json","w",encoding="utf-8"), indent=1)
t = json.load(open("team.json", encoding="utf-8"))
for m in t["members"]:
    if m["name"] == member:
        m["status"] = "done " + now + " (" + lane + " READY at " + commit + ")"
t["updated_at"] = now
json.dump(t, open("team.json","w",encoding="utf-8"), indent=1)
with open("journal/tick-log.jsonl","a",encoding="utf-8") as f:
    f.write(json.dumps({"t":now,"event":lane+" READY_FOR_VALIDATION "+commit+" ("+member+")","lanes_moved":[lane+" READY"],"commits":[commit],"progress":"direct: product commit"})+"\n")
print("ok", now)
