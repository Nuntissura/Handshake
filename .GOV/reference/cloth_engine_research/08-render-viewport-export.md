---
file_id: cloth-engine-08-render-viewport-export
topic_id: T-RENDER-VIEWPORT
title: "Viewport, Visual Debugging, and Render/Export Handoff"
status: draft
depends_on:
  - T-CLOTH-SOLVER
  - T-CODEBASE
summary: "Throwaway Bevy testbed viewport, Handshake-native wgpu viewport panel, model-readable visual capture, and geometry cache export (glTF/USD/OBJ-sequence) to Blender/UE for photoreal render."
sources: 32
updated_at: "2026-06-17"
---

## [T-RENDER-VIEWPORT] Viewport, Visual Debugging, and Render/Export Handoff

This document covers three coupled concerns for the Tailor creative module:

1. **Throwaway testbed viewport** — Bevy 0.18 + bevy_silk pattern used during solver crate development; kept outside `handshake_core`; never shipped.
2. **Handshake-native Tailor viewport** — a wgpu render surface embedded in the Tauri 2 shell or surfaced as a rendered-to-texture panel in the Handshake control room UI; this is the production viewport.
3. **Render/export handoff** — the engine is not the photoreal renderer; garment geometry caches are exported to Blender, Unreal Engine, or USD pipelines for final render; the Rust export path uses glTF morph-target sequences, OBJ frame sequences, or USD time-sample meshes.

The Handshake visual debugger precedent (existing `app/src-tauri/src/commands/visual_debugger.rs`) already establishes the model-readable visual capture contract using CDP `Page.captureScreenshot` with `fromSurface: true`. The Tailor visual capture subsystem follows the same pattern and extends it with a structured metadata sidecar describing simulation state so an LLM agent can evaluate cloth output without a human in the loop.

---

### [T-RENDER-VIEWPORT.md-mapping] Marvelous Designer Feature Mapping

Marvelous Designer's viewport is a first-class production surface with a dedicated shader pipeline per render mode. The features the Tailor viewport must cover:

| MD Feature | Viewport Mode | Engine Design Target |
|---|---|---|
| PBR shaded cloth | Solid / PBR | wgpu PBR fragment shader over draped mesh |
| Wireframe overlay | Wireframe | WGSL line-topology pipeline or geometry wireframe |
| Particle / mesh dual view | N/A (implicit) | Debug pass: draw particles as point sprites + edge lines |
| Normal visualization | N/A (debug) | Debug pass: WGSL draw normal arrows |
| UV Editor mode | 2D UV layout | Separate 2D flat-uv panel (future; not MVP) |
| Constraint overlay | N/A | Custom debug pass: draw constraint edges colored by residual |
| Toon shader | Toon (2026.0) | Future wgpu toon pass |
| Fur strands | Experimental | Out of scope for MVP |

The MD approach to UV visualization (UV islands = exact flattened 2D pattern pieces) is addressed in the UV/texturing topic and is not the viewport MVP.

---

### [T-RENDER-VIEWPORT.bevy-testbed] Bevy Throwaway Testbed Viewport

#### Purpose and Scope

The Bevy testbed is a developer-only utility crate (`handshake-cloth-testbed`) that imports `handshake-cloth-solver` as a dependency and provides a windowed viewport for visually inspecting solver runs during crate development. It is:

- **Throwaway**: it is never a dependency of `handshake_core` or `app/src-tauri`.
- **Solver-agnostic**: it uses Bevy's ECS to spin up a scene, feed a particle/triangle mesh into the solver crate via the `ClothSolverHandle` trait, and render the result.
- **Disposable**: when the solver is mature enough to be tested through the Handshake-native viewport, the testbed crate can be deprecated.

#### OSS Reference: bevy_silk

