---
file_id: WP-KERNEL-012-PRODUCT-STATUS-AND-HANDOFF-20261112
file_kind: operator-handoff
updated_at: "2026-11-12T06:22:52Z"
wp_id: WP-KERNEL-012
product_commit: ca0e9adcb691128cd52eb89dece2666980cfaf69
master_spec: v02.229
---

# WP-KERNEL-012 product status and handoff

<topic id="scope-and-proof" wp="WP-KERNEL-012">

## Scope and proof

WP012 remains in progress. The published Product checkpoint is `ca0e9adcb691128cd52eb89dece2666980cfaf69` on `origin/feat/WP-KERNEL-012`. Remote `main` was `7659138874df91ae258839a7fbd72cc894459fae` at inspection and did not contain that checkpoint. CKC and WP-1 are folded into the WP012 work through scoped source imports. The pinned CKC SurrealDB and WP-1 branch tips are outside the published checkpoint's ancestry. Their imported paths and original acceptance are mapped by the reconciliation, the selected v8 handoff, and MT171–175. Original standalone scope outside the consumed imports stays with its owning packet. This handoff is a dated readout. The Master Spec, packet, original MTs, and independent evidence govern execution and acceptance.

The live WP012 directory has 205 original or successor MT JSON contracts. Their recorded lifecycle labels are 140 PASS, 40 BLOCKED, 12 FAIL, 8 PENDING, and 5 READY_FOR_VALIDATION. These counts include historical and scoped verdicts. The packet's ten required HBR matrix rows remain PENDING. Read the exact proof, inputs, and independent verdict before reusing any PASS. MT183 and MT184 establish scoped schema reconciliation and frozen Core compilation. The five MT171–175 combined acceptance rows are still blocked.

Read [the packet](packet.json), [selected Phase 2 handoff](WP-KERNEL-012-phase02-successor-handoff-v8.json), [source reconciliation](WP-KERNEL-012-phase-01-source-reconciliation-v1.json), [selected session handoff](WP-KERNEL-012-phase02-session-handoff-20261026.json), and [Master Spec entrypoint](../../spec/SPEC_CURRENT.md). Product paths below are relative to the published Product tree. Proof paths starting with `MT-212/` are relative to the discovered WP012 artifact root under `Handshake_Artifacts`.

</topic>

<topic id="product-status" wp="WP-KERNEL-012">

## Handshake (Product)

### SurrealDB and EventLedger

The app-wide architecture uses embedded SurrealDB for durable Product authority and EventLedger state. Published Core uses SurrealDB SDK 3.2.0, RocksDB for production and an isolated in-memory engine for focused tests. The typed stores cover documents, workspace and account state, Atelier/CKC, model lanes, and other domains. Core rejects legacy storage selection. DuckDB remains a rebuildable Flight Recorder diagnostic projection. ArtifactStore remains the owner of materialized artifacts.

MT136–139 record scoped storage, backend infrastructure, Atelier and schema-provenance passes. MT141, MT144–145 and MT150 record backend test conversions and focused regression proof. Revision 170 schema and INFO pins are published. The independent BASE/FINAL catalog measurement qualified the memory engine. Genuine RocksDB fresh startup and close/reopen, durable readback under the actual account, Swarm load, and the Loom/non-Loom authority failures remain open. The latest authenticated Workspace Create attempt returned 403 at the first-claim receipt query and its fixture settlement remains unknown. The generic SurrealDB error family still permits more than one explanation. Legacy traces remain in the Tauri development launcher, Docker test config and an isolated SQLx test lock. These files require owning-scope cleanup. The production storage selector and the genuine acceptance cases should decide cutover status.

