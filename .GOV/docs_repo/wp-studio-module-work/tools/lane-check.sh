#!/usr/bin/env bash
# Bundled lane check (Operator 2026-10-20 build strategy): ONE disk-lock hold runs clippy over the tested crates and
# their direct workspace dependents, then the one test run, so a lane never takes separate lock acquisitions for
# check, clippy and each test. The test command is written out literally after `--` (it must start with
# `cargo nextest run`), so the gameplan hook sees and gates it like any test run and the retry guard keys on it.
# Usage (from the lane workspace, leading cd first):
#   cd "<lane worktree>/handshake-studio-assets" && bash "<kernel>/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" \
#     --owner <lane-id-lowercase> [--disk ssd|d] [--tests NAME[,NAME...]] -- cargo nextest run -p <crate> [-p ...] [nextest args]
# --owner     STUDIO_OWNER (required; lane id). --disk: ssd (default for lanes: the granted C: target) or d.
# --tests     test names joined into ONE filter `-E 'test(A) | test(B)'` appended to the test command (substring match);
#             or pass your own -E in the test command (not both).
# Crates: every -p/--package of the test command; their --features/-F apply to clippy of those crates only.
# Dependents: workspace members that depend on a tested crate (any dependency kind), from `cargo metadata --no-deps`;
#   clippy --tests runs over them without features (STUDIO_LANE_CHECK_NO_DEPENDENTS=1 skips them and logs
#   dependents=SKIPPED(STUDIO_LANE_CHECK_NO_DEPENDENTS=1)). A failed `cargo metadata` (exit, unreadable JSON, no
#   workspace members) stops the check with "lane-check: cargo metadata failed" and exit 2, never dependents=[].
# Step 0 (round parity, F-C55-1, lead 2026-10-24): FIRST, before clippy and outside the disk lock (read-only scan,
#   ~1 s), the round's step-0 policy over the WHOLE lane workspace (cwd): V4-03 machine-local paths (crates/*/src and
#   crates/*/tests, .config/machine-paths.toml [pinned]) and V4-05 frozen duplicates (crates/*/src,
#   .config/duplicate-fingerprints.toml allowed sites). Logic = the round's own: the step-0 policy heredoc is read
#   from the lane tree's tools/run-round.sh (the code the round runs on that candidate); the identical embedded copy
#   below is used only if that file or block is missing, and a textual mismatch is reported (the round's wins).
#   A hit prints the round's messages and exits with the round's code (72 V4-03, 71 V4-05) without clippy or tests.
#   lane-check.sh --step0-only            run only step 0 on the cwd workspace (no --owner, no test command).
#   lane-check.sh --step0-self-test DIR   run the round's block and the embedded copy on DIR, compare exit, stdout
#                                         (JSON) and stderr; prints STEP0_SELF_TEST OK|FAIL and exits 0|1.
# Per-commit compile check (F-R48-4, F-R49-1; steward 2026-10-31), run before every push (brief rule RED COMMITS):
#   lane-check.sh --owner <lane> [--disk ssd|d] --per-sha <base>[..<tip>]   (no test command; <tip> defaults to HEAD)
#   For every commit of `git rev-list --reverse <base>..<tip>` (base excluded, as in git), oldest first: the commit's
#   OWN tree (`git archive <sha>:<workspace dir>` unpacked with current mtimes into <owner tmp>/per-sha-<tip>/tree; the
#   lane's working tree is not used) gets the round's step-0 policy, then `cargo check --offline --tests` over the
#   commit's changed crates (crates/<dir>/ paths of `git diff <sha>^ <sha>`; root Cargo.toml, Cargo.lock,
#   rust-toolchain.toml, .cargo/ or .config/round-features.toml = every member) and their direct workspace dependents
#   (path dependency of any kind, from that commit's metadata): one default-features unit over the whole set, then
#   each feature set .config/round-features.toml declares for those crates ([crates] and non-empty [ignored] sets,
#   e.g. harness-view [viewer]), crates ordered dependencies first. It stops at the first failing unit and exits with
#   cargo's status, naming the sha and unit (PER_SHA_RESULT FAIL sha=... unit=...); all clean = PER_SHA_RESULT OK, 0.
#   Target: the lane's own target dir (no new target, P1/P3). Workspace-member units carry a profile override
#   (codegen-units = 255 for every member, dev and test profiles; --config <owner tmp>/per-sha-<tip>/isolate.toml), so
#   their fingerprints differ from the lane's own check/clippy/test units: a later lane build never reuses an artifact
#   compiled from another commit's tree (the reason for the .studio-workspace-root binding), while dependency
#   artifacts are shared. One disk-lock hold for the whole range, as the bundled run. No --locked (a commit waiting
#   for a steward sync may carry an unsynced lock; the round's step 0 reports that). Logs: <artifact root>/WP-KERNEL-
#   STUDIO/<owner>/per-sha-<tip8>/<sha8>-<unit>.log (first line = the sha); the temp tree is removed afterwards.
# Exit: 72/71 step 0 policy hit (round codes), 2 usage or step 0 config error; else clippy's status when it fails
#   (the tests are not run), else the test run's status. --per-sha: the first failing unit's status (step 0: 72/71/2),
#   2 on a range/archive/metadata error, else 0.
set -uo pipefail
owner="" disk=ssd tests="" step0_only=0 step0_selftest="" per_sha=""
while [ $# -gt 0 ]; do
  case "$1" in
    --owner) owner=${2:?}; shift 2 ;;
    --per-sha) per_sha=${2:?}; shift 2 ;;
    --disk) disk=${2:?}; shift 2 ;;
    --tests) tests=${2:?}; shift 2 ;;
    --step0-only) step0_only=1; shift ;;
    --step0-self-test) step0_selftest=${2:?}; shift 2 ;;
    --) shift; break ;;
    *) echo "lane-check: unknown argument $1 (test command goes after --)" >&2; exit 2 ;;
  esac
