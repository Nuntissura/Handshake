---
file_id: cloth-engine-15-animation
topic_id: T-ANIMATION
title: "Animation, Keyframe Timeline, and Dynamics"
status: draft
depends_on:
  - T-CLOTH-SOLVER
  - T-FABRIC-MODELS
  - T-RENDER-VIEWPORT
  - T-KERNEL-INTEGRATION
summary: "Keyframe timeline authority model, keyframeable physical/fabric properties, wind/pose/avatar-motion keyframing, MTN/FBX/glTF animation import, animation-range export, CRDT of a timeline, and model-first animation API — closes MOAT-4 and Group 6."
sources: 22
updated_at: "2026-06-17"
---

## [T-ANIMATION] Animation, Keyframe Timeline, and Dynamics

### [T-ANIMATION.overview] Scope and MD Feature Coverage

This topic covers the full animation and dynamics stack for the Tailor creative module, closing
**MOAT-4 (keyframeable physical properties during simulation)** and
**Group 6 (Animation and Dynamics)** from `[T-MD-FEATURES.group-6-animation-dynamics]`.

Marvelous Designer's animation system evolved from a record-and-playback model to a full
timeline with keyframeable avatar pose, wind, and — crucially — fabric physics (since 2025.0/2025.2).
The Handshake-native equivalent must cover:

| MD Feature | Version | Tailor Design Target |
|---|---|---|
| Keyframe timeline UI | 2025.0 | `GarmentAnimationDraftV1` JSONB in Postgres; CRDT-tracked |
| Keyframeable fabric properties (Pressure, Solidify, Shrinkage Weft/Warp, Tack Strength, Trim Weight) | 2025.2 | `MaterialKeyframe[]` in the animation draft; per-substep `GpuSimParams` upload |
| Wind keyframing (position, angle, strength) | 2025.0 | `WindKeyframe[]`; GPU UBO upload per frame |
| Avatar pose keyframing (joints) | 2025.0 | `PoseKeyframe[]`; capsule proxy buffer update per frame |
| Tack constraint animation (stitching/unstitching) | 2025.2 | `TackKeyframe.compliance` track; per-substep upload |
| Animation timeline markers | 2026.0 | `AnimationMarker[]` in the draft JSONB |
| Animation recording (record-and-replay) | legacy | Covered by simulation run pipeline + frame capture |
| MTN animation import | legacy/2025.0 | Transcoded to `AvatarPoseSequenceV1` on import |
| FBX joint animation import / auto-convert to motion | 2025.0 | `fbxcel-dom` parse → `AvatarPoseSequenceV1` |
| Animation range export (FBX/glTF) | 2026.0 | Frame-range filter on export pipeline (see `[T-RENDER-VIEWPORT.export-handoff]`) |
| FBX auto-key on empty frames | 2026.0 | Upstream export pass; not solver concern |
| Morph animation (Alembic blend shapes) | 2026.0 extended | Blend-shape buffer uploaded as avatar pose override |

**MOAT-4 framing:** Per-substep keyframeable material parameters are absent from every
OSS GPU cloth solver reviewed in `[T-CLOTH-SOLVER]` and `[T-FABRIC-MODELS]`. Implementing
them on the Tailor solver already landed in `[T-CLOTH-SOLVER.gpu-architecture.data-layout]`
as `GpuSimParams` fields (`pressure_target`, `solidify_blend`, `shrink_u`, `shrink_v`) and in
`[T-FABRIC-MODELS.rust-design]` as `MaterialKeyframe`. This topic designs the **authority
layer** that sits above the solver: how keyframe data is authored, stored, versioned, interpolated,
fed to the solver, and how a CRDT-layer handles a collaborative timeline.

---

### [T-ANIMATION.keyframe-data-model] Keyframe Data Model and Interpolation

#### [T-ANIMATION.keyframe-data-model.tracks] Track Types

A Tailor animation is a set of **typed keyframe tracks**, each targeting a specific animatable
property. The track types map directly to the MD feature set:

| Track type | Target property | Interpolation | Per-substep upload? |
|---|---|---|---|
| `MaterialPressureTrack` | `GpuSimParams.pressure_target` | LINEAR / STEP | Yes |
| `MaterialSolidifyTrack` | `GpuSimParams.solidify_blend` | LINEAR | Yes |
| `MaterialShrinkWeftTrack` | `GpuSimParams.shrink_u` | LINEAR | Yes |
| `MaterialShrinkWarpTrack` | `GpuSimParams.shrink_v` | LINEAR | Yes |
| `TackComplianceTrack` | per-tack `compliance` in `GpuSeamConstraint` | LINEAR / STEP | Yes |
| `WindStrengthTrack` | `GpuSimParams.wind` magnitude | LINEAR | Yes |
| `WindDirectionTrack` | `GpuSimParams.wind` direction vec3 | LINEAR (slerp) | Yes |
| `WindTurbulenceTrack` | turbulence seed / scale factor | LINEAR | Yes |
| `AvatarPoseTrack` | capsule proxy positions + orientations | LINEAR / CUBICSPLINE | Per-frame (capsule buffer upload) |
| `AvatarBlendShapeTrack` | blend shape weights for avatar morph | LINEAR | Per-frame |
| `MarkerTrack` | named timeline markers (no interpolation) | STEP | No |

The solver crate (`tailor-solver`) sees only the **resolved parameter value** at each substep,
not the track structure. Track evaluation is done in `handshake_core::tailor::animation`
before each solver substep.

#### [T-ANIMATION.keyframe-data-model.keyframe-types] Rust Keyframe Types