Code: `src/backend/handshake_core/Cargo.toml:216`, `src/backend/handshake_core/src/storage/mod.rs:197`, `src/backend/handshake_core/src/storage/surreal/schema.rs:28`, `app/src-tauri/src/lib.rs:376`, `docker-compose.test.yml:13`. Authority and proof: [Master Spec §2.3.13.8](../../spec/master-spec-v02.229/spec-modules/02-system-architecture.md), [Master Spec product-surface storage rule](../../spec/master-spec-v02.229/spec-modules/10-product-surfaces.md), [MT139](MT-139.json), [MT140](MT-140.json), [MT142](MT-142.json), [MT153](MT-153.json), [MT154](MT-154.json), [MT173](MT-173.json), [MT210](MT-210.json), [MT211](MT-211.json).

### Focused Rust crates and Notes

Native Code and Rich Notes share one Notes worksurface and the typed storage and Loom APIs. The published branch contains code editing, tree-sitter structure, find/replace, diff/merge, rename, gutter, LSP wire, rich document and formatting, wikilinks, embeds, daily notes and journal, graph, backlinks, folder tree, search, and canvas paths. Cabinet Host binds Cabinet file trees. The published parity manifest records 32 PASS and 11 REQUIRES_SURREALDB. Those 11 cover transclusion, HTML export, draft recovery, folder colors, tag hubs, pins, breadcrumbs, Kanban moves, semantic search, faceted search and find-in-files. Preserve the original evidence and qualify the current mounted paths where required. Live LSP, CKC embeds and code/note cross-references, Swarm edits, large-document performance, FEMS writes, journal navigation and some editor host/navigation cases still have blocked or failed records. Keep the original cross-pillar workflows, shared authoring state and one durable authority.

The manifests already consume separate editor crates and multiple focused crates per feature where ownership warrants it. Examples include document model plus SurrealDB adapter, CKC template/parser/search metrics, contact sheet/OpenPose, and model-lane routing plus navigation. Core and Native retain substantial composition code. MT191/193 have standalone rich-document and formatting verdicts. Text mechanics MT189 and pinned host adoption MT190/192/194 remain blocked. Split further only along coherent build and ownership boundaries, move real callers, and record affected warm compile time and relink counts where HBR-COMPART-002 applies.

Code and evidence: `src/frontend/handshake_native/src/lib.rs:26` and `:112`, `src/frontend/handshake_native/Cargo.toml:112`, `src/frontend/handshake_native/tests/parity_manifest.json`, `src/frontend/cabinet_host/src/lib.rs`, `src/backend/handshake_core/Cargo.toml:174`. Authority and proof: [Master Spec UX module](../../spec/master-spec-v02.229/spec-modules/07-user-experience-and-development.md), [build rules](../../roles_shared/records/HANDSHAKE_BUILD_RULES.json), [MT044](MT-044.json), [MT046](MT-046.json), [MT167](MT-167.json), [MT189](MT-189.json), [MT191](MT-191.json), [MT193](MT-193.json).

### Carried CKC/PoseKit and WP-1 Multi-Model Orchestration

**CKC/PoseKit.** Published Core serves CKC media/search, sheet versions, operations and prompt feedback. The source includes ArtifactStore and lineage, OpenPose and focused CKC crates. Native mounts Atelier with inspection IDs. The original CKC packet preserves its historical technical pass and withheld merge. MT171 and the original CKC contracts still own genuine authorized intake and facial flows, PoseKit/Atelier editing, contact sheets, model lease, lineage/export, and combined-host proof. The published router mounts media, operations and sheets. Intake/facial implementations need their genuine account-bound mount and independent runtime coverage. Code: `src/backend/handshake_core/src/api/mod.rs:233`, `api/atelier_ckc_sheets.rs:66` and `api/atelier_ckc_media.rs:51` under Core. [Original CKC packet](../WP-CKC-posekit-overhaul/packet.json), [MT171](MT-171.json).

**WP-1.** Core carries coordinated dispatch, owned process lifecycle, ModelLane, typed navigation, durable routing and Swarm outbox work. The joined routing/recovery change is published at `f96d076e3`. Native exposes Operator Chat under RUN and Lane Diagnostics under LAB. Standalone Dexterity Engine Profiles passed MT195. Full native profile adoption and genuine local/nonlocal dispatch remain open. Keep the complete selected profile, current account/workspace/Locus grants, selected model, and committed launch audit bound through execution. Preserve cancellation, reclaim, restart, shutdown drain, and original WP-1 MT017's one canonical event and one correlated Flight Recorder event across replay and failure.

