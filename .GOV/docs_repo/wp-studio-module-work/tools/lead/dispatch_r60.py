"""Lead: STUDIO-R60 ATTEMPT 4 dispatch generator (research/decision-20261113-r60-round-fit.json, option d): FULL with the
canary on the round slot <sole artifact root>/WP-KERNEL-STUDIO/studio-round/target. Extends the attempt-3 generator; earlier
attempts' prompts are in kernel history (33a6d64d, 1316c8b0, 94044b95).
Usage: python -X utf8 tools/lead/dispatch_r60.py <tip_sha> [--repo DIR] [--preview DIR | --write]
  <tip_sha>  the frozen origin tip (8-40 hex). Counts since c4d06841 (R59), 35c50d3b (attempt 2) and b57c82d1 (attempt 3),
             the ancestor checks (those three plus the trunk repair 89f27330) and the root inputs since b57c82d1 are read
             from --repo (default <Handshake Worktrees>/wt-studio-assets, the export's object source; read-only git).
  no flag    checks, then prints the READY count, the counts, the pre-freeze measurements and the claim rows; writes nothing
  --preview  dry run: writes the three dispatch files into DIR (any scratch dir)
  --write    writes <S>/validation/dispatch-prompt-R60.txt, <S>/validation/dispatch-R60.json and the free-space guard
             <studio-round>/validator-R60-a4/a4_dguard.sh; refuses when attempt 4's results or console already exist
S (Studio records root): env STUDIO_S, else two levels above this file. The kernel and <Handshake Worktrees> derive from S.
Sole artifact root (HSK-ART-001, CX-984-012/013): <Handshake Worktrees>/Handshake_Artifacts by discovery; HANDSHAKE_ARTIFACTS_ROOT
or HANDSHAKE_ARTIFACT_ROOT, when set, must name that same existing directory. Nothing is hard-coded to a disk.
Read-only apart from the files above: no build, no lock, no deletion, no process action. run-round.sh is not touched
(it keeps CX-984-014 line-tables-only and CARGO_INCREMENTAL=0)."""
import datetime, glob, hashlib, importlib.util, json, os, pathlib, re, shutil, subprocess, sys

ATTEMPT = 4
ALERT_FREE_GIB = 1000          # decision file guard value: message the lead
FLOOR_FREE_GIB = 500           # clean infrastructure stop below this free space (rationale in floor_rationale())
ROUND_GROWTH_UPPER_GIB = 456   # decision space_arithmetic.d_attempt_4: about 450 canary + 6 union (UNVERIFIED projection)
BASES = {"c4d06841": "R59 candidate", "35c50d3b": "attempt 2", "b57c82d1": "attempt 3"}
TRUNK_REPAIR = "89f27330"      # SMB-FIX-MENU-CATALOG-COVERAGE-1 landed (MENU-INV-1 trunk red repaired; kernel f973efe7)
CANARY, UNION = f"R60-canary-{ATTEMPT}", f"R60-union-{ATTEMPT}"
WORKDIR, GUARD = f"validator-R60-a{ATTEMPT}", f"a{ATTEMPT}_dguard.sh"
GIB = 1024.0 ** 3


def die(msg):
    sys.exit(f"dispatch_r60: {msg}")


def opt(name):
    if name not in sys.argv:
        return None
    i = sys.argv.index(name)
    if i + 1 >= len(sys.argv) or sys.argv[i + 1].startswith("--"):
        die(f"{name} needs a value")
    return sys.argv[i + 1]


S_PATH = pathlib.Path(os.environ.get("STUDIO_S") or pathlib.Path(__file__).resolve().parents[2]).resolve()
if not (S_PATH / "ledger.json").is_file():
    die(f"no ledger.json in {S_PATH}: run from tools/lead/ or set STUDIO_S to the Studio records root")
S = S_PATH.as_posix()
KERNEL = S_PATH.parents[2]
WORKTREES = KERNEL.parent
preview, repo_arg = opt("--preview"), opt("--repo")
args = [a for a in sys.argv[1:] if not a.startswith("--") and a not in (preview, repo_arg)]
if len(args) != 1 or not re.fullmatch(r"[0-9a-f]{8,40}", args[0]):
    sys.exit(__doc__)
if preview and "--write" in sys.argv:
    die("--preview and --write are exclusive")


def same(a, b):
    return os.path.normcase(os.path.realpath(a)) == os.path.normcase(os.path.realpath(b))


def artifact_root():
    found = WORKTREES / "Handshake_Artifacts"
    if not found.is_dir():
        die(f"sole artifact root {found.as_posix()} does not exist (CX-984-012: never auto-create or substitute)")
    for var in ("HANDSHAKE_ARTIFACTS_ROOT", "HANDSHAKE_ARTIFACT_ROOT"):
        val = os.environ.get(var)
        if val is None:
            continue
        if not os.path.isabs(val) or not same(val, found):
            die(f"{var}={val} is not the discovered sole artifact root {found.as_posix()} (CX-984-013)")
        return found.resolve(), f"env {var}, equal to discovery"
    return found.resolve(), "discovery: <kernel parent>/Handshake_Artifacts"


