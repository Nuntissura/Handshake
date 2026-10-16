# Studio module team build environment (WP-KERNEL-STUDIO). Source AFTER the leading `cd` so the gameplan hook
# resolves the Studio pointer:  cd "<dir>" && source "<this file>" && cargo ...
# Operator 2026-10-11: no cargo builds/tests/checks on C:; all build output, TMP and TEMP under Handshake_Artifacts/<WP>/<owner>.
# Operator grant 2026-10-14 (CX-984-014, START_LOG): the shared builder edit-tree target (owner studio-team-dev only)
# lives on the C: SSD at C:/HandshakeBuild/WP-KERNEL-STUDIO/studio-team-dev/target (cap 150 GB, cleaned at WP close)
# with its own per-disk lock C:/HandshakeBuild/.disk-build-lock; every other owner (exports, sync-export, canary,
# probes, red-proof trees) and all TMP/TEMP stay under Handshake_Artifacts on D:. Validator rounds are unchanged.
# SSD opt-in for lane builders (SMB-STEWARD-LOCK-THROUGHPUT, 2026-10-16): STUDIO_BUILD_DISK=ssd moves a lane
# owner's target INSIDE the one granted target, at <granted target>/lanes/<owner> (own tree binding, fingerprints and
# nextest store: cargo hashes member paths relative to the workspace root, so lane trees never share units), under
# the C: lock; lock records keep the lane owner. D: round gaps are then not needed by that lane (rounds stay on D:).
# Guard: an SSD build is refused (status 77, before queueing) while the whole C:/HandshakeBuild/WP-KERNEL-STUDIO
# tree is over STUDIO_SSD_CAP_GB (default 140 of the 150 GB grant; see studio_ssd_cap_check). Unset or
# STUDIO_BUILD_DISK=d: unchanged D: target. STUDIO_SSD_ROOT is a self-test knob only (default C:/HandshakeBuild).
_studio_art="D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO"
_studio_ssd="${STUDIO_SSD_ROOT:-C:/HandshakeBuild}"
_studio_owner="${STUDIO_OWNER:-studio-team-dev}"
_STUDIO_SSD_LANE=""
case "${STUDIO_BUILD_DISK:-}" in
  ""|d|D|hdd) ;;
  ssd)
    case "/${_studio_owner}/" in
      /studio-team-dev/) ;;
      *//*|*/./*|*/../*|*\\*|*:*|*" "*)
        echo "studio-env: STUDIO_BUILD_DISK=ssd needs a plain lane owner (STUDIO_OWNER=<lane-id-lowercase>), got '${_studio_owner}'" >&2
        return 1 ;;
      *) _STUDIO_SSD_LANE="${_studio_owner}" ;;
    esac ;;
  *) echo "studio-env: STUDIO_BUILD_DISK='${STUDIO_BUILD_DISK}' unknown (ssd = granted C: target, unset/d = D:)" >&2
     return 1 ;;
esac
export _STUDIO_SSD_LANE
if [ "${_studio_owner}" = "studio-team-dev" ]; then
  export CARGO_TARGET_DIR="${_studio_ssd}/WP-KERNEL-STUDIO/${_studio_owner}/target"
  _studio_lock_default="${_studio_ssd}/.disk-build-lock"
elif [ -n "${_STUDIO_SSD_LANE}" ]; then
  export CARGO_TARGET_DIR="${_studio_ssd}/WP-KERNEL-STUDIO/studio-team-dev/target/lanes/${_STUDIO_SSD_LANE}"
  _studio_lock_default="${_studio_ssd}/.disk-build-lock"
  export _STUDIO_SSD_TREE="${_studio_ssd}/WP-KERNEL-STUDIO"
else
  export CARGO_TARGET_DIR="${_studio_art}/${_studio_owner}/target"
  _studio_lock_default="${_studio_art}/.disk-build-lock"
fi
export TMP="${_studio_art}/${_studio_owner}/tmp"
export TEMP="${TMP}"
export TMPDIR="${TMP}"
export CARGO_PROFILE_DEV_DEBUG=line-tables-only
export CARGO_PROFILE_TEST_DEBUG=line-tables-only
mkdir -p "${CARGO_TARGET_DIR}" "${TMP}"

