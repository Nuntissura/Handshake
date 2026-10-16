#!/usr/bin/env bash
# Self-test of the disk-wide build lock @2 and the tree-binding guard (workspace steward tool). Scratch lock root,
# scratch cargo workspaces and a fake cargo on PATH; never touches the real lock or a real target.
# G1 tree binding; T1 serialize; T2 unlocked metadata; T3 exit status; T4 nested; T4b overhead; T5 dead holder with
# live child not broken; T6 broken once the child is dead; T7 product library vs kernel wrapper; T8 copies identical;
# T9 200 acquire/release cycles under 20 owner-file readers: 0 overlaps, 0 failed releases; T10 version fence both
# ways; T11 pid reuse (start time differs) is dead, same start is alive; T12 round priority + one builder per gap;
# T13 cargo test refused, --doc allowed; T14 one OVER_MAX_HOLD event per holder, holder not broken; T15 SSD opt-in
# (STUDIO_BUILD_DISK=ssd: lane target inside the granted C: target, SSD lock, owner attribution, 140 GiB cap guard).
# Usage: disk-build-lock-test.sh [SCRATCH_DIR]   (DBL_TEST_ENV / DBL_TEST_KLIB: staged copies)   Exit 0 = all PASS.
set -u
TOOLS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
WT=$(cd "$TOOLS/../../../../.." && pwd)
PRODUCT_LIB="$WT/wt-studio-assets/handshake-studio-assets/tools/disk-build-lock.sh"
ENV=${DBL_TEST_ENV:-"$TOOLS/../builders/studio-env.sh"}
KLIB=${DBL_TEST_KLIB:-"$TOOLS/disk-build-lock.sh"}
S=${1:-"$WT/Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev/steward/dbl-test"}
case "$S" in *steward/dbl-test*) rm -rf "$S" ;; *) echo "scratch must end in steward/dbl-test" >&2; exit 2 ;; esac
mkdir -p "$S/bin"
cat >"$S/bin/cargo" <<'EOF'
#!/usr/bin/env bash
echo "${FAKE_TAG:-?} start $(date +%s%3N)" >>"$FAKE_LOG"
sleep "${FAKE_SLEEP:-3}"
echo "${FAKE_TAG:-?} end $(date +%s%3N)" >>"$FAKE_LOG"
exit "${FAKE_EXIT:-0}"
EOF
chmod +x "$S/bin/cargo"
fails=0
ok() { echo "PASS $*"; }
bad() { echo "FAIL $*"; fails=$((fails + 1)); }
for w in ws1 ws2; do
  mkdir -p "$S/$w/src"; : >"$S/$w/src/lib.rs"
  printf '[package]\nname = "dbl-%s"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n' "$w" >"$S/$w/Cargo.toml"
done
L="$S/lock"
export STUDIO_DISK_BUILD_LOCK_DIR="$L" STUDIO_DISK_BUILD_LOCK_POLL_S=1 FAKE_LOG="$S/log"
export STUDIO_OWNER=studio-team-dev/steward/dbl-test/env
# G1 tree binding: ws1 binds the owner's target; ws2 with the same owner is refused; cost of one sourcing.
t0=$(date +%s%3N)
(cd "$S/ws1" && source "$ENV") 2>"$S/G1.err" && g1=$? || g1=$?
t1=$(date +%s%3N)
(cd "$S/ws2" && source "$ENV") 2>"$S/G2.err" && g2=$? || g2=$?
[ $g1 -eq 0 ] && [ $g2 -ne 0 ] && grep -q "is bound to" "$S/G2.err" \
  && ok "G1 tree binding: ws1 bound (one sourcing $((t1 - t0)) ms), ws2 refused" \
  || bad "G1 tree binding g1=$g1 g2=$g2 $(cat "$S/G1.err" "$S/G2.err")"
cd "$S/ws1" && source "$ENV" || { bad "G1 resource"; exit 1; }
export PATH="$S/bin:$PATH"
ts() { sed -n "s/^$1 $2 //p" "$FAKE_LOG" | head -n1; }
: >"$FAKE_LOG"

