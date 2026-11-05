"""Read-only prefilter: which MT spec anchors are named anywhere in product crates at the tree given.
Writes S/research/truth-check-20261105-mt-prefilter.json. A prefilter only: a named anchor does not prove behaviour, and an unnamed anchor does not prove absence."""
import glob, json, os, re, subprocess, sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
KERNEL = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], cwd=HERE, text=True).strip()
T = KERNEL + "/.GOV/task_packets/WP-KERNEL-STUDIO/"
OUT = sys.argv[2]  # output JSON path
TREE = sys.argv[1]  # a product tree: <lane worktree>/handshake-studio-assets
ANCHOR = re.compile(r"\b(?:STU|GUI-DESKTOP|NATIVE-RENDER|HBR)-[A-Z0-9]*-?\d+[a-z]?\b")

src_hits = defaultdict(Counter)
test_hits = defaultdict(Counter)
crate_loc = Counter()
crate_tests = Counter()
for crate in sorted(os.listdir(TREE + "/crates")):
    root = f"{TREE}/crates/{crate}"
    for dp, _, fs in os.walk(root):
        if "/target" in dp.replace("\\", "/"):
            continue
        for f in fs:
            if not f.endswith(".rs"):
                continue
            p = os.path.join(dp, f)
            try:
                s = open(p, encoding="utf-8", errors="replace").read()
            except OSError:
                continue
            rel = p.replace("\\", "/")
            is_test = "/tests/" in rel
            crate_loc[crate] += s.count("\n")
            crate_tests[crate] += len(re.findall(r"#\[(?:tokio::)?test\]", s))
            for a in ANCHOR.findall(s):
                (test_hits if is_test else src_hits)[a][crate] += 1

rows = []
for f in sorted(glob.glob(T + "MT-*.json")):
    try:
        d = json.load(open(f, encoding="utf-8"))
    except Exception as e:
        rows.append({"mt": os.path.basename(f)[:-5], "error": str(e)[:80]})
        continue
    lc = d.get("lifecycle")
    status = lc.get("status") if isinstance(lc, dict) else lc
    anchors = d.get("spec_anchor") or []
    if isinstance(anchors, str):
        anchors = [anchors]
    notes = d.get("implementation_notes") or {}
    sub = notes.get("source_subdirectory", "") if isinstance(notes, dict) else ""
    in_src = sorted({c for a in anchors for c in src_hits.get(a, {})})
    in_test = sorted({c for a in anchors for c in test_hits.get(a, {})})
    scope = d.get("scope") or {}
    rows.append({
        "mt": d.get("mt_id") or os.path.basename(f)[:-5],
        "status": status,
        "domain": d.get("domain"),
        "anchors": anchors,
        "planned_crate": sub.rsplit("/", 1)[-1] if sub else None,
        "anchor_in_src": in_src,
        "anchor_in_tests": in_test,
        "gui_obligation": bool(d.get("gui_obligation")),
        "summary": (scope.get("summary") or "")[:160] if isinstance(scope, dict) else "",
    })

by_domain = defaultdict(Counter)
for r in rows:
    if "error" in r:
        continue
    dom = r["domain"] if isinstance(r["domain"], str) else "null"
    by_domain[dom]["total"] += 1
    by_domain[dom]["status_pass" if str(r["status"]).startswith("PASS") else "status_pending"] += 1
    by_domain[dom]["anchor_named_in_src"] += 1 if r["anchor_in_src"] else 0
    by_domain[dom]["anchor_named_in_tests"] += 1 if r["anchor_in_tests"] else 0
    by_domain[dom]["anchor_named_nowhere"] += 1 if not (r["anchor_in_src"] or r["anchor_in_tests"]) else 0

out = {
    "file_kind": "truth_check_prefilter",
    "created_for": "lead truth check 2026-11-05",
    "tree": TREE,
    "caveat": "PREFILTER ONLY. An anchor named in code does not prove the behaviour, and an anchor not named does not prove absence (many crates cite no anchors). Status comes from the MT files as they stand.",
    "totals": {"mt_files": len(rows), "crates": len(crate_loc), "rust_lines": sum(crate_loc.values()), "test_fns": sum(crate_tests.values())},
    "by_domain": {k: dict(v) for k, v in sorted(by_domain.items(), key=lambda kv: -kv[1]["total"])},
    "by_crate": {c: {"rust_lines": crate_loc[c], "test_fns": crate_tests[c]} for c in sorted(crate_loc, key=lambda c: -crate_loc[c])},
    "mts": rows,
}
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
    f.write("\n")
print(json.dumps(out["totals"]))
print(json.dumps(out["by_domain"], indent=0)[:3600])
print(sorted(out["by_crate"].items(), key=lambda kv: -kv[1]["rust_lines"])[:16])
print("size MB", os.path.getsize(OUT) / 1e6)