# Tree binding (diagnostics/stale-artifacts.md M2): Cargo hashes workspace-member paths relative to the workspace
# root, so a second tree built into the same target shares unit hashes, fingerprints and artifacts, and mtime
# freshness then reuses artifacts compiled from the other tree. The marker ${CARGO_TARGET_DIR}/.studio-workspace-root
# binds the target to ONE workspace root (as `cargo locate-project --workspace` prints it); sourcing from another
# root (or from outside a workspace) fails before any cargo runs. Export, canary, probe and red-proof trees set
# STUDIO_OWNER to their own owner dir. One locate-project call per sourcing (measured ~0.3-0.5 s).
_studio_ws="$(command cargo locate-project --workspace --message-format plain 2>/dev/null)"
_studio_bind="${CARGO_TARGET_DIR}/.studio-workspace-root"
if [ -z "${_studio_ws}" ]; then
  echo "studio-env: no cargo workspace at ${PWD}; cd to the workspace root before sourcing" >&2
  return 1
fi
if [ -f "${_studio_bind}" ] && [ "$(cat "${_studio_bind}")" != "${_studio_ws}" ]; then
  echo "studio-env: ${CARGO_TARGET_DIR} is bound to $(cat "${_studio_bind}"), not ${_studio_ws}; set STUDIO_OWNER to a per-tree owner (export, canary, probe and red-proof trees never share the edit tree's target)" >&2
  return 1
fi
[ -f "${_studio_bind}" ] || printf '%s' "${_studio_ws}" >"${_studio_bind}"

# Disk-wide build lock (CX-984-002: one build per physical disk; D: is the only build disk). After sourcing, `cargo`
# is a shell function (exported to child bash scripts) that runs build|check|clippy|run|bench|doc|rustc|rustdoc|fix|
# install|nextest|miri|tarpaulin|llvm-cov and `test --doc` (and the one-letter aliases) holding the lock
# ${_studio_art}/.disk-build-lock as a queued builder (protocol @2, FIFO queue, round priority, stale-lock rule,
# max hold 900 s, waits: ../tools/disk-build-lock.sh, byte-identical to the product tools/disk-build-lock.sh used by
# tools/run-round.sh). `cargo test` without --doc is refused (libtest has no per-test timeout, so one hung test would
# hold the disk lock for every builder; CX-EXEC-005): run tests with `cargo nextest run ...` (per-test timeout from
# the workspace .config/nextest.toml). Other subcommands (metadata, tree, fetch, --version, ...) run unlocked. Waits
# print to stderr (first wait, then every 60 s); STUDIO_DISK_BUILD_LOCK_TIMEOUT_S (default 7200) bounds a wait
# (status 75). A build may wait for a validator round unit: run builds in the background.
# `studio_disk_lock_run LABEL CMD...` locks any other build (e.g. a direct rustc).
source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../tools" && pwd)/disk-build-lock.sh"
# An explicit caller override wins; a value this file derived on an earlier sourcing (possibly for another owner and
# disk) is recomputed, so re-sourcing with a different STUDIO_OWNER never keeps the other disk's lock.
if [ -z "${STUDIO_DISK_BUILD_LOCK_DIR:-}" ] || [ "${STUDIO_DISK_BUILD_LOCK_DIR}" = "${_STUDIO_ENV_LOCK_DERIVED:-}" ]; then
  export STUDIO_DISK_BUILD_LOCK_DIR="${_studio_lock_default}"
  export _STUDIO_ENV_LOCK_DERIVED="${_studio_lock_default}"