done
# Step 0 policy (round parity). Embedded copy of the step-0 policy heredoc of tools/run-round.sh (handshake-studio-assets
# 42b304c7); keep it byte-identical to the round's block (--step0-self-test compares both).
STEP0_PY_EMBEDDED=$(cat <<'PY'
import json, pathlib, re, sys, tomllib

sys.stdout.reconfigure(newline="\n")
root = pathlib.Path(sys.argv[1])
def load(name, schema):
    p = root / ".config" / name
    if not p.exists():
        print(f"run-round: .config/{name} missing", file=sys.stderr); sys.exit(2)
    doc = tomllib.loads(p.read_text(encoding="utf-8"))
    if doc.get("schema") != schema:
        print(f"run-round: .config/{name} schema must be {schema}", file=sys.stderr); sys.exit(2)
    return doc
def files(*subs):
    for crate in sorted(p for p in (root / "crates").iterdir() if p.is_dir()):
        for sub in subs:
            if (crate / sub).is_dir():
                for f in sorted((crate / sub).rglob("*"), key=lambda q: q.as_posix()):
                    data = f.read_bytes() if f.is_file() else b"\0"
                    if b"\0" not in data:
                        yield f.relative_to(root).as_posix(), data.decode("utf-8", errors="replace").splitlines()
paths_cfg = load("machine-paths.toml", "hsk.studio.machine-paths@1")
dup_cfg = load("duplicate-fingerprints.toml", "hsk.studio.duplicate-fingerprints@1")
pinned = paths_cfg.get("pinned", {})
MP = re.compile(r"(?<![A-Za-z0-9])[A-Za-z]:(?:/|\\(?![\"']))|Users[\\/]")
hits = {}
for rel, lines in files("src", "tests"):
    for i, line in enumerate(lines, 1):
        if MP.search(line):
            hits.setdefault(rel, []).append(f"{rel}:{i}: {line.strip()[:160]}")
path_violations = [{"file": rel, "hits": len(h), "pinned": pinned.get(rel), "lines": h[:20]}
                   for rel, h in sorted(hits.items()) if len(h) > pinned.get(rel, 0)]
pins_dropped = {rel: {"pinned": n, "now": len(hits.get(rel, []))} for rel, n in pinned.items()
                if len(hits.get(rel, [])) < n}
src = list(files("src"))
dups = []
for fp in dup_cfg.get("fingerprint", []):
    pats = [re.compile(p) for p in fp["patterns"]]
    sites = {}
    for rel, lines in src:
        for i, line in enumerate(lines, 1):
            if any(p.search(line) for p in pats):
                sites.setdefault(rel, []).append(f"{rel}:{i}: {line.strip()[:160]}")
    allowed = list(fp["allowed"])  # path prefixes relative to the workspace root (exact files or dirs)
    ok = lambda rel: any(rel.startswith(a) for a in allowed)
    new = {rel: sites[rel][:20] for rel in sorted(sites) if not ok(rel)}
    dups.append({"id": fp["id"], "new_sites": new, "sites": sorted(sites),
                 "allowed_without_match": [a for a in allowed if not any(r.startswith(a) for r in sites)]})
new_dups = [d for d in dups if d["new_sites"]]
print(json.dumps({"machine_paths": {"violations": path_violations, "pins_dropped": pins_dropped,
                                    "files_with_hits": len(hits)},
                  "duplicates": dups}, indent=1))
for v in path_violations:
    print(f"V4-03 machine-local path: {v['file']} hits={v['hits']} pinned={v['pinned']}", *v["lines"], sep="\n  ",
          file=sys.stderr)
for d in new_dups:
    for rel, lines in d["new_sites"].items():
        print(f"V4-05 new {d['id']} site: {rel}", *lines, sep="\n  ", file=sys.stderr)
sys.exit(72 if path_violations else 71 if new_dups else 0)
PY
)
step0_py=$(command -v python || command -v python3) || { echo "lane-check: python 3.11+ required" >&2; exit 2; }
# The round's own block from <ws>/tools/run-round.sh (between the step0.policy.json heredoc line and its PY end).
step0_round_block() {
  [ -f "$1/tools/run-round.sh" ] || return 1
  tr -d '\r' <"$1/tools/run-round.sh" | awk '/step0\.policy\.json.*<<.PY.$/{f=1;next} f&&/^PY$/{exit} f'
}
# step0_scan WS SCRIPT -> exit code of the round's policy; stdout JSON to $step0_json, stderr to $step0_txt.
step0_scan() {
  local out err rc
  out=$(mktemp) err=$(mktemp)
  "$step0_py" -X utf8 -I - "$1" <<<"$2" >"$out" 2>"$err"; rc=$?
  step0_json=$(cat "$out"); step0_txt=$(cat "$err"); rm -f "$out" "$err"
  return $rc
}
lane_step0() {
  local ws=$1 script round rc
  round=$(step0_round_block "$ws")
  if [ -n "$round" ]; then
    script=$round
    [ "$round" = "$STEP0_PY_EMBEDDED" ]       || echo "lane-check: note: embedded step-0 copy differs from $ws/tools/run-round.sh; using the round's block (update lane-check.sh)" >&2
  else
    script=$STEP0_PY_EMBEDDED
    echo "lane-check: note: no step-0 block in $ws/tools/run-round.sh; using the embedded copy" >&2
  fi
  step0_scan "$ws" "$script"; rc=$?
  case $rc in
    0) echo "lane-check: step 0 policy clean (V4-03 machine paths, V4-05 frozen duplicates)" >&2; return 0 ;;
    71|72)
      echo "lane-check: step0 policy: $([ $rc -eq 72 ] && echo 'V4-03 machine-local path' || echo 'V4-05 new frozen-duplicate site') (round would exit $rc; clippy and tests not run)" >&2
      printf '%s