The published Operator Chat checkpoint prevents a rejected or cancelled replacement selection from launching the previous model and acknowledges an Argus launch only after dispatch. Both focused component cases have independent PASS evidence. The exact production account, routing, live CLI, and durable audit cases remain open. [Exact component records](WP-KERNEL-012-status-and-continuation-20261112.json).

MT172 maps 41 picotasks and has no explicit accepted picotask state in its plan. PT003 is the current non-GUI assignment. Codex `gpt-6.1-sol` is the operator-selected live routing input. Genuine authentication and runtime admission still need proof. PT039 is retired from the current sequence and PT040 retains the independent combined verdict. Code: `src/backend/handshake_core/src/swarm_orchestration/routing_execution.rs:287`, `src/backend/handshake_core/src/api/model_lane_navigation.rs:344`. [Master Spec §4.3.7.6 and §4.3.9](../../spec/master-spec-v02.229/spec-modules/04-llm-infrastructure.md), [original WP-1 packet](../WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/packet.json), [original MT017](../WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/MT-017.json), [MT172](MT-172.json), [MT195](MT-195.json), [MT196](MT-196.json).

### Flight Recorder, internal_diagnostics, Palmistry and Argus

| Surface | Published implementation | Remaining final-host proof |
| --- | --- | --- |
| Flight Recorder and EventLedger | EventLedger owns durable governed events and replay. The Flight Recorder mirror and operational projection capture correlated activity, including a scoped Argus receipt route. | Genuine authorized write/readback and replay across original workflows, including deduplication, privacy and recovered survivor-event ingestion. |
| `internal_diagnostics` | Native in-process liveness, panic, frame/resource counters and typed mechanical events feed the V2 shared-memory ring and the Diagnostics Panel. | Prove the mounted UI advances its heartbeat under backend failure, and that new GUI/GPU/lane failures remain observable and privacy bounded. |
| Palmistry | Native starts a signed standalone watcher before GUI startup. The watcher binds the actual process identity, reads the ring independently, and stores survivor records. Native has same-operation adoption controls. | Prove protected adoption of the existing watcher after login, genuine registration, signed challenge and current process generation, independent survival after a controlled freeze/crash, correlated later forwarding, and normal ACK/drain. The served router currently mounts the Argus receipt subset. The Native maintenance method has no production caller in the inspected tree. |
| Argus | The headless MCP/AccessKit route inspects, steers and captures the GUI through stable IDs, scoped actions and receipts. | Run actual before/action/after cases on the integrated build, check layout and privacy and confirm authorized durable mirrors where required. Use Argus to operate and observe the UI, then verify the relevant diagnostic or durable record independently. |

MT081–096 preserve historical passes, including real-binary freeze, crash and backend-down tests. MT096 explicitly deferred the live managed Flight Recorder round trip. MT155 authenticated screenshot execution is ready for validation. MT156 has a scoped PASS for diagnostic bundle authorization. New or changed proof must bind the actual app, watcher, run and PID/birth identity. Keep pre-login liveness and authenticated post-login adoption separate. The approved kernel-owned pre-login GPU authority covers only the login screen and shell. Protected work needs current grants, and logout retains ownership of in-flight work through actual settlement.

Code: `src/backend/handshake_core/src/flight_recorder/event_ledger.rs:27`, `src/backend/handshake_core/src/api/palmistry.rs:3627`, `src/backend/handshake_core/src/api/mod.rs:111`, `src/frontend/handshake_native/src/main.rs:507`, `src/frontend/handshake_native/src/diagnostics/palmistry_launch.rs:1077`, `src/frontend/handshake_native/src/mcp/argus.rs:142`. [Master Spec §5.8](../../spec/master-spec-v02.229/spec-modules/05-security-and-observability.md), [§6.13](../../spec/master-spec-v02.229/spec-modules/06-mechanical-integrations.md), [Argus protocol](../../roles_shared/docs/ARGUS_VISUAL_INSPECTION_PROTOCOL.md), [MT096](MT-096.json), [MT212](MT-212.json).