fi
# SSD cap guard (lane opt-in only), checked BEFORE queueing for the C: lock. Record
# ${_STUDIO_SSD_TREE}/.studio-ssd-size = "<KiB> <measure start epoch>". Measured 2026-10-16 on a 9.4 GB lane target:
# MSYS du -sk 202 s, robocopy /L listing 3.4 s (sums apparent sizes and counts cargo's uplift hardlinks twice, so it
# errs high = fail-safe); du only if robocopy is missing or unparseable. The measure never runs inside the lock or the
# caller: studio_ssd_measure runs detached, one measurer at a time (mutex dir .studio-ssd-size.measuring, taken over
# after 180 min). Decision uses the last record of any age: over STUDIO_SSD_CAP_GB (default 140) -> status 77; a record
# older than STUDIO_SSD_SIZE_MAX_AGE_S (default 3600) or a missing record starts a re-measure; missing = build
# allowed (size unknown until the first measure lands). Re-read (no measure) once the lock is held, so a record that
# went over cap during the queue wait still refuses. After a cleanup, delete the record to re-admit at once.
studio_ssd_kib() {   # prints the KiB under ${_STUDIO_SSD_TREE} (robocopy listing, du fallback)
  local w b
  w=$(cygpath -w "${_STUDIO_SSD_TREE}" 2>/dev/null) || w=${_STUDIO_SSD_TREE}
  b=$(MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' robocopy "$w" NUL /L /S /NJH /BYTES /NC /NDL /NFL /XJ /R:0 /W:0 2>/dev/null \
    | sed -n 's/^ *Bytes *: *\([0-9][0-9]*\).*/\1/p' | head -n 1)
  case "$b" in ''|*[!0-9]*) du -sk "${_STUDIO_SSD_TREE}" 2>/dev/null | { read -r k _; echo "${k:-}"; } ;; *) echo $(( (b + 1023) / 1024 )) ;; esac
}
studio_ssd_measure() {
  local m="${_STUDIO_SSD_TREE}/.studio-ssd-size.measuring" rec="${_STUDIO_SSD_TREE}/.studio-ssd-size" kib t
  if ! mkdir "$m" 2>/dev/null; then
    [ -n "$(find "$m" -maxdepth 0 -mmin +180 2>/dev/null)" ] || return 0
    rmdir "$m" 2>/dev/null; mkdir "$m" 2>/dev/null || return 0
  fi
  dbl_now; t=$DBL_NOW
  kib=$(studio_ssd_kib)
  case "$kib" in ''|*[!0-9]*) ;; *) printf '%s %s\n' "$kib" "$t" >"$rec.$$" && mv -f "$rec.$$" "$rec" || rm -f "$rec.$$" ;; esac
  rmdir "$m" 2>/dev/null
  return 0
}
studio_ssd_cap_check() {   # [held]: re-check after a queue wait (record read only, no re-measure, no unknown note)
  local rec="${_STUDIO_SSD_TREE}/.studio-ssd-size" kib="" at="" cap_kib maxage=${STUDIO_SSD_SIZE_MAX_AGE_S:-3600}
  cap_kib=$(( ${STUDIO_SSD_CAP_GB:-140} * 1024 * 1024 ))
  { read -r kib at || :; } 2>/dev/null <"$rec" || :
  case "$kib$at" in ''|*[!0-9]*) kib="" at=0 ;; esac
  dbl_now
  if [ "${1:-}" != held ] && { [ -z "$kib" ] || [ $((DBL_NOW - at)) -ge "$maxage" ]; }; then
    (studio_ssd_measure </dev/null >/dev/null 2>&1 &)
  fi
  if [ -n "$kib" ] && [ "$kib" -gt "$cap_kib" ]; then
    echo "studio-env: ${_STUDIO_SSD_TREE} held $((kib / 1048576)) GiB at $(TZ=UTC0 date -d "@$at" +%FT%TZ 2>/dev/null) > cap ${STUDIO_SSD_CAP_GB:-140} GiB (Operator grant CX-984-014: 150 GB); SSD build refused (77). Unset STUDIO_BUILD_DISK to build on D:; cleanup is a steward/Operator action." >&2
    return 77
  fi
  [ -n "$kib" ] || [ "${1:-}" = held ] || echo "studio-env: SSD tree size unknown (no record yet); measuring in the background, build allowed" >&2
  return 0
}
studio_disk_lock_run() {
  local label=$1 child=""
  shift
  (
    trap 'dbl_release' EXIT
    trap '[ -n "$child" ] && kill "$child" 2>/dev/null; exit 130' INT TERM HUP
    if [ -n "${_STUDIO_SSD_LANE:-}" ] && [ -z "${STUDIO_DISK_BUILD_LOCK_TOKEN:-}" ]; then studio_ssd_cap_check || exit $?; fi
    dbl_acquire "$STUDIO_DISK_BUILD_LOCK_DIR" builder "${STUDIO_OWNER:-studio-team-dev}" "$label (cwd $PWD)" studio-env || exit $?
    if [ "$DBL_NESTED" -eq 1 ]; then "$@"; exit $?; fi
    if [ -n "${_STUDIO_SSD_LANE:-}" ]; then studio_ssd_cap_check held || exit $?; fi
    "$@" &
    child=$!
    dbl_note_child "$child"
    wait "$child"
  )
}
# Test-runner store pin (steward-4, 2026-10-11): the runner resolves its store (junit, run data) against the
# WORKSPACE ROOT (embedded default config `[store] dir = "target/nextest"`), not CARGO_TARGET_DIR, so every edit-tree
# `cargo nextest run` wrote <workspace>/target/nextest (observed 07:49Z and 10:12Z; CX-984: no repo-local target).
# The wrapper passes a tool config pinning the store under CARGO_TARGET_DIR (same mechanism as tools/run-round.sh)
# to `nextest run|r|list|l` unless the caller passes its own --tool-config-file. The config's content is a function
# of its own directory only, so it is written once per target dir and never goes stale.
_studio_nx_cfg="${CARGO_TARGET_DIR}/.studio-nextest-store.toml"
[ -f "${_studio_nx_cfg}" ] || { printf '[store]\ndir = "%s/nextest"\n' "${CARGO_TARGET_DIR}" >"${_studio_nx_cfg}.$$" \
  && mv -f "${_studio_nx_cfg}.$$" "${_studio_nx_cfg}"; } || rm -f "${_studio_nx_cfg}.$$"
