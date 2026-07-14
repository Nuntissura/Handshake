---
title: "Assessment: Porting Box3D to Rust for Handshake"
project: "Handshake"
document_type: "Technical Architecture Assessment"
version: "1.0"
date: "2026-07-12"
status: "Initial assessment"
---

# Assessment: Porting Box3D to Rust for Handshake

## 1. Executive Assessment

Porting Box3D to Rust could be strategically valuable for Handshake, provided it is positioned correctly.

Box3D should not be treated as the complete simulation foundation for a full-feature-parity Marvelous Designer and DAZ3D replacement. It is fundamentally a rigid-body physics and collision engine. Its strongest role inside Handshake would be as the native rigid-body, collision-query, spatial-indexing, and physical-scene substrate shared by the figure, garment, animation, and environment modules.

A Rust-native Box3D-derived engine could offer Handshake:

- Direct integration with the application's scheduler and runtime.
- Native ownership and lifetime safety.
- Deterministic simulation and replay.
- Structured diagnostics for operators and AI agents.
- A common rigid-body and spatial-query layer across creative modules.
- Reduced dependence on a foreign-language runtime boundary.
- Greater control over profiling, serialization, checkpoints, and simulation state.

The main architectural conclusion is:

> Build a Handshake-owned Rust port of Box3D as a rigid-body and collision engine, while keeping cloth, hair, soft tissue, skinning, morphing, and other deformable systems as separate native solvers.

---

## 2. Where Box3D Fits in Handshake

### 2.1 Strong Fits

| Handshake subsystem | Suitability |
|---|---:|
| Rigid props and scene objects | Excellent |
| Character ragdolls | Strong |
| Mechanical and articulated joints | Strong |
| Character collision proxies | Strong |
| Environment and furniture collision | Excellent |
| Ray casts and shape casts | Excellent |
| Selection and spatial queries | Strong |
| Character-controller collision | Strong |
| Physics-assisted posing | Useful |
| Deterministic playback | Highly useful |
| Simulation diagnostics | Highly useful |
| Shared or server-authoritative simulation | Strong |
| Cloth-to-rigid collision | Useful as one half of the coupling |
| Cloth self-collision | Poor fit |
| Fabric deformation | Not provided |
| Soft tissue and muscle simulation | Not provided |
| Hair-strand simulation | Not provided |
| Morph targets and skinning | Unrelated |

### 2.2 Recommended Role

Box3D should be described internally as:

> The rigid-body and spatial-collision substrate shared by Handshake's figure, garment, environment, and animation systems.

It should not be described as:

> The physics engine that replaces Marvelous Designer.

That would create a false architectural assumption because cloth simulation requires a different class of solver.

---

## 3. Benefits of a Native Rust Port

## 3.1 Direct Integration with the Handshake Scheduler

Box3D already supports host-provided task scheduling. A Rust port would allow simulation work to become native scheduled work instead of passing through C callbacks.

Potential integration:

```text
Handshake scheduler
├── Model execution
├── Rendering preparation
├── Asset processing
├── Cloth simulation
├── Rigid-body simulation
└── Geometry processing
```

This enables:

- Explicit task dependencies.
- Cancellation and interruption.
- Project-level resource budgets.
- Deterministic task ordering when required.
- Native profiling spans.
- Cooperative scheduling.
- Better crash and stall diagnostics.
- Integration with Handshake's external monitoring systems.

This is one of the strongest reasons to port Box3D instead of relying indefinitely on FFI.

## 3.2 Shared Native Scene Data

A foreign-function interface often requires:

- Transform conversion.
- Vector and quaternion conversion.
- Mirrored identifiers.
- Foreign allocation ownership.
- Callback translation.
- Synchronization between C-owned and Rust-owned structures.

A native Rust port would let Handshake define a narrow and deterministic scene-to-physics boundary.

Example:

```rust
struct PhysicsBodyLink {
    entity: EntityId,
    body: BodyId,
    transform_channel: TransformChannelId,
}
```

The physics engine should not own the entire Handshake scene graph. Instead, it should expose stable identifiers and explicit synchronization channels.

## 3.3 Deterministic Creative Workflows

Deterministic simulation is unusually important for a creative-authoring system.

Potential uses include:

- Replaying a garment drop from the same initial state.
- Reconstructing physics-assisted poses.
- Comparing solver versions.
- Reproducing AI-generated edits.
- Recording the physical operations that produced a scene.
- Debugging divergence between local and cloud agents.
- Deterministic regression tests.
- Timeline scrubbing through simulation checkpoints.
- Forensic reconstruction after a crash or project corruption.