' "$step0_txt" | head -n 40 >&2
      return $rc ;;
    *) printf '%s
' "$step0_txt" >&2; echo "lane-check: step 0 policy check failed (exit $rc)" >&2; return 2 ;;
  esac
}
if [ -n "$step0_selftest" ]; then
  round=$(step0_round_block "$step0_selftest") || round=""
  [ -n "$round" ] || { echo "lane-check: $step0_selftest/tools/run-round.sh has no step-0 block" >&2; exit 1; }
  step0_scan "$step0_selftest" "$round"; rc_r=$?; json_r=$step0_json; txt_r=$step0_txt
  step0_scan "$step0_selftest" "$STEP0_PY_EMBEDDED"; rc_e=$?
  ident=$([ "$round" = "$STEP0_PY_EMBEDDED" ] && echo yes || echo no)
  echo "step0 self-test on $step0_selftest: round exit=$rc_r embedded exit=$rc_e text_identical=$ident"
  printf '%s
' "$txt_r"
  if [ $rc_r -eq $rc_e ] && [ "$json_r" = "$step0_json" ] && [ "$txt_r" = "$step0_txt" ]; then
    echo "STEP0_SELF_TEST OK (exit $rc_r)"; exit 0
  fi
  echo "STEP0_SELF_TEST FAIL"; exit 1