studio_nextest_args() {   # "$@" of a cargo call -> STUDIO_NX_ARGS (store pin inserted after the runner subcommand)
  local a pos=0 done=0 cfg="${CARGO_TARGET_DIR}/.studio-nextest-store.toml"
  STUDIO_NX_ARGS=()
  case " $* " in *" --tool-config-file"*) done=1 ;; esac
  [ -f "$cfg" ] || done=1
  for a in "$@"; do
    STUDIO_NX_ARGS+=("$a")
    [ $done -eq 0 ] || continue
    if [ $pos -eq 0 ]; then [ "$a" = nextest ] && pos=1; continue; fi
    case "$a" in
      -*) ;;
      run|r|list|l) STUDIO_NX_ARGS+=(--tool-config-file "studio-env:$cfg"); done=1 ;;
      *) done=1 ;;
    esac
  done
}
cargo() {
  local a sub="" skip=0
  for a in "$@"; do
    if [ $skip -eq 1 ]; then skip=0; continue; fi
    case "$a" in
      -Z|--config|-C|--color|--explain|--manifest-path|--target-dir|--message-format|-p|--package|-j|--jobs|--features|-F|--profile|--target) skip=1 ;;
      +*|-*) ;;
      *) sub=$a; break ;;
    esac
  done
  case "$sub" in
    test|t)
      case " $* " in
        *" --doc "*) studio_disk_lock_run "cargo $*" command cargo "$@" ;;
        *) echo "studio-env: 'cargo $*' refused: cargo test has no per-test timeout and holds the disk build lock (CX-EXEC-005). Run: cargo nextest run <same -p/--features/--test args> (per-test timeout from .config/nextest.toml; -- --nocapture becomes --no-capture); doctests: cargo test --doc ..." >&2
           return 2 ;;
      esac ;;
    nextest)
      studio_nextest_args "$@"
      studio_disk_lock_run "cargo $*" command cargo "${STUDIO_NX_ARGS[@]}" ;;
    build|b|check|c|clippy|run|r|bench|doc|d|rustc|rustdoc|fix|install|miri|tarpaulin|llvm-cov)
      studio_disk_lock_run "cargo $*" command cargo "$@" ;;
    *) command cargo "$@" ;;
  esac
}
export -f cargo studio_disk_lock_run studio_nextest_args studio_ssd_cap_check studio_ssd_measure studio_ssd_kib  # the dbl_* functions export themselves (tools/disk-build-lock.sh)