```rust
// src/tailor/animation/track.rs

/// A single typed keyframe: time + value for one animatable property.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframe<T> {
    /// Time in seconds from animation start (t = 0).
    pub time_s: f32,
    /// The property value at this keyframe.
    pub value: T,
    /// Interpolation to use from this keyframe to the NEXT keyframe.
    pub interpolation: KeyframeInterpolation,
    /// Cubic spline in-tangent (only valid when interpolation = CubicSpline).
    pub in_tangent: Option<T>,
    /// Cubic spline out-tangent (only valid when interpolation = CubicSpline).
    pub out_tangent: Option<T>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum KeyframeInterpolation {
    /// Linear lerp between keyframes (default for material and wind tracks).
    Linear,
    /// No interpolation; hold the previous value until the next keyframe.
    Step,
    /// Cubic Hermite spline using in/out tangents (for avatar pose).
    CubicSpline,
}

/// A keyframe track: ordered sequence of keyframes for one property.
/// T must implement Lerpable for Linear/CubicSpline interpolation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyframeTrack<T: Lerpable> {
    pub track_id: String,          // stable ID, e.g. "mat_pressure" or "wind_strength"
    pub keyframes: Vec<Keyframe<T>>,
    pub default_value: T,          // value when track has no keyframes or before t=0
}

impl<T: Lerpable> KeyframeTrack<T> {
    /// Evaluate the track at time `t_s`, returning the interpolated value.
    pub fn sample(&self, t_s: f32) -> T {
        if self.keyframes.is_empty() { return self.default_value.clone(); }
        let idx = self.keyframes.partition_point(|k| k.time_s <= t_s);
        if idx == 0 { return self.keyframes[0].value.clone(); }
        if idx >= self.keyframes.len() { return self.keyframes.last().unwrap().value.clone(); }
        let k0 = &self.keyframes[idx - 1];
        let k1 = &self.keyframes[idx];
        let dt = k1.time_s - k0.time_s;
        let u = if dt < 1e-8 { 1.0 } else { (t_s - k0.time_s) / dt };
        match k0.interpolation {
            KeyframeInterpolation::Step => k0.value.clone(),
            KeyframeInterpolation::Linear => T::lerp(&k0.value, &k1.value, u),
            KeyframeInterpolation::CubicSpline => {
                // Cubic Hermite: same formula as glTF CUBICSPLINE
                // p(u) = (2u³-3u²+1)p0 + (u³-2u²+u)*dt*m0 + (-2u³+3u²)*p1 + (u³-u²)*dt*m1
                let m0 = k0.out_tangent.as_ref().unwrap_or(&k0.value);
                let m1 = k1.in_tangent.as_ref().unwrap_or(&k1.value);
                T::cubic_hermite(&k0.value, m0, &k1.value, m1, u, dt)
            }
        }
    }
}

/// Lerpable is satisfied by f32, glam::Vec3, glam::Quat, and any newtype around them.
pub trait Lerpable: Clone + std::fmt::Debug + serde::Serialize + for<'de> serde::Deserialize<'de> {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;
    fn cubic_hermite(p0: &Self, m0: &Self, p1: &Self, m1: &Self, t: f32, dt: f32) -> Self;
}

impl Lerpable for f32 {
    fn lerp(a: &f32, b: &f32, t: f32) -> f32 { a + (b - a) * t }
    fn cubic_hermite(p0: &f32, m0: &f32, p1: &f32, m1: &f32, t: f32, dt: f32) -> f32 {
        let t2 = t * t; let t3 = t2 * t;
        (2.0*t3 - 3.0*t2 + 1.0)*p0
        + (t3 - 2.0*t2 + t)*dt*m0
        + (-2.0*t3 + 3.0*t2)*p1
        + (t3 - t2)*dt*m1
    }
}

impl Lerpable for glam::Vec3 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self { a.lerp(*b, t) }
    fn cubic_hermite(p0: &Self, m0: &Self, p1: &Self, m1: &Self, t: f32, dt: f32) -> Self {
        // component-wise scalar cubic_hermite
        glam::Vec3::new(
            f32::cubic_hermite(&p0.x, &m0.x, &p1.x, &m1.x, t, dt),
            f32::cubic_hermite(&p0.y, &m0.y, &p1.y, &m1.y, t, dt),
            f32::cubic_hermite(&p0.z, &m0.z, &p1.z, &m1.z, t, dt),
        )
    }
}
```

The `keyframe` crate (github.com/HannesMann/keyframe, docs.rs/keyframe) and `spanda`
(aarambhdevhub.medium.com) were evaluated as OSS candidates. `keyframe` supports
`AnimationSequence` and `keyframes![]` macro but lacks cubic spline tangents needed for
avatar pose quality; `spanda` adds `KeyframeTrack<T>` and GPU batch animation (v0.9.0) but
targets Bevy/WASM contexts and carries unwanted dependencies. The Tailor track
implementation is custom (80 lines of Rust) and carries no mandatory crate dependency,
keeping `tailor-solver` clean.

---

### [T-ANIMATION.draft-authority] Animation Authority Model: `GarmentAnimationDraftV1`

#### [T-ANIMATION.draft-authority.jsonb-design] JSONB Authority Schema

The animation authority lives as a `JSONB` column on the `tailor_garments` row — it is NOT a
separate first-class table. The rationale: an animation belongs to a specific garment draft.
Separating it into its own table would split a single operator concern across two promotion gates.
The design approach is: **animation is versioned inside the garment draft**, not an independent
entity.

```sql
-- Additions to tailor_garments migration (column added via ALTER TABLE in a new migration):
ALTER TABLE tailor_garments ADD COLUMN IF NOT EXISTS
    animation_json JSONB;   -- NULL = no animation; populated when operator adds timeline
```

The `GarmentAnimationDraftV1` Rust struct serializes into this column:

```rust
// src/tailor/animation/draft.rs

/// Top-level animation draft: all keyframe tracks for one garment's animation.
/// Serializes to the tailor_garments.animation_json JSONB column.
/// Schema ID: "hsk.cloth.garment_animation_draft@1"
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GarmentAnimationDraftV1 {
    pub schema_version: String,            // "hsk.cloth.garment_animation_draft@1"
    pub garment_id: String,
    pub fps: f32,                          // simulation frames per second (default 30.0)
    pub total_frames: u32,                 // total animation length in frames
    /// Keyframeable material property tracks (MOAT-4 core).
    pub material_tracks: MaterialAnimationTracks,
    /// Wind keyframe tracks.
    pub wind_tracks: WindAnimationTracks,
    /// Avatar pose keyframe tracks (one PoseKeyframe per bone per keyframe).
    pub pose_tracks: Vec<AvatarBonePoseTrack>,
    /// Avatar blend-shape tracks (for morph-target avatars, e.g. Alembic blend shapes).
    pub blend_shape_tracks: Vec<BlendShapeTrack>,
    /// Named timeline markers (2026.0 feature).
    pub markers: Vec<AnimationMarker>,
    /// Active frame range for export (2026.0: export specific range).
    pub export_range: Option<(u32, u32)>,  // (start_frame, end_frame) inclusive
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MaterialAnimationTracks {
    pub pressure:     KeyframeTrack<f32>,   // GpuSimParams.pressure_target
    pub solidify:     KeyframeTrack<f32>,   // GpuSimParams.solidify_blend
    pub shrink_weft:  KeyframeTrack<f32>,   // GpuSimParams.shrink_u
    pub shrink_warp:  KeyframeTrack<f32>,   // GpuSimParams.shrink_v
    /// Per-tack compliance tracks; keyed by tack_id.
    pub tack_compliance: std::collections::HashMap<String, KeyframeTrack<f32>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindAnimationTracks {
    pub strength:    KeyframeTrack<f32>,        // wind magnitude (N/m²)
    pub direction:   KeyframeTrack<glam::Vec3>, // unit direction vector
    pub turbulence:  KeyframeTrack<f32>,        // turbulence scale 0-1
    /// Wind source position (for positioned wind, matching MD wind actor).
    pub position:    Option<KeyframeTrack<glam::Vec3>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AvatarBonePoseTrack {
    pub bone_id:    String,
    pub translation: KeyframeTrack<glam::Vec3>,
    pub rotation:    KeyframeTrack<glam::Quat>,
    pub scale:       Option<KeyframeTrack<glam::Vec3>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlendShapeTrack {
    pub blend_shape_name: String,
    pub weight: KeyframeTrack<f32>,   // 0.0 = no blend, 1.0 = full blend
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimationMarker {
    pub marker_id: String,
    pub frame:     u32,
    pub label:     String,
    pub color:     Option<[f32; 3]>,   // RGB 0-1 for timeline display
}
```

