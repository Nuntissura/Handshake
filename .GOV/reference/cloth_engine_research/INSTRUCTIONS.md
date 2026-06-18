# How to Use and Edit This Research Paper

Machine- and human-facing guide for the Handshake-native cloth-engine research paper.
Linked from `index.yaml` → `research_paper.instruction_file`.

## [INSTR.what] What this is

A multi-file research paper on a **Handshake-native, model-steerable cloth/garment "detailer"** (a Marvelous-Designer-equivalent built inside Handshake, no side apps). It inventories the `wtc-kernel-009` codebase, maps MD's full feature set, studies open-source cloth/garment projects + their code, and translates everything to a Handshake-native Rust design.

The feature is named **Tailor** (Handshake Tailor) — see `index.yaml` → `research_paper.naming`. It joins the **Atelier · Loom · Tailor** textile-word family; `Tailor` is the feature/module/crate/event identifier while `cloth` stays the physics terminology.

It is **reference/landscape material**, not authority: per `KERNEL_BUILDER_PROTOCOL` it is input to a *future* Work Packet, not a packet itself. It does not gate code by itself.

## [INSTR.layout] Layout

- `index.yaml` — the machine-readable map (topics, files, summaries, `depends_on`, subtopics, `planned_topics`, `known_issues`, `review`). **Start here.**
- `INSTRUCTIONS.md` — this file.
- `NN-<slug>.md` — one topic per file (`00`–`12`).

## [INSTR.headers] Header convention (machine-readable)

Every topic file has:

1. **YAML frontmatter**: `file_id, topic_id, title, status, depends_on, summary, sources, updated_at`.
2. **One topic header**: `## [<TOPIC_ID>] <Title>` (e.g. `## [T-CLOTH-SOLVER] ...`).
3. **Subtopic headers**: `### [<TOPIC_ID>.<suffix>] <Title>` — every subtopic carries the bracketed machine id (e.g. `### [T-CLOTH-SOLVER.gpu-architecture] ...`).
4. A final `### [<TOPIC_ID>.sources] Sources` block with the URLs used.

Full subtopic id = `<topic_id>.<suffix>`. IDs are **stable — do not renumber**; reference any section as `<TOPIC_ID>` or `<TOPIC_ID>.<suffix>`.

## [INSTR.read] How to read

- Whole map: open `index.yaml`.
- List every topic: `rg '^## \[' .`
- Subtopics of one topic: `rg '^### \[T-CLOTH-SOLVER' .`
- Find a concept: grep the bracket ids (they are greppable by design).
- Sources for a topic: its final `### [<TOPIC_ID>.sources]` block.
- Reading order for a newcomer: `00-overview` → `02-md-feature-map` (what we're matching) → `01-codebase-inventory` (what we build on) → `12-roadmap` (the plan); then dive into the subsystem topics (`03`–`11`).

## [INSTR.edit] How to edit

- **Edit a topic**: edit its file, bump `updated_at` in its frontmatter, and update its `summary` in BOTH the frontmatter and `index.yaml` if it changed.
- **Add a subtopic**: add `### [<TOPIC_ID>.<new-suffix>] <Title>`, then add `<new-suffix>` to that topic's `subtopics:` list in `index.yaml`.
- **Add a topic**: create `NN-<slug>.md` with frontmatter + `## [<NEW_TOPIC_ID>] ...`, then add an entry under `topics:` in `index.yaml` (`id, title, file, status, depends_on, summary, subtopics`). If it was listed under `planned_topics`, move it into `topics:`.
- **Resolve a known issue**: fix the file(s), then update or remove the matching `known_issues` entry in `index.yaml`.
- **Status values**: `draft` | `review` | `stable` | `planned` | `superseded`.

## [INSTR.discipline] Editing discipline (Handshake)

- This is a `.GOV/reference/` surface — role/machine-facing. Edit it from **`wt-gov-kernel`**, never through a WP-worktree `.GOV` junction.
- Keep it typed/greppable: the bracket ids + frontmatter + `index.yaml` are the contract. Do not break the header convention.
- It is reference, not authority. Implementation intent/proof comes from the Master Spec + a signed WP, not from this paper.

## [INSTR.next] Current state and next edits

The package is a **codebase-verified landscape+architecture foundation** (13 topics). Before it can gate an implementation WP, see `index.yaml` → `planned_topics` + `known_issues`:

1. Author the 3 missing moat topics: `T-TRIM-RIGID`, `T-UV-TEXTURE`, `T-ANIMATION`.
2. Author `T-CONTRACTS` (reconcile the one canonical `GarmentSpec`, `KernelEventType` additions, body-proxy/avatar schema + the missing `tailor_avatars` table, dated migration naming, and the consolidated `ValidationDescriptor` catalog).
3. Resolve `KI-DETERMINISM-VS-PROMOTION` (tolerance-based mesh compare, not exact hash) and add the missing OSS refs (Warp, XRTailor).

The standalone solver-crate R&D (roadmap slices 1–2) has **no kernel dependency** and can start in parallel now; the kernel-integration slices (3–6) should wait on the contracts reconciliation.