ART, ART_HOW = artifact_root()
STUDIO = ART / "WP-KERNEL-STUDIO"
SLOT = STUDIO / "studio-round"
TARGET = SLOT / "target"
EXPORT_WS = SLOT / "src" / "handshake-studio-assets"
for p in (STUDIO, SLOT, TARGET, EXPORT_WS):
    if not p.is_dir():
        die(f"missing {p.as_posix()}: the round slot must exist (HSK-ART-003); this tool creates no target")
VOL = os.path.splitdrive(str(TARGET))[0].upper() or "/"
rel_art = lambda p: "Handshake_Artifacts/" + pathlib.Path(p).relative_to(ART).as_posix()


def git(*a):
    r = subprocess.run(["git", "-C", str(REPO), *a], capture_output=True, text=True, encoding="utf-8")
    if r.returncode not in (0, 1):
        die(f"git {' '.join(a)}: {r.stderr.strip()[:300]}")
    return r


REPO = pathlib.Path(repo_arg).resolve() if repo_arg else WORKTREES / "wt-studio-assets"
r = git("rev-parse", "--verify", "--quiet", args[0] + "^{commit}")
if r.returncode:
    die(f"{args[0]} is not a commit in {REPO.as_posix()}: fetch origin there first (read-only for this tool)")
cand = r.stdout.strip()
cand8 = cand[:8]
counts, ancestors = {}, {}
for base in [*BASES, TRUNK_REPAIR]:
    ancestors[base] = git("merge-base", "--is-ancestor", base, cand).returncode == 0
    if base in BASES:
        counts[base] = int(git("rev-list", "--count", f"{base}..{cand}").stdout.strip())
missing = [b for b, ok in ancestors.items() if not ok]
if missing:
    die(f"{cand8}: not descended from {', '.join(missing)} (the trunk repair {TRUNK_REPAIR} must be in the candidate)")
root_inputs = sorted(
    p.split("handshake-studio-assets/", 1)[1]
    for p in git("diff", "--name-only", f"b57c82d1..{cand}", "--", "handshake-studio-assets").stdout.split()
    if not re.match(r"handshake-studio-assets/crates/[^/]+/(src|tests)/", p))
config_changed = [p for p in root_inputs if p.startswith(".config/")]

# Pre-freeze measurements (decision step 3), read-only.
sys.dont_write_bytecode = True  # no __pycache__ in the kernel from this import
spec = importlib.util.spec_from_file_location("target_report", S_PATH / "tools" / "lead" / "target_report.py")
tr = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tr)
slot_row = tr.inspect_target("studio-round", str(TARGET), VOL.rstrip(":").lower(), {}, {})
binding_ok = bool(slot_row.get("bound_to")) and same(slot_row["bound_to"], EXPORT_WS / "Cargo.toml")
if not binding_ok:
    die(f"slot target binding {slot_row.get('bound_to')} is not {(EXPORT_WS / 'Cargo.toml').as_posix()}: run-round would refuse it")
du = shutil.disk_usage(TARGET)
free_gib, total_gib = round(du.free / GIB, 1), round(du.total / GIB, 1)
R60 = json.load(open(S + "/validation/round-R60.json", encoding="utf-8"))
a3 = R60["attempt_3"]
c_tree = a3["c_tree_end"]["tree"]
c_round_target = a3["canary"]["argv"].split("--target-dir ", 1)[1].split(" ", 1)[0]
c_free = round(shutil.disk_usage(os.path.splitdrive(c_tree)[0] + "/").free / GIB, 1) if os.path.splitdrive(c_tree)[0] else None


def commit_free_gb():
    ps = ("$m=Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory; "
          "'{0:N1}' -f (([double]$m.CommitLimit-[double]$m.CommittedBytes)/1e9)")
    try:
        out = subprocess.run(["powershell", "-NoProfile", "-Command", ps], capture_output=True, text=True, timeout=60).stdout.strip()
        return float(out.replace(",", "")) if out else None
    except (OSError, ValueError, subprocess.TimeoutExpired):
        return None


def lock_state(root):
    held = root / "held" / "owner"
    row = {"path": rel_art(root), "exists": root.is_dir()}
    if held.is_file():
        kv = dict(l.split("=", 1) for l in held.read_text(encoding="utf-8", errors="replace").splitlines() if "=" in l)
        row["holder"] = {k: kv.get(k) for k in ("class", "owner", "command", "acquired_at")}
    else:
        row["holder"] = None
    q = root / "queue"
    row["queued"] = sorted(e.name for e in q.iterdir()) if q.is_dir() else []
    return row


