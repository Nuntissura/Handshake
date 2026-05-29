---
file_id: SANDBOX-ENGINE-RESEARCH-BASIS-V1
file_kind: research_basis
authority: PROSE_RESEARCH_NOTE_NON_NORMATIVE
wp_id: WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1
operator_signature: ilja290520260357
authored_by: KERNEL_BUILDER-20260529-020021
updated_at: 2026-05-29T02:20:00Z
research_policy: GLOBAL-RESEARCH-001..059
host_constraint: Windows 11, Rust/Tauri desktop product
---

<topic id="purpose" summary="Why this research exists and what decision it informs">

# Native Sandbox Engine — Research Basis (WP-KERNEL-004 sandbox tier)

## Purpose
Decide which open-source sandbox/VM engine(s) Handshake should integrate **natively** (own the abstraction, plug the engine underneath — same model as `ModelRuntime` §4.6), to run untrusted AI-agent-written code, without depending on proprietary Docker Desktop. Triggered by operator instruction (2026-05-29) + signature `ilja290520260357`. This note is the [GLOBAL-RESEARCH-048] research basis recorded **before** the spec enrichment and implementation.

## Current spec state (what already exists)
`SandboxAdapter` trait already exists at Master Spec §3.5 (added v02.186) with three implementations, **all shared-kernel**:
- `WSL2PodmanAdapter` (DEFAULT) — rootless Podman in WSL2, NVIDIA GPU passthrough.
- `WindowsNativeJailAdapter` — Win32 Job Objects/AppContainer wrapping `rappct`/`codex-windows-sandbox`.
- `DockerAdapter` — compat-only, KERNEL-003 preservation.
Gap: **no strong-isolation (microVM / syscall-interception) tier.** §5.2.2 separately covers *plugin* WASM sandboxing — a different concern, out of scope here.

</topic>

<topic id="sources" summary="Sources checked per GLOBAL-RESEARCH-049">

## Sources checked (current, 2026)
GitHub repos/issues/releases, official docs, vendor + independent 2026 write-ups across three parallel research sweeps:
- MicroVM/VMM: Firecracker, Cloud Hypervisor, Kata, QEMU/WHPX, libkrun, Microsoft OpenVMM, rust-vmm.
- Container/syscall: youki (`libcontainer` crates), runc, crun, containerd, Podman, gVisor.
- Agent-sandbox frameworks (interface patterns): SWE-ReX, E2B, microsandbox, Daytona, Modal, sandbox-core.

Key source URLs: emirb.github.io/blog/microvm-2026; northflank.com (firecracker-vs-cloud-hypervisor, how-to-sandbox-ai-agents, guide-to-cloud-hypervisor, what-is-gvisor); github.com/{firecracker-microvm/firecracker, cloud-hypervisor/cloud-hypervisor, containers/libkrun, microsoft/openvmm, youki-dev/youki, google/gvisor, SWE-agent/SWE-ReX, e2b-dev/E2B, zerocore-ai/microsandbox, daytonaio/daytona, autohandai/sandbox-core}; docs.rs/cloud-hypervisor-client; qemu.org/docs/master/system/whpx.html; learn.microsoft.com Hyper-V GPU partitioning; modal.com/docs/guide/sandboxes; crates.io/crates/youki.

</topic>

<topic id="findings" summary="Decisive findings, especially the Windows-host constraint">

## Decisive findings

### The Windows-11 host constraint (the deciding factor)
Every strong-isolation engine (Firecracker, Cloud Hypervisor, gVisor, Kata, libkrun) requires **Linux + KVM**. On Windows 11 the realistic substrate is **WSL2** (a Hyper-V Linux VM with nested KVM) — which is exactly why the existing default adapter is `WSL2Podman`. "Cloud Hypervisor MSHV support" is a *Linux-root-partition-on-Hyper-V* path, **not** a Win32 user-mode host. The only thing that boots a VM **natively on Windows without WSL2** is **QEMU via the WHPX accelerator**. Microsoft's **OpenVMM** (Rust) is the only VMM that natively targets the Windows host + Hyper-V, but it is immature/unstable in 2026 — watch, do not ship.

### Engine assessment (ranked for: Rust desktop, Win11 host, embeddable, GPU-capable, strong isolation)
- **Cloud Hypervisor** — Rust (rust-vmm), VFIO GPU passthrough, typed `cloud-hypervisor-client` crate, Apache-2.0, healthy. Best practical strong-isolation fit; run in WSL2, drive over REST/socket.
- **libkrun** — Rust, purpose-built **embeddable C-API library** (the cleanest "driver under the abstraction" shape; powers microsandbox). Linux/macOS only → WSL2 on Windows; paravirtualized GPU (venus/Vulkan).
- **QEMU/WHPX** — only engine that runs natively on Win11 without WSL2. C (memory-unsafe), GPL-2.0 (embedding friction), QMP socket. Compatibility fallback.
- **Firecracker** — superb isolation/boot, but **no GPU passthrough by design** (disqualifying for in-sandbox LLMs) and standalone-binary ergonomics. Apache-2.0.
- **youki** — Rust OCI runtime shipping `libcontainer`/`libcgroups`/`oci-spec` crates → **in-process Rust embedding** for the *light* container tier; rootless, Apache-2.0, CNCF. Shared-kernel; Linux → WSL2.
- **gVisor (runsc)** — syscall-interception (stronger than containers, weaker than microVM), Go, Linux → WSL2, subprocess. Good middle tier.
- Rejected as primary: **Kata** (k8s runtime, wrong layer, not embeddable), **OpenVMM** (immature 2026), **Firecracker** (no GPU), bare **Docker Desktop** (proprietary bundle + licensing — but Docker Engine/containerd/runc are Apache-2.0 and embeddable without Desktop).