fi
if [ $step0_only -eq 1 ]; then lane_step0 "$PWD"; exit $?; fi
[ -n "$owner" ] || { echo "lane-check: --owner <lane-id-lowercase> is required" >&2; exit 2; }
lane_env() {   # build disk, owner, studio-env (lane target, TMP, disk lock, cargo wrapper)
  case "$disk" in ssd) export STUDIO_BUILD_DISK=ssd ;; d|D) export STUDIO_BUILD_DISK=d ;; *) echo "lane-check: --disk ssd|d" >&2; return 2 ;; esac
  export STUDIO_OWNER=$owner
  unset STUDIO_DISK_BUILD_LOCK_DIR _STUDIO_ENV_LOCK_DERIVED
  TOOLS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
  source "$TOOLS/../builders/studio-env.sh" || return 2
}
# lane_metadata DIR OUT: `cargo metadata --no-deps --offline` of the workspace at DIR into OUT. Fails closed (steward
# 2026-10-31: lanes logged dependents=[] while the same crates had 21 dependents in another run, because a failed
# metadata call was hidden by 2>/dev/null): a non-zero exit, unreadable JSON or a JSON without workspace members prints
# "lane-check: cargo metadata failed" plus cargo's stderr and returns non-zero; never an empty list.
lane_metadata() {
  local err="$2.stderr.txt" rc
  (cd "$1" && command cargo metadata --no-deps --offline --format-version 1) >"$2" 2>"$err"; rc=$?
  if [ $rc -eq 0 ]; then
    "$step0_py" -X utf8 -I -c 'import json, sys
m = json.load(open(sys.argv[1], encoding="utf-8"))
sys.exit(0 if m.get("packages") and m.get("workspace_members") else "metadata JSON lists no workspace members")' "$2" 2>>"$err" || rc=3
  fi
  if [ $rc -ne 0 ]; then
    echo "lane-check: cargo metadata failed in $1 (exit $rc$([ $rc -eq 3 ] && echo ': unreadable JSON or no workspace members'))" >&2
    cat "$err" >&2
  fi
  rm -f "$err"
  return $rc
}
if [ -n "$per_sha" ]; then
  # --per-sha (header): every commit's own tree, step 0 + cargo check --tests of its changed crates, their direct
  # dependents and their round feature units, in the lane target under one disk-lock hold; stop at the first failure.
  [ $# -eq 0 ] || { echo "lane-check: --per-sha takes no test command" >&2; exit 2; }
  case "$per_sha" in *..*) ps_base=${per_sha%%..*} ps_tip=${per_sha#*..} ;; *) ps_base=$per_sha ps_tip=HEAD ;; esac
  base_sha=$(git rev-parse --verify --quiet "${ps_base}^{commit}") && tip_sha=$(git rev-parse --verify --quiet "${ps_tip:-HEAD}^{commit}") \
    || { echo "lane-check: --per-sha: cannot resolve $per_sha to commits" >&2; exit 2; }
  mapfile -t per_sha_list < <(git rev-list --reverse "$base_sha..$tip_sha")
  ws_dir=$(git rev-parse --show-prefix) || { echo "lane-check: --per-sha: not in a git work tree" >&2; exit 2; }
  ws_dir=${ws_dir%/} git_top=$(git rev-parse --show-toplevel)
  echo "PER_SHA lane=$owner range=${base_sha:0:8}..${tip_sha:0:8} shas=${#per_sha_list[@]} workspace=${ws_dir:-.} started=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  [ ${#per_sha_list[@]} -gt 0 ] || { echo "PER_SHA_RESULT OK shas=0 (empty range)"; exit 0; }
  lane_env || exit $?
  per_root="$TMP/per-sha-${tip_sha:0:8}" per_logs="$(dirname "$TMP")/per-sha-${tip_sha:0:8}"
  per_tree="$per_root/tree"
  mkdir -p "$per_root" "$per_logs" || exit 2
  echo "lane-check: owner=$owner disk=$STUDIO_BUILD_DISK target=$CARGO_TARGET_DIR per-sha logs=$per_logs" >&2
  # per_sha_units TREE CHANGED_LIST METADATA_JSON ISOLATE_TOML -> rows "closure<TAB>changed<TAB>dependents",
  # "default<TAB>crates (dependencies first)", "features|absent<TAB>crate<TAB>f1,f2"; writes the isolation profile file.
  per_sha_units() {
"$step0_py" -X utf8 -I - "$@" <<'PY'
import json, os, pathlib, sys, tomllib

sys.stdout.reconfigure(newline="\n")
tree, changed_file, meta_file, iso_file = (pathlib.Path(a) for a in sys.argv[1:5])
meta = json.loads(meta_file.read_text(encoding="utf-8"))
ids = set(meta["workspace_members"])
members = {p["name"]: p for p in meta["packages"] if p["id"] in ids}
ws_root = meta["workspace_root"]   # cargo's own spelling of the tree (8.3 short names and slashes as cargo sees them)
by_dir = {pathlib.Path(os.path.relpath(pathlib.Path(p["manifest_path"]).parent, ws_root)).as_posix(): n
          for n, p in members.items()}
ROOT_ALL = {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".config/round-features.toml"}
changed = set()
for line in changed_file.read_text(encoding="utf-8").splitlines():
    f = line.strip().replace("\\", "/")
    if not f:
        continue
    if f in ROOT_ALL or f.startswith(".cargo/"):
        changed |= set(members)
        continue
    parts = f.split("/")
    for k in range(len(parts) - 1, 0, -1):   # the deepest member dir holding the file
        d = "/".join(parts[:k])
        if d in by_dir:
            changed.add(by_dir[d])
            break
deps = {n: {d["name"] for d in p["dependencies"] if d.get("path") and d["name"] in members} for n, p in members.items()}
dependents = {n for n in members if n not in changed and deps[n] & changed}
closure = changed | dependents
order, done, pending = [], set(), sorted(closure)
while pending:   # dependencies first (a failure is named at the lowest crate); a cycle takes the first name
    ready = [n for n in pending if not (deps[n] & closure) - done] or pending[:1]
    order += ready
    done |= set(ready)
    pending = [n for n in pending if n not in done]
cfg = tree / ".config" / "round-features.toml"
doc = tomllib.loads(cfg.read_text(encoding="utf-8")) if cfg.exists() else {}
sets = {}
for table in ("crates", "ignored"):
    for crate, ss in doc.get(table, {}).items():
        for s in (ss if isinstance(ss, list) else []):
            if isinstance(s, list) and s:
                key = tuple(sorted(set(s)))
                if key not in sets.setdefault(crate, []):
                    sets[crate].append(key)
print("closure\t" + " ".join(sorted(changed)) + "\t" + " ".join(sorted(dependents)))
if order:
    print("default\t" + " ".join(order))
for n in order:
    for s in sets.get(n, []):
        missing = [f for f in s if f not in members[n].get("features", {})]
        print(("absent" if missing else "features") + f"\t{n}\t{','.join(s)}")
iso_file.write_text("".join(f"[profile.{prof}.package.{n}]\ncodegen-units = 255\n"
                            for n in sorted(members) for prof in ("dev", "test")), encoding="utf-8", newline="\n")
PY
  }
  per_sha_body() {
    local sha s8 rows kind a b unit ulog rc cc c args=() pk_=()
    for sha in "${per_sha_list[@]}"; do
      s8=${sha:0:8}
      rm -rf "$per_tree" && mkdir -p "$per_tree" || { echo "lane-check: per-sha: cannot reset $per_tree" >&2; return 2; }
      # from the top level: in a subdirectory, git archive would limit itself to that directory's path again
      git -C "$git_top" archive --format=tar "$sha${ws_dir:+:$ws_dir}" | (cd "$per_tree" && tar -xmf -) && [ -f "$per_tree/Cargo.toml" ] \
        || { echo "PER_SHA_RESULT FAIL sha=$sha unit=archive (git archive | tar; no Cargo.toml)"; return 2; }
      git diff --relative --name-only "$sha^" "$sha" >"$per_root/changed.txt" \
        || { echo "PER_SHA_RESULT FAIL sha=$sha unit=diff"; return 2; }
      lane_metadata "$per_tree" "$per_root/meta.json" || { echo "PER_SHA_RESULT FAIL sha=$sha unit=metadata"; return 2; }
      rows=$(per_sha_units "$per_tree" "$per_root/changed.txt" "$per_root/meta.json" "$per_root/isolate.toml") \
        || { echo "PER_SHA_RESULT FAIL sha=$sha unit=plan"; return 2; }
      lane_step0 "$per_tree"; rc=$?
      if [ $rc -ne 0 ]; then echo "PER_SHA $s8 step0 FAIL exit=$rc"; echo "PER_SHA_RESULT FAIL sha=$sha unit=step0 exit=$rc"; return $rc; fi
      while IFS=$'\t' read -r kind a b; do
        case $kind in
          closure) echo "PER_SHA $s8 changed=[$a] dependents=[$b]"; continue ;;
          absent) echo "PER_SHA $s8 features:$a+${b//,/+} absent at this commit (the round records it absent, exit 74)"; continue ;;
          default) unit=default; read -ra pk_ <<<"$a"; args=(); for c in "${pk_[@]}"; do args+=(-p "$c"); done ;;
          features) unit="features:$a+${b//,/+}"; args=(-p "$a" --features "$b") ;;
          *) continue ;;
        esac
        ulog="$per_logs/$s8-${unit//[:+]/_}.log"
        printf 'SHA=%s unit=%s started=%s\n' "$sha" "$unit" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >"$ulog"
        (cd "$per_tree" && cargo check --offline --tests --config "$per_root/isolate.toml" "${args[@]}") >>"$ulog" 2>&1 </dev/null
        rc=$?
        if [ $rc -ne 0 ]; then
          cc=$(grep -o 'could not compile `[^`]*` ([^)]*)' "$ulog" | sort -u | tr '\n' ';')
          echo "PER_SHA $s8 $unit FAIL exit=$rc ${cc}log=$ulog"
          echo "PER_SHA_RESULT FAIL sha=$sha unit=$unit exit=$rc"
          return $rc
        fi
        echo "PER_SHA $s8 $unit ok"
      done <<<"$rows"
    done
    echo "PER_SHA_RESULT OK shas=${#per_sha_list[@]} range=${base_sha:0:8}..${tip_sha:0:8}"
  }
  studio_disk_lock_run "lane-check $owner: per-sha cargo check ${base_sha:0:8}..${tip_sha:0:8}" per_sha_body
  rc=$?
  rm -rf "$per_root"
  exit $rc
