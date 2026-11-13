"""Lead: write validation/dispatch-prompt-R60.txt for STUDIO-R60 ATTEMPT 3 from the ledger's READY lanes at the freeze.
Attempt 3 generator (replaces the attempt 2 text; draft research/prep-20261112/round-R60-a3/dispatch_r60_attempt3.py).
Usage: python -X utf8 tools/lead/dispatch_r60.py <candidate_sha_8> <commits_since_c4d06841> <commits_since_35c50d3b>
         [--preview PATH] [--write]
  no flag    prints the READY count and the claim rows only
  --preview  writes the full prompt to PATH (any scratch path)
  --write    writes <S>/validation/dispatch-prompt-R60.txt (attempt 1 and 2 precedent: 33a6d64d, 1316c8b0)
S (Studio records root): env STUDIO_S, else two levels above this file (tools/lead/); it must hold ledger.json.
The kernel path in the prompt is derived from S (<kernel>/.GOV/docs_repo/wp-studio-module-work); nothing is hard-coded."""
import glob, json, os, pathlib, sys

S_PATH = pathlib.Path(os.environ.get("STUDIO_S") or pathlib.Path(__file__).resolve().parents[2]).resolve()
if not (S_PATH / "ledger.json").is_file():
    sys.exit(f"no ledger.json in {S_PATH}: run from tools/lead/ or set STUDIO_S to the Studio records root")
S = S_PATH.as_posix()
KERNEL = S_PATH.parents[2].as_posix()
preview = None
if "--preview" in sys.argv:
    preview = sys.argv[sys.argv.index("--preview") + 1]
args = [a for a in sys.argv[1:] if not a.startswith("--") and a != preview]
if len(args) != 3:
    sys.exit(__doc__)