**Why JSONB not EventLedger events for individual keyframes:** An animation timeline is a
compact, authoritatively versioned document — not a stream of business events. Writing one
EventLedger event per keyframe drag would produce unbounded event stream pollution with zero
audit value. Instead:

- The **whole `GarmentAnimationDraftV1` document** is what gets versioned.
- Mutations to the document emit a single `TailorAnimationDraftUpdated` event whose payload
  is the new schema version + garment_id + content_hash.
- The actual track data lives in the `animation_json` JSONB column on `tailor_garments`.
- Fine-grained collaborative edits to individual keyframe values go through the CRDT layer
  (see `[T-ANIMATION.crdt-timeline]`).

This follows the same "document is authority, events are receipts" pattern used for garment
panel geometry in `[T-KERNEL-INTEGRATION.postgres-authority]`.

#### [T-ANIMATION.draft-authority.eventledger] EventLedger Events for Animation

New `KernelEventType` variants added by the Tailor animation subsystem:

```rust
// Additions to KernelEventType enum in kernel/mod.rs:

/// Operator or model authored the first animation draft for a garment.
TailorAnimationDraftCreated,

/// Animation draft was updated (any keyframe, track, or marker change that
/// promoted from CRDT snapshot to a new authority JSONB version).
TailorAnimationDraftUpdated,

/// Animation sandbox run was requested (simulate with animation playback).
TailorAnimationSimRunRequested,

/// Animation simulation run completed (final frame-sequence artifact bundle).
TailorAnimationSimRunCompleted,

/// Animation simulation run failed (solver error or policy denial).
TailorAnimationSimRunRejected,

/// Animation draft was promoted to authority (passes validation gate).
TailorAnimationDraftPromoted,
```

`TailorAnimationDraftUpdated` payload:

```json
{
  "garment_id": "GAR-...",
  "animation_schema": "hsk.cloth.garment_animation_draft@1",
  "content_hash": "<sha256 of canonical JSON of GarmentAnimationDraftV1>",
  "fps": 30.0,
  "total_frames": 150,
  "tracks_summary": {
    "material_track_count": 4,
    "wind_track_count": 2,
    "pose_bone_count": 24,
    "blend_shape_count": 0,
    "marker_count": 3
  },
  "crdt_seq": 42
}
```

---

### [T-ANIMATION.crdt-timeline] CRDT Layer for the Animation Timeline

#### [T-ANIMATION.crdt-timeline.mapping] How the Timeline Maps to the CRDT Document

The same CRDT document that tracks garment panel geometry (`crdt_document_id =
"CRDT-GAR-{garment_id}"`) also covers the animation draft. The animation is a **nested
sub-map** inside the CRDT document:

```
CRDT document: "CRDT-GAR-{garment_id}"
  /panels/...           <- pattern panel geometry (existing)
  /seams/...            <- seam definitions (existing)
  /animation/
    /fps                <- f32 scalar
    /total_frames       <- u32 scalar
    /material_tracks/
      /pressure/keyframes/[{time_s, value, interp}, ...]
      /solidify/keyframes/[...]
      /shrink_weft/keyframes/[...]
      /shrink_warp/keyframes/[...]
    /wind_tracks/
      /strength/keyframes/[...]
      /direction/keyframes/[...]
      /turbulence/keyframes/[...]
    /pose_tracks/
      /{bone_id}/
        /translation/keyframes/[...]
        /rotation/keyframes/[...]
    /markers/[{marker_id, frame, label}, ...]
    /export_range       <- [start, end] or null
```

This mapping uses the `yjs_bridge` path (`push_yjs_update()`) for real-time multi-user editing.
Each operator or model editing the timeline generates a `YjsUpdateEnvelopeV1` with a diff over
the CRDT `/animation/` sub-tree.

#### [T-ANIMATION.crdt-timeline.conflict-resolution] Conflict Resolution on Timeline

| Concurrent edit scenario | CRDT resolution strategy |
|---|---|
| Two operators add keyframes to different tracks | No conflict: tracks are independent CRDT maps |
| Two operators add keyframes at different times on the same track | No conflict: keyframe list is a CRDT ordered list; both are inserted |
| Two operators edit the same keyframe's value simultaneously | Conflict: `KnowledgeCrdtConflictDetected` event; operator picks winner |
| Model proposes a keyframe edit; operator edits the same keyframe | `AiEditProposalRequestV1` state machine; operator approves or rejects model proposal first |
| Operator changes fps or total_frames while model animates | fp/total_frames are CRDT scalar fields; last-write-wins; a `TailorAnimationDraftUpdated` event fires on snapshot |

**Snapshot cadence:** A snapshot of the animation CRDT document is promoted to the Postgres
`animation_json` column on:
1. Operator explicit "save checkpoint" action.
2. Any simulation run request (the simulation must read from the snapshotted authority).
3. Model `AiEditProposalRequestV1` approval.

Between snapshots, the CRDT update stream in `kernel_crdt_updates` is the authoritative
running state; the `animation_json` JSONB column is the last promoted snapshot.

---

### [T-ANIMATION.simulation-loop] Animation-Driven Simulation Loop

#### [T-ANIMATION.simulation-loop.per-frame-loop] Per-Frame Simulation Architecture

During an animated simulation run, the solver must advance one frame at a time, sampling the
keyframe tracks before each frame and uploading updated parameters to the GPU:

```rust
// src/tailor/animation/sim_runner.rs

pub struct AnimatedSimRunner {
    solver:    Arc<dyn ClothSolver>,
    animation: GarmentAnimationDraftV1,
    avatar_sequence: AvatarPoseSequenceV1,   // pre-loaded pose cache (see §import)
}

impl AnimatedSimRunner {
    pub async fn run_animation(
        &mut self,
        n_substeps: u32,
        n_iters: u32,
        gravity: glam::Vec3,
    ) -> Result<AnimatedSolverResult, TailorError> {
        let dt_frame = 1.0 / self.animation.fps;
        let mut frames: Vec<GarmentFrame> = Vec::with_capacity(self.animation.total_frames as usize);

        for frame_idx in 0..self.animation.total_frames {
            let t_s = frame_idx as f32 * dt_frame;

            // 1. Sample all keyframe tracks at this frame time
            let material_params = MaterialFrameParams {
                solidify_blend:  self.animation.material_tracks.solidify.sample(t_s),
                pressure_target: self.animation.material_tracks.pressure.sample(t_s),
                shrink_u:        self.animation.material_tracks.shrink_weft.sample(t_s),
                shrink_v:        self.animation.material_tracks.shrink_warp.sample(t_s),
                tack_compliance: 0.0, // placeholder; per-tack tracks resolved separately
            };
            let wind_dir  = self.animation.wind_tracks.direction.sample(t_s);
            let wind_str  = self.animation.wind_tracks.strength.sample(t_s);
            let wind_turb = self.animation.wind_tracks.turbulence.sample(t_s);

            // 2. Update solver material params (uploads GpuSimParams UBO to GPU)
            self.solver.update_params(material_params);
            self.solver.update_wind(wind_dir * wind_str, wind_turb);

            // 3. Update avatar capsule proxy positions for this frame
            if let Some(pose) = self.avatar_sequence.frame(frame_idx) {
                self.solver.update_body_proxies(pose).await?;
            }

            // 4. Advance one simulation frame (substeps inside solver)
            let sim_params = SimRunParams {
                n_substeps,
                n_iters,
                dt_frame,
                mode: SimMode::Animation,
                seed: frame_idx as u64,   // deterministic per frame
            };
            let frame_result = self.solver.simulate_frame(sim_params).await?;
            frames.push(GarmentFrame::from_solver_result(frame_result, frame_idx));
        }

        Ok(AnimatedSolverResult { frames, garment_id: self.animation.garment_id.clone() })
    }
}
```

**Key design decisions:**

1. **`simulate_frame()` not `simulate(n_frames)`**: The `ClothSolver` trait needs a
   `simulate_frame()` method in addition to the batch `simulate(n_frames)` method already
   in `[T-CLOTH-SOLVER.crate-design.trait-boundary]`. The per-frame variant keeps the
   solver's GPU state (particle positions, velocities) across frames, so the cloth maintains
   physical continuity across the animation. The `handshake_core` tailor module calls
   `simulate_frame()` in a loop, injecting updated parameters between frames.

2. **Determinism per frame index**: Setting `seed = frame_idx` for wind turbulence Perlin
   noise means the turbulence is deterministic when the simulation is re-run with the same
   animation data — required for the promotion/validation hash gate.

3. **Pre-stabilization pass for fast-moving avatar**: The capsule proxy update
   (`update_body_proxies`) must run before the substep loop for the current frame, not after
   (confirmed by XPBD cloth-with-animation literature: kinematic objects moving inside the
   PBD loop produce unstable behavior). The solver implements a one-pass constraint projection
   against the NEW capsule positions before beginning substeps.

#### [T-ANIMATION.simulation-loop.gpu-wind] Wind Force on GPU: WGSL Wind Model

The predict shader (`predict.wgsl`) from `[T-CLOTH-SOLVER.gpu-architecture.shaders]` already
carries `params.wind` as a vec3. For animated wind with turbulence, the implementation adds
Perlin-noise-based turbulence sampled per particle:

```wgsl
// Additions to predict.wgsl for animated wind with turbulence:
// params.wind          = base wind vector (from WindStrengthTrack + WindDirectionTrack)
// params.wind_turb     = turbulence scale [0-1]
// params.wind_time_seed = deterministic time seed = frame_idx as f32

// Simplified 3D hash-based noise (no texture required, GPU-portable):
fn hash_noise3(p: vec3<f32>) -> f32 {
    var n = dot(p, vec3(127.1, 311.7, 74.7));
    return fract(sin(n) * 43758.5453);
}

fn smooth_noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (vec3(3.0) - 2.0 * f);   // smoothstep
    return mix(
        mix(mix(hash_noise3(i), hash_noise3(i + vec3(1,0,0)), u.x),
            mix(hash_noise3(i + vec3(0,1,0)), hash_noise3(i + vec3(1,1,0)), u.x), u.y),
        mix(mix(hash_noise3(i + vec3(0,0,1)), hash_noise3(i + vec3(1,0,1)), u.x),
            mix(hash_noise3(i + vec3(0,1,1)), hash_noise3(i + vec3(1,1,1)), u.x), u.y),
        u.z
    );
}

// In predict shader main():
let turb_pos = p.position_pred.xyz * 2.3 + vec3(params.wind_time_seed * 0.17);
let turb_factor = smooth_noise3(turb_pos) * 2.0 - 1.0;   // [-1, 1]
let wind_actual = params.wind * (1.0 + params.wind_turb * turb_factor);
let f_ext = vec3(params.gravity) + wind_actual * (1.0 - p.inv_mass);
p.velocity += f_ext * params.dt_sub;
```

This is a resolution-independent Perlin-style noise computed inline in the predict pass
with no texture sampling — the particle's world-space position serves as the noise coordinate,
giving spatially coherent turbulence that varies smoothly across the cloth surface.

**Per-frame `GpuSimParams` upload cost:** 64 bytes per frame update (the `GpuSimParams`
struct per `[T-CLOTH-SOLVER.gpu-architecture.data-layout]`). At 30 fps this is 1920 bytes/sec
— completely negligible GPU bandwidth.

#### [T-ANIMATION.simulation-loop.capsule-update] Avatar Capsule Proxy Per-Frame Update

The avatar body proxy is a set of capsules/spheres stored in a GPU storage buffer
(see `[T-COLLISION]`). For animated simulation, the host CPU must upload updated capsule
positions for the current frame before the substep loop:

```rust
// In ClothSolver trait (addition to trait in tailor-solver crate):
/// Upload updated avatar body proxy positions for the current frame.
/// Called once per frame before simulate_frame(), outside substep loop.
async fn update_body_proxies(&mut self, pose: &AvatarPoseSample) -> Result<(), ClothSolverError>;

/// Simulate exactly one frame (n_substeps internally). Retains particle state between calls.
/// Must be called in sequence: update_body_proxies, then simulate_frame.
async fn simulate_frame(
    &mut self,
    params: SimRunParams,
) -> Result<SolverResult, ClothSolverError>;
```

`AvatarPoseSample` is a per-frame snapshot of capsule world-space positions and orientations,
pre-computed from the `AvatarBonePoseTrack` keyframe tracks by the `AnimatedSimRunner`.
The CPU computes joint FK (forward kinematics) from the pose track, applies capsule
offsets for each joint to get capsule world-space transforms, and uploads as a compact
buffer of `(center_a: [f32;3], center_b: [f32;3], radius: f32)` per capsule — matching
the format already used by the body collision shader in `[T-CLOTH-SOLVER]`.

---

### [T-ANIMATION.import] Animation Import: MTN, FBX, glTF

#### [T-ANIMATION.import.mtn] MTN Import

Marvelous Designer's `*.mtn` format is a binary format carrying avatar joint animation
(position + rotation keyframes per joint per frame). The format is proprietary and not
publicly documented. **MD's own workflow** for using MTN with external animation:

1. Import FBX animation into MD → MD auto-converts to MTN via "Auto Convert to Motion".
2. MTN is MD-internal and is not consumed outside MD.

**Handshake approach:** Do not target MTN directly. Instead, target the upstream source
(FBX or glTF with skeletal animation) and convert to `AvatarPoseSequenceV1`. This avoids
reverse-engineering a proprietary binary format and is more interoperable.