### Desktop GUI, Handpick and GPU ownership

The current host requests and verifies Vulkan through Kumihimo-egui and eframe/wgpu. The requested end state is direct Vulkan through canonical Kumihimo and the centralized GPU owner. Product pins Kumihimo `8a97b9ae445872e4143fad1eb92f9ad082017595`. Canonical `3b3494f898bfd6450fe1980dfd8959a1acb70e8d` has shared clipping. D8 direct presentation needs the exact validated Studio UI pass release and RF-12 typed attachment, reservation, submission and settlement interface. The Studio UI pass source is reachable at `88b5500c90b212e2c13702b3732816f19a6b42c3` and the owning lane remains READY_FOR_VALIDATION with an empty verdict. Resolve the trusted shell issuer and registered GPU capability in the owning path before protected use. Preserve budget, staging, cancellation, window/device loss and reclamation ownership.

MT212 maps 92 GUI picotasks to 302 original MT contracts. Keep shared-edge docked panes, module and feature Settings, complete Engine Profiles, keyboard/focus/IME, DPI, accessibility, multiwindow and recovery in the mounted proof. Handpick Portal's source intent has a prefilled confirmation path. Native recognizes the intent and awaits a Handshake-owned authenticated review/confirmation destination. Cabinet's latest mounted case stopped at Workspace Create, before file-tree assertions. Keep the Cabinet fixture held until its owner establishes settlement.

Code: `src/frontend/handshake_native/src/main.rs:564` and `:614`, `src/frontend/handshake_native/Cargo.toml:112`. [Master Spec §15](../../spec/master-spec-v02.229/spec-modules/15-native-desktop-gui-vulkan-gpu-ownership.md), [MT197](MT-197.json), [MT198](MT-198.json), [MT199](MT-199.json), [MT200](MT-200.json), [MT201](MT-201.json), [MT202](MT-202.json), [MT203](MT-203.json), [MT212](MT-212.json).

</topic>

<topic id="continue" wp="WP-KERNEL-012">

## Repo Governance and continuation

The source of truth stays in the existing WP/MT records. This new handoff adds navigation and operating guidance. It changes no contract, verdict, ownership, attempt budget or source pin.

### Do next

1. **Use the qualified GUI compile artifacts for the next scoped proof.** Native attempt 02 qualified and produced both selected executables after the AccessKit corrections. Cold composition Cargo emitted the integration test executable, and the capture failed after three live scans hit the 450,000-entry limit. The separately admitted warm composition capture qualified with all 915 artifact rows fresh, the exact independently held executable, complete live and final V11 evidence, and checked handback at 06:22:52 UTC. Retain the failed cold capture. No GUI test body ran. The ring and composition runtime cases still require their applicable publication, source-equivalence and admission.
2. **Run PT003 before consuming dependent mounted cases.** The Core discriminator changes only the first-claim receipt projection to a scalar while preserving the writes, filter and bindings. The cold Core compile emitted the test executable and failed qualification on one live 25-second scan deadline. The separately admitted warm acquisition qualified with all 819 artifact rows fresh and the exact independently held Core executable. The original observer verified checked handback. No test body ran. Bind the one protected fixture, exact selector and reviewed terminal decoder under separate runtime admission. Classify the real result only after independently checked rollback, source, resource and process evidence. Preserve the earlier 403 failure and held fixtures.
3. **Keep independent GUI work moving.** The rebased recorder extraction preserves the Lane Diagnostics event and existing public ring-test imports. The two changed patch sections cleared independent source review. Primary application and reconciliation of the Native and composition-test Cargo locks remain. Route protected Palmistry adoption and Handpick confirmation to their actual Core and host owners. Keep the canonical Kumihimo and Studio work distinct. Publish and independently reread the validated asset revision before a separate exact host adoption.
4. **Return to original acceptance.** Carry the genuine Workspace Create result to compatible Cabinet and routing consumers. Follow MT185–188 for registry publication, real deployment/authentication, ModelLane recovery and production shutdown. Preserve MT204–209 for MCP request/reconnect and settlement, resource admission, claim fencing, replay and Dev Command Center. Keep MT210/211 source/host schema separation and MT202 integrated GUI proof. Record evidence in the original owning rows when its verdict changes.