now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
disk_lock, gpu_lock = STUDIO / ".disk-build-lock", STUDIO / ".gpu-test-lock"
pre = {
    "at": now,
    "free_gib": {VOL: free_gib, **({os.path.splitdrive(c_tree)[0].upper(): c_free} if c_free is not None else {})},
    "volume_total_gib": {VOL: total_gib},
    "commit_free_gb": commit_free_gb(),
    "slot_target": {k: slot_row.get(k) for k in ("gib", "parts_gib", "older_binary_variants_gib", "newest_file_utc", "bound_to")},
    "disk_lock": lock_state(disk_lock),
    "gpu_lock": lock_state(gpu_lock),
    "export_identity_sha": json.load(open(SLOT / "ROUND_IDENTITY.json", encoding="utf-8")).get("sha"),
}
results_exist = [n for n in (f"results/{CANARY}", f"results/{UNION}", f"{CANARY}.console.txt", f"{UNION}.console.txt")
                 if (SLOT / n).exists()]


def floor_rationale():
    end = round(free_gib - ROUND_GROWTH_UPPER_GIB)
    return (f"{FLOOR_FREE_GIB} GiB is the decision's upper projection for this whole round (about {ROUND_GROWTH_UPPER_GIB} GiB: "
            f"canary about 450 plus union about 6, UNVERIFIED) rounded up, so at the floor {VOL} can still take one more "
            f"round-sized writer without filling; it is {FLOOR_FREE_GIB / total_gib * 100:.1f}% of the {total_gib} GiB volume. "
            f"From {free_gib} GiB free at generation the round alone should end near {end} GiB free, so the floor trips only "
            f"if other writers take about {end - FLOOR_FREE_GIB} GiB more: a backstop, not an expected stop. Between two 60 s "
            f"samples at most about 15 GiB is written (UNVERIFIED: about 250 MB/s HDD sequential write), so a stop lands far "
            f"above zero. The alert at {ALERT_FREE_GIB} GiB (decision value) gives about one more round of warning.")


# READY claims from the ledger at generation (unchanged from attempt 3).
L = json.load(open(S + "/ledger.json", encoding="utf-8"))["lanes"]
items = L.items() if isinstance(L, dict) else [(x.get("lane_id"), x) for x in L]
ready = [(k, v) for k, v in items if isinstance(v, dict) and v.get("status") == "READY_FOR_VALIDATION"]
lane_files = {}
for f in glob.glob(S + "/builders/lanes/*.json"):
    d = json.load(open(f, encoding="utf-8"))
    if d.get("lane"):
        lane_files[d["lane"]] = (os.path.basename(f), d)
rows = []
for lid, v in ready:
    fn, d = lane_files.get(lid, ("?", {}))
    slug = lid.lower()
    reviews = sorted(os.path.basename(p) for p in glob.glob(S + f"/research/review-{slug}-*.json"))
    notes = d.get("notes", {})
    cut = notes.get("cut") or []
    mts = notes.get("mts_proved") or []
    rows.append(
        f"- **{lid}** | {v.get('claim')} | `builders/lanes/{fn}` | brief `builders/briefs/lane-{slug}.md`"
        + (f" | pre-build review {', '.join('`research/' + r + '`' for r in reviews)}" if reviews else "")
        + (f"\n  - MTs the builder names: {'; '.join(str(m) for m in mts)[:400]}" if mts else "")
        + (f"\n  - Cut, named and not claimed: {'; '.join(str(c) for c in cut)[:400]}" if cut else "")
    )
claims = "\n".join(rows)

slot = SLOT.as_posix()
target = TARGET.as_posix()
guard_path = (SLOT / WORKDIR / GUARD).as_posix()
canary_argv = ["bash", "tools/run-round.sh", "--canary", "--all", "--results", f"{slot}/results/{CANARY}", "--target-dir", target]
union_argv = ["bash", "tools/run-round.sh", "--all", "--results", f"{slot}/results/{UNION}", "--target-dir", target]
shq = lambda argv: " ".join(f'"{a}"' if " " in a else a for a in argv)
root_inputs_text = ", ".join(root_inputs) if root_inputs else "none"
config_note = (f"changes {', '.join(config_changed)}" if config_changed else "changes no `.config/` file")