`bevy_silk` (https://github.com/ManevilleF/bevy_silk) is the canonical Bevy cloth rendering reference. It targets Bevy 0.17 (v0.10.0, released December 2025). It uses Verlet integration with stick constraints, not XPBD — but its ECS wiring pattern is the template:

- `ClothBuilder` component attached to any entity with `Handle<Mesh>`.
- `Transform` + `GlobalTransform` required alongside the cloth component.
- Mesh assets edited in-place each frame as cloth updates vertex positions.
- Anchored vertices pinned to world positions or other entities.
- No GPU compute; CPU Verlet only.

The testbed adapts this pattern: instead of using bevy_silk's solver, it invokes `handshake-cloth-solver`'s XPBD GPU path, then feeds the output vertex buffer back into Bevy's `Mesh` asset each frame via the standard `extract_meshes` path.

#### Bevy 0.18 Capabilities Used in the Testbed

Bevy 0.18 (released March 2026) provides:

- `EasyScreenshotPlugin`: captures a PNG from the active camera with a single keypress (PrintScreen by default). Used in the testbed to generate solver-output inspection images.
- `FreeCamera` / `PanCamera`: first-party camera controllers for navigating the cloth in 3D space without implementing an arcball camera.
- `RenderTarget::Image`: render the Tailor viewport to a GPU texture instead of a window, enabling headless capture.
- `ImageCopyDriver` node (in the `RenderGraph`): copies the render target texture to a CPU-mapped buffer via `copy_texture_to_buffer`.

The headless path (documented in `bevy/examples/app/headless_renderer.rs`) is used for CI-style visual regression tests against the solver:

```rust
// Testbed headless capture pattern (pseudocode)
let mut render_image = Image::new_target_texture(width, height, TextureFormat::Rgba8UnormSrgb);
render_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
let render_handle = images.add(render_image);
camera.render_target = RenderTarget::Image(render_handle.clone().into());
// ImageCopier component + MainWorldReceiver/RenderWorldSender channels carry
// the frame data back to the main world where it is saved via the `image` crate.
```

#### Testbed Crate Layout

```
handshake-cloth-testbed/
  Cargo.toml              # depends on handshake-cloth-solver, bevy 0.18, bevy_silk (optional)
  src/
    main.rs               # Bevy App builder, plugins
    scene.rs              # avatar proxy capsules, ground plane, lighting
    cloth_plugin.rs       # ECS plugin: spawn cloth entity, feed solver each Update tick
    debug_overlay.rs      # egui-wgpu panels: particle count, constraint residuals, timing
    capture.rs            # headless PNG capture path (RenderTarget::Image + ImageCopier)
    export.rs             # frame-by-frame OBJ dump for visual diff tooling
  examples/
    drape_sphere.rs       # sphere avatar, single draped panel
    xpbd_seam_test.rs     # seam constraint visualization
```

The testbed must not import `sqlx`, `axum`, `tauri`, or any Handshake kernel type. The solver crate boundary is the `ClothSolverHandle` trait plus `GarmentFrame` output type — that is all the testbed needs.

---

### [T-RENDER-VIEWPORT.native-viewport] Handshake-Native Tailor Viewport

#### Architecture Decision: Rendered-to-Texture Panel

Embedding a raw wgpu surface inside a Tauri 2 WebView window is technically possible but carries documented friction: the wgpu surface and the WebView compositor compete for the same OS window surface, causing flickering on some platforms (Tauri issue #9220). The Graphite editor project (https://github.com/GraphiteEditor/Graphite) is blocked on Wayland and Windows compositor issues with this approach as of 2025.

The Handshake-native viewport uses **rendered-to-texture** instead:

1. The Tailor viewport renders into a `wgpu::Texture` (offscreen, no OS surface contention).
2. The rendered texture is transferred to CPU via `copy_texture_to_buffer` on demand or at a reduced frame rate (e.g. 15 fps for live preview; single-frame on demand for inspection captures).
3. The PNG bytes are sent to the Tauri frontend via a Tauri `invoke` command response or an `emit` event.
4. The frontend renders the image into an `<img>` or `<canvas>` element in the Handshake control room Tailor panel.

This is the same architecture as the existing Handshake visual debugger (CDP `Page.captureScreenshot`) but driven by the cloth solver's wgpu pipeline rather than the WebView2 compositor. It requires no platform-specific OS compositor patches.

An alternative for a richer interactive viewport (later iteration): use the `tauri-plugin-steam-overlay` approach (`qwook/tauri-plugin-steam-overlay`, cracked for macOS as of June 2025, Windows planned) to layer the WebView UI over a native wgpu surface beneath. This remains a future upgrade path, not the MVP design.

#### wgpu Render Pipeline for the Tailor Viewport

The Tailor viewport render pipeline lives inside `handshake-cloth-solver` (or a thin `handshake-tailor-viewport` sub-crate) and exposes a single struct:

```rust
// In handshake-cloth-solver/src/viewport.rs
pub struct ClothViewport {
    device: wgpu::Device,
    queue: wgpu::Queue,
    // Offscreen render target
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_texture: wgpu::Texture,
    // Readback staging buffer
    staging_buffer: wgpu::Buffer,
    // Configurable render passes
    solid_pipeline: wgpu::RenderPipeline,    // PBR solid shading
    wireframe_pipeline: wgpu::RenderPipeline, // topology: LineList
    debug_pipeline: wgpu::RenderPipeline,    // particles, normals, constraints
    config: ViewportConfig,
}

pub struct ViewportConfig {
    pub width: u32,
    pub height: u32,
    pub render_mode: RenderMode,
    pub show_wireframe_overlay: bool,
    pub show_constraint_residuals: bool,
    pub show_normals: bool,
    pub show_particles: bool,
    pub background_color: [f32; 4],
    pub camera: CameraUniform,
}

pub enum RenderMode { Solid, Wireframe, Toon }

pub struct CapturedFrame {
    pub png_bytes: Vec<u8>,
    pub frame_index: u64,
    pub sim_step: u64,
    pub captured_at_utc: String,
    pub metadata: SimFrameMetadata,
}

pub struct SimFrameMetadata {
    pub particle_count: u32,
    pub constraint_count: u32,
    pub max_constraint_residual: f32,
    pub avg_constraint_residual: f32,
    pub collision_count: u32,
    pub kinetic_energy: f32,
    pub step_time_ms: f32,
}
```

The `CapturedFrame` is the model-readable visual output: the PNG contains the rendered image; the `SimFrameMetadata` is a structured JSON companion that an LLM agent can parse to judge simulation quality without pixel analysis.

#### WGSL Render Shaders

The solid pipeline uses a standard PBR-lite fragment shader. The wireframe pipeline uses `wgpu::PrimitiveTopology::LineList` with an index buffer that enumerates triangle edges. The debug pipeline renders particles as point sprites and constraint edges as colored lines (green = low residual, yellow = medium, red = high):

```wgsl
// debug_constraints.wgsl (fragment)
struct ConstraintDebugIn {
    @location(0) residual: f32, // normalized 0-1
};

@fragment
fn fs_main(in: ConstraintDebugIn) -> @location(0) vec4<f32> {
    // Green (low residual) -> yellow -> red (high residual)
    let r = clamp(2.0 * in.residual, 0.0, 1.0);
    let g = clamp(2.0 * (1.0 - in.residual), 0.0, 1.0);
    return vec4<f32>(r, g, 0.0, 1.0);
}
```

Normal arrows use a geometry-shader-equivalent trick via a CPU-side arrow mesh generated per normal and uploaded as a vertex buffer each frame (not requiring geometry shaders, which wgpu/WGSL does not expose).

#### Headless Capture Path (GPU-to-CPU Readback)

This follows the exact pattern documented at `sotrh.github.io/learn-wgpu/showcase/windowless/`:

```rust
// In ClothViewport::capture_frame()
let buffer_size = (4 * self.config.width * self.config.height) as u64;
let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
    size: buffer_size,
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
    label: Some("cloth_viewport_readback"),
});
// Encode render + copy
let mut encoder = self.device.create_command_encoder(&Default::default());
// ... render pass to color_texture ...
encoder.copy_texture_to_buffer(
    self.color_texture.as_image_copy(),
    wgpu::TexelCopyBufferInfo {
        buffer: &output_buffer,
        layout: wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * self.config.width),
            rows_per_image: Some(self.config.height),
        },
    },
    wgpu::Extent3d { width: self.config.width, height: self.config.height, depth_or_array_layers: 1 },
);
self.queue.submit(std::iter::once(encoder.finish()));
// Map and read
let slice = output_buffer.slice(..);
slice.map_async(wgpu::MapMode::Read, |_| {});
self.device.poll(wgpu::PollType::Wait);
let data = slice.get_mapped_range().to_vec();
output_buffer.unmap();
// Encode PNG via `image` crate
let img = image::RgbaImage::from_raw(self.config.width, self.config.height, data).unwrap();
let mut png_bytes = Vec::new();
img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageOutputFormat::Png).unwrap();
```

#### egui-wgpu Debug Panel (Testbed and Future Control Room)

`egui-wgpu` v0.34.3 (https://docs.rs/egui-wgpu) provides the `CallbackTrait` for injecting custom wgpu draw calls inside an egui `Window`. In the testbed this is used for live simulation statistics:

```rust
// In debug_overlay.rs (testbed)
egui::Window::new("Cloth Sim").show(ctx, |ui| {
    ui.label(format!("Particles: {}", stats.particle_count));
    ui.label(format!("Constraints: {}", stats.constraint_count));
    ui.label(format!("Max residual: {:.4}", stats.max_residual));
    ui.label(format!("Kinetic energy: {:.4}", stats.kinetic_energy));
    ui.label(format!("Step time: {:.2} ms", stats.step_time_ms));
    egui::plot::Plot::new("residual_history").show(ui, |plot_ui| {
        plot_ui.line(egui::plot::Line::new(residual_history.clone()));
    });
});
```

In the Handshake-native production viewport the egui-wgpu panel is not used; instead statistics are surfaced as JSON via a Tauri command and rendered in the React/Svelte control room UI.

---

### [T-RENDER-VIEWPORT.model-capture] Model-Readable Visual Capture

#### Handshake Visual Debugger Precedent

The existing `visual_debugger.rs` Tauri command establishes the precedent for Tailor's capture contract. Key types already defined:

```rust
pub struct VisualCaptureResult {
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
    pub captured_at_utc: String,
}
```

Tailor extends this with a `TailorVisualCapture` type that adds simulation metadata so an LLM agent can judge cloth quality without relying solely on pixel analysis:

```rust
// In src/tailor/capture.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailorVisualCapture {
    // Visual
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
    pub captured_at_utc: String,
    pub render_mode: String,        // "solid" | "wireframe" | "debug_constraints"
    // Simulation state
    pub garment_id: String,
    pub simulation_run_id: String,
    pub frame_index: u64,
    pub sim_time_seconds: f64,
    // Numeric diagnostics (model-parseable)
    pub metadata: SimFrameMetadata,
    // Operator note or model-authored verdict (optional)
    pub annotation: Option<String>,
    // EventLedger event_id that recorded this capture
    pub event_ledger_event_id: Option<String>,
}
```

#### LLM Inspection Workflow

When a model agent (e.g. a `TailorModelAdapter` task) needs to inspect a simulation run, the workflow is:

1. Model calls `POST /tailor/garments/:id/simulate` → sandbox runs XPBD solver → `TailorSimulationRun` record created.
2. Model calls `POST /tailor/garments/:id/capture` with a frame index and render mode → `ClothViewport::capture_frame()` renders and returns `TailorVisualCapture`.
3. Model inspects the PNG via its vision capability (LLaVA-style or GPT-4V-style call through `LlmClient`) and reads `metadata` as structured JSON alongside the image.
4. Model writes a `TailorCaptureAnnotation` as an EventLedger receipt and stores verdict in the garment's CRDT document.

The `SimFrameMetadata` numeric fields (`max_constraint_residual`, `avg_constraint_residual`, `kinetic_energy`, `collision_count`, `step_time_ms`) serve as a quantitative pre-filter: if `kinetic_energy` is still falling, the simulation has not settled and the model should not accept. If `max_constraint_residual` is above a threshold (configurable in `SandboxPolicyV1` extension fields), the solver has not converged.

This mirrors the model-readable pattern already used by the visual debugger for DOM/AX inspection — structured JSON + PNG together give the model more reliable signal than pixels alone.

#### Tauri Commands for Tailor Capture

Following the `api/` + Tauri command pattern established in the codebase:

```rust
// app/src-tauri/src/commands/tailor.rs
#[tauri::command]
pub async fn tailor_capture_frame(
    garment_id: String,
    simulation_run_id: String,
    frame_index: u64,
    render_mode: String,
    state: tauri::State<'_, AppState>,
) -> Result<TailorVisualCapture, String> {
    // Resolves the simulation run, invokes ClothViewport::capture_frame(),
    // emits GarmentFrameCaptured EventLedger receipt, returns TailorVisualCapture.
    todo!()
}

#[tauri::command]
pub async fn tailor_viewport_config(
    garment_id: String,
    config: ViewportConfigPatch,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Live-updates render mode, overlay toggles, camera pose without re-running simulation.
    todo!()
}
```

---

### [T-RENDER-VIEWPORT.export-handoff] Render/Export Handoff

Tailor is not a photoreal renderer. Its output is a simulated garment mesh (vertex positions + normals + UVs per frame). Final photoreal rendering happens in Blender (Cycles/EEVEE) or Unreal Engine 5 (Lumen/Nanite) or another DCC tool. The export pipeline converts the solver's `GarmentFrame` sequence into a format those tools can consume.

#### Target Export Formats

| Format | Target DCC | Rust Crate | Status |
|---|---|---|---|
| glTF 2.0 morph-target sequence | Blender, Three.js, Godot | `gltf` v1.4.1 (read) + hand-written binary writer | Write support requires custom binary GLB authoring |
| OBJ numbered sequence | Blender, Houdini, Maya | `obj-rs` / custom write | Simple; full write support trivially implementable |
| USD time-sample mesh | Houdini, Unreal 5, Blender | `openusd` v0.5.0 (mxpv/openusd) | Read+write; Mesh time samples; UsdPhysics schema |
| Alembic Ogawa | Blender, Maya, Houdini | `ogawa-rs` (Traverse-Research) | Read-only; write requires C++ Alembic lib or custom Ogawa |
| FBX animated mesh | Unreal Engine, Maya | No production-quality pure-Rust writer | Use USD or Alembic instead |

**MVP recommendation**: OBJ numbered sequence (frame_0000.obj, frame_0001.obj, ...) as the lowest-friction export; USD as the high-fidelity path once `openusd` v0.5.0 authoring API is exercised. glTF morph targets are the web/realtime path.

#### OBJ Sequence Export (MVP)

The simplest complete export path. Each frame becomes one `.obj` file with vertex positions, normals, and UV coordinates. No external crate needed beyond basic string formatting:

```rust
// In src/tailor/export/obj_sequence.rs
pub fn export_obj_frame(frame: &GarmentFrame, frame_index: u64, out_dir: &Path) -> std::io::Result<()> {
    let path = out_dir.join(format!("frame_{:06}.obj", frame_index));
    let mut f = std::fs::File::create(&path)?;
    writeln!(f, "# Handshake Tailor frame {}", frame_index)?;
    for p in &frame.positions {
        writeln!(f, "v {:.6} {:.6} {:.6}", p[0], p[1], p[2])?;
    }
    for n in &frame.normals {
        writeln!(f, "vn {:.6} {:.6} {:.6}", n[0], n[1], n[2])?;
    }
    for uv in &frame.uvs {
        writeln!(f, "vt {:.6} {:.6}", uv[0], uv[1])?;
    }
    for tri in &frame.triangles {
        // OBJ 1-indexed: f v/vt/vn v/vt/vn v/vt/vn
        writeln!(f, "f {0}/{0}/{0} {1}/{1}/{1} {2}/{2}/{2}",
            tri[0] + 1, tri[1] + 1, tri[2] + 1)?;
    }
    Ok(())
}
```

The Blender Alembic/OBJ import workflow (documented at `versluis.com/2025/06/alembic-workflow-from-iclone-to-md-to-ue-via-blender/`) shows that a mesh sequence imported into Blender via the "Import Sequence" add-on drives a Mesh Sequence Cache modifier, giving per-frame vertex animation playback. The OBJ sequence export enables this path immediately.

#### glTF Morph-Target Sequence

The `gltf` crate v1.4.1 is read-only (no write API). Writing a GLB with morph-target animation requires hand-authoring the glTF binary format directly. The structure is:

```json
{
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 },
      "targets": [
        { "POSITION": 3 },  // frame 1 position delta
        { "POSITION": 4 }   // frame 2 position delta
      ]
    }]
  }],
  "animations": [{
    "channels": [{ "target": { "node": 0, "path": "weights" } }],
    "samplers": [{ "input": 5, "output": 6, "interpolation": "LINEAR" }]
  }]
}
```

Each frame becomes a morph target (delta positions from the rest pose). The animation sampler drives the weights so at time `t` only frame `n`'s weight is 1.0. This is playable in Blender and Three.js without extra addons. The `gltfgen` project (https://github.com/elrnv/gltfgen, last updated 2019, input format VTK) is dormant and not suitable as a dependency; Tailor writes its own minimal GLB encoder for this specific use case.

```rust
// In src/tailor/export/gltf_morph.rs
pub fn export_gltf_morph_sequence(frames: &[GarmentFrame], out_path: &Path) -> std::io::Result<()> {
    // Computes per-frame position deltas from frames[0] as the base mesh.
    // Serializes positions/normals/uvs as f32 binary accessors.
    // Writes morph targets as POSITION delta accessors.
    // Writes animation sampler driving mesh.weights over time.
    // Packs all binary data into a single .glb chunk.
    todo!()
}
```

#### USD Time-Sample Mesh Export

`openusd` v0.5.0 (https://github.com/mxpv/openusd, released June 8, 2026) supports read + write of `.usda` (text) and `.usdc` (binary) and includes UsdGeom (Mesh) and UsdPhysics. The `openusd` authoring API allows setting time-varying attribute values:

```rust
// In src/tailor/export/usd_export.rs
use openusd::{Stage, UsdGeomMesh};

pub fn export_usd_sequence(frames: &[GarmentFrame], fps: f64, out_path: &Path) -> anyhow::Result<()> {
    let stage = Stage::create(out_path)?;
    stage.set_start_time_code(0.0)?;
    stage.set_end_time_code(frames.len() as f64 - 1.0)?;
    stage.set_frames_per_second(fps)?;

    let mesh = UsdGeomMesh::define(&stage, "/World/GarmentMesh")?;

    // Write topology once (static)
    let base = &frames[0];
    mesh.face_vertex_counts().set(vec![3u32; base.triangles.len()])?;
    mesh.face_vertex_indices().set(base.triangles.iter().flatten().cloned().collect::<Vec<_>>())?;
    mesh.st().set(base.uvs.iter().flatten().cloned().collect::<Vec<_>>())?;

    // Write per-frame time samples for positions and normals
    for (i, frame) in frames.iter().enumerate() {
        let t = i as f64;
        let points: Vec<[f32; 3]> = frame.positions.clone();
        let normals: Vec<[f32; 3]> = frame.normals.clone();
        mesh.points().set_at_time(t, points)?;
        mesh.normals().set_at_time(t, normals)?;
    }

    stage.save()?;
    Ok(())
}
```

The USD file is importable in Blender 5.x (which has native USD support) and Unreal Engine 5.x via the USD plugin. UE5's Chaos Cloth can consume physics-annotated USD for cloth parameter re-simulation on the game engine side (the `UsdPhysics` schema in `openusd` would carry the cloth material parameters alongside the geometry).

#### Alembic: Gap and Workaround

No production-quality pure-Rust Alembic **writer** exists as of June 2026:

- `ogawa-rs` (https://github.com/Traverse-Research/ogawa-rs, v0.4.0): reads Ogawa-backend Alembic files; write support is absent.
- `alembic` crate (crates.io): placeholder only (https://crates.io/crates/alembic).
- `ennis/alembic-rs` (https://github.com/ennis/alembic-rs): WIP Rust bindings to the Alembic C++ library; not production-ready.

The recommended workaround for Alembic output:

1. Export OBJ sequence from the Rust engine.
2. Convert to Alembic via a small Python script using the Blender Python API (`bpy.ops.wm.alembic_export`) run headlessly (`blender --background --python convert_to_abc.py`).
3. Document the conversion step in the garment export recipe.

Alternatively, use USD (which Blender can re-export as Alembic if needed).

#### Export EventLedger Receipt

Every export operation emits a `GarmentExportCompleted` EventLedger receipt recording the target format, output path, frame range, and artifact hash:

```rust
// In src/tailor/export/mod.rs
let export_event = NewKernelEvent::builder(
    task_run_id,
    session_run_id,
    KernelEventType::GarmentExportCompleted,
    KernelActor::System("tailor.export".to_string()),
)
.aggregate("tailor_garment", &garment_id)
.idempotency_key(&format!("garment-export-{}-{}-{}", garment_id, format_name, export_run_id))
.payload(json!({
    "garment_id": garment_id,
    "simulation_run_id": simulation_run_id,
    "export_format": format_name,   // "obj_sequence" | "gltf_morph" | "usd"
    "frame_count": frame_count,
    "fps": fps,
    "output_path": out_dir.to_string_lossy(),
    "artifact_hash": sha256_hex,
    "frame_range": [start_frame, end_frame],
}))
.source_component("tailor::export")
.build()?;
```

The export artifact (directory or single file) is also registered via `write_dir_artifact()` with `ArtifactPayloadKind::Bundle` (multi-file for OBJ sequence) or `ArtifactPayloadKind::File` (single GLB or USDC). This makes the exported geometry discoverable by any downstream model agent via the standard artifact query path.

---

### [T-RENDER-VIEWPORT.kernel-bindings] Kernel Primitive Bindings

#### EventLedger Event Types

New `KernelEventType` variants added by the Tailor viewport and export subsystem (added to `kernel/mod.rs` and registered in `required_first_slice_events()`):

- `GarmentFrameCaptured` — visual capture of a simulation frame; payload: `garment_id`, `simulation_run_id`, `frame_index`, `render_mode`, `png_artifact_id`, `metadata` JSON.
- `GarmentExportStarted` — export job initiated.
- `GarmentExportCompleted` — export finished; payload: format, path, artifact hash.
- `GarmentExportFailed` — export failed; payload: error reason.
- `GarmentCaptureAnnotated` — model or operator added a quality verdict to a captured frame; payload: annotation text, verdict (accept/reject/needs_resim), annotator actor.

#### Axum API Endpoints

Added to `src/api/tailor.rs`:

```rust
// Tailor capture and export routes
.route("/tailor/garments/:id/capture", post(capture_frame))
.route("/tailor/garments/:id/capture/latest", get(get_latest_capture))
.route("/tailor/garments/:id/export", post(export_garment))
.route("/tailor/garments/:id/exports", get(list_exports))
.route("/tailor/garments/:id/captures/:frame/annotate", post(annotate_capture))
```

#### PostgreSQL Schema Additions

Two new tables for the Tailor viewport and export subsystem:

```sql
-- Migration: tailor_captures
CREATE TABLE tailor_captures (
    capture_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    garment_id      UUID NOT NULL REFERENCES tailor_garments(garment_id),
    simulation_run_id UUID,
    frame_index     BIGINT NOT NULL,
    render_mode     TEXT NOT NULL,
    png_artifact_id TEXT,           -- FK into artifact_manifests
    metadata_json   JSONB NOT NULL, -- SimFrameMetadata
    annotation      TEXT,
    verdict         TEXT,           -- 'accept' | 'reject' | 'needs_resim' | NULL
    event_ledger_event_id TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Migration: tailor_exports
CREATE TABLE tailor_exports (
    export_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    garment_id      UUID NOT NULL REFERENCES tailor_garments(garment_id),
    simulation_run_id UUID,
    export_format   TEXT NOT NULL,  -- 'obj_sequence' | 'gltf_morph' | 'usd'
    frame_count     INT NOT NULL,
    fps             FLOAT8 NOT NULL,
    output_path     TEXT NOT NULL,
    artifact_hash   TEXT,
    status          TEXT NOT NULL DEFAULT 'pending', -- 'pending'|'completed'|'failed'
    error_reason    TEXT,
    event_ledger_event_id TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at    TIMESTAMPTZ
);
```

#### CRDT Layer

Visual captures and export records are not CRDT documents (they are immutable receipts). The CRDT layer is used for the garment authoring side (panel geometry, seam definitions, material parameters) covered in the T-CLOTH-SOLVER and T-GARMENT-SCHEMA topics. The viewport and export subsystem reads garment state from the promoted authority row; it does not write back into the CRDT document.

The one exception: a model agent writing a `TailorCaptureAnnotation` (quality verdict) is a light collaborative edit. This uses `ai_edit_proposal` in `kernel/crdt/ai_edit_proposal.rs` — the model proposes an annotation, it runs through the CRDT promotion bridge, and the accepted verdict lands in the `tailor_captures` table `verdict` column and is recorded as a `GarmentCaptureAnnotated` EventLedger event.

---

### [T-RENDER-VIEWPORT.model-api] Model-First (LLM-Steerable) API Surface

The Tailor viewport provides three model-callable surfaces:

1. **Capture request** (`TailorCaptureRequest`): model specifies garment, simulation run, frame, render mode, and optional camera pose override. Returns `TailorVisualCapture` with PNG + metadata JSON.
2. **Annotation write** (`TailorAnnotateCapture`): model writes a quality verdict (accept/reject/needs_resim) and optional text note to a capture record. Becomes an EventLedger receipt.
3. **Export request** (`TailorExportRequest`): model specifies garment, frame range, fps, and format. Kicks off an export job; returns an export receipt with output path.

These are exposed as MCP tool definitions through the model-lane gate, following the same tool-dispatch pattern as existing Handshake model tools:

```rust
// Tool definition (simplified)
TailorTool::CaptureFrame {
    garment_id: String,
    simulation_run_id: String,
    frame_index: u64,
    render_mode: String,         // "solid" | "wireframe" | "debug_constraints"
    camera_preset: Option<String>, // "front" | "side" | "top" | "isometric"
}

TailorTool::AnnotateCapture {
    capture_id: String,
    verdict: String,              // "accept" | "reject" | "needs_resim"
    note: Option<String>,
}

TailorTool::ExportGarment {
    garment_id: String,
    simulation_run_id: String,
    format: String,               // "obj_sequence" | "gltf_morph" | "usd"
    start_frame: u64,
    end_frame: u64,
    fps: f64,
}
```

The model inspection loop for a garment simulation run:

1. Simulate → captures frame at key points (start, mid-drape, settled).
2. Calls `CaptureFrame` with `render_mode: "debug_constraints"` to inspect residuals.
3. Reads `SimFrameMetadata.kinetic_energy` and `max_constraint_residual` to decide if the simulation has settled.
4. If settled, calls `CaptureFrame` with `render_mode: "solid"` for a quality image.
5. Sends PNG to vision LLM via `LlmClient` to judge visual quality.
6. Writes `AnnotateCapture` with verdict.
7. If accepted, calls `ExportGarment` with `format: "usd"` for downstream render handoff.

---

### [T-RENDER-VIEWPORT.risks] Risks and Open Questions

1. **wgpu offscreen + Tauri performance**: copying texture bytes CPU-side at 30 fps for a live preview panel is expensive. The mitigation is to reduce preview frame rate to 5-10 fps in the production panel; use single-frame on-demand capture for model inspection (no live loop). Async transfer via `map_async` does not block the Tauri main thread.

2. **USD authoring API stability**: `openusd` v0.5.0 (mxpv) carries "no API stability until v1.0". Time-sample mesh write is partially documented and requires exercising the `set_at_time` authoring API with empirical validation. If it does not support this use case, the fallback is the OBJ sequence export.

3. **Alembic write gap**: The pure-Rust Alembic write path does not exist. Projects that specifically need Alembic (some Houdini pipelines) require a Blender Python conversion step. Document this as a known limitation; revisit if the Alembic C++ FFI bindings in `ennis/alembic-rs` mature.

4. **Bevy testbed version pinning**: Bevy releases approximately every 4 months. The testbed crate must pin to a specific Bevy version and be explicitly upgraded when the solver crate API changes. Bevy minor upgrades commonly require downstream ECS component changes that break compile. The testbed must have `allow-dirty` version pinning in its `Cargo.toml` and be treated as a dev-only artifact.

5. **Model-readable capture quality gate**: the `SimFrameMetadata` thresholds for `max_constraint_residual` and `kinetic_energy` that determine "settled" are initially heuristic. They should be derived empirically from known-good simulations and stored as configurable parameters in `SandboxPolicyV1` extension fields, not hardcoded.

6. **glTF morph-target delta precision**: morph-target position deltas are stored as normalized integers in some encoders for file size; for cloth simulation geometry the delta range can be large (especially at the start of drape). Tailor's GLB writer must use `FLOAT` accessors for deltas, not normalized integer types, to preserve sub-millimeter position accuracy.

7. **No Alembic-in, Alembic-out round-trip**: the engine can import avatar geometry as OBJ or glTF; it cannot currently import Alembic animation caches (which some avatar rigs come from). Avatar import is handled separately (see T-AVATAR-INTEGRATION topic); this is noted here as a pipeline gap for animated avatar inputs.

8. **Tauri native surface future upgrade**: the `qwook/tauri-plugin-steam-overlay` approach for a true native wgpu surface beneath the WebView is macOS-only as of June 2025. If this matures to full cross-platform support, the Tailor viewport can be upgraded from rendered-to-texture to a real-time interactive surface without changing the solver crate API — only the Tauri shell integration changes.

---

### [T-RENDER-VIEWPORT.sources] Sources

- https://github.com/ManevilleF/bevy_silk — bevy_silk Bevy 0.17 cloth ECS plugin; Verlet + stick constraints; no export; visual pattern reference
- https://bevy.org/news/bevy-0-18/ — Bevy 0.18 release notes: EasyScreenshotPlugin, FreeCamera, PanCamera, RenderTarget::Image headless capture
- https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs — Bevy headless renderer example: ImageCopyDriver, RenderTarget::Image, map_async readback pattern
- https://sotrh.github.io/learn-wgpu/showcase/windowless/ — Learn wgpu windowless/headless rendering: copy_texture_to_buffer pattern, staging buffer MAP_READ
- https://docs.rs/egui-wgpu — egui-wgpu v0.34.3: CallbackTrait, wgpu render loop integration, screenshot capability
- https://github.com/emilk/egui — egui repository: immediate mode GUI, winit+wgpu backend, live debug panels
- https://github.com/gfx-rs/wgpu — wgpu v29.0.3: cross-platform GPU compute, WGSL compute pipelines, texture readback
- https://github.com/mxpv/openusd — openusd v0.5.0 (June 8, 2026): pure-Rust USD read+write, UsdGeom Mesh, UsdPhysics, authoring API
- https://docs.rs/gltf/latest/gltf/ — gltf crate v1.4.1: read-only glTF 2.0 loader; confirms write support absent
- https://github.com/elrnv/gltfgen — gltfgen: VTK mesh-sequence to glTF; v0.2.0 (2019); dormant; not suitable as dependency
- https://github.com/Traverse-Research/ogawa-rs — ogawa-rs: Rust Alembic Ogawa reader; read-only; no write support confirmed
- https://crates.io/crates/alembic — alembic crate: placeholder only
- https://github.com/ennis/alembic-rs — ennis/alembic-rs: WIP Rust bindings to Alembic C++ library; not production-ready
- https://docs.rs/obj-rs/ — obj-rs: Wavefront OBJ read/write; basis for OBJ sequence export
- https://github.com/tauri-apps/tauri/discussions/11944 — Tauri: render wgpu frames as webview overlay; rendered-to-texture approach confirmed; qwook steam-overlay plugin for native surface (macOS only)
- https://github.com/tauri-apps/tauri/issues/8246 — Tauri: WebView on top of native GPU content feature request (unresolved as of 2025)
- https://github.com/tauri-apps/tauri/issues/9220 — Tauri: wgpu + WebView2 + transparency flickering bug
- https://github.com/GraphiteEditor/Graphite/issues/2541 — Graphite: wgpu viewport beneath Tauri WebView; blocked on Wayland + Windows compositor issues
- https://github.com/dceddia/wgpu-tauri-experiment — wgpu-tauri-experiment: non-functional attempt; confirms surface competition issue
- https://arxiv.org/html/2507.11794 — Real-Time Cloth Simulation Using WebGPU (2025): wireframe + texture render mode pair for cloth debug visualization; WebGPU handles 640K nodes at 60fps
- https://www.cgchannel.com/2026/04/clo-virtual-fashion-releases-marvelous-designer-2026-0/ — MD 2026.0: Alembic/FBX/USD export with morph animation; glTF/VRM import; UV filter; Toon shader
- https://www.versluis.com/2025/06/alembic-workflow-from-iclone-to-md-to-ue-via-blender/ — Alembic workflow 2025: iClone -> MD -> Blender -> UE; Alembic Mesh Sequence Cache modifier pattern
- https://artofmaking.substack.com/p/bringing-a-marvelous-designer-cloth — MD cloth to Unreal Engine: Alembic Geometry Cache track in UE5 Sequencer
- https://dev.epicgames.com/documentation/unreal-engine/machine-learning-cloth-simulation-overview — UE5 ML cloth simulation: USD physics parameters for cloth; Chaos Cloth import pipeline
- https://arewevfxyet.rs/ecosystem/caches/ — Are We VFX Yet: Alembic and OpenVDB crates both placeholder status
- https://arewevfxyet.rs/ecosystem/mesh/ — Are We VFX Yet: mesh tools crates; alembic and opensubdiv placeholders
- https://support.marvelousdesigner.com/hc/en-us/articles/47358199862553-Compatible-File-Format — MD compatible file formats: OBJ/FBX/Alembic/USD/glTF/VRM/MDD/PC2 full list
- https://support.marvelousdesigner.com/hc/en-us/articles/47358198817689--Mode-UV-EDITOR — MD UV Editor mode: UV islands = exact flattened 2D pattern pieces; automatic UV packing
- https://github.com/strayspark.studio/blog/bevy-rust-game-engine-2026-indie-guide — Bevy 0.18 released March 2026; editor preview; ECS scheduler improvements
- https://github.com/qwook/tauri-plugin-steam-overlay — tauri-plugin-steam-overlay: OSX wgpu-under-WebView native surface; Windows planned
- https://docs.rs/nannou/latest/nannou/wgpu/struct.TextureCapturer.html — Nannou TextureCapturer: GPU-to-CPU texture capture pattern reference
- https://gist.github.com/zicklag/b9c1be31ec599fd940379cecafa1751b — egui-wgpu custom rendering example: CallbackTrait for injecting wgpu draw calls inside egui Window