### Agent-sandbox interface patterns to adopt (steal the shape, not the runtime)
`microsandbox` (Rust, embeddable, libkrun) is the closest model to Handshake's stack; `Modal` has the cleanest typed surface; `SWE-ReX` proves the "one Runtime trait, swappable backends" lesson. Recurring patterns: (1) typed `Sandbox` handle from a builder; (2) one backend-agnostic `Runtime`/`SandboxEngine` trait; (3) `exec()` returns a process handle with streamed stdout/stderr + exit code (not a string); (4) stateful named sessions vs one-shot; (5) first-class filesystem namespace (read/write/upload/download/list/stat); (6) per-task network policy declared at create (block_network, cidr_allowlist); (7) hard `timeout` + `idle_timeout` auto-kill; (8) snapshot/restore-as-new-sandbox (maps onto Handshake's validate-then-promote); (9) discovery + ownership tracking (`from_id`/`list`/tags → ProcessOwnershipLedger §5.7); (10) explicit teardown returning exit status.

</topic>

<topic id="selected-approach" summary="The selected approach + rejected options per GLOBAL-RESEARCH-052/053">

## Selected approach
**Extend the existing `SandboxAdapter` trait (§3.5) with an isolation-TIER model and a strong-isolation microVM adapter, keyed to code trust level — do not replace the trait.**

- **Tier 1 — container (default, trusted/reviewed code):** keep `WSL2PodmanAdapter`; optionally add a youki/`libcontainer` in-process Rust path later.
- **Tier 2 — syscall-interception (untrusted-but-cheap):** `gVisor (runsc)` inside WSL2 as a drop-in OCI runtime.
- **Tier 3 — microVM (untrusted agent-written code):** `CloudHypervisorAdapter` (VFIO GPU, Rust client) inside WSL2; `libkrun` as the embeddable alternative.
- **Windows-native fallback:** QEMU/WHPX adapter (no WSL2 dependency).
- **Watch:** OpenVMM as the future Windows-native Rust microVM backend.
- **Selection policy gains a trust dimension:** trusted/reviewed → Tier 1; untrusted/agent-written → Tier 3 (microVM). Trait contract (§3.5.1) absorbs the 10 interface patterns above.

Rationale: matches the reset-brief §6.5 direction ("Docker/Podman first; gVisor/Firecracker later"), reuses the existing adapter layer + ProcessOwnershipLedger + ModelRuntime wiring, keeps everything WSL2-backed (no new host dependency beyond what's already required), and respects [GLOBAL-PORTABILITY] (repo-relative, disk-agnostic).

</topic>

<topic id="risks" summary="Risks, mitigations, validation plan per GLOBAL-RESEARCH-054..056">

## Risks + mitigations
- **WSL2 GPU passthrough to a microVM is fragile** (WSL2 owns the GPU; nested passthrough/DDA is finicky). Mitigation: keep GPU-bound local-model work in Tier 1 (WSL2+Podman, proven NVIDIA path); reserve Tier 3 microVM for code execution that does not need the GPU, until nested GPU is proven.
- **Cloud Hypervisor is driven out-of-process (socket), not in-process.** Mitigation: the `SandboxAdapter` trait already abstracts spawn/exec; wrap the socket client behind it. libkrun remains the in-process option if tighter embedding is needed.
- **QEMU GPL-2.0 + C unsafety.** Mitigation: QEMU is fallback-only and subprocess-isolated (no static linking → no GPL propagation into Handshake).
- **Scope creep / over-build.** Mitigation: ship Tier 3 as ONE new adapter (Cloud Hypervisor or libkrun) behind the existing trait; do not build all tiers at once.
- **Nested virtualization may be unavailable on some hosts.** Mitigation: capability declaration (§3.5.4) advertises tier availability; selection policy degrades to the strongest available tier and records the downgrade.

## Validation plan
- Trait + adapter unit/integration tests (spawn/exec/fs-bind/net-policy/kill/status) headless where the engine runs in WSL2; mark genuinely host-VM-dependent paths `NEEDS_EXTERNAL_RESOURCE` per the Spec-Realism Gate.
- Negative tests: escape attempts fail closed; network policy enforced; every spawn recorded in ProcessOwnershipLedger (§5.7); reclaim on teardown (CX-503D).
- Capability-declaration test: selection policy picks the correct tier per trust level and degrades correctly when a tier is unavailable.
- HBR gate: applicable SWARM/QUIET/INT rows on the sandbox WP.

</topic>
