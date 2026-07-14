# Spindle Kernel Technical Documentation

**Project name:** Spindle  
**Document type:** Technical architecture documentation  
**Primary implementation language:** Rust  
**Compatibility target:** ComfyUI-compatible workflows, APIs, and node semantics at the boundary  
**Core design principle:** ComfyUI-compatible at the edges; Rust-native inside

---

## Table of Contents

1. [Preamble: What Spindle Is Trying to Solve](#1-preamble-what-spindle-is-trying-to-solve)
2. [Native Rust Compatibility with the Host App](#2-native-rust-compatibility-with-the-host-app)
3. [Limitations of ComfyUI / CUI](#3-limitations-of-comfyui--cui)
4. [Technical Design of Spindle](#4-technical-design-of-spindle)
5. [Compatibility Strategy](#5-compatibility-strategy)
6. [Internal Graph IR](#6-internal-graph-ir)
7. [Wireless Node Design](#7-wireless-node-design)
8. [Heartbeat and Liveness](#8-heartbeat-and-liveness)
9. [Live Graph Mutation](#9-live-graph-mutation)
10. [Node Lifecycle Model](#10-node-lifecycle-model)
11. [Cache Correctness](#11-cache-correctness)
12. [Runtime Backends](#12-runtime-backends)
13. [API Surface](#13-api-surface)
14. [Scheduler and Worker Model](#14-scheduler-and-worker-model)
15. [Artifact Store](#15-artifact-store)
16. [Rust Crate Layout](#16-rust-crate-layout)
17. [Compatibility Modes](#17-compatibility-modes)
18. [Security and Isolation](#18-security-and-isolation)
19. [Licensing Considerations](#19-licensing-considerations)
20. [MVP Roadmap](#20-mvp-roadmap)
21. [Testing Strategy](#21-testing-strategy)
22. [Open Design Decisions](#22-open-design-decisions)
23. [Implementation Skeleton](#23-implementation-skeleton)
24. [Final Recommendation](#24-final-recommendation)
25. [References](#25-references)

---

# 1. Preamble: What Spindle Is Trying to Solve

Spindle is a proposed **Rust-native workflow kernel** for embedding ComfyUI-compatible graph execution into a larger Rust application without inheriting the hard technical limitations of stock ComfyUI.

The goal is not to build a cosmetic wrapper around ComfyUI. The goal is to build a kernel that can:

- Run deeply inside a Rust application.
- Expose a graph/node workflow model similar to ComfyUI.
- Import and execute ComfyUI-compatible workflows where needed.
- Solve architectural limitations that are hard to solve from a frontend-only CUI layer.
- Provide a more production-oriented execution core: heartbeat, liveness, cache correctness, event replay, worker isolation, scheduling, and native Rust node execution.

## 1.1 Problem Statement

The application is mainly built in Rust and needs a graph-based AI/workflow execution system that can interoperate with ComfyUI-style workflows. However, stock ComfyUI is a Python-first local workflow engine. It was not originally designed as a Rust-embeddable, production-grade, multi-runtime kernel.

The key tension is:

```text
Need ComfyUI compatibility
        │
        ▼
But also need Rust-native integration, deterministic execution, liveness, security, and production orchestration
```

Spindle resolves this by separating **compatibility** from **internal architecture**.

```text
ComfyUI compatibility is an adapter layer.
Spindle is the real kernel.
```

## 1.2 What Spindle Should Become

Spindle should become the workflow execution substrate of the Rust application.

It should provide:

- A canonical graph IR.
- A scheduler.
- A node runtime system.
- Native Rust nodes.
- Python/ComfyUI bridge support.
- Persistent run history.
- Artifact storage.
- Event streaming.
- Heartbeat/liveness.
- Cache fingerprinting.
- Worker management.
- Optional API compatibility with ComfyUI.

At the outer boundary, Spindle should be able to speak ComfyUI-like formats and routes. Internally, it should use its own Rust-native abstractions.

## 1.3 Design Principle

The core principle is:

```text
Spindle should be ComfyUI-compatible at the protocol/workflow boundary,
but CUI-native and Rust-native in its internal execution model.
```

This allows Spindle to remain interoperable with the ComfyUI ecosystem without being structurally constrained by ComfyUI’s current implementation.

---

# 2. Native Rust Compatibility with the Host App

The host application is mainly built in Rust. Therefore, Spindle should not behave like an external black-box Python service unless compatibility requires it.

Spindle should support three integration modes:

```text
1. In-process Rust API       → deepest app integration
2. Local HTTP/WebSocket API  → compatibility and external clients
3. Worker bridge protocols   → Python/ComfyUI, WASM, gRPC, HTTP runtimes
```

## 2.1 In-Process Rust API

The primary interface should be a Rust API exposed directly to the host application.

Example kernel object:

```rust
pub struct SpindleKernel {
    pub graph_store: Arc<dyn GraphStore>,
    pub scheduler: Arc<Scheduler>,
    pub node_registry: Arc<NodeRegistry>,
    pub artifact_store: Arc<dyn ArtifactStore>,
    pub event_bus: Arc<EventBus>,
    pub worker_registry: Arc<WorkerRegistry>,
}
```

The host app should be able to:

- Register native Rust nodes.
- Submit graph runs.
- Cancel runs.
- Subscribe to events.
- Read artifacts.
- Query worker state.
- Inspect cache state.
- Inject host-specific services.

Example:

```rust
let kernel = SpindleKernel::new(config).await?;

kernel.node_registry.register(Box::new(MyNativeRustNode::new()))?;

let run = kernel.submit_run(SpindleRunRequest {
    graph,
    mode: ExecutionMode::NativeSpindle,
    priority: Priority::Normal,
    metadata,
}).await?;

let mut events = kernel.subscribe_run_events(run.id).await?;
```

## 2.2 Local API Server

Spindle should also expose a local API server for compatibility and tool integration.

Recommended stack:

```text
tokio  → async runtime
axum   → HTTP/WebSocket API
serde  → JSON serialization
tracing → observability
```

The local server should expose:

- ComfyUI-compatible routes.
- Spindle-native routes.
- WebSocket event streams.
- Health and readiness endpoints.

## 2.3 Runtime Embedding

The host Rust application should control the lifecycle of Spindle.

The host should be able to:

- Start/stop Spindle.
- Configure worker pools.
- Enable/disable Python compatibility.
- Set artifact paths.
- Set cache policy.
- Define project/session isolation boundaries.
- Bind Spindle to the app’s auth, project model, telemetry, and persistence.

This is the key difference from stock ComfyUI:

```text
ComfyUI is a Python application with an API.
Spindle is a Rust library/kernel that may also expose an API.
```

## 2.4 Rust-Native Node ABI

Native nodes should be Rust traits, not Python class conventions.

```rust
#[async_trait::async_trait]
pub trait SpindleNode: Send + Sync {
    fn schema(&self) -> NodeSchema;

    async fn execute(
        &self,
        ctx: NodeContext,
        inputs: ValueMap,
    ) -> anyhow::Result<NodeOutput>;
}
```

This allows the app to implement nodes that directly call internal Rust services without crossing a Python or HTTP boundary.

Good candidates for native Rust nodes:

- Application-specific business logic.
- Prompt routing.
- Project metadata access.
- File indexing.
- Asset management.
- Database queries.
- Image pre/post-processing.
- Local model calls using Rust-native runtimes.
- Telemetry and audit nodes.
- Security-sensitive nodes.

## 2.5 Integration Boundary

Spindle should avoid making Python the center of the application.

Recommended runtime hierarchy:

```text
Native Rust nodes        → first-class
Spindle Graph IR         → first-class
Spindle scheduler        → first-class
Python/ComfyUI workers   → compatibility bridge
External runtimes        → optional plugin backends
```

---

# 3. Limitations of ComfyUI / CUI

This section describes the limitations Spindle is intended to solve.

Terminology:

- **ComfyUI** refers to the upstream Python-based node workflow system.
- **CUI** refers here to the ComfyUI-compatible UI/workflow layer or frontend-facing graph experience that the application wants to incorporate.
- **Spindle** is the proposed Rust-native kernel that preserves compatibility where useful but fixes architectural limitations internally.

## 3.1 Summary of Known Limitations

| Limitation | Practical Impact | Root Cause |
|---|---|---|
| No first-class true wireless nodes | Wireless sender/receiver behavior is fragile unless compiled into real links. | Explicit edge-list graph model. |
| No robust built-in heartbeat | External app cannot reliably infer liveness from idle WebSocket events. | Event-driven WebSocket rather than liveness subsystem. |
| No live graph mutation during queued execution | UI edits after queue submission do not affect the running job. | Workflow snapshot is submitted to the backend. |
| No complete universal node event stream | Hard to build reliable orchestration around every node lifecycle step. | Events are optimized for UI progress, not persistent kernel eventing. |
| Hidden/global state breaks cache correctness | Receivers may not rerun when hidden upstream values change. | Cache sees explicit inputs/widgets, not implicit dependencies. |
| No native actor/daemon node model | Always-on nodes are awkward. | Nodes are primarily execute-and-return units. |
| Limited production orchestration | Single local queue/process is not a distributed job scheduler. | Local interactive design assumptions. |
| Python custom-node security risk | Third-party nodes run arbitrary Python code. | In-process plugin model. |
| Weak Rust integration | Rust app must talk to Python process externally. | ComfyUI is Python-first. |

## 3.2 True Wireless Nodes Are Not Native

ComfyUI uses explicit links between node outputs and node inputs. This is good for ordinary directed dataflow, but it makes true wireless nodes difficult.

A true wireless relation means:

```text
Sender node publishes value to channel "prompt.main"
Receiver node subscribes to channel "prompt.main"
No explicit visual wire required
```

The problem is that if this relationship exists only as a string name or global registry, the scheduler cannot reliably see it.

Failure modes:

1. Receiver can run before sender.
2. Type validation can be bypassed.
3. Cache may reuse stale receiver output.
4. Exported workflow may not preserve semantic dependency.
5. Multiple graphs/runs may collide on the same channel name.
6. Partial invalidation becomes incorrect.

Spindle should solve this by representing wireless links as **semantic channels** that lower into scheduler-visible dependencies.

## 3.3 Heartbeat Is Not a First-Class Kernel Feature

Stock ComfyUI exposes WebSocket events for queue and execution updates. However, this does not equal a robust heartbeat system.

A production Rust application often needs:

- Kernel alive/dead state.
- Worker alive/dead state.
- Queue depth.
- Active run information.
- Last-seen timestamps.
- Health/readiness status.
- Reconnect and event replay.

ComfyUI’s WebSocket stream is useful for UI feedback but is not sufficient as the sole liveness contract for an embedded Rust application.

Spindle should include explicit heartbeat and health services.

## 3.4 Live Mutation Is Limited

ComfyUI’s execution model is based on submitting a workflow snapshot. After a job is queued, later UI edits are not part of that run.

That is predictable, but it limits advanced UX patterns such as:

- Modify not-yet-executed nodes during execution.
- Recompute downstream nodes after a graph edit.
- Cancel only affected subgraphs.
- Maintain a reactive graph session.

Spindle should support strict snapshot mode for compatibility, but also define future live-mutation policies using run generations.

## 3.5 Node Lifecycle Is Too Narrow

ComfyUI nodes are mostly synchronous or asynchronous transform nodes from inputs to outputs. This does not naturally model:

- Always-on sources.
- Stream producers.
- Stream consumers.
- Actor nodes.
- Stateful session nodes.
- Service provider nodes.

Spindle should define explicit node lifecycle kinds.

## 3.6 Cache Correctness Is Hard with Hidden Dependencies

ComfyUI caching is based on visible inputs/widgets and custom change-detection hooks. This works for many ordinary workflows, but hidden state and external side effects create problems.

Examples:

- A node reads a file whose path is unchanged but file contents changed.
- A receiver reads from a wireless global registry.
- A node calls an external service whose output changed.
- A node depends on model files, seed policy, device state, or environment state.

Spindle should require explicit cache fingerprints or explicit non-cacheable policies.

## 3.7 Python Integration Is Powerful but Not Isolated

ComfyUI’s custom node ecosystem is one of its strengths. But custom nodes are arbitrary Python code.

For a Rust application, this creates issues:

- Security boundary is weak.
- Python dependencies can conflict.
- GPU memory ownership is implicit.
- Crashes can affect the whole process.
- App state access must be carefully restricted.

Spindle should treat Python execution as a compatibility runtime, preferably isolated in worker processes or containers.

---

# 4. Technical Design of Spindle

## 4.1 Architecture Overview

Spindle should be a Rust-native kernel with compatibility adapters.

```text
┌──────────────────────────────────────────────────────────────┐
│                     Host Rust Application                     │
│                                                              │
│  product logic / UI / project state / auth / storage / etc.  │
└──────────────────────────────┬───────────────────────────────┘
                               │ in-process Rust API
                               ▼
┌──────────────────────────────────────────────────────────────┐
│                         Spindle Kernel                        │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                    API Boundary Layer                   │  │
│  │  Comfy-compatible routes + Spindle-native routes + WS   │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                     Compiler Layer                      │  │
│  │  import → validate → normalize → lower → execution plan │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                      Graph IR Layer                     │  │
│  │  explicit edges, virtual edges, channels, resources     │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                    Runtime/Scheduler                    │  │
│  │  queue, workers, cache, event log, cancellation         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                    Runtime Backends                     │  │
│  │  Native Rust | Python Comfy | WASM | gRPC | HTTP        │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## 4.2 Core Components

| Component | Responsibility |
|---|---|
| `spindle-ir` | Canonical graph, node, type, channel, and metadata model. |
| `spindle-comfy-compat` | Import/export ComfyUI workflow JSON and API prompt format. |
| `spindle-compiler` | Validate, normalize, lower wireless channels, produce execution plan. |
| `spindle-runtime` | Execute graphs, schedule nodes, cancel runs, manage state. |
| `spindle-cache` | Fingerprints, cache keys, artifact reuse. |
| `spindle-events` | Persistent event log and WebSocket event fanout. |
| `spindle-api` | HTTP/WebSocket server. |
| `spindle-nodes` | Native Rust node SDK. |
| `spindle-python-worker` | Python/ComfyUI bridge. |
| `spindle-wasm` | Optional WASM plugin runtime. |
| `spindle-app-bridge` | Host application integration layer. |

## 4.3 Execution Flow

```text
1. Receive workflow or API prompt.
2. Import into Spindle Graph IR.
3. Validate nodes, inputs, types, and links.
4. Resolve wireless channels.
5. Lower virtual edges into scheduler-visible dependencies.
6. Compute cache fingerprints.
7. Partition graph by runtime.
8. Acquire workers/resources.
9. Execute nodes/subgraphs.
10. Store artifacts.
11. Emit persistent event records.
12. Return history and artifact references.
```

---

# 5. Compatibility Strategy

Spindle should define compatibility in layers.

| Compatibility Layer | Description | Recommended Support |
|---|---|---:|
| Workflow JSON import/export | Read/write ComfyUI visual workflow files. | Yes |
| API prompt format | Accept ComfyUI API-format prompt graphs. | Yes |
| Server routes | Implement common Comfy-compatible HTTP routes. | Yes |
| WebSocket events | Emit Comfy-style events for compatible clients. | Yes |
| Node schema format | Expose `/object_info`-like node schemas. | Yes |
| Python custom-node execution | Run existing Python custom nodes. | Through sidecar/bridge only |
| Stock frontend compatibility | Allow ComfyUI frontend to talk to Spindle. | Optional / fragile |
| Full third-party node parity | Exact behavior of all node packs. | Only via Python ComfyUI delegation |

## 5.1 Boundary Compatibility Principle

```text
Import Comfy workflows.
Expose Comfy-compatible APIs.
Emit Comfy-compatible events where useful.
Delegate Python nodes where necessary.
But keep Spindle’s scheduler, IR, cache, heartbeat, and runtime model native.
```

## 5.2 Why Not Execute Arbitrary Python Nodes in Rust?

Existing ComfyUI custom nodes are Python modules/classes. They usually depend on:

- Python runtime behavior.
- PyTorch tensors.
- ComfyUI internals.
- Third-party Python packages.
- Custom model loading code.
- Side effects and global state.

A Rust kernel can emulate the schema and protocol, but it cannot natively execute arbitrary Python node packs without a Python runtime.

Therefore, Spindle should use a bridge:

```text
Spindle scheduler
   │
   └── Python/ComfyUI worker for Python-compatible subgraphs
```

---

# 6. Internal Graph IR

The Spindle internal graph IR should be richer than ComfyUI workflow JSON.

## 6.1 Graph IR Sketch

```rust
pub struct GraphIr {
    pub graph_id: GraphId,
    pub nodes: IndexMap<NodeId, NodeInstance>,
    pub edges: Vec<DataEdge>,
    pub virtual_edges: Vec<VirtualEdge>,
    pub channels: Vec<ChannelBinding>,
    pub control_edges: Vec<ControlEdge>,
    pub resource_constraints: Vec<ResourceConstraint>,
    pub metadata: GraphMetadata,
}

pub struct NodeInstance {
    pub id: NodeId,
    pub class_type: String,
    pub runtime: RuntimeKind,
    pub inputs: ValueMap,
    pub widgets: ValueMap,
    pub cache_policy: CachePolicy,
    pub lifecycle: NodeLifecycleKind,
}

pub enum RuntimeKind {
    NativeRust,
    PythonComfy,
    Wasm,
    ExternalGrpc,
    ExternalHttp,
}

pub enum NodeLifecycleKind {
    PureTransform,
    EffectfulTransform,
    SourceStream,
    Sink,
    Actor,
    Service,
}
```

## 6.2 Why the Native IR Matters

A native IR allows Spindle to model concepts that ComfyUI does not model directly:

- Wireless semantic channels.
- Long-lived actor nodes.
- Streaming sources.
- Runtime isolation policies.
- Worker affinity.
- GPU and memory constraints.
- Cache policies.
- Live graph update generations.
- Resource leases and locks.
- Persistent event-log references.

The ComfyUI adapter should import/export this IR where possible, but Spindle should not be internally constrained by ComfyUI’s visual JSON schema.

---

# 7. Wireless Node Design

## 7.1 Problem

A frontend-only wireless node system typically creates hidden dependencies. Hidden dependencies break scheduling and caching.

Bad model:

```text
Node A writes to global map["main_prompt"]
Node B reads from global map["main_prompt"]
No graph edge exists
```

Better model:

```text
Node A publishes to typed channel "main_prompt"
Node B subscribes to typed channel "main_prompt"
Compiler lowers channel into scheduler-visible dependency
```

## 7.2 Spindle Channel Binding

```rust
pub struct ChannelBinding {
    pub channel_id: ChannelId,
    pub scope: ChannelScope,
    pub value_type: TypeId,
    pub publisher: NodeOutputRef,
    pub subscribers: Vec<NodeInputRef>,
    pub cardinality: ChannelCardinality,
}

pub enum ChannelScope {
    Graph,
    Subgraph(SubgraphId),
    Run(RunId),
    Session(SessionId),
}

pub enum ChannelCardinality {
    ExactlyOne,
    OptionalOne,
    ManyOrdered,
    Latest,
}
```

## 7.3 Compilation Flow

```text
Wireless UI relation
        │
        ▼
Channel binding in Graph IR
        │
        ▼
Virtual dependency edge
        │
        ▼
Scheduler-visible execution dependency
        │
        ▼
Optional export lowering to explicit ComfyUI links
```

## 7.4 Design Rule

```text
Wireless in presentation.
Explicit in execution.
```

Spindle can support true wireless UX without hidden runtime state.

---

# 8. Heartbeat and Liveness

## 8.1 Problem

A WebSocket event stream is not enough for robust liveness.

A production Rust app needs to know:

- Is the Spindle kernel alive?
- Is the worker alive?
- Is the Python/ComfyUI sidecar alive?
- Is the current run still executing?
- Has the GPU worker stalled?
- What was the last event sequence number?
- Can a client reconnect and resume event reading?

## 8.2 Spindle Health Routes

Recommended routes:

```text
GET /healthz
GET /readyz
GET /metrics
GET /spindle/workers
GET /spindle/runs/{run_id}/events?after_seq=123
WS  /spindle/ws
```

## 8.3 Heartbeat Message

```json
{
  "type": "spindle.heartbeat",
  "seq": 18422,
  "time": "2026-07-03T12:34:56Z",
  "kernel_id": "local-gpu-0",
  "queue_depth": 2,
  "active_runs": 1,
  "active_node": "KSampler:17",
  "workers": [
    {
      "id": "gpu0-python-comfy",
      "state": "busy",
      "last_seen_ms": 317,
      "vram_used_mb": 14420
    }
  ]
}
```

## 8.4 Compatibility

Spindle should expose:

- `/ws` for Comfy-compatible clients.
- `/spindle/ws` for native clients.
- Persistent event log replay for reliability.
- Health/readiness endpoints for the host app and watchdogs.

---

# 9. Live Graph Mutation

## 9.1 Problem

ComfyUI executes queued workflow snapshots. This is stable, but it does not support advanced reactive behavior.

## 9.2 Spindle Run Generations

Use run generations to represent graph mutations during execution.

```rust
pub struct RunGeneration {
    pub run_id: RunId,
    pub generation: u64,
    pub graph_hash: Hash,
    pub changed_nodes: Vec<NodeId>,
    pub invalidated_cache_keys: Vec<CacheKey>,
}
```

## 9.3 Mutation Policies

| Policy | Behavior |
|---|---|
| Strict snapshot | Match ComfyUI behavior. Edits affect only future runs. |
| Soft live update | Apply changes to nodes not yet executed. |
| Reactive update | Cancel/recompute affected downstream region. |

## 9.4 Recommended Default

Start with strict snapshot mode. Add soft live updates and reactive recomputation after the scheduler, cache, and event log are stable.

---

# 10. Node Lifecycle Model

## 10.1 Problem

ComfyUI nodes are mostly transform nodes. Spindle should support more execution shapes.

## 10.2 Lifecycle Kinds

| Lifecycle Kind | Meaning | Example |
|---|---|---|
| `PureTransform` | Deterministic function of inputs. | Resize image, normalize prompt. |
| `EffectfulTransform` | Reads external state or performs side effects. | Load file, HTTP call. |
| `SourceStream` | Produces values over time. | Camera, socket, file watcher. |
| `Sink` | Consumes values externally. | Save image, publish event. |
| `Actor` | Stateful node with mailbox. | Stateful session, queue consumer. |
| `Service` | Long-lived resource provider. | Model server, device service. |

## 10.3 Actor Model Example

```rust
pub enum NodeLifecycleKind {
    PureTransform,
    EffectfulTransform,
    SourceStream,
    Sink,
    Actor,
    Service,
}
```

Actor/service nodes should be explicit. They should not be disguised as normal transform nodes.

---

# 11. Cache Correctness

## 11.1 Problem

Cache invalidation fails when real dependencies are hidden in global state, files, network calls, random seeds, or device state.

## 11.2 Cache Fingerprint

```rust
pub struct CacheFingerprint {
    pub node_class: String,
    pub node_version: SemVer,
    pub input_hashes: Vec<Hash>,
    pub widget_hash: Hash,
    pub model_hashes: Vec<Hash>,
    pub external_file_hashes: Vec<Hash>,
    pub runtime_hash: Hash,
    pub seed: Option<u64>,
    pub device: Option<DeviceFingerprint>,
}
```

## 11.3 Cache Policy

```rust
pub enum CachePolicy {
    Pure,
    Fingerprinted,
    AlwaysRun,
    NeverCache,
    TimeToLive(Duration),
    ExternalInvalidationKey(String),
}
```

## 11.4 Rule

Every node must declare one of the following:

1. It is pure and fully determined by its visible inputs.
2. It is effectful and declares its cache policy explicitly.
3. It is not cacheable.
4. It uses external invalidation keys or fingerprints.

---

# 12. Runtime Backends

Spindle should support multiple execution runtimes.

## 12.1 Native Rust Runtime

Native Rust nodes are first-class.

```rust
#[async_trait::async_trait]
pub trait SpindleNode: Send + Sync {
    fn schema(&self) -> NodeSchema;

    async fn execute(
        &self,
        ctx: NodeContext,
        inputs: ValueMap,
    ) -> anyhow::Result<NodeOutput>;
}
```

Best suited for:

- Workflow control.
- File IO.
- Metadata handling.
- Prompt processing.
- Routing.
- Image pre/post-processing.
- Business logic.
- Observability.
- Host app integration.

## 12.2 Python/ComfyUI Worker Bridge

Use a Python bridge for existing ComfyUI nodes.

```text
Spindle scheduler
   │
   ├─ Native Rust node island
   │
   ├─ Python/ComfyUI subgraph island
   │      └─ Stock ComfyUI or custom Python worker
   │
   └─ Native Rust post-processing island
```

Bridge options:

| Bridge | Pros | Cons |
|---|---|---|
| Stock ComfyUI sidecar over HTTP | Highest compatibility, easiest to start. | More overhead, less internal control. |
| Custom Python worker process | Better control, compact RPC possible. | More maintenance. |
| Embedded CPython via PyO3 | Tighter process integration. | GIL, dependency coupling, isolation concerns. |
| Full Rust reimplementation | Best integration. | Lowest compatibility with Python node ecosystem. |

Recommended initial path:

```text
Start with stock ComfyUI sidecar.
Partition compatible subgraphs into Python islands.
Gradually replace hot paths with native Rust nodes.
```

## 12.3 External Runtimes

Long-term runtime targets:

- WASM plugin nodes.
- gRPC plugin nodes.
- HTTP plugin nodes.
- ONNX Runtime nodes.
- Native ML nodes using Rust ML libraries.
- Hardware service nodes.

---

# 13. API Surface

## 13.1 Comfy-Compatible Routes

Implement these for compatibility:

```text
POST /prompt
GET  /queue
POST /queue
GET  /history
GET  /history/{prompt_id}
GET  /object_info
GET  /object_info/{node_class}
POST /interrupt
POST /upload/image
GET  /view
GET  /ws
```

## 13.2 Spindle-Native Routes

Add namespaced routes:

```text
POST   /spindle/compile
POST   /spindle/runs
GET    /spindle/runs/{run_id}
PATCH  /spindle/runs/{run_id}/graph
POST   /spindle/runs/{run_id}/cancel
GET    /spindle/runs/{run_id}/events
GET    /spindle/runs/{run_id}/artifacts
GET    /spindle/workers
GET    /spindle/healthz
GET    /spindle/readyz
WS     /spindle/ws
```

## 13.3 Event Log Message

```json
{
  "seq": 10291,
  "run_id": "run_abc",
  "type": "node.finished",
  "node_id": "17",
  "status": "success",
  "started_at": "2026-07-03T10:00:02Z",
  "finished_at": "2026-07-03T10:00:11Z",
  "outputs": ["artifact://run_abc/17/0"]
}
```

Core event types:

```text
run.queued
run.started
run.cancel_requested
run.cancelled
run.finished
run.failed
node.scheduled
node.started
node.progress
node.cached
node.finished
node.failed
worker.heartbeat
worker.lost
artifact.created
cache.hit
cache.miss
```

---

# 14. Scheduler and Worker Model

## 14.1 Scheduler Responsibilities

The scheduler should own:

- Graph dependency resolution.
- Wireless channel lowering.
- Type validation.
- Cache lookup.
- Worker assignment.
- GPU/resource leases.
- Cancellation.
- Retry policy.
- Event emission.
- Artifact registration.

## 14.2 Worker Descriptor

```rust
pub struct WorkerDescriptor {
    pub worker_id: WorkerId,
    pub runtime: RuntimeKind,
    pub capabilities: Vec<Capability>,
    pub resources: ResourceInventory,
    pub last_seen: Instant,
    pub state: WorkerState,
}

pub enum WorkerState {
    Starting,
    Ready,
    Busy,
    Draining,
    Lost,
    Failed,
}
```

## 14.3 Resource Constraints

```rust
pub struct ResourceConstraint {
    pub node_id: NodeId,
    pub required_runtime: Option<RuntimeKind>,
    pub min_vram_mb: Option<u64>,
    pub preferred_device: Option<DeviceId>,
    pub exclusive_resources: Vec<ResourceKey>,
}
```

---

# 15. Artifact Store

Spindle should avoid sending large binary outputs through JSON or WebSocket messages. Use artifact references.

Artifact URI examples:

```text
artifact://run_abc/node_17/output_0
artifact://run_abc/images/final.png
artifact://cache/sha256/...
```

Artifact metadata:

```rust
pub struct ArtifactMetadata {
    pub artifact_id: ArtifactId,
    pub uri: ArtifactUri,
    pub media_type: String,
    pub byte_size: u64,
    pub hash: Hash,
    pub producer_run: RunId,
    pub producer_node: NodeId,
    pub created_at: DateTime<Utc>,
}
```

Recommended storage backends:

- Local filesystem for MVP.
- SQLite metadata index.
- Optional object storage for distributed mode.
- Content-addressed storage for cacheable outputs.

---

# 16. Rust Crate Layout

Suggested repository structure:

```text
spindle/
  crates/
    spindle-ir/              # Graph IR, node schema, workflow model
    spindle-comfy-compat/    # Comfy workflow/API import/export
    spindle-compiler/        # Validation, lowering, wireless resolution
    spindle-runtime/         # Scheduler, run state, cancellation
    spindle-cache/           # Fingerprints, artifact store, cache index
    spindle-events/          # Event log, WebSocket messages, replay
    spindle-api/             # HTTP/WS server
    spindle-nodes/           # Native Rust node SDK
    spindle-python-worker/   # Python/Comfy sidecar protocol
    spindle-wasm/            # Optional WASM plugin runtime
    spindle-app-bridge/      # Integration layer for host Rust app
```

Recommended Rust stack:

| Concern | Recommended Crates / Tools |
|---|---|
| Async runtime | `tokio` |
| HTTP/WebSocket API | `axum` |
| Serialization | `serde`, `serde_json` |
| JSON schema | `schemars` |
| Stable maps | `indexmap` |
| Graph algorithms | `petgraph` or custom DAG implementation |
| Local metadata DB | `sqlx` + SQLite |
| Logging/tracing | `tracing` |
| Metrics/tracing export | `opentelemetry` |
| WASM plugins | `wasmtime` |
| Embedded Python, optional | `pyo3` |
| Error handling | `anyhow`, `thiserror` |

---

# 17. Compatibility Modes

## 17.1 Strict Comfy Mode

Only features that can round-trip to stock ComfyUI.

```text
No native wireless semantics.
No actor nodes.
No live mutation.
No Spindle-only runtime nodes.
No Spindle-only cache policies.
```

Use this mode when users must export workflows that run in stock ComfyUI.

## 17.2 Comfy-Plus Mode

Workflow remains mostly Comfy-compatible, but Spindle stores additional metadata.

```json
{
  "version": 1,
  "nodes": [],
  "links": [],
  "extra": {
    "spindle": {
      "wireless_channels": [],
      "cache_policy": {},
      "runtime_hints": {}
    }
  }
}
```

Stock ComfyUI may open the workflow, but Spindle-specific behavior may be ignored unless lowered to explicit Comfy-compatible links/nodes.

## 17.3 Native Spindle Mode

Full kernel functionality:

```text
Wireless channels.
Actor nodes.
Streaming sources.
Live mutation.
Distributed workers.
Persistent event log.
Native Rust nodes.
Python islands.
WASM/gRPC plugin runtimes.
```

Use this mode for first-class workflows inside the Rust host application.

---

# 18. Security and Isolation

## 18.1 Threat Model

Spindle may execute:

- Native Rust code written by the app team.
- Third-party Rust plugins.
- Python ComfyUI custom nodes.
- WASM plugins.
- External worker calls.

Python custom nodes should be treated as untrusted or semi-trusted unless fully audited.

## 18.2 Recommended Controls

| Risk | Control |
|---|---|
| Arbitrary Python execution | Run Python nodes in sidecar process/container. |
| File-system access | Per-worker sandbox paths. |
| Network access | Disable or restrict network for untrusted workers. |
| Resource exhaustion | Worker memory/GPU limits and timeouts. |
| Malicious plugin | Signed plugin registry or allowlist. |
| Cache poisoning | Content-addressed outputs and producer metadata. |
| Cross-project data leakage | Project-scoped artifact stores and channel scopes. |

---

# 19. Licensing Considerations

ComfyUI and the official ComfyUI frontend are GPL-3.0 licensed. A clean Rust kernel that implements protocol/workflow compatibility is architecturally different from embedding or modifying GPL code, but licensing should be reviewed before distribution.

Practical risk profile:

| Approach | Relative License/Integration Risk |
|---|---|
| Clean-room Rust kernel with compatible JSON/API | Lower |
| Running stock ComfyUI as separate user-installed process | Often cleaner |
| Bundling stock or modified ComfyUI directly | Higher |
| Forking ComfyUI frontend into proprietary app | Higher |
| Writing custom UI and protocol adapter | Cleaner |

This is an engineering assessment, not legal advice.

---

# 20. MVP Roadmap

## Phase 1 — Compatibility Shell

Build:

```text
Comfy API parser
/prompt
/queue
/history
/object_info
/ws
artifact storage
event log
stock ComfyUI sidecar runner
```

Exit criterion:

```text
The Rust app can submit an existing ComfyUI API workflow and receive outputs/events through Spindle.
```

## Phase 2 — Native Rust Scheduler

Build:

```text
Spindle Graph IR
topological scheduler
node registry
native Rust node trait
cache fingerprints
persistent event stream
heartbeat
```

Exit criterion:

```text
Simple workflows run fully inside Spindle without ComfyUI.
```

## Phase 3 — Wireless + Compiler

Build:

```text
semantic channels
wireless publisher/subscriber nodes
virtual edge lowering
strict type validation
Comfy export lowering
```

Exit criterion:

```text
Wireless-looking graphs execute deterministically and can export to strict Comfy mode where possible.
```

## Phase 4 — Python Island Partitioning

Build:

```text
graph partitioner
Python worker protocol
artifact exchange
subgraph execution
worker liveness
retry/cancel handling
```

Exit criterion:

```text
Mixed Rust + ComfyUI workflows execute under one Spindle scheduler.
```

## Phase 5 — Production Features

Build:

```text
multi-worker scheduling
GPU/resource leases
streaming nodes
actor nodes
live mutation
WASM/gRPC plugin runtime
metrics/tracing
```

Exit criterion:

```text
Spindle is no longer a ComfyUI wrapper; it is the app’s native workflow engine with Comfy compatibility.
```

---

# 21. Testing Strategy

## 21.1 Compatibility Tests

- Import known ComfyUI workflow JSON files.
- Convert visual workflow JSON to API prompt format.
- Submit API-format prompts through `/prompt`.
- Emit expected `/ws` compatibility events.
- Match `/object_info` schema expectations for native nodes.

## 21.2 Scheduler Tests

- Topological execution order.
- Virtual wireless dependency lowering.
- Cache hit/miss behavior.
- Partial invalidation.
- Cancellation.
- Worker failure/retry.

## 21.3 Runtime Tests

- Native Rust node execution.
- Python sidecar execution.
- Mixed Rust/Python graph partitioning.
- Artifact transfer between runtimes.
- Large image/model output handling.

## 21.4 Production Tests

- Worker heartbeat timeout.
- Event replay after WebSocket reconnect.
- Queue backpressure.
- GPU resource lock contention.
- Concurrent runs.
- Crash recovery from persisted event log.

---

# 22. Open Design Decisions

These should be resolved before implementation hardening:

1. Should Spindle use SQLite only, or support pluggable metadata storage from the beginning?
2. Should artifact storage be content-addressed in MVP, or added after initial execution works?
3. Should Python integration start with stock ComfyUI HTTP sidecar or a custom Python worker protocol?
4. Should the stock ComfyUI frontend be supported, or should the app use a custom UI immediately?
5. What level of workflow export compatibility is required: strict, best-effort, or Spindle-only?
6. Should WASM plugin support be part of MVP or deferred?
7. Should live graph mutation be strict snapshot only until version 2?

Recommended MVP choices:

```text
SQLite metadata.
Local filesystem artifact store.
Stock ComfyUI sidecar first.
Custom UI or minimal compatibility UI.
Strict Comfy import/export plus Spindle-native metadata.
Defer WASM.
Strict snapshot execution first.
```

---

# 23. Implementation Skeleton

## 23.1 Minimal Kernel Object

```rust
pub struct SpindleKernel {
    pub graph_store: Arc<dyn GraphStore>,
    pub scheduler: Arc<Scheduler>,
    pub node_registry: Arc<NodeRegistry>,
    pub artifact_store: Arc<dyn ArtifactStore>,
    pub event_bus: Arc<EventBus>,
    pub worker_registry: Arc<WorkerRegistry>,
}
```

## 23.2 Minimal Run Request

```rust
pub struct SpindleRunRequest {
    pub graph: GraphIr,
    pub mode: ExecutionMode,
    pub priority: Priority,
    pub metadata: RunMetadata,
}

pub enum ExecutionMode {
    StrictComfy,
    ComfyPlus,
    NativeSpindle,
}
```

## 23.3 Minimal Execution Flow

```text
1. Receive workflow or API prompt.
2. Import into Spindle Graph IR.
3. Validate node schemas and links.
4. Resolve wireless channels.
5. Lower virtual edges into execution dependencies.
6. Compute cache fingerprints.
7. Partition graph by runtime.
8. Acquire workers/resources.
9. Execute nodes/subgraphs.
10. Store artifacts.
11. Emit event log records.
12. Return history/artifact references.
```

---

# 24. Final Recommendation

Build **Spindle** as a native Rust workflow engine with ComfyUI adapters.

Do not build it as a ComfyUI fork. Do not make arbitrary Python node execution the center of the architecture. Treat Python/ComfyUI compatibility as one runtime backend under the control of the Spindle scheduler.

The durable design is:

```text
Import Comfy workflows.
Expose Comfy-compatible APIs.
Delegate Python nodes when required.
Execute native Rust nodes directly.
Represent wireless, heartbeat, cache, lifecycle, and workers as first-class Rust kernel concepts.
```

This preserves interoperability while avoiding the root causes of ComfyUI’s current hard limitations.

---

# 25. References

- ComfyUI GitHub repository: https://github.com/comfy-org/comfyui
- ComfyUI frontend GitHub repository: https://github.com/Comfy-Org/ComfyUI_frontend
- ComfyUI workflow JSON specification: https://docs.comfy.org/specs/workflow_json
- ComfyUI workflow API format: https://docs.comfy.org/development/api-development/workflow-api-format
- ComfyUI server routes: https://docs.comfy.org/development/comfyui-server/comms_routes
- ComfyUI server communication overview: https://docs.comfy.org/development/comfyui-server/comms_overview
- ComfyUI WebSocket messages: https://docs.comfy.org/development/comfyui-server/comms_messages
- ComfyUI custom-node lifecycle: https://docs.comfy.org/custom-nodes/backend/lifecycle
- ComfyUI custom-node server overview: https://docs.comfy.org/custom-nodes/backend/server_overview
- aiohttp WebSocketResponse reference: https://docs.aiohttp.org/en/stable/web_reference.html