Handshake should treat replayability and traceability as first-class product features, not merely test infrastructure.

## 3.4 Better Extensibility

A native implementation can expose internal data that a generic public API may hide.

Useful Handshake-specific extensions could include:

- Contact graph inspection.
- Solver-island visualization.
- Per-constraint diagnostics.
- Batch scene queries.
- Stable instrumentation hooks.
- Custom memory arenas.
- Simulation diffing.
- Solver-state snapshots.
- Constraint metadata for authoring tools.
- Agent-readable diagnostic events.
- Physics reasoning summaries.

Example agent-facing diagnostic output:

```text
BODY chair_004
STATUS unstable
CAUSE center_of_mass_outside_support_polygon
PRIMARY_CONTACT left_front_leg
CORRECTION rotate +1.8 degrees around local Z
CONFIDENCE 0.94
```

This kind of structured output would make physics understandable and actionable for AI co-authors.

## 3.5 Rust Safety at the Application Boundary

A Rust API can encode restrictions that are only documented in C.

Potential guarantees include:

- Body IDs cannot be confused with shape IDs.
- Destroyed objects cannot be accessed safely.
- Read-only contact callbacks cannot mutate the world.
- References cannot outlive their simulation step.
- Non-thread-safe context cannot be passed into worker tasks.
- Simulation mutation cannot occur during parallel solver work.

Example:

```rust
world.step(dt, |step| {
    step.inspect_contacts(|contact| {
        // Read-only contact access.
    });
});
```

The goal is not to eliminate all `unsafe` code. The goal is to confine it to tightly audited implementation boundaries.

---

## 4. What Box3D Does Not Solve

## 4.1 Marvelous Designer Feature Parity

A Marvelous Designer-class module requires a dedicated deformable-body and garment solver.

Required capabilities include:

- Triangular cloth meshes.
- Stretch constraints.
- Shear constraints.
- Anisotropic warp and weft behavior.
- Bending resistance.
- Sewing constraints.
- Pressure and inflatable garments.
- Layered cloth.
- Cloth-to-body collision.
- Cloth-to-cloth collision.
- Self-collision.
- Fabric friction.
- Pinning and attachments.
- Adaptive or controlled remeshing.
- Pattern-space to 3D-space correspondence.
- Thin-shell collision handling.
- Stable offline settling.
- Interactive preview simulation.
- Measurable fabric-property presets.

Box3D can contribute:

- Rigid collision proxies.
- Environmental collision.
- Convex collision routines.
- Broad-phase acceleration.
- Spatial queries.
- Rigid accessories such as buttons, buckles, and props.

It cannot directly provide the cloth solver.

## 4.2 DAZ3D Feature Parity

Box3D can assist with:

- Ragdolls.
- Joint constraints.
- Prop collisions.
- Physics-assisted posing.
- Dynamic accessories.
- Rigid environment interaction.

It does not provide:

- Skeleton evaluation.
- Linear or dual-quaternion skinning.
- Morph targets.
- Joint corrective morphs.
- Pose-space deformation.
- Inverse kinematics.
- Facial rigs.
- Animation curves.
- Autofitting.
- Soft tissue.
- Hair simulation.
- Material systems.
- Rendering.
- Asset dependency management.

The DAZ3D replacement should therefore treat rigid physics as one service within a larger figure and animation architecture.

---

## 5. Main Risks

## 5.1 Upstream Instability

Box3D is very young and should be treated as an evolving upstream project.

Likely risks:

- API changes.
- Internal data-layout changes.
- Solver changes.
- Undiscovered correctness bugs.
- Unstable performance characteristics.
- Replay or serialization changes.
- New joints or shape types.
- Divergence between the Rust port and upstream development.

Handshake should assume ownership of a downstream engine rather than relying on a permanently synchronized translation.

## 5.2 Porting Risk

A literal line-by-line C-to-Rust rewrite would preserve many C assumptions:

- Raw pointer relationships.
- Manual lifetime rules.
- Aliasing assumptions.
- Global mutation patterns.
- Callback-driven control flow.
- C-style error handling.
- Manual allocation behavior.

The result could be Rust syntax around a C architecture without gaining the full advantages of Rust.

## 5.3 Determinism Risk

Floating-point determinism can be disrupted by:

