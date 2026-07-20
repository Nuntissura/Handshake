---
file_id: stage-mvp-research-plan
file_kind: reference-research-plan
updated_at: "2026-07-19"
status: full-feature-research-basis-hardened-validation-pending
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-research-plan" status="full-feature-research-basis-hardened-validation-pending" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage MVP research plan

## Research objective

Produce enough current implementation evidence to choose a Servo-primary Stage architecture, tightly bound the Chromium bootstrap, define safety and compatibility gates, and support a no-context model in creating the later refinement and microtask DAG without guessing.

## Required research workstreams

1. Servo embedding API, release cadence, LTS posture, build and packaging.
2. Multiprocess operation, OS sandboxing, Windows support, crash containment, and hostile-content boundary.
3. Web compatibility for authentication, forms, uploads, downloads, media, modern JavaScript applications, accessibility, and browser storage.
4. Minimal Chromium bootstrap candidates and the maintenance cost of WebView2, CEF, ungoogled Chromium, or controlled external-process approaches.
5. Servo request interception, Rust ad-block engines, filter formats, updates, allowlists, diagnostics, and rollback.
6. Agent-native observation and action protocols using DOM, accessibility, screenshots, network events, stable identities, receipts, and replay.
7. Profile/session isolation, authenticated-session handoff, explicit cross-engine transfer, retention, and portable manifests.
8. Visual debugging, Argus compatibility, quiet/background operation, representative-site corpora, and regression testing.
9. Servo dependency pinning, LTS consumption, downstream patch queue, upstream contribution, licensing, and vulnerability response.
10. Current Handshake Stage, Argus, WebView2/CDP, artifact, ASR, Media Downloader, storage, and governance integration points.
11. High-volume tab lifecycle, renderer suspension/discard, virtualized navigation, session restore, cache quotas, and 3,000+ tab performance testing.
12. Stage folders/bookmarks/tags and LoomBlock/tag/backlink/project-knowledge integration without duplicate authority.
13. Cookie editing/export and governed authenticated-session handoff to Media Downloader.
14. Project asset intake from page/media selection through ArtifactStore into Lens/Atelier.
15. Page/text translation, Markdown/PDF export, fuzzy metadata search, full-text captured-content search, and optional semantic retrieval.
16. Complete native browser-product workflows omitted by embedded WebView runtimes: omnibox, history, bookmarks, recently closed, settings, permissions, downloads, onboarding, accessibility, localization, and recovery.
17. Authenticated-site compatibility, Google/YouTube embedded-login limits, external-user-agent OAuth, WebAuthn/MFA/client certificates, and governed session acquisition.
18. Browser-agent prompt injection, source-to-sink data-flow control, exfiltration, consequence classes, confirmation/watch/takeover, and adversarial evaluation.
19. Production persistence, EventLedger/outbox/projectors, concurrency, migrations, backup/restore, resource scheduling, failure recovery, and health schemas.
20. Windows packaging, engine/runtime updates, SBOM/licensing/signing, safe mode, support bundles, incident operations, and release promotion.

## Required source classes

- Official Servo documentation, API docs, source, issues, release notes, security material, and WPT results.
- Official Chromium/WebView2/CEF documentation and source where relevant to the bootstrap decision.
- Primary source and issue-tracker evidence from real embedders and adjacent Rust browser projects.
- Brave `adblock-rust` and other mature Rust filtering implementations.
- Browser automation, accessibility, replay, sandbox, and process-isolation implementation sources.
- Current Handshake authority, contracts, product code, diagnostics, tests, and completed packet evidence.
- Relevant field reports, postmortems, benchmarks, and compatibility discussions, with claims separated from primary evidence.

## Research completion standard

Each workstream must record sources checked, implementation patterns, reuse opportunities, rejected options, selected recommendations, risks, mitigations, and a validation plan. Unsupported recommendations remain `UNVERIFIED` and cannot drive the Stage architecture or acceptance criteria.

</topic>
