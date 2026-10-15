---
file_id: WAVE-PARITY-2-COMMON
file_kind: dispatch_preamble
updated_at: 2026-10-15T00:50:00Z
---

<topic id="common" status="active" summary="Shared rules for parity wave 2 builders">

## Shared rules (parity wave 2)

- Read first: `builders/DISPATCH_TEMPLATES.md` (builder + v4-rules, binding), `START_LOG.md` (repo-rules, cargo-tooling, decisions, all 2026-10-14 entries), `research/parity-matrix-20261014b.json` (your app's features, gaps and evidence), your owning crate's lane files under `builders/lanes/`, and the Studio spec module (resolve `.GOV/spec/SPEC_CURRENT.md` to module `14-studio-creative-suite.md`; grep the clauses for your feature; quote them as spec_basis in the lane notes; do not read the whole module).
- Product workspace: `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets` (branch `codex/handshake-studio-assets`). Own only the paths named in your dispatch.
- Cargo: leading `cd` to the workspace, then `source <kernel>/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh`; builds go to the C: SSD target. One cargo process at a time; run anything over 2 minutes in the background. Check + clippy in one invocation over your crate and its direct dependents (additive changes only for shared engines). Focused tests via the per-test runner, 5 or fewer per slice.
- Minimal vertical slices: commit and push per compiling slice, with pathspec commits only. Leave Cargo.lock uncommitted. Any new dependency needs a `builders/dep-requests.jsonl` line BEFORE pushing (pure Rust, permissive licence, version older than 2 weeks). New module edges are steward-owned: report them to the lead.
- NEVER add, confirm or retire steps in the shared gameplan (`.GOV/task_packets/WP-KERNEL-STUDIO/gameplans/STUDIO-MODULE-BUILD.yaml`) for your own test failures, wrong expectations or tolerances. A step there blocks EVERY builder. If the retry guard blocks an identical rerun, commit your fix (HEAD changes) and rerun. Only the lead or steward adds steps, and only for shared infrastructure causes. Record your own causes in your lane notes. Never run gated commands from the kernel root, where the hook resolves WP-KERNEL-012's gameplan; use the leading `cd` into the product workspace.
- Oracle corpus (`Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/`): add new files only. Never rename, move or delete a corpus file while a validation round may be open (R29: a rename between canary and union made 10 lanes inconclusive); ask the lead, who schedules renames between rounds.
- Shared files: if another lane also has uncommitted edits in a file you commit, stage only your own hunks (apply a patch of your hunks to the index), never the whole file, and record it in your lane notes; a commit that carries another lane's hunk must be reported to the lead.
- Never push a test with a placeholder expectation (e.g. a golden digest "pending"); a pushed commit must pass its own tests, because rounds may freeze any pushed commit (R32: a docgraph test with a "pending" pin failed the union).
- Before every push, run `git diff --stat <origin tip>..HEAD` and confirm it lists ONLY your own paths; a temporary-index commit must start from the CURRENT origin tree (fetch, then read-tree), never from an older HEAD, using a PRIVATE index file (GIT_INDEX_FILE) and never leaving entries in the shared .git/index, or it silently reverts other lanes (db231fb deleted three AE-KEY-TRACK-3 files this way).
- Hostile input (V4-01/02), no silent green (V4-03), claims cite tests (V4-07). Never claim PASS. Set READY_FOR_VALIDATION in your lane file (workflow-v3 keys) per slice on a pushed commit.
- Where Adobe or Figma behaviour is the reference and unverified, label it UNVERIFIED (it goes to the D5 oracle session). Never invent parity.
- Clean-room: no copyleft (GPL/LGPL/MPL) source; permissive source may be read and adapted with attribution.
- Deadline: first pushed commit within 30 minutes, then one at least every 30 minutes. Final message of 10 lines or fewer: shas per slice, capabilities, tests, gaps left.

</topic>