GUARD_SH = r'''#!/usr/bin/env bash
# STUDIO-R60 attempt @@ATTEMPT@@ free-space guard, generated by tools/lead/dispatch_r60.py (kernel); do not edit, regenerate.
# Watches the free space of the volume holding the round target ($HERE/../target) with df, every 60 s (30 s after the alert).
# Below @@ALERT@@ GiB free: ALERT line once (the validator messages the lead). Below @@FLOOR@@ GiB free: stops the run cleanly
# as infrastructure: SIGTERM to this session's run-round bash after re-checking its identity (msys pid -> win pid), so its
# TERM trap writes round.json complete=false and releases the disk lock; then taskkill /T /F of the running unit's first
# non-bash descendant (a process this session started). Read-only otherwise; ends when the console carries <TAG>_EXIT=.
# Log: $HERE/dguard.log. Usage: a@@ATTEMPT@@_dguard.sh <TAG> <console name> <results name> <msys pid> <win pid>
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
TAG=${1:?tag}; C="$HERE/../${2:?console name}"; RES="$HERE/../results/${3:?results name}"; MSYS_PID=${4:?msys pid}; WIN_PID=${5:?win pid}
T="$HERE/../target"; L="$HERE/dguard.log"; ALERT_GIB=@@ALERT@@; FLOOR_GIB=@@FLOOR@@; alerted=0; sent=0
[ -d "$T" ] || { echo "dguard: no round target at $T" >&2; exit 2; }
now() { date -u +%Y-%m-%dT%H:%M:%SZ; }
free_gib() { df -Pk "$T" 2>/dev/null | awk 'NR==2 && $4 ~ /^[0-9]+$/ {printf "%.1f", $4/1048576}'; }
lt() { awk -v a="$1" -v b="$2" 'BEGIN{exit !(a+0<b+0)}'; }
echo "$(now) GUARD_START tag=$TAG alert_below=$ALERT_GIB floor=$FLOOR_GIB msys=$MSYS_PID win=$WIN_PID free=$(free_gib) GiB volume_of=$T" >>"$L"
while ! grep -q "^${TAG}_EXIT=" "$C" 2>/dev/null; do
  f=$(free_gib)
  u=$( { wc -l <"$RES/units.tsv"; } 2>/dev/null ); last=$( { tail -1 "$RES/units.tsv" | cut -f1; } 2>/dev/null )
  if [ -z "$f" ]; then
    echo "$(now) READ_FAILED df $T" >>"$L"
  else
    echo "$(now) free=${f} GiB units=${u:-0} last=${last}" >>"$L"
    if [ $alerted -eq 0 ] && lt "$f" "$ALERT_GIB"; then alerted=1; echo "$(now) ALERT_${ALERT_GIB} free=${f} GiB (message the lead)" >>"$L"; fi
    if [ $sent -eq 0 ] && lt "$f" "$FLOOR_GIB"; then
      wp=$(cat /proc/$MSYS_PID/winpid 2>/dev/null)
      if [ "$wp" = "$WIN_PID" ]; then
        kill -TERM "$MSYS_PID"; sent=1
        echo "$(now) FLOOR_SIGTERM msys $MSYS_PID win $WIN_PID at free=${f} GiB (< $FLOOR_GIB)" >>"$L"
        kid=$(powershell -NoProfile -Command "\$q=New-Object System.Collections.Queue; \$q.Enqueue($WIN_PID); \$hit=\$null; while(\$q.Count -and -not \$hit){ \$p=\$q.Dequeue(); foreach(\$c in @(Get-CimInstance Win32_Process -Filter ('ParentProcessId=' + \$p) | Select-Object ProcessId,Name)){ if (\$c.Name -ne 'bash.exe') { \$hit=\$c.ProcessId; break } else { \$q.Enqueue(\$c.ProcessId) } } }; \$hit" 2>/dev/null | tr -d '\r' | head -1)
        [ -n "$kid" ] && taskkill //PID "$kid" //T //F >>"$L" 2>&1
        echo "$(now) FLOOR_STOP_SENT unit tree win ${kid:-none}" >>"$L"
      else
        echo "$(now) FLOOR_STOP_SKIPPED identity mismatch (winpid now '${wp}')" >>"$L"; sent=2
      fi
    fi
  fi
  if [ $alerted -eq 1 ]; then sleep 30; else sleep 60; fi
done
echo "$(now) ${TAG}_EXIT seen: $(grep "^${TAG}_EXIT=" "$C")" >>"$L"
'''.replace("@@ATTEMPT@@", str(ATTEMPT)).replace("@@ALERT@@", str(ALERT_FREE_GIB)).replace("@@FLOOR@@", str(FLOOR_FREE_GIB))
guard_sha = hashlib.sha256(GUARD_SH.encode("utf-8")).hexdigest()