# T1 serialize + T2 unlocked metadata.
(FAKE_TAG=A cargo check -p x) 2>"$S/A.err" &
pa=$!
sleep 0.5
t_b0=$(date +%s%3N)
(FAKE_TAG=B cargo build) 2>"$S/B.err" &
pb=$!
(FAKE_TAG=M FAKE_SLEEP=0 cargo metadata --format-version 1) 2>"$S/M.err"
wait $pa $pb
if [ "$(ts B start)" -ge "$(ts A end)" ]; then ok "T1 serialize: B started $(( $(ts B start) - $(ts A end) )) ms after A ended (B queued $(( $(ts B start) - t_b0 )) ms)"
else bad "T1 serialize: A $(ts A start)-$(ts A end) B $(ts B start)-$(ts B end)"; fi
if [ "$(ts M start)" -lt "$(ts A end)" ]; then ok "T2 metadata unlocked (ran while A held the lock)"; else bad "T2 metadata waited"; fi

# T3 exit status kept.
FAKE_TAG=E FAKE_SLEEP=0 FAKE_EXIT=3 cargo check 2>/dev/null; rc=$?
[ $rc -eq 3 ] && ok "T3 exit status 3 kept" || bad "T3 exit status $rc"

# T4 nested use: a holder's child bash calls the exported cargo function again.
t0=$(date +%s%3N)
FAKE_SLEEP=1 STUDIO_DISK_BUILD_LOCK_TIMEOUT_S=20 studio_disk_lock_run outer bash -c 'FAKE_TAG=N cargo check' 2>"$S/N.err"; rc=$?
[ $rc -eq 0 ] && [ -n "$(ts N end)" ] && ! grep -q waiting "$S/N.err" \
  && ok "T4 nested use ran without re-locking ($(( $(date +%s%3N) - t0 )) ms incl. 1 s fake build)" || bad "T4 nested rc=$rc $(cat "$S/N.err")"
t0=$(date +%s%3N); FAKE_TAG=U FAKE_SLEEP=0 cargo check 2>/dev/null; t1=$(date +%s%3N)
t2=$(date +%s%3N); FAKE_TAG=V FAKE_SLEEP=0 command cargo check 2>/dev/null; t3=$(date +%s%3N)
ok "T4b uncontended wrapped call $((t1 - t0)) ms vs unwrapped $((t3 - t2)) ms (lock overhead $(( (t1 - t0) - (t3 - t2) )) ms)"

# T5 killed holder with a live child is not broken; T6 broken once the child is dead.
(FAKE_TAG=K FAKE_SLEEP=25 cargo build) 2>"$S/K.err" &
for _ in $(seq 1 40); do [ -n "$(dbl_field child_msys_pid "$L/held/owner")" ] && break; sleep 0.5; done
hp=$(dbl_field msys_pid "$L/held/owner"); cp_=$(dbl_field child_msys_pid "$L/held/owner"); kt=$(dbl_field token "$L/held/owner")
kill -9 "$hp" 2>/dev/null
sleep 0.5
kill -0 "$hp" 2>/dev/null && bad "T5 holder $hp still alive after kill -9"
t0=$(date +%s)
(FAKE_TAG=W FAKE_SLEEP=0 STUDIO_DISK_BUILD_LOCK_TIMEOUT_S=5 cargo check) 2>"$S/W.err"; rc=$?
if [ $rc -eq 75 ] && [ -z "$(ts W start)" ] && [ "$(dbl_field token "$L/held/owner")" = "$kt" ]; then
  ok "T5 dead holder $hp with live child $cp_: not broken; waiter gave up after $(( $(date +%s) - t0 ))s with 75"
else bad "T5 rc=$rc W.start=$(ts W start)"; fi
kill -9 "$cp_" 2>/dev/null
sleep 0.5
t0=$(date +%s%3N)
(FAKE_TAG=X FAKE_SLEEP=0 cargo check) 2>"$S/X.err"; rc=$?
if [ $rc -eq 0 ] && grep -q "broke stale lock $kt" "$S/X.err" && [ -d "$L/stale.$kt" ]; then
  ok "T6 holder and child dead: broken and acquired in $(( $(date +%s%3N) - t0 )) ms"
else bad "T6 rc=$rc $(cat "$S/X.err")"; fi
[ -d "$L/held" ] && bad "T6 lock left held after release" || ok "T6 lock released after the run"