cand, ncommits, ncommits_a2 = args[0], args[1], args[2]
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
text = f"""ROLE: You are the independent validator for validation round STUDIO-R60 of WP-KERNEL-STUDIO, attempt 3. Studio Kit is a Rust rewrite of the Adobe suite, Figma and Affinity. The product repo is Nuntissura/assets, branch codex/handshake-studio-assets, subfolder handshake-studio-assets.
- Reasoning tier: opus/xhigh.
- You judge and write no product code. Be adversarial. A READY is only a builder's claim. Every lane below had a pre-build code-check and red-team read and fixed what it found; those reads are leads, not findings you may reuse, and nothing in them was run by their authors.

## Authority
Read these first. They win over this prompt.
- Kernel `{KERNEL}`.
- Codex `.GOV/codex/Handshake_Codex_v1.4.md`, rules CX-VAL-001..011, CX-EXEC-015, CX-VERIFY-002, CX-SAFE-001, CX-STATUS-001..003 and CX-GIT-001A.
- Studio records `.GOV/docs_repo/wp-studio-module-work/`:
  - `validation/round-R59.json` is the latest opened round. `validation/round-R57.json` is the latest FULL round. `validation/round-R60.json` holds attempts 1 and 2 (not opened); `validation/round-R53.json` is the precedent for a round that opened on a later attempt.
  - `ledger.json` keys `round_R60`, `direction.workflow_20261107`, `direction.operator_direction_20261107`, `direction.blocker_decisions_20261107` (window_layer, gpu_contention, matrix_cut, landing, spine_gate), `direction.lead_rulings_20261108`, `direction.resume_20261108` (r60_attempt_2, plan_t, c_cap_20261111), `direction.pause_20261111` and `direction.pause_20261112`.
  - `research/lead-rulings-20261108-by-lane.json` (rulings given by message, written down).
  - HANDOFF.md topics `lessons`, `how-to-lead`, `workflow` and `state`.
  - `builders/DISPATCH_TEMPLATES.md` topic `validator`, including step 6 (MT status). Its step 3 gameplan confirmation is retired (CX-VAL-010, 2026-10-20): skip it.
- Binding on every claim: the HOST INTEGRATION rules in `builders/briefs/wave-parity-2-common.md`, and Operator decisions D1, D6, D7 and D8. Write a HOST INTEGRATION section per claim.
- Master Spec v02.229.

## Candidate
- This is ATTEMPT 3 of STUDIO-R60. Neither earlier attempt opened (CX-VAL-010), so the round id stays STUDIO-R60. Attempt 1 at cede5958: canary step 0 exit 75 (F-R60-1). Attempt 2 at 35c50d3b: step 0 green, then the lead stopped the canary at 2026-11-08T09:22:39Z on the C: cap projection after 11 of 164 units plus the fixture, all exit 0 (failure_class c-cap-projection-canary-identity-rebuild). Read `validation/round-R60.json` (`attempts`, `attempt_2`, `attempt_2_probes`) and the ledger `round_R60` entry.
- Record form (lead decision: R53 precedent, `validation/round-R53.json` `record_form`; do not open a new round id):
  - `validation/round-R60.json`, if attempt 3 opens: the top level becomes attempt 3; attempt 1's present top-level record moves verbatim under `attempt_1`, except `attempt` and `status`, which take attempt 1's values from `attempts[0]` (attempt 2 changed only those two keys, R60 `record_form`); `attempt_2` and `attempt_2_probes` stay; `attempts` gains an attempt 3 entry; `record_form` says so. If it does not open: add `attempt_3` beside `attempt_2` and an attempt 3 entry in `attempts`, and change only the top-level `attempt` and `status`, as attempt 2 did.
  - Ledger `round_R60` already keeps the latest attempt at the top level with attempt 1 under `attempt_1`. In either case its top level becomes attempt 3 and attempt 2's present top-level record moves verbatim under `attempt_2`; `attempt_1` stays.
- Since attempt 2 the Operator approved, and the lead executed, C: plans T (T-C01..T-C04) and U; the warm round target `studio-round/target` was kept (plan T `rejected`). C: room was the blocker named in ledger `round_R60.next`.
- Freeze exactly {cand}, the origin tip at dispatch. {ncommits} commits since the R59 candidate c4d06841, {ncommits_a2} since attempt 2's 35c50d3b. Re-derive both counts and the claim owner of every commit (attempt 2 form: validator-R60-a2/claims.json); name unowned commits.
- Entrypoint (CX-VAL-007; `builders/DISPATCH_TEMPLATES.md` topic `validator` steps 1-3): from the refreshed export, cwd `studio-round/src/handshake-studio-assets`, run `bash tools/run-round.sh --canary --all --results <attempt-3 canary results dir> --target-dir <round target>`, then only if the canary is green `bash tools/run-round.sh --all --results <attempt-3 union results dir> --target-dir <round target>`. The round target is the warm C: target `studio-round/target` (path in `validation/round-R59.json` `build_target`). Nothing goes before `bash` and nothing is added after: no environment variable, flag or wrapper. With the target on C:, run-round queues for the C: build lock as class round and holds the GPU test lock under the D: artifact root for its test steps; there is no D: round hold.
- This round is FULL with the canary. CX-VAL-011: R57 full, R58 and R59 scoped, R60 never opened. CX-VAL-010: root inputs changed after 35c50d3b; the lead's read of 35c50d3b..{cand} lists Cargo.lock, crates/hsk-studio-vulkan/Cargo.toml, crates/hsk-studio-vulkan/spv-build/Cargo.toml and Cargo.lock, and tools/workspace-probe.rs (SMB-VK-UI-PASS-1 503f6874, 980685f4). Re-derive the root-input list yourself from c4d06841..{cand}. Attempt 2's partial canary is not reusable (incomplete, and the lock changed since; CX-VAL-002, CX-VERIFY-002).
- Lead pre-freeze check (HANDOFF state, F-R60-2): `python -X utf8 -I tools/registry-sync.py .` check mode exit 0 at {cand} (approved_added, approved_removed, unrequested and spec_defects empty). Step 0 is still yours to run and record.
- Matrix cut: in the lead's diff 35c50d3b..{cand} no `.config/` file changed. Verify it; if the steward's cut of the 5 strict-subset harness feature units is not in the candidate, run the matrix as it stands and say so.
- Attempt 2's 10 probe failures (`attempt_2_probes`; ledger `round_R60.probe_failures`) are leads: re-check each against {cand}. The ledger names their fixing lanes (`direction.resume_20261108.r60_attempt_2.result.probe_fixes`): SMB-R60-FOLLOWUPS-1, SMB-WEB-SITE-1, SMB-DEVELOP-COMMANDS-1 and SMB-MOTION-COMMANDS-2. Reuse `validator-R60/review-R60-*.json` only as leads.
- New crates since R59 include hsk-studio-vulkan and hsk-studio-native-host (winit =0.30.13 there only, ash =0.38.0, offline SPIR-V) and kumihimo-ui. Check direction.blocker_decisions_20261107: winit only in native-host; no kumihimo-* crate has an hsk-studio-* dependency in its closure; permissive licences only (cargo-deny or the repo's own check). Their tests take the GPU lock; a GPU or window test that cannot get a device is SKIPPED with its reason, never PASS.
- Vulkan validation layer: at {cand} code requests it. `crates/hsk-studio-vulkan/src/owner.rs:716` enables VK_LAYER_KHRONOS_validation in debug builds when the loader lists it (SMB-VK-UI-PASS-1 item V; `crates/hsk-studio-native-host/README.md:178`). Attempt 2's statement that no code requests a layer no longer holds. Set no VK_* or other variable (CX-VAL-007). Record the loader version `vulkaninfoSDK --summary` reports (Vulkan SDK 1.4.363.1 Bin folder, not on PATH; path in round-R60.json `attempt_2.vulkan_loader`), judge `validation:` receipts against owner.rs:926 as written, and classify any GPU test that behaves differently from R59 from its output.
- PASS verdicts carry forward under CX-VAL-002 only where their inputs are unchanged; in a full round every unit runs. At the freeze, CX-VAL-009 re-opens every PASS lane whose relevant inputs intersect the candidate diff (attempt 1 recorded 407 in validator-R60/reopen.json at cede5958): recompute it at {cand}.
- Builders run only on D: and may land during your round (CX-VAL-001: the candidate stays frozen). None builds on C: while you run (Operator's plan T condition). Lane worktrees and lane branches exist under CX-GIT-001A; the ledger lists them at `direction.workflow_20261107.lane_branches`; they are not defects in your inventory comparison. If anything else holds C: or D: during your round, record it; do not touch it.
- MEMORY: Chrome stays open by the Operator's instruction. Record commit free at start, during and at the end. A build failure from memory is infrastructure (CX-VAL-006): classify it from the failure output, never from timing alone, and report it rather than rerunning blind. You stop nothing you did not start.
- C: Studio tree: measure it at the freeze, during and at the end against the cap of 360 GiB. Alert the lead at 330 GiB. Stop cleanly above 355 GiB. In attempt 2 the tree grew from 244.5 GiB (freeze, 07:27Z) to 307.9 GiB (end, 09:43Z) on 11 of 164 canary units; project canary plus union from your own samples and report the projection with your first sample. Record `df` at start and end. You delete nothing.
- If the canary or any step 0 check fails, record it and end your turn AT ONCE with the failure output in your final message, so the lead can fix it and resume you.
- TIME: record the union's duration against R57 (FULL, 160 units, 3 h 48) and R59 (50 units, 3 h 02). Report build and run time from your own logs, and what else was building. Do not investigate further.
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
- Working files in `Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/`; results under `studio-round/results/` with an attempt-3 name (attempt 2 used R60-canary-2); the running record appends to `studio-round/RUNNING-R60.jsonl`.

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
- commit free, C: tree and `df` at start, peak and end;
- the kernel commit sha.
"""
print(f"{len(ready)} READY claims")
missing = [lid for lid, _ in ready if lid not in lane_files]
if missing:
    print("WARNING: READY lanes without a lane file:", ", ".join(missing))
if preview:
    with open(preview, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print("preview written:", preview)
if "--write" in sys.argv:
    with open(S + "/validation/dispatch-prompt-R60.txt", "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print("written")
if not preview and "--write" not in sys.argv:
    print(claims)