If MTN import is specifically requested (e.g., a user has existing MTN files), the
`fbxcel-dom` parser can read the FBX that MTN was derived from; a file-format bridge
(FFI to MD's own SDK, if licensed) would be needed for pure MTN. **Defer MTN import**
until there is a concrete operator demand and a licensed SDK path.

#### [T-ANIMATION.import.fbx] FBX Joint Animation Import

FBX is the primary avatar animation source. The `fbxcel-dom` crate
(github.com/lo48576/fbxcel-dom, crates.io v0.0.6) is the only available pure-Rust FBX
parser. Status as of June 2026: read-only; FBX 7.4 and 7.5 supported; object type API is
experimental and changes frequently. It is **not production-stable** but is the only
pure-Rust option.

```toml
# tailor-solver is NOT the right place for FBX import — it belongs in handshake_core::tailor.
# In handshake_core/Cargo.toml:
[dependencies]
fbxcel-dom = "0.0.6"   # experimental; pin exact version
fbxcel     = "0.7.0"   # required by fbxcel-dom
```

**Import pipeline** (`src/tailor/animation/import/fbx.rs`):

```rust
use fbxcel_dom::v7400::{Document, object::{animation, model}};

pub fn import_fbx_animation(
    reader: impl std::io::Read + std::io::Seek,
) -> Result<AvatarPoseSequenceV1, TailorError> {
    let parser = fbxcel::pull_parser::any::from_seekable_reader(reader)?;
    let (doc, _warnings) = Document::from_parser(parser)?;

    let fps = extract_fps(&doc).unwrap_or(30.0);
    let anim_stack = doc.objects()
        .filter(|o| o.class() == "AnimStack")
        .next()
        .ok_or(TailorError::NoAnimation)?;

    // Extract per-bone keyframes from AnimCurveNode objects
    let mut bone_tracks: std::collections::HashMap<String, AvatarBonePoseTrack> = Default::default();
    for curve_node in anim_stack.animation_curve_nodes(&doc) {
        let bone_name = curve_node.model_name(&doc)?;
        let curves = curve_node.curves(&doc);
        let (tx, ty, tz) = (curves.tx?, curves.ty?, curves.tz?);
        let (rx, ry, rz) = (curves.rx?, curves.ry?, curves.rz?);
        bone_tracks.insert(bone_name.to_string(), build_pose_track(bone_name, tx, ty, tz, rx, ry, rz));
    }

    Ok(AvatarPoseSequenceV1 {
        fps,
        total_frames: (anim_stack.local_stop_s(&doc) * fps) as u32,
        bone_tracks: bone_tracks.into_values().collect(),
    })
}
```

**Limitation:** `fbxcel-dom`'s animation API (`AnimCurveNode`, `AnimCurve`) is underdocumented.
The most reliable fallback for complex FBX animation is to evaluate the full node transform
at each frame time using `GlobalSettings.CustomFrameRate` and the `AnimCurve` key time
array, rather than relying on high-level API helpers. Document this as a known complexity
in the Tailor animation import recipe.

**Alternative for production:** If `fbxcel-dom` proves too fragile, the pipeline can
accept glTF-format animation (Blender exports rigged characters with animation tracks to
glTF reliably) and skip FBX as an internal format. The Tailor UI recipe would instruct
operators to re-export from Blender/Maya as glTF before import.

#### [T-ANIMATION.import.gltf] glTF Skeletal Animation Import

The `gltf` crate v1.4.1 (read-only) provides glTF animation samplers. This is the
preferred import path for animated avatars because glTF is well-supported in every major
DCC tool and `gltf` is production-stable:

```rust
// src/tailor/animation/import/gltf.rs
use gltf::{animation::{Interpolation, Property}, Gltf};

pub fn import_gltf_animation(
    bytes: &[u8],
) -> Result<AvatarPoseSequenceV1, TailorError> {
    let (gltf, buffers, _images) = gltf::import_slice(bytes)?;
    let animation = gltf.animations().next().ok_or(TailorError::NoAnimation)?;
    let fps = 30.0f32;  // glTF stores time in seconds; choose display fps

    let mut bone_tracks: Vec<AvatarBonePoseTrack> = Vec::new();

    for channel in animation.channels() {
        let target_node = channel.target().node();
        let bone_name = target_node.name().unwrap_or("bone").to_string();
        let reader = channel.reader(|buf| Some(&buffers[buf.index()]));
        let sampler = channel.sampler();
        let interp = match sampler.interpolation() {
            Interpolation::Linear => KeyframeInterpolation::Linear,
            Interpolation::Step   => KeyframeInterpolation::Step,
            Interpolation::CubicSpline => KeyframeInterpolation::CubicSpline,
        };

        match channel.target().property() {
            Property::Translation => {
                let outputs: Vec<[f32; 3]> = reader.read_outputs()?.into_f32().collect();
                let inputs: Vec<f32>       = reader.read_inputs()?.collect();
                // build KeyframeTrack<glam::Vec3> from inputs (times) + outputs (values)
                // (CubicSpline: glTF stores [in_tangent, value, out_tangent] per keyframe)
                let track = build_vec3_track(&bone_name, &inputs, &outputs, interp);
                // add to bone_tracks or merge with existing bone entry
            }
            Property::Rotation => { /* similar for glam::Quat */ }
            Property::Scale    => { /* optional */ }
            Property::MorphTargetWeights => {
                // Blend-shape weights for blend-shape avatar tracks
            }
        }
    }

    let total_frames = (animation.channels()
        .flat_map(|c| c.reader(|buf| Some(&buffers[buf.index()])).read_inputs())
        .flatten()
        .fold(0.0f32, f32::max) * fps) as u32;

    Ok(AvatarPoseSequenceV1 { fps, total_frames, bone_tracks })
}
```

glTF animation CUBICSPLINE stores `[in_tangent, value, out_tangent]` interleaved per keyframe
(documented in the glTF 2.0 spec). The import code must de-interleave these into the Tailor
`Keyframe<T>` struct's `in_tangent` / `out_tangent` fields.

---

### [T-ANIMATION.export] Animation-Range Export

The 2026.0 animation range export feature (`[T-MD-FEATURES.group-6-animation-dynamics]`)
limits the exported frame sequence to the `GarmentAnimationDraftV1.export_range` tuple.
This is a filter applied at export time in the existing export pipeline from
`[T-RENDER-VIEWPORT.export-handoff]`:

```rust
// src/tailor/export/animation_range.rs

pub fn apply_export_range(
    frames: &[GarmentFrame],
    range: Option<(u32, u32)>,
) -> &[GarmentFrame] {
    match range {
        None => frames,
        Some((start, end)) => {
            let start = (start as usize).min(frames.len());
            let end   = ((end + 1) as usize).min(frames.len());
            &frames[start..end]
        }
    }
}
```

The `GarmentExportCompleted` EventLedger event payload already includes `"frame_range"`
(see `[T-RENDER-VIEWPORT.export-handoff]`), which carries the effective exported range.

**FBX auto-key on empty frames (2026.0):** When exporting joint animation in FBX format,
gaps between keyframes should have keys baked at the target frame rate to avoid
software-specific interpolation drift. The export pipeline adds this as a baking pass
before writing the FBX: for each frame in the export range, evaluate all joint transforms
and write an explicit keyframe — even if the value matches the linear interpolation.
This is implemented in `src/tailor/export/fbx_key_baker.rs` (post-MVP).

---

### [T-ANIMATION.model-first-api] Model-First Animation API

#### [T-ANIMATION.model-first-api.tools] MCP Tool Definitions

The Tailor animation system exposes four LLM-callable MCP tools:

**Tool: `tailor_animation_draft_create`**
```json
{
  "name": "tailor_animation_draft_create",
  "description": "Create or replace the animation draft for a garment. Specify fps, total_frames, and optional initial keyframes for fabric properties and wind.",
  "input_schema": {
    "type": "object",
    "required": ["garment_id", "fps", "total_frames"],
    "properties": {
      "garment_id":    { "type": "string" },
      "fps":           { "type": "number", "minimum": 1, "maximum": 120, "default": 30 },
      "total_frames":  { "type": "integer", "minimum": 1 },
      "description":   { "type": "string", "description": "Natural language description of the animation intent, e.g. 'inflate a balloon over 90 frames then deflate'" }
    }
  }
}
```

**Tool: `tailor_animation_add_keyframe`**
```json
{
  "name": "tailor_animation_add_keyframe",
  "description": "Add or update a keyframe on a specific animation track. Track names: pressure, solidify, shrink_weft, shrink_warp, wind_strength, wind_direction, and per-bone pose tracks.",
  "input_schema": {
    "type": "object",
    "required": ["garment_id", "track_name", "frame", "value"],
    "properties": {
      "garment_id":     { "type": "string" },
      "track_name":     { "type": "string" },
      "frame":          { "type": "integer", "minimum": 0 },
      "value":          { "description": "Track-type-specific: f32 for material/wind tracks; [x,y,z] for wind_direction; [x,y,z,w] for rotation." },
      "interpolation":  { "type": "string", "enum": ["linear", "step", "cubic_spline"], "default": "linear" }
    }
  }
}
```

**Tool: `tailor_animation_simulate`**
```json
{
  "name": "tailor_animation_simulate",
  "description": "Run the cloth simulation with the current animation draft, producing a frame-sequence artifact. Returns a simulation_run_id; poll for completion or await the TailorAnimationSimRunCompleted event.",
  "input_schema": {
    "type": "object",
    "required": ["garment_id"],
    "properties": {
      "garment_id":    { "type": "string" },
      "n_substeps":    { "type": "integer", "default": 8 },
      "n_iters":       { "type": "integer", "default": 5 },
      "frame_range":   { "type": "array", "items": { "type": "integer" }, "minItems": 2, "maxItems": 2 }
    }
  }
}
```

**Tool: `tailor_animation_export`**
```json
{
  "name": "tailor_animation_export",
  "description": "Export the simulated animation frame sequence in the requested format. Calls the export pipeline from [T-RENDER-VIEWPORT.export-handoff] with animation-range filtering.",
  "input_schema": {
    "type": "object",
    "required": ["garment_id", "simulation_run_id", "format"],
    "properties": {
      "garment_id":          { "type": "string" },
      "simulation_run_id":   { "type": "string" },
      "format":              { "type": "string", "enum": ["obj_sequence", "gltf_morph", "usd", "alembic_via_blender"] },
      "fps":                 { "type": "number", "default": 30 },
      "frame_range":         { "type": "array", "items": { "type": "integer" }, "minItems": 2, "maxItems": 2 }
    }
  }
}
```

#### [T-ANIMATION.model-first-api.authoring-loop] Model Animation Authoring Loop

A concrete model workflow for "inflate a balloon garment over 2 seconds then deflate":

1. Model calls `tailor_animation_draft_create` with `fps=30, total_frames=120`.
2. Model calls `tailor_animation_add_keyframe`:
   - `track=pressure, frame=0, value=0.0, interpolation=linear`
   - `track=pressure, frame=60, value=1.0, interpolation=linear`
   - `track=pressure, frame=120, value=0.0, interpolation=linear`
3. Model calls `tailor_animation_simulate` → polls for `TailorAnimationSimRunCompleted`.
4. Model calls `tailor_capture_frame` (from `[T-RENDER-VIEWPORT.model-capture]`) at
   frames 0, 30, 60, 90, 120 to inspect simulation output.
5. Model reads `SimFrameMetadata.kinetic_energy` and `max_constraint_residual` per frame
   to confirm the inflation/deflation behaved physically.
6. If quality passes: model calls `tailor_animation_export` with `format=gltf_morph`.
7. EventLedger receipt `TailorAnimationSimRunCompleted` + `GarmentExportCompleted` emitted.

**Natural-language keyframe authoring:** When a model receives a free-text animation intent
("make the dress ripple in strong wind during the chorus"), the
`TailorModelAdapter::invoke()` path uses `LlmClient.completion()` with a structured JSON
output schema to generate the `GarmentAnimationDraftV1`. The model outputs a list of
keyframes for `wind_strength`, `wind_direction`, and `wind_turbulence` tracks. These are
validated (values in physical range) before being written to the CRDT document.

---

### [T-ANIMATION.kernel-binding] Kernel Primitive Binding Summary

| Concern | Kernel primitive used |
|---|---|
| Animation data storage | `tailor_garments.animation_json JSONB` (Postgres authority) |
| Animation draft mutations | `TailorAnimationDraftUpdated` EventLedger event |
| Collaborative timeline editing | CRDT `yjs_bridge` + `ai_edit_proposal` for `/animation/` sub-tree |
| Animation simulation run | `TailorSandboxAdapter` → `TailorAnimationSimRunRequested/Completed` events |
| Model-authored animation | `TailorModelAdapter.invoke()` → `LlmClient.completion()` → `GarmentAnimationDraftV1` JSON |
| Animation export | Extended export pipeline from `[T-RENDER-VIEWPORT.export-handoff]` with range filter |
| No-SQLite tripwire | `guard_authority_write(AuthorityMode::PostgresPrimary)` before every animation JSONB write |
| Per-substep material param upload | `ClothSolver.update_params(MaterialFrameParams)` — already in solver trait |
| Per-frame capsule proxy upload | `ClothSolver.update_body_proxies(AvatarPoseSample)` — new trait method |

**Schema ID addition:**
```rust
pub const SCHEMA_CLOTH_GARMENT_ANIMATION_DRAFT_V1: &str = "hsk.cloth.garment_animation_draft@1";
```

**EventLedger `event_family` addition:**
```rust
pub const TAILOR_ANIMATION: &str = "tailor.animation";
```

---

### [T-ANIMATION.risks] Risks and Open Questions

**RISK-1 — fbxcel-dom instability.** `fbxcel-dom` (v0.0.6) is explicitly marked "highly
experimental" and "updated frequently with breaking changes." Pinning the exact version
in `Cargo.toml` is required. If the API surface changes significantly, the FBX import
adapter will need rewriting. Mitigation: accept glTF as the primary import format;
treat FBX as a secondary path gated behind a feature flag.

**RISK-2 — Animation draft JSONB grows unbounded with dense keyframe tracks.** A
30 fps animation with 24 bones, 8 keyframes per bone per second, over 300 frames
produces ~17,000 keyframe records. At ~100 bytes per keyframe (JSON), this is ~1.7 MB
of JSONB per garment animation. This is acceptable for Postgres but may slow CRDT sync.
Mitigation: cap `total_frames` at 1800 (60 seconds at 30 fps) for MVP; implement
delta-compression or binary keyframe encoding (MessagePack) if storage becomes an issue.

**RISK-3 — CRDT conflict on the same keyframe value.** Two operators editing the same
keyframe at the same time produce a `KnowledgeCrdtConflictDetected` event. The existing
conflict UI does not have animation-specific domain rendering (e.g., showing both values
on a curve graph). This is a UI design gap; the backend conflict detection is correct but
the frontend resolution UX is not yet designed.

**RISK-4 — Per-frame avatar capsule buffer upload adds CPU overhead.** At 300 frames,
the animation runner uploads 300 capsule buffer updates to the GPU. At ~10 capsules × 28
bytes each = 280 bytes per frame × 300 = 84 KB total transfer. This is negligible. The
overhead risk is wall-clock time: wgpu buffer writes + GPU command submission latency
adds ~0.5–2 ms per frame on typical hardware. For a 300-frame animation this is 150–600 ms
of extra overhead — acceptable for offline simulation but would rule out real-time
interactive preview. For interactive preview at lower fidelity, use a coarser capsule
set (4 capsules instead of 24) and fewer substeps.

**RISK-5 — Determinism of wind turbulence across GPU drivers.** The Perlin-style noise
function uses `sin()` and `fract()`, which have documented precision differences across
GPU vendors at specific input values. The turbulence track is aesthetic (it controls
visual flutter); the exact output value is less critical than the structural correctness
of the cloth. Scoping the determinism guarantee to "same GPU driver" is sufficient.
The content hash in `SolverResult` will differ across GPU vendors for animated runs with
turbulence; the promotion validation gate should use **mesh-shape comparison** (bounding
box, seam distance) rather than exact hash for animated runs.

**RISK-6 — No pure-Rust MTN parser exists.** MTN is proprietary; no OSS reader was found.
If operators need to import existing MD `.mtn` files, the only paths are: (a) re-export
from MD as FBX and import via `fbxcel-dom`; (b) license MD's SDK. Document this
limitation clearly in the Tailor animation import recipe.

**OPEN-1 — Tack constraint per-frame compliance upload.** Each tack point has its own
compliance track (`TackComplianceTrack`). In the XPBD solver, tack compliance is a field
on individual `GpuSeamConstraint` structs in a storage buffer, not a scalar UBO. Animating
it requires writing to individual buffer entries per substep for each animated tack — more
complex than the simple UBO upload used for pressure/solidify/shrinkage. Design required:
either a separate `tack_params_buffer` (one f32 per tack, indexed by tack_id) uploaded
per substep, or a CRDT-gated flag that marks a tack as "animated" and causes the solver
to read from a per-substep CPU upload instead of the static per-constraint buffer. Flag
as a post-MVP feature until the constraint GPU layout is finalized.

**OPEN-2 — glam::Quat slerp vs lerp for rotation tracks.** Avatar pose rotation
interpolation should use `glam::Quat::slerp()` (spherical linear interpolation) for
correct rotation blending, not component-wise lerp. The `Lerpable` impl for `glam::Quat`
must call `slerp` not `lerp`. CUBICSPLINE rotation tracks in glTF store quaternion
tangents that must also be slerped, not linearly added. Confirm this in the glTF
CUBICSPLINE spec before implementing.

**OPEN-3 — CRDT timeline snapshot cadence.** The rule "snapshot on any simulation run
request" is correct but means an operator cannot queue multiple animation edits before
simulating without a snapshot per batch. A lightweight "stage for simulation" command
that explicitly promotes a CRDT snapshot would give operators more control. Design
this as part of the Tailor UI control room panel (not a solver concern).

---

### [T-ANIMATION.sources] Sources

1. **Marvelous Designer Keyframe Animation 2025.0 (support docs)** — properties that can
   be keyframed, interpolation, wind and avatar pose keyframing:
   https://support.marvelousdesigner.com/hc/en-us/articles/47358157019161-Keyframe-Animation-ver-2025-0
2. **MD 2025.2 release — MetaHumans, Pleats, Keyframes (Digital Production, Nov 2025)** —
   keyframeable fabric properties (Pressure, Solidify, Shrinkage, Tack Strength, Trim Weight):
   https://digitalproduction.com/2025/11/18/marvelous-designer-2025-2-metahumans-pleats-keyframes/
3. **MD 2026.0 CGChannel release notes** — animation timeline markers, animation range
   export, FBX auto-key on empty frames, morph animation (Alembic), blend shape avatar:
   https://www.cgchannel.com/2026/04/clo-virtual-fashion-releases-marvelous-designer-2026-0/
4. **MD 2026.0 The Rookies Blog** — animation range export for FBX/glTF; markers; blend shapes
   for body-shape avatar tuning:
   https://www.therookies.co/blog/headlines/marvelous-designer-2026
5. **MD Animation Mode: Play Animation (support docs)** — animation recording and playback
   pipeline:
   https://support.marvelousdesigner.com/hc/en-us/articles/47358280523033-ANIMATION-MODE-Play-Animation
6. **MD Motion (*.mtn) Open/Save (support docs)** — MTN format is avatar movement only;
   FBX/COLLADA is the upstream source via Auto Convert to Motion:
   https://support.marvelousdesigner.com/hc/en-us/articles/360036955772-Motion-mtn-Open-Save
7. **MD Compatible File Formats (support docs)** — full list of supported formats:
   https://support.marvelousdesigner.com/hc/en-us/articles/47358272195609-Compatible-File-Format
8. **MD 2026.0 FBX animation export (support docs)** — export joint keyframe animation:
   https://support.marvelousdesigner.com/hc/en-us/articles/12193753080601-FILE-Export-Joint-Keyframe-Animation-with-FBX
9. **Real-Time Skirt Simulation with XPBD on GPU (Sueray Wang, 2025)** — avatar body-proxy
   data drives per-frame cloth simulation; 35k vertex cloth on 9k vertex body at 8-10 ms/frame:
   https://sueraywang.github.io/project/xpbdmmd/
10. **Cloth Simulation with Three.js + Compute Shaders on Skeletal Meshes (Bandinopla, Medium)** —
    per-frame skeletal skinning position upload, cloth-vs-skinned-mesh coupling, wind force
    application in vertex force pass; world-space vertex blend (skeleton weight paint):
    https://medium.com/@pablobandinopla/simple-cloth-simulation-with-three-js-and-compute-shaders-on-skeletal-animated-meshes-acb679a70d9f
11. **Real-Time Cloth Simulation in XR with PBD + GPU (App. Sci. 2025)** — kinematic capsule
    update pre-stabilization pass requirement; pre-PBD-loop capsule position upload:
    https://doi.org/10.3390/app15126611
12. **keyframe crate (Rust, github.com/HannesMann/keyframe)** — AnimationSequence, CanTween,
    keyframes! macro; evaluated; lacks CubicSpline tangents needed for avatar pose:
    https://github.com/HannesMann/keyframe
    https://docs.rs/keyframe
13. **Spanda animation engine (Rust, Aarambh Dev Hub, Mar 2026)** — KeyframeTrack<T>,
    38+ easing curves, GPU batch animation (v0.9.0), no_std ready:
    https://aarambhdevhub.medium.com/spanda-the-high-performance-animation-engine-built-for-every-platform-in-rust-3c3715683ae3
14. **gltf crate v1.4.1 (Rust)** — animation.Interpolation enum (LINEAR, STEP, CUBICSPLINE);
    channel.target.property; read_inputs / read_outputs for sampler data:
    https://docs.rs/gltf/latest/gltf/animation/enum.Interpolation.html
15. **glTF 2.0 Animation Spec — CUBICSPLINE keyframe layout** — in_tangent/value/out_tangent
    interleaved layout per keyframe; Catmull-Rom spline; sampler/channel structure:
    https://github.khronos.org/glTF-Tutorials/gltfTutorial/gltfTutorial_007_Animations.html
16. **glTF Animation Vulkan Tutorial** — glTF animation evaluation: sampler input/output,
    interpolation evaluation at time t, translation/rotation/scale channel types:
    https://docs.vulkan.org/tutorial/latest/Building_a_Simple_Engine/Advanced_Topics/GLTF_Animation.html
17. **fbxcel-dom crate (Rust, lo48576)** — FBX DOM library; v0.0.6; read-only; FBX 7.4/7.5;
    AnimCurveNode, AnimCurve key times; marked "highly experimental":
    https://github.com/lo48576/fbxcel-dom
    https://docs.rs/fbxcel-dom/0.0.6/fbxcel_dom/index.html
18. **fbxcel crate (Rust, lo48576)** — low-level FBX binary pull parser; required by
    fbxcel-dom; FBX 7.4/7.5:
    https://lib.rs/crates/fbxcel
    https://crates.io/crates/fbxcel
19. **NVIDIA GPU Gems Chapter 5 — Implementing Improved Perlin Noise** — gradient noise
    implementation reference; basis for wind turbulence noise in WGSL compute:
    https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-5-implementing-improved-perlin-noise
20. **Velvet CUDA XPBD cloth simulator (vitalight)** — spatial-hash neighbor reuse across
    substep iterations; Jacobi delta accumulation; per-frame body proxy update pattern:
    https://github.com/vitalight/Velvet
21. **MD Animation Recording (cgpress.org, 2025.2 release)** — fabric property animation:
    shrinkage weft/warp, pressure, solidify, tack strength, trim weight all keyframeable
    on timeline from 2025.2; real-time wind adjustment during recording:
    https://cgpress.org/archives/marvelous-designer-2025-2-introduces-improved-animation-controls-trim-conversion-fbx-export-metahuman-support-and-a-new-pleat-tool.html
22. **MD 2025.0 CGChannel release** — keyframe animation feature introduction (2025.0):
    avatar joint keyframing, wind keyframing, simulation property keyframing added:
    https://www.cgchannel.com/2025/04/clo-virtual-fashion-releases-marvelous-designer-2025-0/