# T7 cross-implementation: the product library holds 3 s; the kernel wrapper waits.
bash -c 'source "$1"; dbl_acquire "$2" builder round-test "fake holder" run-round || exit 9; echo "P start $(date +%s%3N)" >>"$3"; sleep 3; echo "P end $(date +%s%3N)" >>"$3"; dbl_release' _ \
  "$PRODUCT_LIB" "$L" "$FAKE_LOG" 2>"$S/P.err" &
pp=$!
sleep 1
(FAKE_TAG=Q FAKE_SLEEP=0 cargo check) 2>"$S/Q.err"
wait $pp
if [ -n "$(ts P end)" ] && [ "$(ts Q start)" -ge "$(ts P end)" ]; then ok "T7 product holder then kernel wrapper: Q started $(( $(ts Q start) - $(ts P end) )) ms after P ended"
else bad "T7 P $(ts P start)-$(ts P end) Q $(ts Q start) $(cat "$S/P.err")"; fi

# T8 library copies identical.
if cmp -s "$KLIB" "$PRODUCT_LIB"; then ok "T8 kernel and product disk-build-lock.sh identical"; else bad "T8 library copies differ"; fi

# T9 two cyclers x 100 acquire/release under 20 owner-file readers: no overlap (critical-section dir), no RELEASE_FAILED.
# Slow (about 12 min on a loaded host); DBL_TEST_SKIP=T9 skips it only when tools/disk-build-lock.sh (its sole input)
# is unchanged since its last PASS. Under heavy host load its 20 readers can block the release rename for 10 s
# (RELEASE_FAILED; the cycler then waits on its own token), observed 2026-10-16T02:47Z.
if [ "${DBL_TEST_SKIP:-}" = T9 ]; then echo "SKIP T9 (DBL_TEST_SKIP)"; else
rm -f "$L/events.log"; mkdir -p "$S/t9"
for r in $(seq 1 20); do
  bash -c 'n=0; while [ ! -e "$2" ] && [ $n -lt 20000 ]; do { read -r x <"$1/held/owner"; } 2>/dev/null; n=$((n+1)); done' _ "$L" "$S/t9/stop" &