Current controls and result navigation: [session evidence and exact input hashes](WP-KERNEL-012-status-and-continuation-20261112.json). Qualified Native result: `MT-212/mt212-lane-diagnostics-native-dual-codegen-20261112-02/result.json`. Qualified composition warm result: `MT-212/mt212-lane-diagnostics-composition-warm-acquisition-20261112-01/result.json`. Qualified Core warm result: `MT-212/mt172-pt003-receipt-projection-core-warm-20261112-01/result.json`. Recorder extraction: `MT-212/mt212-gui-recorder-lane-diagnostics-rebase-20261112-01/binding.json`.

### Avoid

- Keep one non-GUI assignment and one shared compiler slot. Preserve actual file, lock, target and process custody. Do not create worktrees or branches, revive ticks, restart spent or stopped cases, or infer release from Cargo exit or silence. Preserve the 210 GB target cap, 205 GB live tripwire, 50 GB free-space floor and 55 GB live tripwire on the approved shared target.
- Do not bulk-delete artifacts, disturb held fixtures, or stop foreign processes without exact applicable authority. Use actual custody handback before mutating the pinned source or starting another build.
- Do not add SQL fallback, synthetic grants, fabricated audit receipts, broad Palmistry route mounts, or host copies of reusable asset logic. A real authorization or registration gap belongs to the actual owner.
- Preserve evidence levels. Historical PASS, source review, compile-only success, component tests, prototypes and screenshots carry their original scope. Final runtime, privacy, recovery, Vulkan and independent acceptance still require their own bound observations.

### Observed time sinks and proposed workflow improvements

| Observed time sink | Proposed improvement |
| --- | --- |
| Cargo finished while the MSVC telemetry uploader VCTIP kept its private Job alive. The two Native attempts drained naturally in about 14.5 and 15 minutes. Microsoft documents optional telemetry opt-out. Its documentation gives no uploader lifetime guarantee. [BuildXL MSVC source](https://github.com/microsoft/BuildXL/blob/main/Public/Sdk/Experimental/Msvc/VisualCpp/visualCpp.dsc#L507-L519), [Visual Studio telemetry settings](https://learn.microsoft.com/en-us/visualstudio/ide/visual-studio-experience-improvement-program?view=visualstudio#opt-in-or-out) | Retain the original observer and use the drain window for disjoint work. Keep batching compatible targets under one manifest as Native attempt 02 did. Composition and PT003 have separate manifests and locked graphs, with one reused Cargo target. |
| An ordered guard stopped a prior run before Cargo, and active V11 scans failed despite usable idle scans. | Preflight exact selectors, ordered guards, tool identity, source map and target under the same capture method. Reuse compatible warm artifacts and combine compatible selectors. Keep active checks and publish only qualified results. |
| Both Cabinet and PT003 spent a mounted attempt at the same Workspace Create prerequisite. | Qualify shared authenticated Workspace Create once for compatible consumers. Use a narrow discriminator before spending another dependent run. Track HTTP outcome, durable settlement and compiler handback as distinct facts. |
| Source, review and runtime evidence can land at different times across WP012, the assets and Studio. Older handoffs can retain stale labels. | Point each original acceptance row to its latest exact qualifying observation. Handoff only changed facts. Use event-driven owner notifications and fresh domain context at the real Cargo boundary. Batch stable source changes under the actual owner. |
| Large host composition targets rebuild and relink more work than a focused leaf needs. | Keep feature-owned crate boundaries, move real callers, run the narrowest proof, and measure warm compile time plus relinks on affected units before broad validation. |

</topic>