text = f"""ROLE: You are the independent validator for validation round STUDIO-R60 of WP-KERNEL-STUDIO, attempt {ATTEMPT}. Studio Kit is a Rust rewrite of the Adobe suite, Figma and Affinity. The product repo is Nuntissura/assets, branch codex/handshake-studio-assets, subfolder handshake-studio-assets.
- Reasoning tier: opus/xhigh.
- You judge and write no product code. Be adversarial. A READY is only a builder's claim. Every lane below had a pre-build code-check and red-team read and fixed what it found; those reads are leads, not findings you may reuse, and nothing in them was run by their authors.

## Authority
Read these first. They win over this prompt.
- Kernel `{KERNEL.as_posix()}`.
- Codex `.GOV/codex/Handshake_Codex_v1.4.md`, rules CX-VAL-001..011, CX-EXEC-015, CX-VERIFY-002, CX-SAFE-001, CX-STATUS-001..003, CX-GIT-001A, CX-984-012..014 and CX-107.
- Studio records `.GOV/docs_repo/wp-studio-module-work/`:
  - `research/decision-20261113-r60-round-fit.json`: why attempt {ATTEMPT} runs on the {VOL} round slot (option d), facts F1-F13, steps and risks.
  - `validation/dispatch-R60.json`: this dispatch as data (candidate, counts, slot, locks, entrypoint argv, guard, pre-freeze measurements). If it and this prompt disagree, stop and ask the lead.
  - `validation/round-R59.json` is the latest opened round. `validation/round-R57.json` is the latest FULL round. `validation/round-R60.json` holds attempts 1, 2 and 3 (none opened); `validation/round-R53.json` is the precedent for a round that opened on a later attempt.
  - `ledger.json` keys `round_R60`, `direction.workflow_20261107`, `direction.operator_direction_20261107`, `direction.operator_direction_20261111`, `direction.blocker_decisions_20261107` (window_layer, gpu_contention, matrix_cut, landing, spine_gate), `direction.lead_rulings_20261108`, `direction.resume_20261108` (r60_attempt_2, plan_t, c_cap_20261111), `direction.pause_20261111` and `direction.pause_20261112`.
  - `research/lead-rulings-20261108-by-lane.json` (rulings given by message, written down, including the common list).
  - HANDOFF.md topics `lessons`, `how-to-lead`, `workflow` and `state`.
  - `builders/DISPATCH_TEMPLATES.md` topic `validator`, including step 6 (MT status). Its step 3 gameplan confirmation is retired (CX-VAL-010, 2026-10-20): skip it.
- Binding on every claim: the HOST INTEGRATION rules in `builders/briefs/wave-parity-2-common.md`, and Operator decisions D1, D6, D7 and D8. Write a HOST INTEGRATION section per claim.
- Master Spec v02.229.

## Candidate
- This is ATTEMPT {ATTEMPT} of STUDIO-R60. No earlier attempt opened (CX-VAL-010), so the round id stays STUDIO-R60. Attempt 1 at cede5958: canary step 0 exit 75 (F-R60-1). Attempt 2 at 35c50d3b: canary stopped on the C: cap projection after 11 of 164 units (c-cap-projection-canary-identity-rebuild). Attempt 3 at b57c82d1: step 0 green, then the 368 GiB C: guard stopped the canary at 2026-11-13T08:25:35Z after 18 of 164 test units plus the fixture and 3 prebuilds, all exit 0 (c-cap-368-canary-cold-harness-identities). Read `validation/round-R60.json` (`attempts`, `attempt_2`, `attempt_2_probes`, `attempt_3`) and the ledger `round_R60` entry.
- Record form (lead decision: R53 precedent, `validation/round-R53.json` `record_form`; do not open a new round id):
  - `validation/round-R60.json`, if attempt {ATTEMPT} opens: the top level becomes attempt {ATTEMPT}; attempt 1's present top-level record moves verbatim under `attempt_1`, except `attempt` and `status`, which take attempt 1's values from `attempts[0]`; `attempt_2`, `attempt_2_probes` and `attempt_3` stay; `attempts` gains an attempt {ATTEMPT} entry; `record_form` says so. If it does not open: add `attempt_{ATTEMPT}` beside `attempt_3` and an attempt {ATTEMPT} entry in `attempts`, and change only the top-level `attempt` and `status`, as attempts 2 and 3 did.
  - Ledger `round_R60` keeps the latest attempt at the top level, with `attempt_1` and `attempt_2` beside it. In either case its top level becomes attempt {ATTEMPT} and attempt 3's present top-level record moves verbatim under `attempt_3`; `attempt_1` and `attempt_2` stay.
- Since attempt 3 the lead chose option (d) of the decision file: this attempt builds only in the {VOL} round slot below. The C: round target `{c_round_target}` keeps attempt 3's completed identities but is NOT used: do not build in it and do not touch it. The R60-a3 fix wave (the SMB-FIX-* claims below, including the trunk repair SMB-FIX-MENU-CATALOG-COVERAGE-1 for the native-host gui_menu MENU-INV-1 red) and the 2026-11-13 wave landed after b57c82d1; their claims are in the list below.
- Freeze exactly {cand8} ({cand}), the origin tip at dispatch (the lead verified it with ls-remote). {counts['c4d06841']} commits since the R59 candidate c4d06841, {counts['35c50d3b']} since attempt 2's 35c50d3b, {counts['b57c82d1']} since attempt 3's b57c82d1; c4d06841, 35c50d3b, b57c82d1 and the trunk repair {TRUNK_REPAIR} are ancestors (generator check). Re-derive the counts and the claim owner of every commit (attempt 3 form: validator-R60-a3/claims.json); name unowned commits.
- Entrypoint (CX-VAL-007; `builders/DISPATCH_TEMPLATES.md` topic `validator` steps 1-3): refresh the export `{slot}/src` to {cand8} with `tools/export-refresh.sh` as attempt 3 did, then from cwd `{EXPORT_WS.as_posix()}` run
  `{shq(canary_argv)}`
  and only if the canary is green
  `{shq(union_argv)}`.
  Nothing goes before `bash` and nothing is added after: no environment variable, flag or wrapper; no `--release`; no other target dir.
- Round target `{target}`: the fixed studio-round slot under the sole artifact root (HSK-ART-003, CX-984-012). At generation it held {slot_row['gib']} GiB, newest file {slot_row['newest_file_utc']} (stale libraries, so nearly every unit builds cold), bound by `.studio-workspace-root` to `{slot_row['bound_to']}`, which run-round's binding check requires. It stays as it is: what still matches is reused and nothing is removed (HSK-ART-005). If run-round refuses the target, stop and report; never create another target dir.
- Locks: under the sole artifact root run-round's target class is artifact-root. It queues for the {VOL} disk build lock `{disk_lock.as_posix()}` as class round (queue priority), holds it per cargo unit and yields one gap per 300 s slice; TMP is `{slot}/tmp`; its test steps take the machine-wide GPU test lock `{gpu_lock.as_posix()}` as before. There is no C: hold. The lead has asked the builders to start no new hold until your final message; a ticket already queued may build in a gap. Record every other holder; touch no lock.
- Debug info: run-round.sh itself sets CARGO_PROFILE_DEV_DEBUG and CARGO_PROFILE_TEST_DEBUG to line-tables-only and CARGO_INCREMENTAL=0 (CX-984-014). That stays as it is; you add nothing.
- This round is FULL with the canary (CX-VAL-011: R57 full, R58 and R59 scoped, R60 never opened; root inputs changed). The generator's read of b57c82d1..{cand8} (paths outside crates/*/src and crates/*/tests): {root_inputs_text}. Re-derive the root-input list yourself from c4d06841..{cand8}. Attempt 3's partial canary is not reusable (incomplete, another candidate, another target; CX-VAL-002, CX-VERIFY-002).
- CX-EXEC-015(4): attempts 2 and 3 stopped on C: cap infrastructure classes and opened no round. The lead's reading (decision file risks; UNVERIFIED): (4) does not bar this attempt, since neither opened a round, and this attempt changes the declared build location to the {VOL} slot, where the C: cap class cannot recur. Record the reading.
- Lead pre-freeze check (F-R60-2): `python -X utf8 -I tools/registry-sync.py .` check mode exit 0 at {cand8}. Step 0 is still yours to run and record.
- Matrix cut: the generator's diff b57c82d1..{cand8} {config_note}. Verify it; if the steward's cut of the 5 strict-subset harness feature units is not in the candidate, run the matrix as it stands and say so.
- Attempt 3's probe failures (`attempt_3.probes`; ledger `round_R60.probe_failures`) and findings leads (`attempt_3.findings_leads`, F-R60-22 onward) are leads: re-check each against {cand8}. The SMB-FIX-* claims below name their fixes. Reuse `validator-R60-a3/review-R60-a3-*.json` only as leads.
- New crates since R59 include hsk-studio-vulkan and hsk-studio-native-host (winit =0.30.13 there only, ash =0.38.0, offline SPIR-V) and kumihimo-ui. Check direction.blocker_decisions_20261107: winit only in native-host; no kumihimo-* crate has an hsk-studio-* dependency in its closure; permissive licences only (cargo-deny or the repo's own check). Their tests take the GPU lock; a GPU or window test that cannot get a device is SKIPPED with its reason, never PASS.
- Vulkan validation layer: `crates/hsk-studio-vulkan/src/owner.rs` enables VK_LAYER_KHRONOS_validation in debug builds when the loader lists it (owner.rs:716 at b57c82d1; re-locate it at {cand8}). Set no VK_* or other variable (CX-VAL-007). Record the loader version `vulkaninfoSDK --summary` reports (Vulkan SDK Bin folder, not on PATH; path in round-R60.json `attempt_2.vulkan_loader`, attempt 3's reading in `attempt_3.vulkan_loader`), judge `validation:` receipts against owner.rs as written, and classify any GPU test that behaves differently from R59 from its output.
- PASS verdicts carry forward under CX-VAL-002 only where their inputs are unchanged; in a full round every unit runs. At the freeze, CX-VAL-009 re-opens every PASS lane whose relevant inputs intersect the candidate diff (attempt 3 counted 407 in validator-R60-a3/reopen.json at b57c82d1): recompute it at {cand8}.
- Lane worktrees and lane branches exist under CX-GIT-001A; the ledger lists them at `direction.workflow_20261107.lane_branches`; they are not defects in your inventory comparison.
- MEMORY: Chrome stays open by the Operator's instruction. Record commit free at start, during and at the end (attempt 3: 123.4 GB at the freeze, min 103.6; generator: {pre['commit_free_gb']} GB). A build failure from memory is infrastructure (CX-VAL-006): classify it from the failure output, never from timing alone, and report it rather than rerunning blind. You stop nothing you did not start.
- DISK ({VOL} free-space guard; it replaces attempt 3's C: cap stop): the generated guard `{guard_path}` (tools/lead/dispatch_r60.py; sha256 in `validation/dispatch-R60.json`; do not edit it) watches the free space of the volume holding the round target. Start it right after every run-round you launch, with that run's identity: `bash "{guard_path}" <TAG> <console name> <results name> <msys pid> <win pid>` (tags R60C{ATTEMPT} and R60U{ATTEMPT}; consoles {CANARY}.console.txt and {UNION}.console.txt in `{slot}`). It samples every 60 s (30 s after the alert) into `{WORKDIR}/dguard.log`; below {ALERT_FREE_GIB} GiB free it writes ALERT_{ALERT_FREE_GIB} once (message the lead at once); below {FLOOR_FREE_GIB} GiB free it stops the run cleanly: SIGTERM to your run-round bash after an identity re-check (its TERM trap writes round.json complete=false and releases the lock), then taskkill /T /F of the running unit's tree. A guard stop is failure_kind infrastructure, failure_class d-free-floor-{FLOOR_FREE_GIB}; it opens no round and gives no verdict; record it and end your turn at once. Floor rationale: {floor_rationale()}
- Record {VOL} free and the slot target's size (tools/lead/target_report.py inspect_target, read-only) at the freeze, after each unit (the guard log plus your own samples) and at the end; report per-unit growth and a projection for the rest of the canary and the union with your first samples. Record the C: Studio tree `{c_tree}` at the freeze and at the end only, to show it unchanged. Record `df` at start and end. You delete nothing: not the slot's stale libraries, not round outputs.
- If the canary or any step 0 check fails, record it and end your turn AT ONCE with the failure output in your final message, so the lead can fix it and resume you.
- TIME: decision F12 records D: as a SATA HDD and C: as a SATA SSD. Record each unit's build and run time and compare with attempt 3 on the C: SSD (default build 16 m 20 s; cold harness feature units 13 to 16 min each); record the union's duration against R57 (FULL, 160 units, 3 h 48) and R59 (50 units, 3 h 02). Report build and run time from your own logs, and what else was building. Do not investigate further.
- Never write an environment dump.

## The READY claims
{claims}

For each claim: re-derive what it claims from the lane file and the diff, run its tests in your union (process tests spawn examples/studio-harness.exe: build it fresh, never trust a stale binary), break it from the outside with your own probes where the claim is behavioural, and judge it PASS_Vn or FAIL_Vn with findings. A builder's cut list is a named limit: judge whether each cut is honestly named or hides a defect.

## MT judging
- Follow step 6 for every MT a claim names. Write status only if judged met on a PASS_Vn lane. Do not edit MT proof commands.

## Output
Same as R52..R59, in the record form above:
- `validation/round-R60.json`, with `mt_judgments` and `mts_passed`.
- Ledger statuses and the `round_R60` entry (python json load-modify-dump, indent=1, ensure_ascii=False, newline '\\n').
- One START_LOG.md line.
- Verdicts in the lane files.
- MT files and `_MT_INDEX.json` per step 6. Write through a temp file plus os.replace, serialised exactly as HEAD: `json.dumps(obj)+'\\n'`.
- Working files in `{rel_art(SLOT)}/{WORKDIR}/`; results under `{rel_art(SLOT)}/results/{CANARY}` and `{UNION}`; the running record appends to `{rel_art(SLOT)}/RUNNING-R60.jsonl`.

To commit:
- Commit only those paths with `git -c core.fscache=false commit -F - -- <paths>`. The message ends with "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>".
- Fetch, and merge only if origin gov_kernel has new commits. Never rebase, never force.
- Push with `SHA=$(git rev-parse HEAD) && [ -n "$SHA" ] && git push origin "$SHA:refs/heads/gov_kernel"`. Check the exit code, then verify against origin.
- Leave `refund-claim-20261021.md` alone, including reads and searches, and leave other untracked files alone.

## Disk and process
- Never stop processes you did not start.
- Never touch locks by hand.
- Never delete anything.
- No runtime env overrides.
- Run long commands in the background.

## Final message
At most 18 lines:
- each claim's verdict and the lane counts;
- HOST INTEGRATION per claim, one line each;
- the MT result;
- findings by severity;
- any PASS lane disturbed;
- the canary result and the matrix cut decision;
- the selection and union numbers;
- GPU backends, Vulkan device, loader version, validation-layer state, and any timeout, classified;
- commit free, {VOL} free, slot size and `df` at start, minimum and end; the C: tree at start and end;
- the kernel commit sha.
"""