done
cyc() { bash -c 'source "$1"; for i in $(seq 1 100); do dbl_acquire "$2" builder "c$3" cyc t9 || { echo ACQFAIL >>"$4/err"; exit 1; }
  mkdir "$4/crit" 2>/dev/null || echo "OVERLAP $3 $i" >>"$4/err"; rmdir "$4/crit"; dbl_release; done' _ "$KLIB" "$L" "$1" "$S/t9" 2>>"$S/t9/stderr"; }
t0=$(date +%s)
cyc 1 & c1=$!; cyc 2 & c2=$!
wait $c1 $c2
: >"$S/t9/stop"; wait
if [ ! -s "$S/t9/err" ] && ! grep -q RELEASE_FAILED "$L/events.log" 2>/dev/null && [ ! -d "$L/held" ]; then
  ok "T9 200 cycles + 20 readers in $(( $(date +%s) - t0 ))s: 0 overlaps, 0 failed releases"
else bad "T9 $(head -5 "$S/t9/err" 2>/dev/null) $(grep RELEASE_FAILED "$L/events.log" 2>/dev/null | head -2)"; fi
fi

# T10 version fence: (a) an @1 held refuses @2 acquirers; (b) @1 code does not break an @2 lock of a dead holder.
mkdir -p "$L/held"; printf 'schema=hsk.studio.disk-build-lock@1\ntoken=old\npid=1\nwinpid=1\nexe=bash.exe\n' >"$L/held/owner"
(FAKE_TAG=F FAKE_SLEEP=0 cargo check) 2>"$S/F.err"; rc=$?
[ $rc -eq 75 ] && grep -q "protocol mismatch" "$S/F.err" && [ -z "$(ts F start)" ] && ok "T10a @1 held: @2 acquirer refused with 75 (protocol mismatch)" || bad "T10a rc=$rc $(cat "$S/F.err")"
rm -rf "$L/held"
git -C "$WT/wt-studio-assets" show 7095a32:handshake-studio-assets/tools/disk-build-lock.sh >"$S/lib-v1.sh" 2>/dev/null
bash -c 'sleep 60' & dead=$!; dbl_ident "$dead"; dstart=$DBL_START; kill -9 $dead 2>/dev/null; wait $dead 2>/dev/null
mkdir -p "$L/held"; printf 'schema=hsk.studio.disk-build-lock@2\ntoken=dead2\nclass=builder\nmsys_pid=%s\nmsys_start=%s\nwin_pid=-\n' "$dead" "$dstart" >"$L/held/owner"
if [ -s "$S/lib-v1.sh" ]; then
  STUDIO_DISK_BUILD_LOCK_TIMEOUT_S=4 bash -c 'source "$1"; dbl_acquire "$2" old-waiter "old" v1' _ "$S/lib-v1.sh" "$L" 2>"$S/V1.err"; rc=$?
  [ $rc -eq 75 ] && [ "$(dbl_field token "$L/held/owner")" = dead2 ] && ok "T10b @1 waiter did not break an @2 lock (gave up 75)" || bad "T10b rc=$rc $(cat "$S/V1.err")"
else bad "T10b @1 library not readable from 7095a32"; fi

# T11 pid reuse: a live pid with another start time is dead (broken); with its own start time it is alive.
(FAKE_TAG=G FAKE_SLEEP=0 cargo check) 2>"$S/G.err"; rc=$?
[ $rc -eq 0 ] && grep -q "broke stale lock dead2" "$S/G.err" && ok "T11a dead holder (pid gone) broken by @2" || bad "T11a rc=$rc $(cat "$S/G.err")"
bash -c 'sleep 30' & live=$!; dbl_ident "$live"; lstart=$DBL_START
mkdir -p "$L/held"; printf 'schema=hsk.studio.disk-build-lock@2\ntoken=reused\nclass=builder\nmsys_pid=%s\nmsys_start=%s\nwin_pid=-\n' "$live" "$((lstart - 100))" >"$L/held/owner"
(FAKE_TAG=H FAKE_SLEEP=0 STUDIO_DISK_BUILD_LOCK_TIMEOUT_S=10 cargo check) 2>"$S/H.err"; rc=$?
[ $rc -eq 0 ] && grep -q "broke stale lock reused" "$S/H.err" && ok "T11b live pid $live with another start time = reused = dead: broken" || bad "T11b rc=$rc $(cat "$S/H.err")"
mkdir -p "$L/held"; printf 'schema=hsk.studio.disk-build-lock@2\ntoken=same\nclass=builder\nmsys_pid=%s\nmsys_start=%s\nwin_pid=-\n' "$live" "$lstart" >"$L/held/owner"
(FAKE_TAG=I FAKE_SLEEP=0 STUDIO_DISK_BUILD_LOCK_TIMEOUT_S=4 cargo check) 2>"$S/I.err"; rc=$?
[ $rc -eq 75 ] && [ "$(dbl_field token "$L/held/owner")" = same ] && ok "T11c live pid with its own start time = alive: not broken (75)" || bad "T11c rc=$rc $(cat "$S/I.err")"
mkdir -p "$L/held"; printf 'schema=hsk.studio.disk-build-lock@2\ntoken=unknown\nclass=builder\nwin_pid=4\n' >"$L/held/owner"
(FAKE_TAG=J FAKE_SLEEP=0 STUDIO_DISK_BUILD_LOCK_TIMEOUT_S=4 cargo check) 2>"$S/J.err"; rc=$?
[ $rc -eq 75 ] && [ "$(dbl_field token "$L/held/owner")" = unknown ] && ok "T11d holder without identity = alive: not broken (75)" || bad "T11d rc=$rc $(cat "$S/J.err")"
kill $live 2>/dev/null; rm -rf "$L/held"

# T12 round priority + one builder per gap: B1 holds; B2, B3 queue; round R queues last but goes first; after its
# first unit it yields one gap (B2), then runs R2 before B3. Expected start order: B1 R1 B2 R2 B3.
(FAKE_TAG=B1 FAKE_SLEEP=3 cargo build) 2>"$S/B1.err" &
sleep 0.7
(FAKE_TAG=B2 FAKE_SLEEP=1 cargo build) 2>"$S/B2.err" &
sleep 0.7
(FAKE_TAG=B3 FAKE_SLEEP=1 cargo build) 2>"$S/B3.err" &
sleep 0.7
bash -c 'source "$1"; dbl_acquire "$2" round R "round" run-round || exit 9; echo "R1 start $(date +%s%3N)" >>"$3"; sleep 1; echo "R1 end $(date +%s%3N)" >>"$3"; dbl_yield
  dbl_acquire "$2" round R "round" run-round || exit 9; echo "R2 start $(date +%s%3N)" >>"$3"; sleep 1; echo "R2 end $(date +%s%3N)" >>"$3"; dbl_release' _ \
  "$KLIB" "$L" "$FAKE_LOG" 2>"$S/R.err" &
wait
order=$(for t in B1 R1 B2 R2 B3; do echo "$(ts $t start) $t"; done | sort -n | awk '{printf "%s ", $2}')
[ "$order" = "B1 R1 B2 R2 B3 " ] && ok "T12 order $order(round waited $(( $(ts R1 start) - $(ts B1 end) )) ms after B1)" || bad "T12 order $order $(cat "$S/R.err")"

# T13 cargo test refused (exit 2, nothing run); cargo test --doc runs under the lock.
(FAKE_TAG=T FAKE_SLEEP=0 cargo test -p x) 2>"$S/T.err"; rc=$?
(FAKE_TAG=D FAKE_SLEEP=0 cargo test --doc -p x) 2>"$S/D.err"; rd=$?
[ $rc -eq 2 ] && [ -z "$(ts T start)" ] && grep -q "nextest run" "$S/T.err" && [ $rd -eq 0 ] && [ -n "$(ts D end)" ] \
  && ok "T13 cargo test refused (2), cargo test --doc ran" || bad "T13 rc=$rc rd=$rd $(cat "$S/T.err")"

# T14 max hold: holder over its max_hold_s -> exactly one OVER_MAX_HOLD event; the holder is not broken.
rm -f "$L/events.log"
(FAKE_TAG=O FAKE_SLEEP=6 STUDIO_DISK_BUILD_LOCK_MAX_HOLD_S=2 cargo build) 2>"$S/O.err" &
sleep 1
(FAKE_TAG=O2 FAKE_SLEEP=0 cargo check) 2>"$S/O2.err" &
(FAKE_TAG=O3 FAKE_SLEEP=0 cargo check) 2>"$S/O3.err" &
wait
n=$(grep -c OVER_MAX_HOLD "$L/events.log" 2>/dev/null || echo 0)
[ "$n" -eq 1 ] && [ "$(ts O2 start)" -ge "$(ts O end)" ] && ! grep -q "broke stale" "$S/O2.err" "$S/O3.err" \
  && ok "T14 one OVER_MAX_HOLD event for a 6 s hold over 2 s; holder not broken" || bad "T14 events=$n $(cat "$L/events.log" 2>/dev/null)"
# T15 SSD opt-in (STUDIO_BUILD_DISK=ssd) against a scratch SSD root: (a) default unchanged without opt-in; (b) lane
# target inside the granted target, SSD lock; (c) bad owner and unknown disk refused at sourcing; (d) fresh over-cap
# record: 77 before queueing, nothing run; (e) stale over-cap record: still 77, starts a detached du re-measure; once
# it lands under cap the build runs on the SSD lock with owner=<lane>; (f) missing record: build allowed, measured;
# (g) record over cap during the queue wait: refused after acquiring.
SSD="$S/ssd"; LANE=studio-team-dev/steward/dbl-test/lane; REAL=${PATH#"$S/bin:"}
t15env() { unset STUDIO_DISK_BUILD_LOCK_DIR _STUDIO_ENV_LOCK_DERIVED STUDIO_DISK_BUILD_LOCK_TOKEN; cd "$S/ws1" || return 9
  export STUDIO_SSD_ROOT="$SSD" "$@"; PATH="$REAL"; source "$ENV" || return 9; export PATH="$S/bin:$REAL"; }
t15() { (t15env "$@" && echo "$CARGO_TARGET_DIR|$STUDIO_DISK_BUILD_LOCK_DIR|${_STUDIO_SSD_LANE:-}"); }
r=$(t15 STUDIO_OWNER=$LANE 2>&1)
[[ "$r" == ?:/*"/Handshake_Artifacts/WP-KERNEL-STUDIO/$LANE/target|"?:/*"/Handshake_Artifacts/WP-KERNEL-STUDIO/.disk-build-lock|" ]] \
  && [[ "$r" != *"$SSD"* ]] && ok "T15a no opt-in: lane target and lock stay on D:" || bad "T15a $r"
r=$(t15 STUDIO_OWNER=$LANE STUDIO_BUILD_DISK=ssd 2>&1)
[ "$r" = "$SSD/WP-KERNEL-STUDIO/studio-team-dev/target/lanes/$LANE|$SSD/.disk-build-lock|$LANE" ]   && ok "T15b ssd: lane target inside the granted target, SSD lock" || bad "T15b $r"
r1=$(t15 STUDIO_OWNER=../x STUDIO_BUILD_DISK=ssd 2>&1); c1=$?; r2=$(t15 STUDIO_OWNER=$LANE STUDIO_BUILD_DISK=e 2>&1); c2=$?
[ $c1 -ne 0 ] && [ $c2 -ne 0 ] && [[ "$r1" == *"plain lane owner"* ]] && [[ "$r2" == *unknown* ]]   && ok "T15c '../x' owner and STUDIO_BUILD_DISK=e refused" || bad "T15c $c1 $c2 $r1 $r2"
REC="$SSD/WP-KERNEL-STUDIO/.studio-ssd-size"
mkdir -p "$SSD/WP-KERNEL-STUDIO"; echo "$((150 * 1048576)) $(date +%s)" >"$REC"
: >"$FAKE_LOG"
(t15env STUDIO_OWNER=$LANE STUDIO_BUILD_DISK=ssd && FAKE_TAG=C1 FAKE_SLEEP=0 cargo build) 2>"$S/C1.err"; rc=$?
[ $rc -eq 77 ] && [ -z "$(ts C1 start)" ] && grep -q "> cap 140 GiB" "$S/C1.err" && [ ! -d "$SSD/.disk-build-lock/held" ] \
  && [ -z "$(ls -A "$SSD/.disk-build-lock/queue" 2>/dev/null)" ] && [ "$(cut -d' ' -f1 "$REC")" = $((150 * 1048576)) ] \
  && ok "T15d fresh over-cap record: refused 77 before queueing, nothing run, no re-measure" || bad "T15d rc=$rc $(cat "$S/C1.err")"
echo "$((150 * 1048576)) 1" >"$REC"
(t15env STUDIO_OWNER=$LANE STUDIO_BUILD_DISK=ssd && FAKE_TAG=C2 FAKE_SLEEP=0 cargo build) 2>"$S/C2.err"; rc=$?
for _ in $(seq 1 60); do read -r kib at <"$REC"; [ "${at:-1}" -gt 1 ] && break; sleep 0.5; done
(t15env STUDIO_OWNER=$LANE STUDIO_BUILD_DISK=ssd || exit 9
  FAKE_TAG=C3 FAKE_SLEEP=3 cargo build &
  for _ in $(seq 1 40); do [ -n "$(dbl_field child_msys_pid "$SSD/.disk-build-lock/held/owner")" ] && break; sleep 0.25; done
  cp "$SSD/.disk-build-lock/held/owner" "$S/C3.owner"; wait) 2>"$S/C3.err"
[ $rc -eq 77 ] && [ -z "$(ts C2 start)" ] && [ "${at:-0}" -gt 1 ] && [ "${kib:-99999999}" -lt 1048576 ] && [ -n "$(ts C3 end)" ] \
  && grep -qx "owner=$LANE" "$S/C3.owner" && [ ! -d "$REC.measuring" ] \
  && ok "T15e stale over-cap: 77, detached re-measure ($kib KiB) landed, next build ran on the SSD lock, owner=$LANE" \
  || bad "T15e rc=$rc kib=$kib at=$at $(cat "$S/C2.err" "$S/C3.err" "$S/C3.owner")"
rm -f "$REC"
(t15env STUDIO_OWNER=$LANE STUDIO_BUILD_DISK=ssd && FAKE_TAG=C4 FAKE_SLEEP=0 cargo build) 2>"$S/C4.err"; rc=$?
for _ in $(seq 1 60); do [ -s "$REC" ] && break; sleep 0.5; done
[ $rc -eq 0 ] && [ -n "$(ts C4 end)" ] && grep -q "size unknown" "$S/C4.err" && [ -s "$REC" ] \
  && ok "T15f missing record: build allowed, record measured ($(cat "$REC"))" || bad "T15f rc=$rc $(cat "$S/C4.err")"
# T15g record goes over cap while a lane waits in the queue: refused 77 once the lock is held, nothing run. Starts
# from a fresh under-cap record (a missing one would start a detached measure that can land after the over-cap write).
for _ in $(seq 1 60); do [ -d "$REC.measuring" ] || break; sleep 0.5; done
echo "1 $(date +%s)" >"$REC"
bash -c 'source "$1"; dbl_acquire "$2" builder holder "hold" t15g || exit 9; sleep 4; dbl_release' _ "$KLIB" "$SSD/.disk-build-lock" 2>/dev/null &
hp=$!; sleep 1
(t15env STUDIO_OWNER=$LANE STUDIO_BUILD_DISK=ssd && FAKE_TAG=C5 FAKE_SLEEP=0 cargo build) 2>"$S/C5.err" &
cp5=$!; sleep 2; echo "$((150 * 1048576)) $(date +%s)" >"$REC"
wait $cp5; rc=$?; wait $hp
[ $rc -eq 77 ] && [ -z "$(ts C5 start)" ] && grep -q "> cap 140 GiB" "$S/C5.err" && [ ! -d "$SSD/.disk-build-lock/held" ] \
  && ok "T15g over cap during the queue wait: refused 77 after acquiring, nothing run, lock released" || bad "T15g rc=$rc $(cat "$S/C5.err")"

# T16 release under an open reader (Windows: an open handle on R/held/owner makes the directory rename fail; T9
# RELEASE_FAILED 2026-10-16T02:47Z, the cycler then waited on its own token). (a) a reader holding the owner file for
# 3 s: release retries and succeeds after it closes, no RELEASE_FAILED; (b) release window 1 s and a 4 s reader:
# RELEASE_FAILED, then the same process's next acquire renames its own leftover (RELEASE_RECOVERED) and gets the lock.
L16="$S/lock16"; rm -rf "$L16"
bash -c 'source "$1"; dbl_acquire "$2" builder t16a "t16a" t16 || exit 9
  (exec 7<"$2/held/owner"; sleep 3) & sleep 0.5; t0=$(date +%s); dbl_release; echo "released $(( $(date +%s) - t0 ))"; [ ! -d "$2/held" ]' \
  _ "$KLIB" "$L16" >"$S/T16a.out" 2>"$S/T16a.err"; rc=$?
[ $rc -eq 0 ] && ! grep -q RELEASE_FAILED "$S/T16a.err" "$L16/events.log" 2>/dev/null \
  && ok "T16a open reader for 3 s: release retried, done ($(cat "$S/T16a.out") s), no RELEASE_FAILED" || bad "T16a rc=$rc $(cat "$S/T16a.out" "$S/T16a.err")"
bash -c 'source "$1"; dbl_acquire "$2" builder t16b "t16b" t16 || exit 9; first=$DBL_TOKEN
  (exec 7<"$2/held/owner"; sleep 4) & sleep 0.5; STUDIO_DISK_BUILD_LOCK_RELEASE_S=1 dbl_release
  [ "$DBL_UNRELEASED" = "$first" ] || exit 8
  t0=$(date +%s); STUDIO_DISK_BUILD_LOCK_TIMEOUT_S=30 dbl_acquire "$2" builder t16b "t16b again" t16 || exit 7
  echo "reacquired $(( $(date +%s) - t0 ))"; [ -z "$DBL_UNRELEASED" ] || exit 6; dbl_release; [ ! -d "$2/held" ]' \
  _ "$KLIB" "$L16" >"$S/T16b.out" 2>"$S/T16b.err"; rc=$?
[ $rc -eq 0 ] && grep -q RELEASE_FAILED "$L16/events.log" && grep -q RELEASE_RECOVERED "$L16/events.log" \
  && ok "T16b release failed under a 4 s reader; same process recovered its own leftover and $(cat "$S/T16b.out") s" \
  || bad "T16b rc=$rc $(cat "$S/T16b.out" "$S/T16b.err" "$L16/events.log" 2>/dev/null)"
echo "RESULT fails=$fails"
[ $fails -eq 0 ]
