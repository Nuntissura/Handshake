"""Lead: write validation/dispatch-prompt-R60.txt from the ledger's READY lanes at the freeze.
Usage: python dispatch_r60.py <candidate_sha_8> <commits_since_r59> [--write]. Without --write it prints only."""
import glob, json, os, sys
S = r"D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work"
cand, ncommits = sys.argv[1], sys.argv[2]
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
text = f"""ROLE: You are the independent validator for validation round STUDIO-R60 of WP-KERNEL-STUDIO. Studio Kit is a Rust rewrite of the Adobe suite, Figma and Affinity. The product repo is Nuntissura/assets, branch codex/handshake-studio-assets, subfolder handshake-studio-assets.
- Reasoning tier: opus/xhigh.
- You judge and write no product code. Be adversarial. A READY is only a builder's claim. Every lane below had a pre-build code-check and red-team read and fixed what it found; those reads are leads, not findings you may reuse, and nothing in them was run by their authors.

## Authority
Read these first. They win over this prompt.
- Kernel `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel`.
- Codex `.GOV/codex/Handshake_Codex_v1.4.md`, rules CX-VAL-001..011, CX-EXEC-015, CX-VERIFY-002, CX-SAFE-001, CX-STATUS-001 and CX-GIT-001A.
- Studio records `.GOV/docs_repo/wp-studio-module-work/`:
  - `validation/round-R59.json` is the latest round. `validation/round-R57.json` is the latest FULL round.
  - `ledger.json` keys `direction.workflow_20261107`, `direction.operator_direction_20261107` and `direction.blocker_decisions_20261107` (window_layer, gpu_contention, matrix_cut, landing, spine_gate).
  - HANDOFF.md topics `lessons`, `how-to-lead` and `workflow`.
  - `builders/DISPATCH_TEMPLATES.md` topic `validator`, including step 6 (MT status).
- Binding on every claim: the HOST INTEGRATION rules in `builders/briefs/wave-parity-2-common.md`, and Operator decisions D1, D6, D7 and D8. Write a HOST INTEGRATION section per claim.
- Master Spec v02.229.

## Candidate
- Freeze exactly {cand}, the origin tip at dispatch. {ncommits} commits since the R59 candidate c4d06841.
- This round is FULL (CX-VAL-011: R59 was scoped, R58 scoped, R57 full). SMB-GUI-STEWARD-1 changed root inputs (Cargo.toml workspace members and pins, Cargo.lock, .config/nextest.toml GPU group, .config/round-features.toml, tools/run-round.sh, tools/workspace-probe.rs, tools/registry-sync.py; read 7ab918bf~1..7879e7e9), and the lead changed the kernel's builders/studio-env.sh GPU lock pattern and tools/edges.py naming, so the canary is required under CX-VAL-010: run it and record the result. Re-derive the root-input list yourself from c4d06841..{cand}.
- Matrix cut: the R59 validator wrote matrix_cut_no_objection. If the steward's cut of the 5 strict-subset harness feature units is in the candidate, verify from the diff that each dropped unit is a strict subset of a kept one and record it; if it is not in the candidate, run the matrix as it stands.
- New crates since R59 include hsk-studio-vulkan and hsk-studio-native-host (winit =0.30.13 there only, ash =0.38.0, offline SPIR-V) and kumihimo-ui. Check direction.blocker_decisions_20261107: winit only in native-host; no kumihimo-* crate has an hsk-studio-* dependency in its closure; permissive licences only (cargo-deny or the repo's own check). Their tests take the GPU lock; a GPU or window test that cannot get a device is SKIPPED with its reason, never PASS.
- Machine change since R59: the Vulkan SDK 1.4.363.1 core was installed on 2026-11-08 at 04:41Z (ledger `direction.lead_rulings_20261108.vulkan_sdk`, with its proof). It updated the system Vulkan loader to 1.4.363 and registered explicit layers machine-wide (HKLM ExplicitLayers), among them VK_LAYER_KHRONOS_validation. An explicit layer loads only when requested, and no code at {cand} requests one, so the lanes' `validation: absent` receipts mean "not enabled": the layer was not installed when they were built. Do not set VK_INSTANCE_LAYERS or any other variable (CX-VAL-007). Record the loader version that `vulkaninfoSDK --summary` reports, judge the receipts as written, and if a GPU test behaves differently from R59, classify the cause from its output. Turning the layer on in code is the job of the scoped lane SMB-VK-UI-PASS-1, not of this round.
- PASS verdicts carry forward under CX-VAL-002 only where their inputs are unchanged; in a full round every unit runs. Lanes touched by this candidate are re-judged under CX-VAL-009.
- Builders keep building on D: and may land during your round (CX-VAL-001: the candidate stays frozen). None builds on C: while you run. Lane worktrees and lane branches exist under CX-GIT-001A; the ledger lists them at `direction.workflow_20261107.lane_branches`; they are not defects in your inventory comparison. If anything else holds C: or D: during your round, record it; do not touch it.
- MEMORY: Chrome stays open by the Operator's instruction. Record commit free at start, during and at the end. A build failure from memory is infrastructure (CX-VAL-006): classify it from the failure output, never from timing alone, and report it rather than rerunning blind. You stop nothing.
- C: Studio tree: measure it at start, during and end against the cap of 360 GiB. Alert the lead at 330 GiB. Stop cleanly above 355 GiB. Record `df` at start and end.
- TIME: record the union's duration against R57 (FULL) and R59 (50 units, 3 h 02). Report build and run time from your own logs, and what else was building. Do not investigate further.
- Never write an environment dump.

## The READY claims
{claims}

For each claim: re-derive what it claims from the lane file and the diff, run its tests in your union (process tests spawn examples/studio-harness.exe: build it fresh, never trust a stale binary), break it from the outside with your own probes where the claim is behavioural, and judge it PASS_Vn or FAIL_Vn with findings. A builder's cut list is a named limit: judge whether each cut is honestly named or hides a defect.

## MT judging
- Follow step 6 for every MT a claim names. Write status only if judged met on a PASS_Vn lane. Do not edit MT proof commands.

## Output
Same as R52..R59:
- `validation/round-R60.json`, with `mt_judgments` and `mts_passed`.
- Ledger statuses and a `round_R60` entry (python json load-modify-dump, indent=1, ensure_ascii=False, newline '\\n').
- One START_LOG.md line.
- Verdicts in the lane files.
- MT files and `_MT_INDEX.json` per step 6. Write through a temp file plus os.replace, serialised exactly as HEAD: `json.dumps(obj)+'\\n'`.

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
- GPU backends, Vulkan device, and any timeout, classified;
- commit free, C: tree and `df` at start, peak and end;
- the kernel commit sha.
"""
print(f"{len(ready)} READY claims")
if "--write" in sys.argv:
    with open(S + "/validation/dispatch-prompt-R60.txt", "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print("written")
else:
    print(text[:200])
    print(claims)