- Fused multiply-add differences.
- Compiler optimization changes.
- SIMD reordering.
- Parallel task ordering.
- Platform-specific math behavior.
- Different allocator behavior.
- Different iteration orders in collections.

Determinism must be tested and designed explicitly.

## 5.4 Scope Expansion

The largest program-management risk is allowing the Box3D port to absorb cloth, hair, soft tissue, posing, animation, or scene-graph responsibilities.

The rigid-body engine should remain a bounded subsystem.

---

## 6. Recommended Architecture

```text
handshake-simulation
│
├── handshake-physics-api
│   ├── Stable Handshake-facing interfaces
│   ├── Scene, body, shape, and joint identifiers
│   └── Diagnostic and replay event schemas
│
├── box3d-reference-sys
│   ├── Original C Box3D
│   ├── Minimal FFI bindings
│   └── Reference-only backend
│
├── box3d-rs
│   ├── Foundation
│   ├── Math
│   ├── Geometry
│   ├── Collision
│   ├── Broad phase
│   ├── Dynamics
│   ├── Constraints
│   ├── Solver
│   ├── Continuous collision detection
│   ├── Scheduler integration
│   └── Diagnostics
│
├── handshake-deform
│   ├── Cloth
│   ├── Soft tissue
│   ├── Hair
│   └── Deformable collision
│
└── handshake-simulation-coupling
    ├── Rigid-to-cloth coupling
    ├── Skeleton-to-rigid coupling
    ├── Skeleton-to-cloth coupling
    └── Scene-to-simulation synchronization
```

The C implementation should remain available temporarily as an executable reference during the Rust port.

---

## 7. API Boundary Recommendation

Separate the implementation into two conceptual layers.

## 7.1 Compatibility Core

The compatibility core preserves Box3D behavior:

- Algorithms.
- Constants.
- Calculation ordering.
- Identifier semantics.
- Solver phases.
- Broad-phase behavior.
- Contact generation.
- Floating-point choices.
- Test scenarios.

## 7.2 Native Handshake Facade

The public Handshake API should be idiomatic Rust and remain stable even when the internal Box3D-derived implementation changes.

Example:

```rust
pub trait RigidWorld {
    fn create_body(&mut self, definition: BodyDefinition) -> BodyId;

    fn remove_body(
        &mut self,
        id: BodyId,
    ) -> Result<(), InvalidBody>;

    fn step(
        &mut self,
        request: StepRequest,
    ) -> StepReport;

    fn cast_shape(
        &self,
        query: ShapeCast,
    ) -> QueryResults;
}
```

This prevents the evolving upstream Box3D API from becoming Handshake's permanent product contract.

---

## 8. Recommended Porting Plan

## Phase 0 — Freeze a Reference Version

Start from a fixed upstream commit.

Record:

- Commit SHA.
- Compiler version.
- CMake flags.
- Floating-point flags.
- SIMD mode.
- Worker count.
- Test seeds.
- Target platforms.
- Sample output hashes.

Never port directly against a moving `main` branch.

## Phase 1 — Build the Reference Backend

Create a minimal reference crate:

```text
Rust application
    ↓
Safe temporary facade
    ↓
box3d-reference-sys
    ↓
Unmodified Box3D C implementation
```

Use large-grained FFI calls:

- Create world.
- Create body.
- Add shape.
- Step world.
- Fetch events.
- Execute query.
- Destroy object.

FFI overhead is unlikely to be material at this granularity.

## Phase 2 — Port Foundation and Identifiers

Port:

- Scalar types.
- Vectors and quaternions.
- Transforms.
- Generational handles.
- Dynamic arrays.
- Bitsets.
- Memory arenas.
- Object pools.
- Assertions.
- Validation.
- Deterministic hashing.

Example:

```rust
#[repr(transparent)]
pub struct BodyId(GenerationalId);

#[repr(transparent)]
pub struct ShapeId(GenerationalId);

#[repr(transparent)]
pub struct JointId(GenerationalId);
```

Do not expose raw array indices as durable public identifiers.

## Phase 3 — Port Geometry and Queries

Port these before rigid-body dynamics:

- AABBs.
- Convex hull construction.
- Closest-point routines.
- Ray casting.
- Shape casting.
- Overlap tests.
- Dynamic trees.
- Mesh queries.
- Height-field queries.
- Contact manifold generation.

This creates immediately useful Handshake functionality before the full solver is finished.

## Phase 4 — Port World and Broad Phase

Port:

- Worlds.
- Bodies.
- Shapes.
- Contacts.
- Collision filtering.
- Sensors.
- Sleeping.
- Island generation.
- Event generation.
- Material interaction.

## Phase 5 — Port Constraints and Solver

Port:

- Contact constraints.
- Revolute joints.
- Prismatic joints.
- Spherical joints.
- Distance joints.
- Weld joints.
- Motor joints.
- Wheel joints.
- Constraint graph coloring.
- Warm starting.
- Sub-stepping.
- Restitution.
- Friction.
- Gyroscopic behavior.

## Phase 6 — Port Continuous Collision Detection

Continuous collision detection should be treated as a separate high-risk phase.

Preserve known limitations explicitly in the Handshake-facing API instead of implying guarantees the engine does not provide.

## Phase 7 — Add Parallelism

Recommended order:

1. Establish deterministic single-threaded parity.
2. Add parallel broad phase.
3. Add parallel narrow phase.
4. Add parallel island work.
5. Add constraint graph coloring.
6. Add SIMD contact solving.
7. Integrate the internal scheduler.
8. Integrate the Handshake scheduler.

Do not simultaneously change the algorithm, SIMD layout, and parallel execution model.

## Phase 8 — Native Optimization

Only after behavioral parity:

- Structure-of-arrays redesign.
- Cache-line alignment.
- SIMD abstraction.
- Specialized arenas.
- Batch-query APIs.
- ECS integration.
- Zero-copy transform extraction.
- Wider scheduler integration.
- GPU experiments where justified.

---

## 9. Differential Validation

Every ported subsystem should be tested against the C reference implementation.

```text
Input scene
├── C Box3D reference
└── Rust Box3D port

Compare per step:
├── Body transforms
├── Linear and angular velocities
├── Sleeping state
├── Contact pairs
├── Manifold points
├── Impulses
├── Joint state
├── Island membership
├── Query results
└── Event order
```

## 9.1 Exact Comparison

Use when deterministic equivalence is required:

- Bitwise float comparison.
- Event ordering.
- Body ordering.
- Contact ordering.
- Replay hashes.

## 9.2 Tolerant Comparison

Use for optimized or reordered implementations:

- Position epsilon.
- Rotation-angle epsilon.
- Velocity epsilon.
- Contact-set equivalence.
- Energy bounds.
- Long-running drift thresholds.

## 9.3 Behavioral Comparison

Use where exact equality is unrealistic:

- Stack remains stable.
- Ragdoll reaches an equivalent rest pose.
- Objects do not tunnel.
- Joints stay within limits.
- Replay remains visually equivalent.
- Energy does not grow unexpectedly.

---

## 10. Handshake-Specific Test Requirements

## 10.1 Figure Tests

- Full humanoid ragdoll.
- Rapid pose-to-ragdoll transition.
- Ragdoll-to-animation recovery.
- Extreme joint limits.
- Large character-scale differences.
- Layered collision proxies.
- Hands contacting deforming clothing.

## 10.2 Garment-Coupling Tests

- Cloth settling on a static body.
- Cloth settling on an animated kinematic body.
- Character motion during cloth simulation.
- Rigid buttons and buckles attached to cloth.
- Zippers and fasteners.
- Rigid accessories between cloth layers.
- High-friction and low-friction fabrics.
- Collider changes caused by character morphs.

## 10.3 Creative-Authoring Tests

- Pause and resume.
- Timeline rewind.
- Save and reload.
- Undo and redo.
- Branch simulation from a checkpoint.
- Deterministic agent replay.
- Collaborative project reconciliation.
- Material changes without rebuilding unrelated state.
- Collider editing while simulation is paused.

---

## 11. Box3D Versus Rapier

Rapier should not automatically replace the Box3D plan, but it should be used as a comparison point.

| Property | Box3D | Rapier |
|---|---|---|
| Primary language | C | Rust |
| Rust integration | Requires FFI or port | Native |
| Maturity | Very early | More established |
| Solver lineage | Box2D-derived design | Dimforge ecosystem |
| Determinism | Major design goal | Supported with configuration limits |
| SIMD and multithreading | Core focus | Supported |
| Recording and replay | Explicitly emphasized | Application-managed |
| Porting burden | High | None |
| Handshake ownership | Requires fork or port | Native fork possible |
| Strategic differentiation | Potentially high | Lower |

Rapier should be used as:

