# Harness Comparative Research — 2026-04-26

Deep technical research on four agent harnesses, motivated by Handshake's token-burn and artifact-repair pain.

## Read order

1. **[00_HARNESS_COMPARATIVE_ANALYSIS.md](00_HARNESS_COMPARATIVE_ANALYSIS.md)** — synthesis. Start here.
2. **[04_gastown.md](04_gastown.md)** — Gastown by Steve Yegge. Closest peer to Handshake.
3. **[03_openclaw.md](03_openclaw.md)** — OpenClaw + ACPX. Direct ACP comparator.
4. **[02_hermes.md](02_hermes.md)** — Hermes Agent by Nous Research. Cache-stability and `<tool_call>` lessons.
5. **[01_pi.md](01_pi.md)** — Pi (`badlogic/pi-mono`). Single-loop, opinionated, philosophical.

## What this answers

- Why is Handshake burning 110M tokens on a single orchestrator run?
- Why does smarter-model adoption fail to translate into faster work?
- Why has microtasks + ACP been slower than the operator-relay days?
- What concrete primitives could Handshake adopt to fix these?

## Local clones inspected

The harness repos referenced throughout these documents are external research clones, not part of the Handshake repo. They are tracked under logical names so the docs stay drive-agnostic:

- `pi-mono`
- `hermes-agent`
- `openclaw`
- `openclaw-acpx`
- `gastown`

### Where the clones live, and how to resolve them

Convention: clones live in a `harnesses/` directory adjacent to (or above) the Handshake repo root. Default layout:

```
<workspace>/
├── Handshake/                         # or wherever the repo is checked out
└── harnesses/
    ├── pi-mono/
    ├── hermes-agent/
    ├── openclaw/
    ├── openclaw-acpx/
    └── gastown/
```

The resolver `.GOV/roles_shared/scripts/lib/resolve-harness-path.mjs` finds the harnesses root in this order:

1. `HANDSHAKE_HARNESSES_ROOT` env var — set this on machines where clones live somewhere else.
2. Walk up from the repo root looking for a `harnesses/` directory (default convention above).
3. Returns `null` if neither resolves; callers handle the missing case.

By convention, harness clones live outside the Handshake repo and so are never tracked. If a `harnesses/` directory ever lands inside a worktree by accident, add `harnesses/` to the root `.gitignore` (this edit must happen on `main` from `handshake_main`, since `.gitignore` is a main-only root file).

Governance docs cite harness code by *logical* path (`hermes-agent/AGENTS.md:521-535`). Scripts that need to actually open the file call `resolveHarnessPath('hermes-agent', 'AGENTS.md')` and get an absolute path back.

## Headline finding

Handshake's pain is not an ACP problem, a model problem, or a microtask problem — it is a **state-in-documents problem**. All four harnesses studied refuse to put coordination state in markdown files the model authors. The fix is a typed-event wire format, machine-written state, and a hard cache-stability rule. Tier-1 changes (cache-stability policy, `details`/`content` split on tool results, `<memory-context>` fence in user message, `coerce_tool_args`-style normalizers) are week-of-work scale and produce measurable wins independently of the deeper architectural changes.