fi
[ "${1:-}" = cargo ] && [ "${2:-}" = nextest ] && [ "${3:-}" = run ] \
  || { echo "lane-check: after -- give the literal test command: cargo nextest run -p <crate> ..." >&2; exit 2; }
shift 3
run_args=("$@")
crates=() feats=() a="" i=0
while [ $i -lt ${#run_args[@]} ]; do
  a=${run_args[$i]}
  case "$a" in
    -p|--package) i=$((i + 1)); crates+=("${run_args[$i]:-}") ;;
    --package=*) crates+=("${a#--package=}") ;;
    -F|--features) i=$((i + 1)); feats+=(--features "${run_args[$i]:-}") ;;
    --features=*) feats+=("$a") ;;
    --all-features) feats+=("$a") ;;
    -E|--filterset|--filter-expr) [ -z "$tests" ] || { echo "lane-check: give --tests or -E, not both" >&2; exit 2; } ;;
  esac
  i=$((i + 1))
done
[ ${#crates[@]} -gt 0 ] || { echo "lane-check: the test command needs at least one -p <crate>" >&2; exit 2; }
if [ -n "$tests" ]; then
  expr=""
  IFS=, read -ra names <<<"$tests"
  for n in "${names[@]}"; do [ -n "$n" ] && expr+="${expr:+ | }test(${n})"; done
  run_args+=(-E "$expr")
fi
# First output line names the built tree so a log can cite it (F-R43-5): HEAD sha, owner, dirty flag, UTC start.
echo "HEAD=$(git rev-parse --short=8 HEAD 2>/dev/null || echo unknown) lane=$owner$( [ -n "$(git status --porcelain --untracked-files=no 2>/dev/null)" ] && echo ' dirty') started=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
lane_step0 "$PWD" || exit $?
lane_env || exit $?
dependents=()
if [ "${STUDIO_LANE_CHECK_NO_DEPENDENTS:-0}" = 1 ]; then
  dep_list="SKIPPED(STUDIO_LANE_CHECK_NO_DEPENDENTS=1)"
else
  # fail closed (lane_metadata): a metadata or parse failure stops the check; it never becomes dependents=[]
  meta_json=$(mktemp) || { echo "lane-check: cargo metadata failed (mktemp)" >&2; exit 2; }
  lane_metadata "$PWD" "$meta_json" || { rm -f "$meta_json"; exit 2; }
  dep_out=$("$step0_py" -X utf8 -I -c '
import json, sys
meta = json.load(open(sys.argv[1], encoding="utf-8"))
ids = set(meta["workspace_members"])
tested = set(sys.argv[2:])
for p in meta["packages"]:
    if p["id"] in ids and p["name"] not in tested and any(d["name"] in tested and d.get("path") for d in p["dependencies"]):
        print(p["name"])' "$meta_json" "${crates[@]}"); dep_rc=$?
  rm -f "$meta_json"
  [ $dep_rc -eq 0 ] || { echo "lane-check: cargo metadata failed (dependents not computed, exit $dep_rc)" >&2; exit 2; }
  mapfile -t dependents < <(printf '%s\n' "$dep_out" | tr -d '\r' | sed '/^$/d' | sort -u)
  dep_list="[${dependents[*]}]"
fi
pk=(); for c in "${crates[@]}"; do pk+=(-p "$c"); done
dk=(); for c in "${dependents[@]}"; do dk+=(-p "$c"); done
echo "lane-check: owner=$owner disk=$STUDIO_BUILD_DISK target=$CARGO_TARGET_DIR crates=[${crates[*]}] dependents=$dep_list" >&2
lane_check_body() {
  cargo clippy "${pk[@]}" ${feats[@]+"${feats[@]}"} --tests || return $?
  if [ ${#dk[@]} -gt 0 ]; then cargo clippy "${dk[@]}" --tests || return $?; fi
  cargo nextest run "${run_args[@]}"
}
studio_disk_lock_run "lane-check $owner: clippy [${crates[*]}] + dependents [${dependents[*]}], nextest run ${run_args[*]}" lane_check_body