- A Rust API reference.
- A benchmark competitor.
- A temporary alternative backend.
- A comparison for memory layout and ownership patterns.
- A way to verify whether the Box3D port delivers real advantages.

---

## 12. Suggested Repository Structure

```text
crates/
├── handshake_physics_api/
├── handshake_physics_types/
├── box3d_reference_sys/
├── box3d_reference_safe/
├── box3d_rs_foundation/
├── box3d_rs_math/
├── box3d_rs_geometry/
├── box3d_rs_collision/
├── box3d_rs_broadphase/
├── box3d_rs_dynamics/
├── box3d_rs_solver/
├── box3d_rs_ccd/
├── box3d_rs_scheduler/
├── box3d_rs_diagnostics/
├── handshake_deform/
├── handshake_cloth/
├── handshake_hair/
├── handshake_softbody/
└── handshake_simulation_coupling/
```

A workspace split at this granularity allows isolated testing and makes it easier to identify divergence.

---

## 13. Additional Suggestions

### 13.1 Keep the Reference Implementation Permanently Available

Even after the Rust port is complete, retain a reference-backend build option for:

- Regression testing.
- Behavioral comparison.
- Upstream tracking.
- Reproduction of old project behavior.
- Migration validation.

### 13.2 Version Simulation Behavior

Creative projects may depend on old solver behavior. Store a simulation-engine version in project metadata.

Example:

```yaml
simulation:
  rigid_backend: box3d-rs
  behavior_version: 1
  source_revision: e9f6f1d
  determinism_profile: strict
```

This prevents newer solver versions from silently changing old projects.

### 13.3 Separate Authoring State from Solver State

Do not serialize raw internal solver memory as the only project representation.

Store:

- Authoring-level bodies.
- Shapes.
- Materials.
- Constraints.
- Initial conditions.
- Simulation settings.
- Optional checkpoints.

The solver state should be rebuildable from canonical authoring data.

### 13.4 Add a Simulation Event Ledger

Handshake should record meaningful events such as:

- Body created or removed.
- Collider changed.
- Constraint added or modified.
- Simulation stepped.
- Deterministic replay started.
- Solver version changed.
- Collision instability detected.
- Checkpoint created.
- State restored.

This supports debugging, undo/redo, agent collaboration, and forensic analysis.

### 13.5 Add Diagnostic Modes

Recommended modes:

- `fast`: optimized interactive preview.
- `stable`: higher sub-step counts and conservative settings.
- `deterministic`: fixed scheduling and reproducible output.
- `diagnostic`: extensive validation and event tracing.
- `offline_quality`: slower convergence for final simulation.

### 13.6 Keep Cross-Solver Coupling Explicit

Rigid, cloth, hair, and soft-body solvers should exchange data through explicit coupling stages.

Example:

```text
1. Evaluate animation and morphs
2. Update kinematic collision proxies
3. Step rigid-body world
4. Export rigid transforms and contacts
5. Step cloth and deformable solvers
6. Resolve rigid-deformable coupling
7. Publish final scene transforms
8. Record diagnostics and checkpoints
```

Avoid hidden callbacks between solvers.

---

## 14. Final Recommendation

Porting Box3D to Rust is justified for Handshake if the goal is long-term ownership of a modern rigid-body and collision subsystem.

The benefit is not that Rust automatically makes Box3D faster.

The benefit is that Handshake gains control over:

- Scheduling.
- Determinism.
- Replay.
- Diagnostics.
- Memory ownership.
- Scene synchronization.
- Checkpointing.
- Agent-readable simulation state.
- Cross-module integration.
- Long-term engine evolution.

The recommended final description is:

> A Handshake-owned, Rust-native, deterministic, data-oriented rigid-body and collision engine derived from Box3D, continuously validated against the upstream C implementation and integrated with Handshake's scheduler, scene graph, diagnostics, replay system, and deformable solvers.

The cloth, hair, soft-tissue, figure-deformation, animation, and rendering systems should be built beside this engine rather than folded into it.

---

## 15. Decision Summary

**Recommendation:** Proceed with a staged Rust port.

**Immediate next step:** Build a pinned C reference backend and a stable Handshake-facing physics API before translating the internals.

**Primary architectural boundary:** Box3D-derived Rust engine handles rigid bodies and collision. Dedicated native solvers handle cloth and other deformable systems.

**Primary risk:** Upstream instability and uncontrolled scope expansion.

**Primary mitigation:** Pin a reference revision, maintain differential tests, version simulation behavior, and keep solver responsibilities strictly separated.