prompt_sha = hashlib.sha256(text.encode("utf-8")).hexdigest()
record = {
    "schema": "hsk.studio.round_dispatch@1",
    "round": "STUDIO-R60",
    "attempt": ATTEMPT,
    "generated_at": now,
    "generator": "tools/lead/dispatch_r60.py",
    "mode": "write" if "--write" in sys.argv else "preview",
    "decision_ref": "research/decision-20261113-r60-round-fit.json (option d)",
    "candidate": {"sha": cand, "commits_since": counts, "ancestors": ancestors, "trunk_repair": TRUNK_REPAIR,
                  "root_inputs_since_b57c82d1": root_inputs, "config_changed_since_b57c82d1": config_changed,
                  "object_source": REPO.name},
    "artifact_root": {"resolved_by": ART_HOW, "path_at_generation": ART.as_posix()},
    "round_slot": {"dir": rel_art(SLOT), "target": rel_art(TARGET), "volume": VOL, "export_workspace": rel_art(EXPORT_WS),
                   "binding_ok": binding_ok, "class": "artifact-root", "tmp": rel_art(SLOT / "tmp"),
                   "removal": "none (no deletion; stale libraries stay; HSK-ART-005)"},
    "locks": {"disk": {"class": "round", "slice_s": 300, **lock_state(disk_lock)}, "gpu": {"class": "round", **lock_state(gpu_lock)},
              "c_disk": "no hold: the C: round target is not used"},
    "entrypoint": {"cwd": rel_art(EXPORT_WS), "canary_argv": canary_argv, "union_argv": union_argv, "env_added": [],
                   "profile": "run-round.sh sets CARGO_PROFILE_{DEV,TEST}_DEBUG=line-tables-only and CARGO_INCREMENTAL=0 (CX-984-014); unchanged"},
    "results": {"canary": CANARY, "union": UNION, "consoles": [f"{CANARY}.console.txt", f"{UNION}.console.txt"],
                "working_dir": WORKDIR, "running_record": "RUNNING-R60.jsonl", "already_present": results_exist},
    "guard": {"script": rel_art(SLOT / WORKDIR / GUARD), "sha256": guard_sha, "volume": VOL, "sample_s": 60,
              "alert_below_free_gib": ALERT_FREE_GIB, "stop_below_free_gib": FLOOR_FREE_GIB,
              "failure_class_on_stop": f"d-free-floor-{FLOOR_FREE_GIB}", "rationale": floor_rationale(),
              "tags": [f"R60C{ATTEMPT}", f"R60U{ATTEMPT}"]},
    "c_tree": {"path": c_tree, "c_round_target": c_round_target, "use": "not used, not touched; measured at the freeze and the end only"},
    "pre_freeze": pre,
    "ready_claims": [lid for lid, _ in ready],
    "prompt": {"path": "validation/dispatch-prompt-R60.txt", "sha256": prompt_sha},
}
out_json = json.dumps(record, indent=1, ensure_ascii=False) + "\n"

print(f"{len(ready)} READY claims; candidate {cand8}; since c4d06841/35c50d3b/b57c82d1: "
      f"{counts['c4d06841']}/{counts['35c50d3b']}/{counts['b57c82d1']}; {VOL} free {free_gib} GiB; slot {slot_row['gib']} GiB "
      f"(newest {slot_row['newest_file_utc']}, binding ok); disk lock holder: {(pre['disk_lock']['holder'] or {}).get('owner')}")
missing_files = [lid for lid, _ in ready if lid not in lane_files]
if missing_files:
    print("WARNING: READY lanes without a lane file:", ", ".join(missing_files))
if results_exist:
    print("WARNING: attempt-4 outputs already present:", ", ".join(results_exist))


def put(path, body):
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(body)
    print("written:", path.as_posix())


if preview:
    d = pathlib.Path(preview)
    put(d / "dispatch-prompt-R60.txt", text)
    put(d / "dispatch-R60.json", out_json)
    put(d / GUARD, GUARD_SH)
elif "--write" in sys.argv:
    if results_exist:
        die("refusing --write: attempt-4 results or consoles already exist (" + ", ".join(results_exist) + ")")
    put(S + "/validation/dispatch-prompt-R60.txt", text)
    put(S + "/validation/dispatch-R60.json", out_json)
    put(SLOT / WORKDIR / GUARD, GUARD_SH)
else:
    print(json.dumps(pre, indent=1, ensure_ascii=False))
    print(claims)
